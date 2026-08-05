use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use reqwest::Client;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock};

use super::BASE_URL;
use crate::secrets::{self, SecretService};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Secret(#[from] secrets::Error),

    #[error("missing `{field}` in response json: {json}")]
    MissingJsonField { field: String, json: String },
}

struct Token {
    value: String,
    expiration: SystemTime,
}

// manual impl so the bearer token never gets printed if a `Session`/`Auth`/`Token`
// ends up in a `{:?}`/`tracing` log line (e.g. `debug!(?session, ...)`)
impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token")
            .field("value", &"<redacted>")
            .field("expiration", &self.expiration)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Auth {
    token: Arc<RwLock<Token>>,
    // serializes refreshes so concurrent callers don't all hit the login endpoint at once
    refresh_lock: Arc<Mutex<()>>,
}

impl Auth {
    pub fn new() -> Self {
        Self {
            token: Arc::new(RwLock::new(Token {
                value: String::new(),
                // set to epoch so first access with refresh
                expiration: UNIX_EPOCH,
            })),
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Return a valid bearer token. If the cached token is expired (or empty)
    /// this will fetch a fresh one from the Typing.com auth endpoint.
    pub async fn token(&self, client: &Client, username: &str, teacher_id: u64) -> Result<String> {
        if let Some(value) = self.fresh_token().await {
            return Ok(value);
        }

        // token was not fresh; only one caller should refresh at a time
        let _guard = self.refresh_lock.lock().await;

        // another caller may have refreshed while we waited for the lock
        if let Some(value) = self.fresh_token().await {
            return Ok(value);
        }

        let password = secrets::load_password(SecretService::TypingCom, username)?;
        let response_data: Value = client
            .post(format!("{BASE_URL}/auth/login"))
            .json(&json!({
                "teacher_id": teacher_id,
                "password": password,
                "login_type": "username"
            }))
            .send()
            .await?
            .json()
            .await?;
        let access_token = response_data
            .pointer("/data/access_token")
            .and_then(|val| val.as_str())
            .ok_or(Error::MissingJsonField {
                field: "access_token".to_owned(),
                json: response_data.to_string(),
            })?;
        let mut expiration = get_jwt_expiration(access_token)
            // fallback of 1 hour
            .unwrap_or_else(|| SystemTime::now() + Duration::from_hours(1));
        // apply a small safety margin so we refresh a little early
        if let Some(adjustment) = expiration.checked_sub(Duration::from_mins(1)) {
            expiration = adjustment;
        }

        let mut token = self.token.write().await;
        token.value = access_token.to_owned();
        token.expiration = expiration;
        Ok(token.value.clone())
    }

    /// Return the cached token if it's still fresh, without attempting a refresh.
    async fn fresh_token(&self) -> Option<String> {
        let token = self.token.read().await;
        (SystemTime::now() < token.expiration).then(|| token.value.clone())
    }
}

fn get_jwt_expiration(jwt: &str) -> Option<SystemTime> {
    let payload_b64 = jwt.split('.').nth(1)?;
    let decoded = BASE64_URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let exp = serde_json::from_slice::<Value>(&decoded)
        .ok()?
        .get("exp")?
        .as_u64()?;
    UNIX_EPOCH.checked_add(Duration::from_secs(exp))
}
