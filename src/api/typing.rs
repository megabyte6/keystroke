use std::{sync::Arc, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};

use crate::{
    secrets::{self, SecretService},
    settings::Settings,
};

type TeacherId = u64;
type ClassId = u64;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Secret(#[from] secrets::Error),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("missing `{field}` in response json: {json}")]
    MissingJsonField { field: String, json: String },
    #[error("no username found for typing.com")]
    UsernameNotSet,
    #[error("no selected class found for typing.com")]
    SelectedClassNotSet,
}

#[derive(Debug, Clone)]
pub struct Session {
    client: Client,
    state: Arc<RwLock<SessionState>>,
    auth: Arc<AuthState>,
}

impl Session {
    pub async fn login(settings: &Settings) -> Result<Session> {
        let client = Client::new();

        let username = settings
            .typing_username
            .clone()
            .ok_or(Error::UsernameNotSet)?;
        let selected_class = settings
            .typing_class
            .clone()
            .ok_or(Error::SelectedClassNotSet)?;
        let response_data: Value = client
            .post("https://api.typing.com/teachers/auth/find")
            .header("X-App-Site", "typing")
            .json(&json!({
                "username": username
            }))
            .send()
            .await?
            .json()
            .await?;
        let teacher_id = response_data
            .pointer("/users/0/teacher_id")
            .and_then(|val| val.as_u64())
            .ok_or(Error::MissingJsonField {
                field: "teacher_id".to_owned(),
                json: response_data.to_string(),
            })?;

        let session = Session {
            client,
            state: Arc::new(RwLock::new(SessionState {
                teacher: Teacher {
                    id: teacher_id,
                    username: username.clone(),
                },
                class: selected_class,
            })),
            auth: Arc::new(AuthState {
                // empty for now as `refresh_access_token()` will set one
                token: RwLock::new(String::new()),
                refresh_lock: Mutex::new(()),
            }),
        };
        session.refresh_access_token().await?;

        Ok(session)
    }

    pub async fn auth_token(&self) -> Result<String> {
        let token = self.auth.token.read().await.clone();
        if !token.is_empty() {
            return Ok(token);
        }

        self.refresh_access_token().await?;
        Ok(self.auth.token.read().await.clone())
    }

    pub async fn refresh_access_token(&self) -> Result<()> {
        let snapshot = self.auth.token.read().await.clone();
        let _guard = self.auth.refresh_lock.lock().await;

        // if another task already refreshed, skip refreshing
        let current = self.auth.token.read().await.clone();
        if current != snapshot {
            return Ok(());
        }

        let username = &self.state.read().await.teacher.username;
        let password = secrets::load_password(SecretService::TypingCom, username)?;
        let response_data: Value = self
            .client
            .post("https://api.typing.com/teachers/auth/login")
            .header("X-App-Site", "typing")
            .json(&json!({
                "teacher_id": self.state.read().await.teacher.id,
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
        *self.auth.token.write().await = access_token.to_owned();

        Ok(())
    }

    pub async fn get_classes(&self) -> Result<Vec<TypingClass>> {
        let response_data: Value = self
            .client
            .get(format!(
                "https://api.typing.com/teachers/teachers/{}/classes",
                self.state.read().await.teacher.id
            ))
            .bearer_auth(self.auth_token().await?)
            .header("X-App-Site", "typing")
            .send()
            .await?
            .json()
            .await?;
        response_data
            .pointer("/data")
            .and_then(|val| val.as_array())
            .ok_or(Error::MissingJsonField {
                field: "/data".to_owned(),
                json: response_data.to_string(),
            })?
            .iter()
            .map(|val| {
                let id = val
                    .get("id")
                    .and_then(|v| v.as_u64())
                    .ok_or(Error::MissingJsonField {
                        field: "id".to_owned(),
                        json: response_data.to_string(),
                    })?;
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or(Error::MissingJsonField {
                        field: "name".to_owned(),
                        json: response_data.to_string(),
                    })?
                    .to_owned();
                Ok(TypingClass { id, name })
            })
            .collect()
    }

    pub async fn get_students(&self) -> Result<Vec<Student>> {
        let now = OffsetDateTime::now_utc() - Duration::from_hours(9);
        let start = now - Duration::from_hours(1);
        let response_data: Value = self
            .client
            .post("https://www.typing.com/apiv1/teacher/reports/run")
            .bearer_auth(self.auth_token().await?)
            .json(&json!({
                "start": start.unix_timestamp(),
                "end": now.unix_timestamp(),
                "sections": [
                    self.state.read().await.class.id
                ],
                "report": "activity",
                "teacher_id": self.state.read().await.teacher.id
            }))
            .send()
            .await?
            .json()
            .await?;
        response_data
            .pointer("/data")
            .and_then(|val| val.as_array())
            .ok_or(Error::MissingJsonField {
                field: "/data".to_owned(),
                json: response_data.to_string(),
            })?
            .iter()
            .map(|val| {
                let first_name = val
                    .get("first_name")
                    .and_then(|v| v.as_str())
                    .ok_or(Error::MissingJsonField {
                        field: "first_name".to_owned(),
                        json: response_data.to_string(),
                    })?
                    .to_owned();
                let last_name = val
                    .get("last_name")
                    .and_then(|val| val.as_str())
                    .ok_or(Error::MissingJsonField {
                        field: "last_name".to_owned(),
                        json: response_data.to_string(),
                    })?
                    .to_owned();
                let time = val
                    .get("time")
                    .and_then(|v| v.as_str())
                    .ok_or(Error::MissingJsonField {
                        field: "time".to_owned(),
                        json: response_data.to_string(),
                    })?
                    .parse::<u64>()
                    .map(Duration::from_secs)?;
                Ok(Student {
                    first_name,
                    last_name,
                    time,
                })
            })
            .collect()
    }
}

#[derive(Debug)]
struct SessionState {
    teacher: Teacher,
    class: TypingClass,
}

#[derive(Debug, Clone)]
struct Teacher {
    id: TeacherId,
    username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypingClass {
    id: ClassId,
    name: String,
}

#[derive(Debug)]
struct AuthState {
    token: RwLock<String>,
    refresh_lock: Mutex<()>,
}

#[derive(Debug)]
pub struct Student {
    first_name: String,
    last_name: String,
    time: Duration,
}
