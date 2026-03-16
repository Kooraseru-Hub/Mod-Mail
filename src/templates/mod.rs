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

pub const TEMPLATE_PLAYER_REPORT: &str = "player_report";
pub const TEMPLATE_MOD_MAIL: &str = "mod_mail";

pub fn get_default(name: &str) -> EmbedTemplate {
    match name {
        TEMPLATE_PLAYER_REPORT => default_player_report(),
        TEMPLATE_MOD_MAIL => default_mod_mail(),
        _ => default_mod_mail(),
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

pub fn default_mod_mail() -> EmbedTemplate {
    EmbedTemplate {
        title: "Mod Mail Ticket".to_string(),
        description: "A new mod mail ticket has been created.".to_string(),
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

pub fn list_template_names() -> Vec<&'static str> {
    vec![TEMPLATE_PLAYER_REPORT, TEMPLATE_MOD_MAIL]
}
