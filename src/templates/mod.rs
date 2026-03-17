use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedTemplate {
    pub title: String,
    pub description: String,
    pub color: u32,
    pub fields: Vec<EmbedField>,
}

impl EmbedTemplate {
    pub fn to_components_v2(&self) -> serde_json::Value {
        let mut inner: Vec<serde_json::Value> = Vec::new();

        inner.push(serde_json::json!({
            "type": 10,
            "content": format!("# {}", self.title)
        }));

        inner.push(serde_json::json!({"type": 14}));

        inner.push(serde_json::json!({
            "type": 10,
            "content": self.description
        }));

        for field in &self.fields {
            inner.push(serde_json::json!({"type": 14}));
            inner.push(serde_json::json!({
                "type": 10,
                "content": format!("**{}**\n{}", field.name, field.value)
            }));
        }

        serde_json::json!({
            "flags": 32768,
            "components": [{
                "type": 17,
                "components": inner,
                "accent_color": self.color
            }]
        })
    }
}

pub const OPTION_PLAYER_REPORT: &str = "player_report";
pub const OPTION_GENERAL_SUPPORT: &str = "general_support";

pub fn get_default(name: &str) -> EmbedTemplate {
    match name {
        OPTION_PLAYER_REPORT => default_player_report(),
        OPTION_GENERAL_SUPPORT => default_general_support(),
        _ => default_general_support(),
    }
}

pub fn default_player_report() -> EmbedTemplate {
    EmbedTemplate {
        title: "Player Report".to_string(),
        description: "A player report has been submitted.".to_string(),
        color: 15158332, // Red-ish
        fields: vec![
            EmbedField {
                name: "Reporter".to_string(),
                value: "{reporter}".to_string(),
                inline: true,
            },
            EmbedField {
                name: "Reported User".to_string(),
                value: "{reported_user}".to_string(),
                inline: true,
            },
            EmbedField {
                name: "Reason".to_string(),
                value: "{reason}".to_string(),
                inline: false,
            },
        ],
    }
}

pub fn default_general_support() -> EmbedTemplate {
    EmbedTemplate {
        title: "General Support Ticket".to_string(),
        description: "A new support ticket has been created.".to_string(),
        color: 10181046, // Purple
        fields: vec![
            EmbedField {
                name: "Author".to_string(),
                value: "{author}".to_string(),
                inline: true,
            },
            EmbedField {
                name: "Type".to_string(),
                value: "{ticket_type}".to_string(),
                inline: true,
            },
            EmbedField {
                name: "Message".to_string(),
                value: "{content}".to_string(),
                inline: false,
            },
        ],
    }
}

pub fn list_option_names() -> Vec<&'static str> {
    vec![OPTION_PLAYER_REPORT, OPTION_GENERAL_SUPPORT]
}

pub fn display_name(key: &str) -> String {
    match key {
        OPTION_PLAYER_REPORT => "Report Player".to_string(),
        OPTION_GENERAL_SUPPORT => "General Support".to_string(),
        other => other
            .split('_')
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}
