use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bytes::Bytes;
use google_apis_common::GetToken;
use google_sheets4::hyper_rustls;
use http_body_util::{BodyExt, Full};
use hyper::{Request, StatusCode};
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
};
use url::Url;

pub(crate) const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const SERVICE: &str = "advent-calendar";
const USER: &str = "google-oauth-tokens";
type OAuthHttpClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

#[derive(Deserialize)]
struct OAuthConfig {
    client_id: String,
    auth_uri: String,
    token_uri: String,
    client_secret: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

#[derive(Clone)]
pub(crate) struct OAuthProvider {
    client_id: Arc<String>,
    client_secret: Option<Arc<String>>,
    token_uri: Arc<String>,
    client: OAuthHttpClient,
    tokens: Arc<Mutex<Tokens>>,
}

impl OAuthProvider {
    pub(crate) async fn connect() -> Result<Self> {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .context("Could not load native TLS roots")?
            .https_only()
            .enable_http2()
            .build();
        let client: OAuthHttpClient = Client::builder(TokioExecutor::new()).build(connector);
        let (raw, config_source) = if std::path::Path::new("credentials.json").exists() {
            (
                tokio::fs::read("credentials.json").await?,
                "credentials.json",
            )
        } else if std::path::Path::new("client_secret.json").exists() {
            (
                tokio::fs::read("client_secret.json").await?,
                "client_secret.json",
            )
        } else {
            (
                include_bytes!("../oauth_client.json").to_vec(),
                "bundled OAuth client",
            )
        };
        let value: serde_json::Value = serde_json::from_slice(&raw)
            .with_context(|| format!("Could not parse {config_source} OAuth configuration"))?;
        let details = value
            .get("installed")
            .or_else(|| value.get("web"))
            .unwrap_or(&value);
        let config: OAuthConfig = serde_json::from_value(details.clone())
            .with_context(|| format!("Could not parse {config_source} OAuth client fields"))?;
        anyhow::ensure!(
            config.client_secret.is_some(),
            "OAuth client configuration is missing client_secret"
        );
        let mut tokens = load_tokens().unwrap_or_else(|| Tokens {
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at: 0,
        });
        if tokens.refresh_token.is_empty() {
            tokens = authorize(&config, &client).await?;
            store_tokens(&tokens)?;
        }
        Ok(Self {
            client_id: Arc::new(config.client_id),
            client_secret: config.client_secret.map(Arc::new),
            token_uri: Arc::new(config.token_uri),
            client,
            tokens: Arc::new(Mutex::new(tokens)),
        })
    }

    async fn refresh(&self, tokens: &mut Tokens) -> Result<()> {
        let body = {
            let mut params = form_urlencoded::Serializer::new(String::new());
            params
                .append_pair("client_id", self.client_id.as_str())
                .append_pair("refresh_token", tokens.refresh_token.as_str())
                .append_pair("grant_type", "refresh_token");
            if let Some(secret) = &self.client_secret {
                params.append_pair("client_secret", secret.as_str());
            }
            params.finish()
        };
        let req = Request::post(self.token_uri.as_str())
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Full::new(Bytes::from(body)))?;
        let response = token_http(&self.client, req).await?;
        let token: TokenResponse = response;
        tokens.access_token = token.access_token;
        tokens.expires_at = unix_now()?.saturating_add(token.expires_in);
        store_tokens(tokens)?;
        Ok(())
    }
}

impl GetToken for OAuthProvider {
    fn get_token<'a>(
        &'a self,
        _scopes: &'a [&str],
    ) -> Pin<
        Box<
            dyn Future<
                    Output = std::result::Result<
                        Option<String>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let mut tokens = self.tokens.lock().await;
            if tokens.expires_at <= unix_now()?.saturating_add(60) {
                self.refresh(&mut tokens).await?;
            }
            Ok(Some(tokens.access_token.clone()))
        })
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

