//! Async REST client for the Nexus API.

use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{NexusError, Result};
use crate::types::Embed;

const DEFAULT_BASE: &str = "http://localhost:3000/api/v1";

/// Async Nexus REST client.
///
/// ```rust,no_run
/// use nexus_sdk::rest::RestClient;
///
/// #[tokio::main]
/// async fn main() -> nexus_sdk::Result<()> {
///     let rest = RestClient::new("Bot mytoken", None)?;
///     let apps = rest.list_applications().await?;
///     println!("{apps:?}");
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct RestClient {
    client: Client,
    base_url: String,
}

impl RestClient {
    pub fn new(token: impl Into<String>, base_url: Option<&str>) -> Result<Self> {
        let token = {
            let t = token.into();
            if t.starts_with("Bot ") { t } else { format!("Bot {t}") }
        };
        let client = Client::builder()
            .default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&token)
                        .map_err(|e| NexusError::Other(e.to_string()))?,
                );
                h.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                h
            })
            .build()
            .map_err(NexusError::Http)?;

        Ok(Self {
            client,
            base_url: base_url.unwrap_or(DEFAULT_BASE).trim_end_matches('/').to_owned(),
        })
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let msg = resp
                .json::<Value>()
                .await
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_owned))
                .unwrap_or_else(|| status.to_string());
            return Err(NexusError::Api { status: status.as_u16(), message: msg });
        }
        if status == StatusCode::NO_CONTENT {
            return serde_json::from_value(Value::Null).map_err(NexusError::Json);
        }
        Ok(resp.json::<T>().await?)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::GET, path, None).await
    }

    async fn post<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    async fn patch<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.request(Method::PATCH, path, Some(body)).await
    }

    async fn put<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.request(Method::PUT, path, Some(body)).await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.delete(&url).send().await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            return Err(NexusError::Api { status, message: resp.text().await.unwrap_or_default() });
        }
        Ok(())
    }

    // ── Applications ──────────────────────────────────────────────────────────

    pub async fn list_applications(&self) -> Result<Vec<Value>> {
        self.get("/applications").await
    }

    pub async fn get_application(&self, app_id: &str) -> Result<Value> {
        self.get(&format!("/applications/{app_id}")).await
    }

    pub async fn create_application(&self, name: &str, description: &str) -> Result<Value> {
        self.post("/applications", &serde_json::json!({ "name": name, "description": description })).await
    }

    pub async fn update_application(&self, app_id: &str, fields: &Value) -> Result<Value> {
        self.patch(&format!("/applications/{app_id}"), fields).await
    }

    pub async fn delete_application(&self, app_id: &str) -> Result<()> {
        self.delete(&format!("/applications/{app_id}")).await
    }

    pub async fn reset_token(&self, app_id: &str) -> Result<Value> {
        self.post(&format!("/applications/{app_id}/reset-token"), &Value::Null).await
    }

    // ── Server installs ───────────────────────────────────────────────────────

    pub async fn list_server_bots(&self, server_id: &str) -> Result<Vec<Value>> {
        self.get(&format!("/servers/{server_id}/bots")).await
    }

    pub async fn install_bot(&self, server_id: &str, app_id: &str, permissions: i64) -> Result<Value> {
        self.post(
            &format!("/servers/{server_id}/bots"),
            &serde_json::json!({ "application_id": app_id, "permissions": permissions }),
        ).await
    }

    pub async fn uninstall_bot(&self, server_id: &str, bot_user_id: &str) -> Result<()> {
        self.delete(&format!("/servers/{server_id}/bots/{bot_user_id}")).await
    }

    // ── Global commands ───────────────────────────────────────────────────────

    pub async fn get_global_commands(&self, app_id: &str) -> Result<Vec<Value>> {
        self.get(&format!("/applications/{app_id}/commands")).await
    }

    pub async fn create_global_command(&self, app_id: &str, data: &Value) -> Result<Value> {
        self.post(&format!("/applications/{app_id}/commands"), data).await
    }

    pub async fn edit_global_command(&self, app_id: &str, cmd_id: &str, data: &Value) -> Result<Value> {
        self.patch(&format!("/applications/{app_id}/commands/{cmd_id}"), data).await
    }

    pub async fn delete_global_command(&self, app_id: &str, cmd_id: &str) -> Result<()> {
        self.delete(&format!("/applications/{app_id}/commands/{cmd_id}")).await
    }

    pub async fn bulk_overwrite_global_commands(&self, app_id: &str, commands: &[Value]) -> Result<Vec<Value>> {
        self.put(&format!("/applications/{app_id}/commands"), &serde_json::json!(commands)).await
    }

    // ── Server commands ───────────────────────────────────────────────────────

    pub async fn get_server_commands(&self, app_id: &str, server_id: &str) -> Result<Vec<Value>> {
        self.get(&format!("/applications/{app_id}/guilds/{server_id}/commands")).await
    }

    pub async fn bulk_overwrite_server_commands(&self, app_id: &str, server_id: &str, cmds: &[Value]) -> Result<Vec<Value>> {
        self.put(&format!("/applications/{app_id}/guilds/{server_id}/commands"), &serde_json::json!(cmds)).await
    }

    // ── Interactions ──────────────────────────────────────────────────────────

    pub async fn create_interaction_response(
        &self,
        interaction_id: &str,
        response_type: u8,
        data: Option<&Value>,
    ) -> Result<()> {
        let mut body = serde_json::json!({ "type": response_type });
        if let Some(d) = data {
            body["data"] = d.clone();
        }
        self.post::<Value>(&format!("/interactions/{interaction_id}/callback"), &body).await?;
        Ok(())
    }

    // ── Webhooks ──────────────────────────────────────────────────────────────

    pub async fn get_channel_webhooks(&self, channel_id: &str) -> Result<Vec<Value>> {
        self.get(&format!("/channels/{channel_id}/webhooks")).await
    }

    pub async fn create_webhook(&self, channel_id: &str, name: &str, avatar: Option<&str>) -> Result<Value> {
        let mut body = serde_json::json!({ "name": name });
        if let Some(a) = avatar { body["avatar"] = serde_json::json!(a); }
        self.post(&format!("/channels/{channel_id}/webhooks"), &body).await
    }

    pub async fn get_webhook(&self, webhook_id: &str) -> Result<Value> {
        self.get(&format!("/webhooks/{webhook_id}")).await
    }

    pub async fn modify_webhook(&self, webhook_id: &str, fields: &Value) -> Result<Value> {
        self.patch(&format!("/webhooks/{webhook_id}"), fields).await
    }

    pub async fn delete_webhook(&self, webhook_id: &str) -> Result<()> {
        self.delete(&format!("/webhooks/{webhook_id}")).await
    }

    pub async fn execute_webhook(
        &self,
        webhook_id: &str,
        webhook_token: &str,
        content: Option<&str>,
        embeds: Option<&[Embed]>,
        username: Option<&str>,
    ) -> Result<()> {
        let mut body = serde_json::json!({});
        if let Some(c) = content { body["content"] = serde_json::json!(c); }
        if let Some(e) = embeds { body["embeds"] = serde_json::to_value(e)?; }
        if let Some(u) = username { body["username"] = serde_json::json!(u); }
        let url = format!("{}/webhooks/{webhook_id}/{webhook_token}", self.base_url);
        let resp = self.client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            return Err(NexusError::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }
}
