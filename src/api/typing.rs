use std::{sync::Arc, time::Duration};

use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::sync::RwLock;

use crate::{api::typing::auth::Auth, settings::Settings};

mod auth;

const BASE_URL: &str = "https://api.typing.com/teachers";

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
        let mut headers = HeaderMap::new();
        headers.insert("X-App-Site", HeaderValue::from_static("typing"));
        let client = Client::builder().default_headers(headers).build()?;

        let username = settings
            .typing_username
            .clone()
            .ok_or(Error::UsernameNotSet)?;
        let selected_class = settings
            .typing_class
            .clone()
            .ok_or(Error::SelectedClassNotSet)?;
        let response_data: Value = client
            .post(format!("{BASE_URL}/auth/find"))
            .json(&json!({
                "username": username
            }))
            .send()
            .await?
            .json()
            .await?;
        let teacher_id = response_data.require("/users/0/teacher_id", Value::as_u64)?;

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
        let username = self.teacher_username().await;
        let teacher_id = self.teacher_id().await;

        // delegate to the auth module. it will fetch and cache/refresh the token as needed
        Ok(self.auth.token(&self.client, &username, teacher_id).await?)
    }

    async fn teacher_id(&self) -> TeacherId {
        self.state.read().await.teacher.id
    }

    async fn teacher_username(&self) -> String {
        self.state.read().await.teacher.username.clone()
    }

    async fn class_id(&self) -> ClassId {
        self.state.read().await.class.id
    }

    pub async fn get_classes(&self) -> Result<Vec<Result<TypingClass>>> {
        let teacher_id = self.teacher_id().await;
        let token = self.auth_token().await?;
        let response_data: Value = self
            .client
            .get(format!("{BASE_URL}/teachers/{teacher_id}/classes"))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;
        let class_data = response_data.require("/data", Value::as_array)?;
        Ok(class_data
            .iter()
            .map(|val| {
                let id = val.require("/id", Value::as_u64)?;
                let name = val.require("/name", Value::as_str)?.to_owned();
                Ok(TypingClass { id, name })
            })
            .collect())
    }

    pub async fn get_students(&self) -> Result<Vec<Result<Student>>> {
        let class_id = self.class_id().await;
        let token = self.auth_token().await?;
        let response_data: Value = self
            .client
            .get(format!("{BASE_URL}/sections/{class_id}?include=users"))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;
        let student_data = response_data.require("/data/students", Value::as_array)?;
        Ok(student_data
            .iter()
            .map(|val| {
                let (first_name, last_name) = parse_student_name(val)?;
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
        let class_id = self.class_id().await;
        let teacher_id = self.teacher_id().await;
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
        let student_data = response_data.require("/data", Value::as_array)?;
        Ok(student_data
            .iter()
            .map(|val| {
                let (first_name, last_name) = parse_student_name(val)?;
                let time = val
                    .require("/time", Value::as_str)?
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

trait JsonFieldExt {
    fn require<'a, T>(
        &'a self,
        pointer: &str,
        extract: impl FnOnce(&'a Value) -> Option<T>,
    ) -> Result<T>;
}

impl JsonFieldExt for Value {
    fn require<'a, T>(
        &'a self,
        pointer: &str,
        extract: impl FnOnce(&'a Value) -> Option<T>,
    ) -> Result<T> {
        self.pointer(pointer)
            .and_then(extract)
            .ok_or_else(|| Error::MissingJsonField {
                field: pointer.to_owned(),
                json: self.to_string(),
            })
    }
}

fn parse_student_name(val: &Value) -> Result<(String, String)> {
    let first_name = val.require("/first_name", Value::as_str)?.to_owned();
    let last_name = val.require("/last_name", Value::as_str)?.to_owned();
    Ok((first_name, last_name))
}