async fn authorize(config: &OAuthConfig, client: &OAuthHttpClient) -> Result<Tokens> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("Could not start OAuth callback listener")?;
    let redirect = format!(
        "http://{}:{}",
        listener.local_addr()?.ip(),
        listener.local_addr()?.port()
    );
    let verifier = random_urlsafe(64)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_urlsafe(32)?;
    let mut auth_url = Url::parse(&config.auth_uri).context("Invalid OAuth authorization URL")?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &redirect)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPE)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    open_browser(auth_url.as_str()).context("Could not open the system browser")?;
    println!("Complete Google sign-in in your browser; waiting for the localhost callback...");
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(120), listener.accept())
        .await
        .context("Google sign-in timed out after 2 minutes; run the command again to retry")?
        .context("OAuth callback failed")?;
    let mut request = vec![0; 8192];
    let count = socket
        .read(&mut request)
        .await
        .context("Could not read OAuth callback")?;
    let request = String::from_utf8_lossy(&request[..count]);
    let callback = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .context("Malformed OAuth callback")?;
    let callback_url = Url::parse(&format!("http://localhost{callback}"))?;
    let query: std::collections::HashMap<_, _> = callback_url.query_pairs().into_owned().collect();
    let response_page = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nGoogle sign-in complete. You can close this tab.";
    socket
        .write_all(response_page)
        .await
        .context("Could not finish OAuth callback")?;
    if query.get("state").map(String::as_str) != Some(state.as_str()) {
        bail!("OAuth state did not match");
    }
    if let Some(error) = query.get("error") {
        bail!("Google authorization failed: {error}");
    }
    let code = query
        .get("code")
        .context("OAuth callback did not include an authorization code")?;
    let mut params = form_urlencoded::Serializer::new(String::new());
    params
        .append_pair("client_id", config.client_id.as_str())
        .append_pair("code", code.as_str())
        .append_pair("code_verifier", verifier.as_str())
        .append_pair("grant_type", "authorization_code")
        .append_pair("redirect_uri", redirect.as_str());
    if let Some(secret) = &config.client_secret {
        params.append_pair("client_secret", secret);
    }
    let body = params.finish();
    let req = Request::post(config.token_uri.as_str())
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Full::new(Bytes::from(body)))?;
    let response: TokenResponse = token_http(client, req).await?;
    Ok(Tokens {
        access_token: response.access_token,
        refresh_token: response
            .refresh_token
            .context("Google did not return a refresh token; retry authorization")?,
        expires_at: unix_now()?.saturating_add(response.expires_in),
    })
}

async fn token_http(
    client: &OAuthHttpClient,
    request: Request<Full<Bytes>>,
) -> Result<TokenResponse> {
    let response = client
        .request(request)
        .await
        .context("OAuth token request failed")?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    if status != StatusCode::OK {
        #[derive(Deserialize)]
        struct OAuthError {
            error: Option<String>,
            error_description: Option<String>,
        }
        if let Ok(error) = serde_json::from_slice::<OAuthError>(&bytes) {
            let code = error.error.as_deref().unwrap_or("unknown OAuth error");
            if let Some(description) = error.error_description {
                bail!("OAuth token endpoint returned {status}: {code}: {description}");
            }
            bail!("OAuth token endpoint returned {status}: {code}");
        }
        bail!("OAuth token endpoint returned {status}");
    }
    serde_json::from_slice(&bytes).context("Could not parse OAuth token response")
}

fn random_urlsafe(length: usize) -> Result<String> {
    let mut bytes = vec![0; length];
    getrandom::getrandom(&mut bytes)
        .map_err(|error| anyhow::anyhow!("Could not generate OAuth PKCE randomness: {error}"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    command.spawn().map(|_| ())
}

fn load_tokens() -> Option<Tokens> {
    let entry = keyring::Entry::new(SERVICE, USER).ok()?;
    serde_json::from_str(&entry.get_password().ok()?).ok()
}

fn store_tokens(tokens: &Tokens) -> Result<()> {
    let entry =
        keyring::Entry::new(SERVICE, USER).context("Could not access OS credential store")?;
    entry
        .set_password(&serde_json::to_string(tokens)?)
        .context("Could not save OAuth tokens to OS credential store")
}

pub(crate) fn clear_tokens() -> Result<()> {
    let entry =
        keyring::Entry::new(SERVICE, USER).context("Could not access OS credential store")?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => {
            Err(error).context("Could not delete Google OAuth tokens from OS credential store")
        }
    }
}

fn unix_now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
