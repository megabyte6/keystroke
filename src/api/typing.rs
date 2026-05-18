use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{Value, json};

use crate::AppContext;

#[derive(Debug, Clone, Default)]
pub struct Session {}

pub async fn login(ctx: &AppContext) -> Result<Session> {
    get_teacher_id(ctx).await?;

    Ok(Session {})
}

async fn get_teacher_id(ctx: &AppContext) -> Result<u64> {
    let client = Client::new();
    let response = client
        .post("https://api.typing.com/teachers/auth/find")
        .header("X-App-Site", "typing")
        .json(&json!({
            "login_type": "",
            "teacher_id": "",
            "username": ctx.settings.typing_username
        }))
        .send()
        .await?;
    let json: Value = response.json().await?;

    let id = json
        .pointer("/users/0/teacher_id")
        .and_then(|val| val.as_u64())
        .ok_or_else(|| {
            let body =
                serde_json::to_string(&json).unwrap_or_else(|_| "<invalid json>".to_string());
            anyhow!("missing `teacher_id` in response JSON: {}", body)
        })?;
    Ok(id)
}
