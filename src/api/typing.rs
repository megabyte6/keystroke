use std::{sync::Arc, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::sync::RwLock;

use crate::{api::typing::auth::Auth, settings::Settings};

mod auth;

type TeacherId = u64;
type ClassId = u64;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Auth(#[from] auth::Error),

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
    auth: Auth,
}

#[derive(Debug)]
struct SessionState {
    teacher: Teacher,
    class: TypingClass,
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
            auth: Auth::new(),
        };

        Ok(session)
    }

    pub async fn auth_token(&self) -> Result<String> {
        let (username, teacher_id) = {
            let state = self.state.read().await;
            (state.teacher.username.clone(), state.teacher.id)
        };

        // delegate to the auth module. it will fetch and cache/refresh the token as needed
        Ok(self.auth.token(&self.client, &username, teacher_id).await?)
    }

    pub async fn get_classes(&self) -> Result<Vec<Result<TypingClass>>> {
        let teacher_id = self.state.read().await.teacher.id;
        let token = self.auth_token().await?;
        let response_data: Value = self
            .client
            .get(format!(
                "https://api.typing.com/teachers/teachers/{teacher_id}/classes"
            ))
            .bearer_auth(token)
            .header("X-App-Site", "typing")
            .send()
            .await?
            .json()
            .await?;
        let class_data = response_data
            .pointer("/data")
            .and_then(|val| val.as_array())
            .ok_or(Error::MissingJsonField {
                field: "/data".to_owned(),
                json: response_data.to_string(),
            })?;
        Ok(class_data
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
            .collect())
    }

    pub async fn get_students(&self) -> Result<Vec<Result<Student>>> {
        let class_id = self.state.read().await.class.id;
        let token = self.auth_token().await?;
        let response_data: Value = self
            .client
            .get(format!(
                "https://api.typing.com/teachers/sections/{class_id}?include=users"
            ))
            .bearer_auth(token)
            .header("X-App-Site", "typing")
            .send()
            .await?
            .json()
            .await?;
        let student_data = response_data
            .pointer("/data/students")
            .and_then(|val| val.as_array())
            .ok_or(Error::MissingJsonField {
                field: "/data/students".to_owned(),
                json: response_data.to_string(),
            })?;
        Ok(student_data
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
                    .and_then(|v| v.as_str())
                    .ok_or(Error::MissingJsonField {
                        field: "last_name".to_owned(),
                        json: response_data.to_string(),
                    })?
                    .to_owned();
                Ok(Student {
                    first_name,
                    last_name,
                    time: None,
                })
            })
            .collect())
    }

    pub async fn get_student_activity(&self) -> Result<Vec<Result<Student>>> {
        let token = self.auth_token().await?;
        let now = OffsetDateTime::now_utc();
        let start = now - Duration::from_hours(1);
        let (class_id, teacher_id) = {
            let state = self.state.read().await;
            (state.class.id, state.teacher.id)
        };
        let response_data: Value = self
            .client
            .post("https://www.typing.com/apiv1/teacher/reports/run")
            .bearer_auth(token)
            .json(&json!({
                "start": start.unix_timestamp(),
                "end": now.unix_timestamp(),
                "sections": [
                    class_id
                ],
                "report": "activity",
                "teacher_id": teacher_id
            }))
            .send()
            .await?
            .json()
            .await?;
        let student_data = response_data
            .pointer("/data")
            .and_then(|val| val.as_array())
            .ok_or(Error::MissingJsonField {
                field: "/data".to_owned(),
                json: response_data.to_string(),
            })?;
        Ok(student_data
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
                    time: Some(time),
                })
            })
            .collect())
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Student {
    first_name: String,
    last_name: String,
    time: Option<Duration>,
}
