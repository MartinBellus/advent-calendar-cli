use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use yup_oauth2::storage::{TokenInfo, TokenStorage, TokenStorageError};

const SERVICE: &str = "advent-calendar";
const USER: &str = "google-oauth-tokens";

#[derive(Default, Deserialize, Serialize)]
struct StoredTokens(Vec<StoredToken>);

#[derive(Deserialize, Serialize)]
struct StoredToken {
    scopes: Vec<String>,
    token: TokenInfo,
}

struct KeyringTokenStore;

#[async_trait]
impl TokenStorage for KeyringTokenStore {
    async fn set(&self, scopes: &[&str], token: TokenInfo) -> Result<(), TokenStorageError> {
        let entry = keyring::Entry::new(SERVICE, USER).map_err(|error| {
            store_error(format!("Could not access the OS credential store: {error}"))
        })?;
        let mut stored = match entry.get_password() {
            Ok(value) => serde_json::from_str::<StoredTokens>(&value).map_err(|error| {
                store_error(format!("Could not read cached OAuth tokens: {error}"))
            })?,
            Err(keyring::Error::NoEntry) => StoredTokens::default(),
            Err(error) => {
                return Err(store_error(format!(
                    "Could not access cached OAuth tokens: {error}"
                )));
            }
        };
        let mut scopes: Vec<String> = scopes.iter().map(|scope| (*scope).to_owned()).collect();
        scopes.sort();
        scopes.dedup();
        if let Some(existing) = stored.0.iter_mut().find(|item| item.scopes == scopes) {
            existing.token = token;
        } else {
            stored.0.push(StoredToken { scopes, token });
        }
        let value = serde_json::to_string(&stored).map_err(|error| {
            store_error(format!("Could not encode cached OAuth tokens: {error}"))
        })?;
        entry.set_password(&value).map_err(|error| {
            store_error(format!(
                "Could not save OAuth tokens to the OS credential store: {error}"
            ))
        })
    }

    async fn get(&self, scopes: &[&str]) -> Option<TokenInfo> {
        let entry = keyring::Entry::new(SERVICE, USER).ok()?;
        let value = entry.get_password().ok()?;
        let stored: StoredTokens = serde_json::from_str(&value).ok()?;
        stored
            .0
            .into_iter()
            .find(|item| {
                scopes
                    .iter()
                    .all(|scope| item.scopes.iter().any(|item_scope| item_scope == scope))
            })
            .map(|item| item.token)
    }
}

pub(crate) fn oauth_storage() -> Box<dyn TokenStorage> {
    Box::new(KeyringTokenStore)
}

fn store_error(message: String) -> TokenStorageError {
    TokenStorageError::Other(Cow::Owned(message))
}
