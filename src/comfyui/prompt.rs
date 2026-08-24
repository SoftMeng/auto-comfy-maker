use std::time::Duration;

use serde_json::{json, Value};
use thiserror::Error;
use tokio::time::sleep;

use super::{build_submit_body, ComfyuiClient, ComfyuiError};

#[derive(Debug, Error)]
pub enum SubmitError {
    #[error("comfyui: {0}")]
    Comfyui(#[from] ComfyuiError),
    #[error("missing prompt_id in response: {0}")]
    MissingPromptId(serde_json::Value),
    #[error("poll timeout after {0:?}")]
    PollTimeout(Duration),
}

pub async fn submit_prompt(
    client: &ComfyuiClient,
    workflow: &Value,
    client_id: &str,
) -> Result<String, SubmitError> {
    let url = format!("{}/prompt", client.base_url());
    let body = build_submit_body(workflow, client_id);
    let resp = client
        .http()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(ComfyuiError::from)?;

    if !resp.status().is_success() {
        return Err(ComfyuiError::InvalidResponse(format!(
            "submit HTTP {}",
            resp.status()
        ))
        .into());
    }

    let v: Value = resp.json().await.map_err(ComfyuiError::from)?;
    let prompt_id = v
        .get("prompt_id")
        .and_then(|p| p.as_str())
        .ok_or_else(|| SubmitError::MissingPromptId(v.clone()))?
        .to_string();

    Ok(prompt_id)
}

pub async fn poll_until_ready(
    client: &ComfyuiClient,
    prompt_id: &str,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<Value, SubmitError> {
    let start = std::time::Instant::now();
    let url = format!("{}/history/{}", client.base_url(), prompt_id);

    loop {
        let resp = client
            .http()
            .get(&url)
            .send()
            .await
            .map_err(ComfyuiError::from)?;

        if resp.status().is_success() {
            let v: Value = resp.json().await.map_err(ComfyuiError::from)?;
            if let Some(entry) = v.get(prompt_id) {
                return Ok(json!({ prompt_id: entry }));
            }
        }

        if start.elapsed() >= timeout {
            return Err(SubmitError::PollTimeout(timeout));
        }

        sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn submit_returns_prompt_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/prompt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "prompt_id": "abc-123",
                "number": 1
            })))
            .mount(&server)
            .await;

        let client = ComfyuiClient::new(server.uri()).unwrap();
        let id = submit_prompt(&client, &json!({}), "client-1").await.unwrap();
        assert_eq!(id, "abc-123");
    }

    #[tokio::test]
    async fn submit_missing_prompt_id_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/prompt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"error": "x"})))
            .mount(&server)
            .await;

        let client = ComfyuiClient::new(server.uri()).unwrap();
        let r = submit_prompt(&client, &json!({}), "c").await;
        assert!(matches!(r, Err(SubmitError::MissingPromptId(_))));
    }

    #[tokio::test]
    async fn poll_until_ready_returns_when_present() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/history/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "abc-123": { "outputs": { "9": { "images": [] } } }
            })))
            .mount(&server)
            .await;

        let client = ComfyuiClient::new(server.uri()).unwrap();
        let v = poll_until_ready(
            &client,
            "abc-123",
            Duration::from_secs(5),
            Duration::from_millis(50),
        )
        .await
        .unwrap();
        assert!(v["abc-123"]["outputs"]["9"]["images"].is_array());
    }

    #[tokio::test]
    async fn poll_times_out_when_absent() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/history/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;

        let client = ComfyuiClient::new(server.uri()).unwrap();
        let r = poll_until_ready(
            &client,
            "missing",
            Duration::from_millis(200),
            Duration::from_millis(50),
        )
        .await;
        assert!(matches!(r, Err(SubmitError::PollTimeout(_))));
    }
}
