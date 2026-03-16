use discord_bot;
use serenity::{
    async_trait,
    model::{gateway::Ready, application::Interaction},
    prelude::*,
    model::gateway::GatewayIntents,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: serenity::model::channel::Message) {
        if msg.guild_id.is_some() || msg.author.bot {
            return; // Ignore guild messages and bot messages (including ourselves)
        }
        println!("[gateway] DM from {} (channel {})", msg.author.name, msg.channel_id);
        discord_bot::messaged::run(&ctx, &msg).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => {
                println!("[gateway] Command '{}' from {}", command.data.name, command.user.name);
                match command.data.name.as_str() {
                    "ping"           => discord_bot::ping::run(&ctx, &interaction).await,
                    "setup"          => discord_bot::setup::run(&ctx, command).await,
                    "reset-defaults" => discord_bot::setup::run_reset(&ctx, command).await,
                    "template-edit"  => discord_bot::builder::run(&ctx, command).await,
                    "Report Player"  => discord_bot::report::run(&ctx, command).await,
                    name => println!("[gateway] Unknown command: {}", name),
                }
            }

            Interaction::Component(component) => {
                println!(
                    "[gateway] Component '{}' from {} in channel {}",
                    component.data.custom_id, component.user.name, component.channel_id
                );
                match component.data.custom_id.as_str() {
                    // Setup wizard components
                    discord_bot::setup::SETUP_BOT_NAME_BTN
                    | discord_bot::setup::SETUP_AVATAR_BTN
                    | discord_bot::setup::SETUP_CHANNEL_SELECT
                    | discord_bot::setup::SETUP_LOG_CHANNEL
                    | discord_bot::setup::SETUP_METHOD_SELECT
                    | discord_bot::setup::SETUP_REPORTS_SELECT
                    | discord_bot::setup::SETUP_SAVE_BTN
                    | discord_bot::setup::SETUP_CANCEL_BTN
                    | discord_bot::setup::SETUP_RESET_BTN
                    | discord_bot::setup::SETUP_EDIT_TPL_BTN => {
                        discord_bot::setup::handle_component(&ctx, component).await;
                    }
                    // Template builder components
                    id if discord_bot::builder::is_template_editor_component(id) => {
                        discord_bot::builder::handle_component(&ctx, component).await;
                    }
                    // Mod mail components
                    discord_bot::messaged::DROPDOWN_ID => {
                        discord_bot::messaged::handle_select(&ctx, component).await;
                    }
                    discord_bot::messaged::CANCEL_ID => {
                        discord_bot::messaged::handle_cancel(&ctx, component).await;
                    }
                    discord_bot::messaged::CREATE_ID => {
                        discord_bot::messaged::handle_create(&ctx, component).await;
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
                    discord_bot::setup::MODAL_BOT_NAME
                    | discord_bot::setup::MODAL_AVATAR => {
                        discord_bot::setup::handle_modal(&ctx, modal).await;
                    }
                    // Template builder modals
                    id if discord_bot::builder::is_template_editor_modal(id) => {
                        discord_bot::builder::handle_modal(&ctx, modal).await;
                    }
                    // Mod mail modal
                    discord_bot::messaged::MODAL_ID => {
                        discord_bot::messaged::handle_modal(&ctx, modal).await;
                    }
                    // Report modal (prefixed with target user id)
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
        println!("[gateway] {} connected (shard {})", ready.user.name, ctx.shard_id);

        let commands = vec![
            discord_bot::ping::register(),
            discord_bot::setup::register(),
            discord_bot::setup::register_reset(),
            discord_bot::builder::register(),
            discord_bot::report::register(),
        ];
        if let Err(e) = serenity::model::application::Command::set_global_commands(&ctx.http, commands).await {
            println!("[gateway] ERROR registering commands: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    let secrets_json = std::fs::read_to_string("secrets.json")
        .expect("Expected a secrets file")
        .parse::<serde_json::Value>()
        .expect("Expected valid JSON");

    let token = secrets_json
        .get("discord-bot")
        .and_then(|v| v.get("token"))
        .and_then(|v| v.as_str())
        .expect("Expected a token in the secrets file")
        .to_owned();

    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("[gateway] Client error: {:?}", why);
    }
}
