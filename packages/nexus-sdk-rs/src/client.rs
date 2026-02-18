//! High-level `NexusClient` combining REST + gateway.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::broadcast;

use crate::error::Result;
use crate::gateway::{GatewayClient, GatewayEvent};
use crate::rest::RestClient;
use crate::types::Embed;

type BoxHandler = Arc<dyn Fn(Value) + Send + Sync + 'static>;

/// The main Nexus bot client.
///
/// ```rust,no_run
/// use nexus_sdk::{NexusClient, builders::SlashCommandBuilder};
///
/// #[tokio::main]
/// async fn main() -> nexus_sdk::Result<()> {
///     let mut client = NexusClient::new("Bot mytoken", None, None)?;
///
///     client.command(
///         SlashCommandBuilder::new().name("ping").description("Pong!").build(),
///         |_interaction| println!("Pong!"),
///     );
///
///     client.login("your-app-id").await
/// }
/// ```
pub struct NexusClient {
    pub rest: RestClient,
    gateway: GatewayClient,
    commands: HashMap<String, (Value, BoxHandler)>,
}

impl NexusClient {
    pub fn new(
        token: impl Into<String>,
        rest_url: Option<&str>,
        gateway_url: Option<&str>,
    ) -> Result<Self> {
        let token_str: String = token.into();
        let token_str = if token_str.starts_with("Bot ") {
            token_str
        } else {
            format!("Bot {token_str}")
        };
        Ok(Self {
            rest: RestClient::new(token_str.clone(), rest_url)?,
            gateway: GatewayClient::new(token_str, gateway_url),
            commands: HashMap::new(),
        })
    }

    /// Register a slash command and its handler. Call before [`login`].
    pub fn command(
        &mut self,
        definition: Value,
        handler: impl Fn(Value) + Send + Sync + 'static,
    ) -> &mut Self {
        let name = definition["name"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        self.commands.insert(name, (definition, Arc::new(handler)));
        self
    }

    /// Subscribe to raw gateway events.
    pub fn subscribe(&self) -> broadcast::Receiver<GatewayEvent> {
        self.gateway.subscribe()
    }

    /// Bulk-register all commands then start the gateway.
    pub async fn login(self, app_id: &str) -> Result<()> {
        if !self.commands.is_empty() {
            let defs: Vec<Value> = self.commands.values().map(|(d, _)| d.clone()).collect();
            self.rest.bulk_overwrite_global_commands(app_id, &defs).await?;
        }

        let commands: Arc<HashMap<String, (Value, BoxHandler)>> = Arc::new(self.commands);
        let mut events = self.gateway.subscribe();
        let cmds = Arc::clone(&commands);
        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                if event.event.as_deref() == Some("INTERACTION_CREATE") {
                    route_interaction(&cmds, event.data);
                }
            }
        });

        self.gateway.connect().await
    }

    pub async fn reply(
        &self,
        interaction_id: &str,
        content: Option<&str>,
        embeds: Option<&[Embed]>,
        ephemeral: bool,
    ) -> Result<()> {
        let mut data = serde_json::json!({});
        if let Some(c) = content { data["content"] = serde_json::json!(c); }
        if let Some(e) = embeds { data["embeds"] = serde_json::to_value(e)?; }
        if ephemeral { data["flags"] = serde_json::json!(64); }
        self.rest.create_interaction_response(interaction_id, 4, Some(&data)).await
    }

    pub async fn defer_reply(&self, interaction_id: &str, ephemeral: bool) -> Result<()> {
        let data = ephemeral.then(|| serde_json::json!({ "ephemeral": true }));
        self.rest.create_interaction_response(interaction_id, 5, data.as_ref()).await
    }
}

fn route_interaction(commands: &HashMap<String, (Value, BoxHandler)>, data: Value) {
    let name = data
        .get("data")
        .and_then(|d| d.get("name").or_else(|| d.get("command_name")))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    if let Some(name) = name {
        if let Some((_, handler)) = commands.get(&name) {
            let handler = Arc::clone(handler);
            tokio::spawn(async move { handler(data) });
        }
    }
}
