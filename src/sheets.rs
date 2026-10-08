use anyhow::{Context, Result};
use google_sheets4::{Sheets, hyper_rustls, hyper_util, yup_oauth2};
use std::env;

pub(crate) type Hub =
    Sheets<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;
pub(crate) async fn connect() -> Result<Hub> {
    let key_path = env::var("GOOGLE_SERVICE_ACCOUNT_KEY")
        .context("Set GOOGLE_SERVICE_ACCOUNT_KEY to your service account JSON key path")?;
    let key = yup_oauth2::read_service_account_key(&key_path)
        .await
        .with_context(|| format!("Could not read service account key at {key_path}"))?;
    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(key)
        .build()
        .await
        .context("Could not initialize Google service account authentication")?;
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .context("Could not load native TLS roots")?
        .https_only()
        .enable_http2()
        .build();
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector);
    Ok(Sheets::new(client, auth))
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
        .with_context(|| format!("Could not read tab '{title}'"))?;
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

pub(crate) fn quote_tab(title: &str) -> String {
    format!("'{}'", title.replace('\'', "''"))
}
