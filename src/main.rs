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
    // Handle messages in DMs to trigger the mod mail process
    async fn message(&self, ctx: Context, msg: serenity::model::channel::Message) {
        println!("Received message: {}", msg.content);
        if msg.guild_id.is_none() {
            if let Ok(payload) = discord_bot::message::load_message_from_file("src/messaged/embed.json") {
                let delivery = discord_bot::message::DeliveryMethod::DirectMessage(msg.channel_id);
                let _ = discord_bot::message::send_message(&ctx, &msg, payload, delivery).await;
            }
        }
    }

    // Handle interactions for slash commands and component interactions
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
                match component.data.custom_id.as_str() {
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

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        
        // Register slash commands
        let commands = vec![
            discord_bot::ping::register(),
        ];
        
        let _ = serenity::model::application::Command::set_global_commands(&ctx.http, commands).await;
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
        println!("Client error: {:?}", why);
    }
}
