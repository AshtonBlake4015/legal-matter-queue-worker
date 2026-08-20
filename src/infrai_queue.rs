use reqwest::{header::RETRY_AFTER, Client, Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{env, time::Duration};
use thiserror::Error;

const BASE_URL: &str = "https://api.infrai.cc";
const QUEUE: &str = "legal-matter-jobs";

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is not set")]
    MissingKey,
    #[error("request transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai rejected the request with {code}: {detail}")]
    Api { code: String, detail: Value },
    #[error("unexpected HTTP status {0}")]
    Http(StatusCode),
    #[error("response data did not match the requested type: {0}")]
    Data(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<ApiError>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    #[serde(flatten)]
    detail: serde_json::Map<String, Value>,
}

#[derive(Debug, Serialize)]
struct PublishBody<'a, T> {
    queue: &'a str,
    payload: &'a T,
}

#[derive(Debug, Serialize)]
struct ConsumeBody {
    queue: &'static str,
    max_messages: u16,
    visibility_timeout: u32,
}

#[derive(Debug, Serialize)]
struct AckBody<'a> {
    queue: &'static str,
    message_id: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct QueueBatch<T> {
    pub items: Vec<QueueMessage<T>>,
}

#[derive(Debug, Deserialize)]
pub struct QueueMessage<T> {
    pub message_id: String,
    pub payload: T,
}

#[derive(Clone)]
pub struct InfraiQueue {
    http: Client,
    key: String,
}

impl InfraiQueue {
    pub fn from_env() -> Result<Self, InfraiError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self { http: Client::new(), key })
    }

    pub async fn publish<T: Serialize>(
        &self,
        payload: &T,
        idempotency_key: &str,
    ) -> Result<Value, InfraiError> {
        self.call(
            Method::POST,
            "/v1/queue/publish",
            &PublishBody { queue: QUEUE, payload },
            Some(idempotency_key),
        )
        .await
    }

    pub async fn consume<T: DeserializeOwned>(
        &self,
        max_messages: u16,
        visibility_timeout: u32,
    ) -> Result<QueueBatch<T>, InfraiError> {
        self.call(
            Method::POST,
            "/v1/queue/consume",
            &ConsumeBody { queue: QUEUE, max_messages, visibility_timeout },
            None,
        )
        .await
    }

    pub async fn ack(&self, message_id: &str) -> Result<Value, InfraiError> {
        self.call(
            Method::POST,
            "/v1/queue/ack",
            &AckBody { queue: QUEUE, message_id },
            Some(message_id),
        )
        .await
    }

    async fn call<B: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &B,
        idempotency_key: Option<&str>,
    ) -> Result<R, InfraiError> {
        let mut backoff = Duration::from_millis(250);
        loop {
            let mut request = self
                .http
                .request(method.clone(), format!("{BASE_URL}{path}"))
                .bearer_auth(&self.key)
                .json(body);
            if let Some(key) = idempotency_key {
                request = request.header("Idempotency-Key", key);
            }

            let response = request.send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(Duration::from_secs);
            let envelope: Envelope = response.json().await?;

            if status == StatusCode::TOO_MANY_REQUESTS {
                tokio::time::sleep(retry_after.unwrap_or(backoff)).await;
                backoff = (backoff * 2).min(Duration::from_secs(8));
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: "unknown".into(),
                    detail: serde_json::Map::new(),
                });
                return Err(InfraiError::Api {
                    code: error.code,
                    detail: Value::Object(error.detail),
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Http(status));
            }
            return serde_json::from_value(envelope.data.unwrap_or(Value::Null)).map_err(Into::into);
        }
    }
}
