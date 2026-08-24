use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};
use thiserror::Error;

pub mod download;
pub mod prompt;
pub mod workflow;

pub use workflow::WorkflowReplacer;

#[derive(Debug, Error)]
pub enum ComfyuiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
}

pub struct ComfyuiClient {
    base_url: String,
    http: Client,
}

impl ComfyuiClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, ComfyuiError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn http(&self) -> &Client {
        &self.http
    }

    pub async fn system_stats(&self) -> Result<Value, ComfyuiError> {
        let url = format!("{}/system_stats", self.base_url);
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(ComfyuiError::InvalidResponse(format!(
                "system_stats HTTP {}",
                resp.status()
            )));
        }
        Ok(resp.json().await?)
    }
}

pub fn make_client_id() -> String {
    format!("auto-comfy-maker-{}-{}", std::process::id(), chrono::Utc::now().timestamp_millis())
}

pub fn build_submit_body(workflow: &Value, client_id: &str) -> Value {
    json!({
        "prompt": workflow,
        "client_id": client_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn system_stats_returns_json() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/system_stats"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "system": { "os": "linux" },
                "devices": []
            })))
            .mount(&server)
            .await;

        let client = ComfyuiClient::new(server.uri()).unwrap();
        let v = client.system_stats().await.unwrap();
        assert_eq!(v["system"]["os"], "linux");
    }

    #[tokio::test]
    async fn system_stats_handles_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/system_stats"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = ComfyuiClient::new(server.uri()).unwrap();
        let r = client.system_stats().await;
        assert!(matches!(r, Err(ComfyuiError::InvalidResponse(_))));
    }

    #[test]
    fn client_id_is_unique() {
        let a = make_client_id();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = make_client_id();
        assert_ne!(a, b);
        assert!(a.starts_with("auto-comfy-maker-"));
    }

    #[test]
    fn submit_body_structure() {
        let wf = json!({"6": {"inputs": {"text": "x"}}});
        let body = build_submit_body(&wf, "client-1");
        assert_eq!(body["client_id"], "client-1");
        assert_eq!(body["prompt"], wf);
    }
}
