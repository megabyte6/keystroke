use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock};

use crate::secrets::{self, SecretService};

#[derive(Debug, Clone)]
pub struct Session {
    client: Client,
    auth_state: Arc<AuthState>,
    teacher_id: u64,
}

#[derive(Debug)]
struct AuthState {
    token: RwLock<String>,
    refresh_lock: Mutex<()>,
}

impl Session {
    pub async fn login(username: &str) -> Result<Session> {
        let client = Client::new();
        let json: Value = client
            .post("https://api.typing.com/teachers/auth/find")
            .header("X-App-Site", "typing")
            .json(&json!({
                "username": username
            }))
            .send()
            .await?
            .json()
            .await?;
        let teacher_id = json
            .pointer("/users/0/teacher_id")
            .and_then(|val| val.as_u64())
            .ok_or_else(|| {
                let body =
                    serde_json::to_string(&json).unwrap_or_else(|_| "<invalid json>".to_string());
                anyhow!("missing `teacher_id` in response JSON: {body}")
            })?;

        let session = Session {
            client,
            auth_state: Arc::new(AuthState {
                // empty for now as `refresh_access_token()` will set one
                token: RwLock::new(String::new()),
                refresh_lock: Mutex::new(()),
            }),
            teacher_id,
        };
        session.refresh_access_token(username).await?;

        Ok(session)
    }

    pub async fn auth_token(&self, username: &str) -> Result<String> {
        let token = self.auth_state.token.read().await.clone();
        if !token.is_empty() {
            return Ok(token);
        }

        self.refresh_access_token(username).await?;
        Ok(self.auth_state.token.read().await.clone())
    }

    pub async fn refresh_access_token(&self, username: &str) -> Result<()> {
        let snapshot = self.auth_state.token.read().await.clone();
        let _guard = self.auth_state.refresh_lock.lock().await;

        // if another task already refreshed, skip refreshing
        let current = self.auth_state.token.read().await.clone();
        if current != snapshot {
            return Ok(());
        }

        let password =
            secrets::load_password(SecretService::TypingCom, username).with_context(|| {
                format!("no password stored in keyring for typing.com user '{username}'")
            })?;
        let json: Value = self
            .client
            .post("https://api.typing.com/teachers/auth/login")
            .header("X-App-Site", "typing")
            .json(&json!({
                "teacher_id": self.teacher_id,
                "password": password,
                "login_type": "username"
            }))
            .send()
            .await?
            .json()
            .await?;
        let access_token = json
            .pointer("/data/access_token")
            .and_then(|val| val.as_str())
            .ok_or_else(|| {
                let body =
                    serde_json::to_string(&json).unwrap_or_else(|_| "<invalid json>".to_string());
                anyhow!("missing `access_token` in response JSON: {body}")
            })?;
        *self.auth_state.token.write().await = access_token.to_string();

        Ok(())
    }

    pub async fn get_classes(&self) -> Vec<String> {
        Vec::new()
    }
}
