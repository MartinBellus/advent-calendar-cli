use crate::config::AppConfig;
use anyhow::{Context, Result};
use google_sheets4::{Sheets, hyper_rustls, hyper_util, yup_oauth2};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

pub(crate) type Hub =
    Sheets<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;
pub(crate) async fn connect(config: &AppConfig) -> Result<Hub> {
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .context("Could not load native TLS roots")?
        .https_only()
        .enable_http2()
        .build();
    let client = Client::builder(TokioExecutor::new()).build(connector);
    if let Some(key_path) = config.service_account_key() {
        let key = yup_oauth2::read_service_account_key(&key_path)
            .await
            .with_context(|| {
                format!(
                    "Could not read service account key at {}",
                    key_path.display()
                )
            })?;
        let auth = yup_oauth2::ServiceAccountAuthenticator::builder(key)
            .build()
            .await
            .context("Could not initialize Google service account authentication")?;
        Ok(Sheets::new(client, auth))
    } else {
        let auth = crate::oauth::OAuthProvider::connect().await?;
        Ok(Sheets::new(client, auth))
    }
}

pub(crate) async fn read_values(
    hub: &Hub,
    spreadsheet_id: &str,
    title: &str,
) -> Result<Vec<Vec<String>>> {
    let range = format!("{}!A1:K1000", quote_tab(title));
    let (_, response) = hub
        .spreadsheets()
        .values_get(spreadsheet_id, &range)
        .value_render_option("FORMATTED_VALUE")
        .doit()
        .await
        .map_err(|error| {
            let context = if is_not_found(&error) {
                format!(
                    "Tab '{title}' was not found. Create it with `advent-calendar month-init` or check the spreadsheet ID"
                )
            } else {
                format!("Could not read tab '{title}'")
            };
            anyhow::Error::new(error).context(context)
        })?;
    Ok(response
        .values
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| v.to_string())
                })
                .collect()
        })
        .collect())
}

pub(crate) fn is_not_found(error: &google_sheets4::Error) -> bool {
    match error {
        google_sheets4::Error::BadRequest(body) => {
            body.pointer("/error/code")
                .and_then(serde_json::Value::as_i64)
                == Some(404)
        }
        google_sheets4::Error::Failure(response) => {
            response.status() == hyper::StatusCode::NOT_FOUND
        }
        _ => false,
    }
}

pub(crate) fn quote_tab(title: &str) -> String {
    format!("'{}'", title.replace('\'', "''"))
}
