mod models;

use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub use models::*;

#[derive(Clone)]
pub struct Client {
    root: String,
    password: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(server_url: &str, password: &str) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(90))
            .build()
            .expect("failed to build http client");
        Self {
            root: normalize_url(server_url),
            password: password.to_string(),
            http,
        }
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1/{}", self.root, path)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T> {
        let response = self
            .http
            .get(self.url(path))
            .query(&[("password", &self.password)])
            .query(query)
            .send()
            .await
            .with_context(|| format!("could not reach {}", self.root))?;
        decode(response).await
    }

    async fn post<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T> {
        let response = self
            .http
            .post(self.url(path))
            .query(&[("password", &self.password)])
            .json(&body)
            .send()
            .await
            .with_context(|| format!("could not reach {}", self.root))?;
        decode(response).await
    }

    pub async fn server_info(&self) -> Result<Value> {
        self.get("server/info", &[]).await
    }

    pub async fn chats(&self, offset: usize, limit: usize) -> Result<Vec<Chat>> {
        self.post(
            "chat/query",
            json!({
                "offset": offset,
                "limit": limit,
                "with": ["lastMessage", "participants"],
                "sort": "lastmessage",
            }),
        )
        .await
    }

    pub async fn messages(&self, chat_guid: &str, before: Option<i64>, limit: usize) -> Result<Vec<Message>> {
        let mut query = vec![
            ("limit", limit.to_string()),
            ("sort", "DESC".to_string()),
            ("with", "attachment,handle".to_string()),
        ];
        if let Some(before) = before {
            query.push(("before", before.to_string()));
        }
        self.get(&format!("chat/{}/message", encode(chat_guid)), &query).await
    }

    pub async fn send_text(&self, chat_guid: &str, temp_guid: &str, text: &str) -> Result<Message> {
        self.post(
            "message/text",
            json!({
                "chatGuid": chat_guid,
                "tempGuid": temp_guid,
                "message": text,
                "method": "apple-script",
            }),
        )
        .await
    }

    pub async fn contacts(&self) -> Result<Vec<Contact>> {
        self.get("contact", &[]).await
    }

    pub async fn attachment(&self, guid: &str, max_width: Option<u32>) -> Result<Vec<u8>> {
        let mut request = self
            .http
            .get(self.url(&format!("attachment/{}/download", encode(guid))))
            .query(&[("password", &self.password)]);
        if let Some(width) = max_width {
            request = request.query(&[("width", width.to_string())]);
        }
        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(anyhow!("attachment download failed: {}", response.status()));
        }
        Ok(response.bytes().await?.to_vec())
    }
}

async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    let bytes = response.bytes().await?;
    let envelope: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("server returned {status} with a non-JSON body"))?;
    if !status.is_success() {
        return Err(anyhow!(error_message(&envelope).unwrap_or_else(|| status.to_string())));
    }
    let data = envelope.get("data").cloned().unwrap_or(Value::Null);
    serde_json::from_value(data).context("unexpected response shape from server")
}

fn error_message(envelope: &Value) -> Option<String> {
    let error = envelope.get("error");
    error
        .and_then(|e| e.get("message"))
        .or_else(|| error.and_then(|e| e.get("error")))
        .or_else(|| envelope.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn encode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bare_host() {
        assert_eq!(normalize_url(" cheap-mini:1234/ "), "http://cheap-mini:1234");
        assert_eq!(normalize_url("https://x.ts.net"), "https://x.ts.net");
    }

    #[test]
    fn encodes_chat_guid_as_one_segment() {
        assert_eq!(encode("any;-;+15551234"), "any%3B-%3B%2B15551234");
    }
}
