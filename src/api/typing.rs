use anyhow::Result;
use reqwest::Client;
use serde_json::json;

use crate::AppContext;

#[derive(Debug, Clone, Default)]
pub struct Session {}

pub async fn login(ctx: &AppContext) -> Result<Session> {
    get_teacher_id(ctx).await?;

    Ok(Session {})
}

async fn get_teacher_id(ctx: &AppContext) -> Result<u64> {
    let client = Client::new();

    struct Login {
        login_type: String,
        teacher_id: String,
        username: String,
    }
    let response = client
        .post("https://api.typing.com/teachers/auth/find")
        .json(&json!({
            "login_type": "",
            "teacher_id": "",
            "username": ctx.settings.typing_username
        }))
        .send()
        .await?;

    let status = response.status();
    let body: serde_json::Value = response.json().await?;
    let pretty = serde_json::to_string_pretty(&body)?;
    println!("status={status}, body={pretty}");

    Ok(0)
}
