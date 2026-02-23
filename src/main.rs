//! # Discord Mod Mail Bot
//!
//! A Discord bot for handling mod mail functionality with DM-based support tickets.
//! Uses a universal message system supporting both Standard and Components V2 formats.

use std::env;
use discord_bot;
use serenity::{
    async_trait,
    model::{gateway::Ready, application::Interaction},
    prelude::*,
    model::gateway::GatewayIntents,
};

/// Event handler for Discord bot events
/// 
/// Implements the `EventHandler` trait to respond to Discord events including
/// direct messages and interactions.
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    /*
        When a player directly messages the bot, uses the universal message system
        to send the mod mail interface.
     */
    async fn message(&self, ctx: Context, msg: serenity::model::channel::Message) {
        // Only respond to direct messages
        if msg.guild_id.is_none() {
            /* Load and send mod mail interface from JSON */
            if let Ok(payload) = discord_bot::message::load_message_from_file("src/messaged/embed.json") {
                let delivery = discord_bot::message::DeliveryMethod::DirectMessage(msg.channel_id);
                let _ = discord_bot::message::send_message(&ctx, &msg, payload, delivery).await;
            }
        }
    }

    /// Processes slash commands and component interactions from users.
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => {
                match command.data.name.as_str() {
                    "ping" => {
                        discord_bot::ping::run(&ctx, &interaction).await;
                    }
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                /* Handle component interactions (buttons, dropdowns, etc.) */
                match component.data.custom_id.as_str() {
                    /* Dropdown for selecting message type */
                    "mod_mail_type" => {
                        if let Err(why) = component.defer(&ctx.http).await {
                            println!("Error deferring component interaction: {}", why);
                            return;
                        }
                        
                        /* This would enable the Create Message button */
                        /* TODO: Update message to enable Create Message button */
                    },
                    /* Cancel button */
                    id if id.contains("cancel") => {
                        if let Err(why) = component.defer(&ctx.http).await {
                            println!("Error deferring cancel interaction: {}", why);
                            return;
                        }
                        
                        /* TODO: Disable all buttons on message */
                    },
                    /* Create Message button */
                    id if id.contains("create") => {
                        if let Err(why) = component.defer(&ctx.http).await {
                            println!("Error deferring create interaction: {}", why);
                            return;
                        }
                        
                        /* TODO: Handle modal or message creation */
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Handles the ready event when the bot connects
    /// 
    /// Registers global slash commands and confirms the bot is online.
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        
        // Register slash commands
        let commands = vec![
            discord_bot::ping::register(),
        ];
        
        let _ = serenity::model::application::Command::set_global_commands(&ctx.http, commands).await;
    }
}

/// Main entry point for the Discord bot
/// 
/// Initializes the bot client with required intents and event handler,
/// then starts listening for Discord events.
#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    // Gateway intents required by the bot:
    // - DIRECT_MESSAGES: Listen to direct messages
    // - GUILD_MESSAGES: Listen to messages in guilds  
    // - MESSAGE_CONTENT: Receive message content in events
    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
