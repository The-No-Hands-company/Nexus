# nexus-sdk (Rust)

Official Rust SDK for the **Nexus** platform. Provides async REST and gateway clients, slash-command
routing, slash-command / embed builders, and a high-level `NexusClient` bot wrapper.

## Features

- Async REST client (`reqwest` / `rustls`)
- WebSocket gateway client (`tokio-tungstenite`) with heartbeat and auto-reconnect
- Type-safe `SlashCommandBuilder` and `EmbedBuilder`
- `NexusClient` combines REST + gateway with a `command()` registration API
- Multi-consumer event broadcasting via `tokio::sync::broadcast`

## Installation

```toml
[dependencies]
nexus-sdk = { path = "../nexus-sdk-rs" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## Quick start

```rust
use nexus_sdk::{NexusClient, builders::SlashCommandBuilder, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = NexusClient::new("Bot YOUR_TOKEN", None, None)?;

    let ping = SlashCommandBuilder::new()
        .name("ping")
        .description("Pong!")
        .build();

    client.command(ping, |interaction| {
        println!("Interaction from {:?}", interaction["member"]);
    });

    client.login("YOUR_APP_ID").await
}
```

### Responding to interactions

Because `command()` handlers run in spawned tasks, keep a `RestClient` clone for responding:

```rust
use std::sync::Arc;
use nexus_sdk::{NexusClient, RestClient, builders::SlashCommandBuilder, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = NexusClient::new("Bot TOKEN", None, None)?;
    let rest = Arc::new(client.rest.clone());

    let ping = SlashCommandBuilder::new().name("ping").description("Pong!").build();
    client.command(ping, move |interaction| {
        let rest = Arc::clone(&rest);
        let id = interaction["id"].as_str().unwrap().to_string();
        tokio::spawn(async move {
            let content = serde_json::json!({ "content": "Pong!" });
            let _ = rest.create_interaction_response(&id, 4, Some(&content)).await;
        });
    });

    client.login("APP_ID").await
}
```

### Listening to raw gateway events

```rust
let mut rx = client.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        println!("GW: {:?} {:?}", event.event, event.data);
    }
});
client.login("APP_ID").await?;
```

## API surface

| Module | Contents |
|---|---|
| `nexus_sdk::rest` | `RestClient` — full HTTP API wrapper |
| `nexus_sdk::gateway` | `GatewayClient`, `GatewayEvent` |
| `nexus_sdk::client` | `NexusClient` (REST + gateway + command routing) |
| `nexus_sdk::builders` | `SlashCommandBuilder`, `SlashCommandOptionBuilder`, `EmbedBuilder` |
| `nexus_sdk::types` | `BotApplication`, `SlashCommand`, `Interaction`, `Embed`, `Webhook`, … |
| `nexus_sdk::error` | `NexusError`, `Result<T>` |

## Configuration

| Option | Env / constructor | Default |
|---|---|---|
| REST base URL | `rest_url` arg | `http://localhost:3000` |
| Gateway URL | `gateway_url` arg | `ws://localhost:3001` |

## License

MIT
