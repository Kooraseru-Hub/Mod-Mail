//! Webhook and message delivery module
//!
//! Handles sending messages via DMs, channel messages, or Discord webhooks.
//! Supports multiple delivery methods: direct messages, guild channels, and webhook URLs.

use serenity::{
    builder::CreateMessage, model::channel::Message, prelude::Context
};

/// Sends a message via the specified delivery method
/// 
/// # Arguments
/// * `ctx` - Serenity context for API access
/// * `msg` - The original message (for DM responses)
/// * `message_builder` - The message to send
/// * `delivery_method` - Where to send the message (DM, Channel, or Webhook)
/// 
/// # Returns
/// Result indicating success or failure of message delivery
pub async fn send_message(
    ctx: &Context,
    msg: &Message,
    message_builder: CreateMessage,
    delivery_method: DeliveryMethod,
) -> Result<(), String> {
    match delivery_method {
        /* Send as direct message (reply to user) */
        DeliveryMethod::DirectMessage => {
            msg.channel_id
                .send_message(&ctx.http, message_builder)
                .await
                .map_err(|e| format!("Error sending DM: {}", e))?;
            Ok(())
        }
        
        /* Send to a specific channel */
        DeliveryMethod::Channel(channel_id) => {
            channel_id
                .send_message(&ctx.http, message_builder)
                .await
                .map_err(|e| format!("Error sending channel message: {}", e))?;
            Ok(())
        }
        
        /* Send via webhook URL */
        DeliveryMethod::Webhook(_webhook_url) => {
            /* TODO: Implement webhook sending when needed */
            /* This would use reqwest to send to the webhook URL */
            Err("Webhook delivery not yet implemented".to_string())
        }
    }
}

/// Specifies where a message should be delivered
#[derive(Clone, Debug)]
pub enum DeliveryMethod {
    /// Send as a direct message to the user
    DirectMessage,
    
    /// Send to a specific channel
    Channel(serenity::all::ChannelId),
    
    /// Send via a Discord webhook URL
    Webhook(String),
}
