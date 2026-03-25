use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::storage::StorageBackend;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageMethod {
    #[serde(rename = "dms")]
    DMs,
    #[serde(rename = "interaction")]
    Interaction,
    #[serde(rename = "both")]
    Both,
    #[serde(rename = "none")]
    None,
}

impl Default for MessageMethod {
    fn default() -> Self {
        Self::Both
    }
}

impl std::fmt::Display for MessageMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DMs => write!(f, "DMs"),
            Self::Interaction => write!(f, "Interaction"),
            Self::Both => write!(f, "Both"),
            Self::None => write!(f, "None"),
        }
    }
}

impl MessageMethod {
    pub fn from_value(s: &str) -> Self {
        match s {
            "dms" => Self::DMs,
            "interaction" => Self::Interaction,
            "both" => Self::Both,
            "none" => Self::None,
            _ => Self::Both,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OptionConfig {
    pub channel_id: Option<u64>,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildConfig {
    pub guild_id: u64,
    pub bot_name: String,
    pub message_channel_id: Option<u64>,
    pub log_channel_id: Option<u64>,
    pub message_method: MessageMethod,
    pub reports_enabled: bool,
    pub templates: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub option_configs: HashMap<String, OptionConfig>,
    #[serde(default)]
    pub custom_formats: Vec<String>,
}

impl GuildConfig {
    pub fn new(guild_id: u64) -> Self {
        Self {
            guild_id,
            bot_name: "Mod Mail".to_string(),
            message_channel_id: None,
            log_channel_id: None,
            message_method: MessageMethod::default(),
            reports_enabled: false,
            templates: HashMap::new(),
            option_configs: HashMap::new(),
            custom_formats: Vec::new(),
        }
    }

    pub async fn load(storage: &StorageBackend, guild_id: u64) -> Self {
        match storage.load_guild(guild_id).await {
            Some(value) => serde_json::from_value(value).unwrap_or_else(|e| {
                println!("[config] Parse error for guild {}: {}", guild_id, e);
                Self::new(guild_id)
            }),
            None => Self::new(guild_id),
        }
    }

    pub async fn save(&self, storage: &StorageBackend) -> Result<(), String> {
        let value = serde_json::to_value(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        storage.save_guild(self.guild_id, &value).await
    }

    pub fn get_template(&self, name: &str) -> crate::templates::EmbedTemplate {
        if let Some(value) = self.templates.get(name) {
            serde_json::from_value(value.clone())
                .unwrap_or_else(|_| crate::templates::get_default(name))
        } else {
            crate::templates::get_default(name)
        }
    }

    pub fn set_template(&mut self, name: &str, template: &crate::templates::EmbedTemplate) {
        if let Ok(value) = serde_json::to_value(template) {
            self.templates.insert(name.to_string(), value);
        }
    }

    pub fn reset_template(&mut self, name: &str) {
        self.templates.remove(name);
    }

    pub fn reset_all_templates(&mut self) {
        self.templates.clear();
    }

    pub fn get_option_config(&self, name: &str) -> OptionConfig {
        self.option_configs.get(name).cloned().unwrap_or_default()
    }

    pub fn set_option_config(&mut self, name: &str, cfg: &OptionConfig) {
        self.option_configs.insert(name.to_string(), cfg.clone());
    }

    pub fn option_channel(&self, option_name: &str) -> Option<u64> {
        self.option_configs
            .get(option_name)
            .and_then(|o| o.channel_id)
            .or(self.message_channel_id)
    }

    pub fn all_format_names(&self) -> Vec<String> {
        let mut names: Vec<String> = crate::templates::list_option_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for name in &self.custom_formats {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
    }

    pub fn add_custom_format(&mut self, key: &str) {
        if !self.custom_formats.contains(&key.to_string()) {
            self.custom_formats.push(key.to_string());
        }
    }

    pub fn remove_custom_format(&mut self, key: &str) {
        self.custom_formats.retain(|k| k != key);
        self.templates.remove(key);
        self.option_configs.remove(key);
    }
}
