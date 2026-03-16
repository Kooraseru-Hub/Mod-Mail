use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildConfig {
    pub guild_id: u64,
    pub bot_name: String,
    pub bot_avatar_url: Option<String>,
    pub message_channel_id: Option<u64>,
    pub log_channel_id: Option<u64>,
    pub message_method: MessageMethod,
    pub reports_enabled: bool,
    pub templates: HashMap<String, serde_json::Value>,
}

impl GuildConfig {
    pub fn new(guild_id: u64) -> Self {
        Self {
            guild_id,
            bot_name: "Mod Mail".to_string(),
            bot_avatar_url: None,
            message_channel_id: None,
            log_channel_id: None,
            message_method: MessageMethod::default(),
            reports_enabled: false,
            templates: HashMap::new(),
        }
    }

    fn config_path(guild_id: u64) -> PathBuf {
        PathBuf::from(format!("data/guilds/{}.json", guild_id))
    }

    pub fn load(guild_id: u64) -> Self {
        let path = Self::config_path(guild_id);
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(data) => match serde_json::from_str(&data) {
                    Ok(config) => return config,
                    Err(e) => println!("[config] Parse error {}: {}", path.display(), e),
                },
                Err(e) => println!("[config] Read error {}: {}", path.display(), e),
            }
        }
        Self::new(guild_id)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path(self.guild_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&path, data)
            .map_err(|e| format!("Failed to write config: {}", e))
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
}
