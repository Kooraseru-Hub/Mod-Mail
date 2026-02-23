//! Universal message system supporting both Standard and Components V2 formats
//!
//! This module provides a unified interface for sending messages using either:
//! - **Standard format**: Traditional embeds, content, and action rows (buttons/selects)
//! - **Components V2 format**: Rich layouts with text blocks, media, and separators
//!
//! The system automatically detects the format and routes messages to the appropriate endpoint.

use serde::Deserialize;
use serde_json::Value;
use serenity::{
    builder::{CreateMessage, CreateEmbed, CreateButton, CreateSelectMenu, CreateSelectMenuKind, 
              CreateSelectMenuOption, CreateActionRow},
    model::channel::Message,
    model::colour::Colour,
    prelude::Context,
    all::ChannelId,
};

/// Detects whether a message payload uses Standard format or Components V2
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageFormat {
    /// Standard format with embeds, content, and action rows (buttons/selects)
    Standard,
    /// Components V2 format with text blocks, media, and separators
    ComponentsV2,
}

/// Specifies where a message should be delivered
#[derive(Clone, Debug)]
pub enum DeliveryMethod {
    /// Send as a direct message to the user
    DirectMessage(ChannelId),
    /// Send to a specific channel
    Channel(ChannelId),
    /// Send via a Discord webhook URL
    Webhook(String),
    /// Send as an interaction response (deferred interaction)
    InteractionResponse,
}

#[derive(Deserialize)]
struct StandardMessage {
    content: Option<String>,
    embeds: Option<Vec<Value>>,
    components: Option<Vec<Value>>,
}

/// Detects the message format based on the JSON payload
/// 
/// Analyzes component types to determine if this is a Standard message
/// (with action rows containing buttons/selects) or Components V2 (with layout blocks)
pub fn detect_format(payload: &Value) -> MessageFormat {
    // Check if payload has Components V2 specific types (10, 12, 14)
    if let Some(components) = payload.get("components").and_then(|c| c.as_array()) {
        for component in components {
            if let Some(comp_type) = component.get("type").and_then(|t| t.as_u64()) {
                // Types 10, 12, 14 are Components V2 only
                match comp_type {
                    10 | 12 | 14 => return MessageFormat::ComponentsV2,
                    _ => {}
                }
            }
        }
    }
    MessageFormat::Standard
}

/// Creates and sends a message from JSON payload
/// 
/// Automatically detects the format and routes to the appropriate sender.
/// 
/// # Arguments
/// * `ctx` - Serenity context for API access
/// * `msg` - The original message (for DM context)
/// * `payload` - JSON payload containing message data
/// * `delivery` - Where to send the message
pub async fn send_message(
    ctx: &Context,
    msg: &Message,
    payload: Value,
    delivery: DeliveryMethod,
) -> Result<(), String> {
    let format = detect_format(&payload);
    
    match format {
        MessageFormat::Standard => {
            send_standard_message(ctx, msg, &payload, delivery).await
        }
        MessageFormat::ComponentsV2 => {
            send_components_v2_message(ctx, msg, &payload, delivery).await
        }
    }
}

/// Sends a Components V2 layout-based message
/// 
/// Uses raw JSON and requires webhook or interaction endpoints.
/// Cannot be sent to persistent DM channels via standard endpoints.
async fn send_components_v2_message(
    _ctx: &Context,
    _msg: &Message,
    payload: &Value,
    delivery: DeliveryMethod,
) -> Result<(), String> {
    match delivery {
        DeliveryMethod::Webhook(webhook_url) => {
            // Send via webhook - Components V2 is perfect for this
            let client = reqwest::Client::new();
            let response = client
                .post(&webhook_url)
                .json(payload)
                .send()
                .await
                .map_err(|e| format!("Webhook request failed: {}", e))?;
            
            if response.status().is_success() {
                Ok(())
            } else {
                Err(format!("Webhook returned status: {}", response.status()))
            }
        }
        DeliveryMethod::InteractionResponse => {
            // For interaction responses, Components V2 cannot be sent this way
            // Use send_components_v2_interaction_response instead
            Err("Use send_components_v2_interaction_response for interaction responses".to_string())
        }
        DeliveryMethod::DirectMessage(_) | DeliveryMethod::Channel(_) => {
            // Components V2 cannot be sent to standard DM/channel endpoints
            Err("Components V2 can only be sent via webhooks or interactions".to_string())
        }
    }
}

