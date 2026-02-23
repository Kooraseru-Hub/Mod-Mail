//! Ping command handler
//!
//! Implements a simple ping command that responds with bot latency and statistics.
//! Uses the Components V2 message format for rich layout support.

use std::time::Instant;
use serenity::{
    builder::CreateCommand,
    model::application::Interaction,
    prelude::Context,
};
use crate::message;

/// Registers the ping slash command
/// 
/// # Returns
/// A CreateCommand configured for the ping command
pub fn register() -> CreateCommand {
    CreateCommand::new("ping").description("Responds with a pong and bot statistics")
}

/// Executes the ping command
/// 
/// # Arguments
/// * `ctx` - Context for API access
/// * `interaction` - The command interaction from Discord
/// 
/// Responds with the full Components V2 embed including actual latency, shard, and region data.
pub async fn run(ctx: &Context, interaction: &Interaction) {
    if let Interaction::Command(command) = interaction {
        /* Acknowledge the interaction immediately so Discord doesn't time out */
        if let Err(e) = command.defer(&ctx.http).await {
            println!("Error deferring response: {}", e);
            return;
        }

        /* Start measuring latency after deferral */
        let start_time = Instant::now();

        /* Get shard information */
        let shard_id = ctx.shard_id.get();

        /* Get public IP for region detection */
        let ip: String = match reqwest::get("https://api.ipify.org").await {
            Ok(resp) => resp.text().await.unwrap_or_else(|_| "Unknown".to_string()),
            Err(_) => "Unknown".to_string(),
        };

        /* Get geolocation data */
        let geo: serde_json::Value = match reqwest::get(format!("https://ipapi.co/{}/json/", ip)).await {
            Ok(resp) => match resp.json().await {
                Ok(data) => data,
                Err(_) => serde_json::json!({}),
            },
            Err(_) => serde_json::json!({}),
        };

        /* Calculate actual latency */
        let latency = start_time.elapsed().as_millis();

        /* Load embed template from JSON */
        if let Ok(mut payload) = message::load_message_from_file("src/ping/embed.json") {
            let payload: &mut serde_json::Value = &mut payload;
            /* Update the dynamic values in the payload */
            if let Some(components) = payload.get_mut("components").and_then(|c: &mut serde_json::Value| c.as_array_mut()) {
                for component in components {
                    if let Some(comp_array) = component.get_mut("components").and_then(|c: &mut serde_json::Value| c.as_array_mut()) {
                        for comp in comp_array {
                            /* Look for the text component with latency/region/shard info */
                            if comp.get("type").and_then(|t: &serde_json::Value| t.as_u64()) == Some(10) {
                                if let Some(content) = comp.get("content").and_then(|c: &serde_json::Value| c.as_str()) {
                                    let content_str = content.to_string();
                                    // For info field
                                    if content_str.contains("[Info]") && !content_str.contains("Latency") {
                                        /* This is the dynamic data field - update it */
                                        let region = geo["region"].as_str()
                                            .or_else(|| geo["city"].as_str())
                                            .or_else(|| geo["country_name"].as_str())
                                            .unwrap_or("Unknown");
                                        let new_content = format!(
                                            "**Latency** {}ms\n**Region** {}\n**Shard** {}",
                                            latency, region, shard_id
                                        );
                                        comp["content"] = serde_json::json!(new_content);
                                    }
                                    // For timestamp field
                                    if content_str.contains("[Timestamp]") {
                                        let unix_ts = chrono::Utc::now().timestamp();
                                        comp["content"] = serde_json::json!(format!("<t:{}:R>", unix_ts));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            /* Edit the deferred response with the Components V2 payload */
            if let Err(e) = message::send_components_v2_interaction_response(
                ctx,
                command.application_id.get(),
                &command.token,
                &payload,
            ).await {
                println!("Error sending Components V2 response: {}", e);
            }
        } else {
            println!("Failed to load ping embed.json");
        }
    }
}