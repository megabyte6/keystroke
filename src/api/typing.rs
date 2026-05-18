use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde_json::{Value, json};

use crate::{
    AppContext,
    secrets::{self, SecretService},
};

#[derive(Debug, Clone, Default)]
pub struct Session {
    auth_token: String,
}

pub async fn login(ctx: &AppContext) -> Result<Session> {
    let username = ctx
        .settings
        .typing_username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("typing.com username is not set"))?;
    let password =
        secrets::load_password(SecretService::TypingCom, username).with_context(|| {
            format!("no password stored in keyring for typing.com user '{username}'")
        })?;

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

    let json: Value = client
        .post("https://api.typing.com/teachers/auth/login")
        .header("X-App-Site", "typing")
        .json(&json!({
            "teacher_id": teacher_id,
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

    Ok(Session {
        auth_token: access_token.to_string(),
    })
}