/// Sends a Standard format message with embeds and action rows
/// 
/// Uses Serenity builders for maximum compatibility.
async fn send_standard_message(
    ctx: &Context,
    _msg: &Message,
    payload: &Value,
    delivery: DeliveryMethod,
) -> Result<(), String> {
    // Parse the standard message format
    let standard_msg: StandardMessage = serde_json::from_value(payload.clone())
        .map_err(|e| format!("Failed to parse standard message: {}", e))?;
    
    let mut message = CreateMessage::default();
    
    // Add content if present
    if let Some(content) = &standard_msg.content {
        message = message.content(content);
    }
    
    // Add embeds if present
    if let Some(embeds) = &standard_msg.embeds {
        for embed in embeds {
            if let Ok(embed_obj) = create_embed_from_json(embed) {
                message = message.add_embed(embed_obj);
            }
        }
    }
    
    // Add action rows if present
    if let Some(components) = &standard_msg.components {
        for component in components {
            if let Ok(action_row) = create_action_row_from_json(component) {
                message = message.components(vec![action_row]);
            }
        }
    }
    
    // Send via the appropriate method
    match delivery {
        DeliveryMethod::DirectMessage(channel_id) | DeliveryMethod::Channel(channel_id) => {
            let _result = channel_id
                .send_message(&ctx.http, message)
                .await
                .map_err(|e| format!("Failed to send message: {}", e))?;
            Ok(())
        }
        DeliveryMethod::Webhook(_) => {
            Err("Standard messages should use webhooks via Components V2 or direct channel send".to_string())
        }
        DeliveryMethod::InteractionResponse => {
            Err("Interaction response requires different context".to_string())
        }
    }
}

/// Builds a CreateEmbed from JSON data
fn create_embed_from_json(json: &Value) -> Result<CreateEmbed, String> {
    let mut embed = CreateEmbed::default();
    
    if let Some(title) = json.get("title").and_then(|t| t.as_str()) {
        embed = embed.title(title);
    }
    
    if let Some(description) = json.get("description").and_then(|d| d.as_str()) {
        embed = embed.description(description);
    }
    
    if let Some(colour) = json.get("color").and_then(|c| c.as_u64()) {
        embed = embed.colour(Colour(colour as u32));
    }
    
    if let Some(fields) = json.get("fields").and_then(|f| f.as_array()) {
        for field in fields {
            if let (Some(name), Some(value)) = (
                field.get("name").and_then(|n| n.as_str()),
                field.get("value").and_then(|v| v.as_str()),
            ) {
                let inline = field.get("inline").and_then(|i| i.as_bool()).unwrap_or(true);
                embed = embed.field(name, value, inline);
            }
        }
    }
    
    Ok(embed)
}

