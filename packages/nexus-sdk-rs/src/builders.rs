//! Fluent builders for slash commands and embeds.

use crate::types::{Embed, EmbedAuthor, EmbedField, EmbedFooter, EmbedImage};
use serde_json::{json, Value};

// ── Option types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum OptionType {
    SubCommand = 1,
    SubCommandGroup = 2,
    String = 3,
    Integer = 4,
    Boolean = 5,
    User = 6,
    Channel = 7,
    Role = 8,
    Mentionable = 9,
    Number = 10,
    Attachment = 11,
}

// ── Slash command option builder ──────────────────────────────────────────────

#[derive(Default)]
pub struct SlashCommandOptionBuilder {
    kind: u8,
    name: String,
    description: String,
    required: bool,
    choices: Vec<Value>,
    options: Vec<Value>,
    min_value: Option<f64>,
    max_value: Option<f64>,
    min_length: Option<u32>,
    max_length: Option<u32>,
}

impl SlashCommandOptionBuilder {
    pub fn new(kind: OptionType) -> Self {
        Self { kind: kind as u8, ..Default::default() }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn choice(mut self, name: impl Into<String>, value: impl serde::Serialize) -> Self {
        self.choices.push(json!({ "name": name.into(), "value": value }));
        self
    }

    pub fn min_value(mut self, v: f64) -> Self {
        self.min_value = Some(v);
        self
    }

    pub fn max_value(mut self, v: f64) -> Self {
        self.max_value = Some(v);
        self
    }

    pub fn min_length(mut self, v: u32) -> Self {
        self.min_length = Some(v);
        self
    }

    pub fn max_length(mut self, v: u32) -> Self {
        self.max_length = Some(v);
        self
    }

    pub fn build(self) -> Value {
        let mut d = json!({
            "type": self.kind,
            "name": self.name,
            "description": self.description,
            "required": self.required,
        });
        if !self.choices.is_empty() {
            d["choices"] = Value::Array(self.choices);
        }
        if !self.options.is_empty() {
            d["options"] = Value::Array(self.options);
        }
        if let Some(v) = self.min_value { d["min_value"] = json!(v); }
        if let Some(v) = self.max_value { d["max_value"] = json!(v); }
        if let Some(v) = self.min_length { d["min_length"] = json!(v); }
        if let Some(v) = self.max_length { d["max_length"] = json!(v); }
        d
    }
}

// ── Slash command builder ─────────────────────────────────────────────────────

/// Fluent builder for a slash command definition.
///
/// ```rust
/// use nexus_sdk::builders::{SlashCommandBuilder, SlashCommandOptionBuilder, OptionType};
///
/// let cmd = SlashCommandBuilder::new()
///     .name("ping")
///     .description("Replies with Pong!")
///     .build();
/// ```
#[derive(Default)]
pub struct SlashCommandBuilder {
    name: String,
    description: String,
    kind: u8,
    options: Vec<Value>,
    default_member_permissions: Option<String>,
    dm_permission: bool,
}

impl SlashCommandBuilder {
    pub fn new() -> Self {
        Self { kind: 1, dm_permission: true, ..Default::default() }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn kind(mut self, k: u8) -> Self {
        self.kind = k;
        self
    }

    pub fn default_member_permissions(mut self, perms: impl ToString) -> Self {
        self.default_member_permissions = Some(perms.to_string());
        self
    }

    pub fn dm_permission(mut self, allow: bool) -> Self {
        self.dm_permission = allow;
        self
    }

    pub fn option(mut self, builder: SlashCommandOptionBuilder) -> Self {
        self.options.push(builder.build());
        self
    }

    pub fn string_option(self, f: impl FnOnce(SlashCommandOptionBuilder) -> SlashCommandOptionBuilder) -> Self {
        self.option(f(SlashCommandOptionBuilder::new(OptionType::String)))
    }

    pub fn integer_option(self, f: impl FnOnce(SlashCommandOptionBuilder) -> SlashCommandOptionBuilder) -> Self {
        self.option(f(SlashCommandOptionBuilder::new(OptionType::Integer)))
    }

    pub fn boolean_option(self, f: impl FnOnce(SlashCommandOptionBuilder) -> SlashCommandOptionBuilder) -> Self {
        self.option(f(SlashCommandOptionBuilder::new(OptionType::Boolean)))
    }

    pub fn user_option(self, f: impl FnOnce(SlashCommandOptionBuilder) -> SlashCommandOptionBuilder) -> Self {
        self.option(f(SlashCommandOptionBuilder::new(OptionType::User)))
    }

    pub fn number_option(self, f: impl FnOnce(SlashCommandOptionBuilder) -> SlashCommandOptionBuilder) -> Self {
        self.option(f(SlashCommandOptionBuilder::new(OptionType::Number)))
    }

    pub fn build(self) -> Value {
        let mut d = json!({
            "name": self.name,
            "description": self.description,
            "type": self.kind,
            "dm_permission": self.dm_permission,
        });
        if !self.options.is_empty() {
            d["options"] = Value::Array(self.options);
        }
        if let Some(p) = self.default_member_permissions {
            d["default_member_permissions"] = json!(p);
        }
        d
    }
}

// ── Embed builder ─────────────────────────────────────────────────────────────

/// Fluent builder for message embeds.
///
/// ```rust
/// use nexus_sdk::builders::EmbedBuilder;
///
/// let embed = EmbedBuilder::new()
///     .title("Hello")
///     .description("World")
///     .color(0x7c6af7u32)
///     .build();
/// ```
#[derive(Default)]
pub struct EmbedBuilder {
    inner: Embed,
}

impl EmbedBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.inner.title = Some(v.into());
        self
    }

    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.inner.description = Some(v.into());
        self
    }

    pub fn url(mut self, v: impl Into<String>) -> Self {
        self.inner.url = Some(v.into());
        self
    }

    pub fn color(mut self, v: u32) -> Self {
        self.inner.color = Some(v);
        self
    }

    pub fn timestamp(mut self, v: impl Into<String>) -> Self {
        self.inner.timestamp = Some(v.into());
        self
    }

    pub fn footer(mut self, text: impl Into<String>, icon_url: Option<String>) -> Self {
        self.inner.footer = Some(EmbedFooter { text: text.into(), icon_url });
        self
    }

    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.inner.image = Some(EmbedImage { url: url.into(), height: None, width: None });
        self
    }

    pub fn thumbnail(mut self, url: impl Into<String>) -> Self {
        self.inner.thumbnail = Some(EmbedImage { url: url.into(), height: None, width: None });
        self
    }

    pub fn author(mut self, name: impl Into<String>, url: Option<String>, icon_url: Option<String>) -> Self {
        self.inner.author = Some(EmbedAuthor { name: name.into(), url, icon_url });
        self
    }

    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>, inline: bool) -> Self {
        self.inner.fields.push(EmbedField { name: name.into(), value: value.into(), inline });
        self
    }

    pub fn build(self) -> Embed {
        self.inner
    }
}
