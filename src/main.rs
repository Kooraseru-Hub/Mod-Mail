use discord_bot;
use discord_bot::storage::{StorageBackend, StorageKey};
use serenity::{
    async_trait,
    model::gateway::GatewayIntents,
    model::{application::Interaction, gateway::Ready},
    prelude::*,
};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: serenity::model::channel::Message) {
        if msg.guild_id.is_some() || msg.author.bot {
            return; // Ignore guild messages and bot messages (including ourselves)
        }
        println!(
            "[gateway] DM from {} (channel {})",
            msg.author.name, msg.channel_id
        );
        discord_bot::messaged::run(&ctx, &msg).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => {
                println!(
                    "[gateway] Command '{}' from {}",
                    command.data.name, command.user.name
                );
                match command.data.name.as_str() {
                    "ping" => discord_bot::ping::run(&ctx, &interaction).await,
                    "setup" => discord_bot::setup::run(&ctx, command).await,
                    "reset" => discord_bot::setup::run_reset(&ctx, command).await,
                    "settings" => discord_bot::setup::run_settings(&ctx, command).await,
                    "message" => discord_bot::messaged::run_message(&ctx, command).await,
                    "Report Player" => discord_bot::report::run(&ctx, command).await,
                    name => println!("[gateway] Unknown command: {}", name),
                }
            }

            Interaction::Component(component) => {
                println!(
                    "[gateway] Component '{}' from {} in channel {}",
                    component.data.custom_id, component.user.name, component.channel_id
                );
                match component.data.custom_id.as_str() {
                    discord_bot::setup::SETUP_BOT_NAME_BTN
                    | discord_bot::setup::SETUP_CHANNEL_SELECT
                    | discord_bot::setup::SETUP_LOG_CHANNEL
                    | discord_bot::setup::SETUP_METHOD_SELECT
                    | discord_bot::setup::SETUP_REPORTS_SELECT
                    | discord_bot::setup::SETUP_SAVE_BTN
                    | discord_bot::setup::SETUP_CANCEL_BTN
                    | discord_bot::setup::SETUP_RESET_BTN
                    | discord_bot::setup::SETUP_EDIT_TPL_BTN
                    | discord_bot::setup::SETTINGS_SELECT
                    | discord_bot::setup::SETTINGS_BACK_BTN => {
                        discord_bot::setup::handle_component(&ctx, component).await;
                    }
                    // Template builder components
                    id if discord_bot::builder::is_template_editor_component(id) => {
                        discord_bot::builder::handle_component(&ctx, component).await;
                    }
                    id if discord_bot::messaged::is_modmail_component(id) => {
                        discord_bot::messaged::handle_component(&ctx, component).await;
                    }
                    id => println!("[gateway] Unhandled component: {}", id),
                }
            }

            Interaction::Modal(modal) => {
                println!(
                    "[gateway] Modal '{}' from {} in channel {}",
                    modal.data.custom_id, modal.user.name, modal.channel_id
                );
                match modal.data.custom_id.as_str() {
                    // Setup modals
                    discord_bot::setup::MODAL_BOT_NAME => {
                        discord_bot::setup::handle_modal(&ctx, modal).await;
                    }
                    // Template builder modals
                    id if discord_bot::builder::is_template_editor_modal(id) => {
                        discord_bot::builder::handle_modal(&ctx, modal).await;
                    }
                    id if discord_bot::messaged::is_modmail_modal(id) => {
                        discord_bot::messaged::handle_modal(&ctx, modal).await;
                    }
                    id if id.starts_with(discord_bot::report::REPORT_MODAL_ID) => {
                        discord_bot::report::handle_modal(&ctx, modal).await;
                    }
                    id => println!("[gateway] Unhandled modal: {}", id),
                }
            }

            _ => {}
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!(
            "[gateway] {} connected (shard {})",
            ready.user.name, ctx.shard_id
        );

        let commands = vec![
            discord_bot::ping::register(),
            discord_bot::setup::register(),
            discord_bot::setup::register_reset(),
            discord_bot::setup::register_settings(),
            discord_bot::messaged::register_message(),
            discord_bot::report::register(),
        ];
        if let Err(e) =
            serenity::model::application::Command::set_global_commands(&ctx.http, commands).await
        {
            println!("[gateway] ERROR registering commands: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").unwrap_or_else(|_| {
        let secrets_json = std::fs::read_to_string(".secrets/secrets.json")
            .expect("Expected either a DISCORD_TOKEN env var or a secrets file")
            .parse::<serde_json::Value>()
            .expect("Expected valid JSON");

        secrets_json
            .get("discord-bot")
            .and_then(|v| v.get("token"))
            .and_then(|v| v.as_str())
            .expect("Expected a token in the secrets file")
            .to_owned()
    });

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    tokio::spawn(async move {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .expect("Failed to bind health check port");
        println!("[health] Listening on port {}", port);
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let _ = socket
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
                        .await;
                });
            }
        }
    });

    let storage = Arc::new(StorageBackend::from_env());

    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<StorageKey>(storage);
    }

    if let Err(why) = client.start().await {
        println!("[gateway] Client error: {:?}", why);
    }
}