/// Builds a CreateActionRow from JSON data
fn create_action_row_from_json(json: &Value) -> Result<CreateActionRow, String> {
    let comp_type = json.get("type").and_then(|t| t.as_u64()).unwrap_or(0);
    
    match comp_type {
        1 => {
            // Action row - process its components
            if let Some(components) = json.get("components").and_then(|c| c.as_array()) {
                for component in components {
                    match component.get("type").and_then(|t| t.as_u64()).unwrap_or(0) {
                        2 => {
                            // Button
                            if let (Some(label), Some(custom_id)) = (
                                component.get("label").and_then(|l| l.as_str()),
                                component.get("custom_id").and_then(|id| id.as_str()),
                            ) {
                                let mut button = CreateButton::new(custom_id).label(label);
                                
                                if let Some(style) = component.get("style").and_then(|s| s.as_u64()) {
                                    button = button.style(match style {
                                        1 => serenity::all::ButtonStyle::Primary,
                                        2 => serenity::all::ButtonStyle::Secondary,
                                        3 => serenity::all::ButtonStyle::Success,
                                        4 => serenity::all::ButtonStyle::Danger,
                                        _ => serenity::all::ButtonStyle::Primary,
                                    });
                                }
                                
                                if let Some(disabled) = component.get("disabled").and_then(|d| d.as_bool()) {
                                    button = button.disabled(disabled);
                                }
                                
                                return Ok(CreateActionRow::Buttons(vec![button]));
                            }
                        }
                        3 => {
                            // Select menu
                            if let (Some(custom_id), Some(options)) = (
                                component.get("custom_id").and_then(|id| id.as_str()),
                                component.get("options").and_then(|o| o.as_array()),
                            ) {
                                let mut menu_options = Vec::new();
                                for option in options {
                                    if let (Some(label), Some(value)) = (
                                        option.get("label").and_then(|l| l.as_str()),
                                        option.get("value").and_then(|v| v.as_str()),
                                    ) {
                                        let mut menu_option = CreateSelectMenuOption::new(label, value);
                                        if let Some(desc) = option.get("description").and_then(|d| d.as_str()) {
                                            menu_option = menu_option.description(desc);
                                        }
                                        menu_options.push(menu_option);
                                    }
                                }
                                
                                if !menu_options.is_empty() {
                                    let select = CreateSelectMenu::new(
                                        custom_id,
                                        CreateSelectMenuKind::String { options: menu_options }
                                    );
                                    return Ok(CreateActionRow::SelectMenu(select));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err("Empty action row".to_string())
        }
        _ => Err("Invalid component type for action row".to_string()),
    }
}

/// Sends a Components V2 message as an interaction response
/// 
/// This sends the payload directly to Discord's webhook endpoint using the interaction token.
/// Used for slash commands and other interactions with Components V2.
/// 
/// # Arguments
/// * `ctx` - Serenity context for API access
/// * `interaction_id` - The interaction ID from the command
/// * `interaction_token` - The interaction token from the command
/// * `payload` - The Components V2 JSON payload
pub async fn send_components_v2_interaction_response(
    _ctx: &Context,
    application_id: u64,
    interaction_token: &str,
    payload: &Value,
) -> Result<(), String> {
    // Use the webhook endpoint to edit the deferred message (no new acknowledgement needed)
    let url = format!(
        "https://discord.com/api/v10/webhooks/{}/{}/messages/@original",
        application_id, interaction_token
    );
    
    let client = reqwest::Client::new();
    
    // Edit the original deferred response with the Components V2 message
    let response = client
        .patch(&url)
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Webhook request failed: {}", e))?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Webhook returned status {}: {}", status, error_text))
    }
}

/// Sends a Components V2 message directly to an interaction callback (no defer needed)
/// 
/// Uses the interaction response endpoint to directly respond to the slash command.
/// This works in all contexts including DMs.
pub async fn send_components_v2_direct_interaction_response(
    _ctx: &Context,
    interaction_id: u64,
    interaction_token: &str,
    payload: &Value,
) -> Result<(), String> {
    // Use the interaction callback endpoint directly
    let url = format!(
        "https://discord.com/api/v10/interactions/{}/{}/callback",
        interaction_id, interaction_token
    );
    
    let client = reqwest::Client::new();
    
    // Respond with type 4 (message response) containing the Components V2 data
    let callback_payload = serde_json::json!({
        "type": 4,
        "data": payload
    });
    
    let response = client
        .post(&url)
        .json(&callback_payload)
        .send()
        .await
        .map_err(|e| format!("Interaction callback request failed: {}", e))?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Interaction response failed with status {}: {}", status, error_text))
    }
}

/// Loads a message from a JSON file
/// 
/// # Arguments
/// * `file_path` - Path to the JSON file containing the message data
pub fn load_message_from_file(file_path: &str) -> Result<Value, String> {
    let data = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read message file: {}", e))?;
    
    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse message JSON: {}", e))
}

/// Sends a message from a JSON file
pub async fn send_message_from_file(
    ctx: &Context,
    msg: &Message,
    file_path: &str,
    delivery: DeliveryMethod,
) -> Result<(), String> {
    let payload = load_message_from_file(file_path)?;
    send_message(ctx, msg, payload, delivery).await
}