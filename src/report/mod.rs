use serenity::{
    all::{ActionRowComponent, InputTextStyle},
    builder::{
        CreateActionRow, CreateCommand, CreateInputText, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateModal,
    },
    model::{
        application::{CommandInteraction, CommandType, ModalInteraction},
    },
    prelude::Context,
};
use crate::config::GuildConfig;

pub const REPORT_MODAL_ID: &str      = "report_player_modal";
pub const REPORT_REASON_INPUT: &str   = "report_reason_input";

pub fn register() -> CreateCommand {
    CreateCommand::new("Report Player")
        .kind(CommandType::User)
        .dm_permission(false)
}

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.get(),
        None => {
            let resp = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("This can only be used in a server.")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, resp).await;
            return;
        }
    };

    let config = GuildConfig::load(guild_id);
    if !config.reports_enabled {
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Player reports are not enabled in this server.")
                .ephemeral(true),
        );
        let _ = command.create_response(&ctx.http, resp).await;
        return;
    }

    let target_user = command
        .data
        .target_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let modal = CreateInteractionResponse::Modal(
        CreateModal::new(
            format!("{}:{}", REPORT_MODAL_ID, target_user),
            "Report Player",
        )
        .components(vec![CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Paragraph, "Reason", REPORT_REASON_INPUT)
                .placeholder("Describe why you are reporting this player...")
                .required(true)
                .min_length(10)
                .max_length(2000),
        )]),
    );

    if let Err(e) = command.create_response(&ctx.http, modal).await {
        println!("[report] ERROR create_response: {}", e);
    }
}

pub async fn handle_modal(ctx: &Context, modal: &ModalInteraction) {
    let guild_id = match modal.guild_id {
        Some(id) => id.get(),
        None => return,
    };

    let config = GuildConfig::load(guild_id);

    let target_user_id = modal
        .data
        .custom_id
        .strip_prefix(&format!("{}:", REPORT_MODAL_ID))
        .unwrap_or("Unknown");

    let reason = modal
        .data
        .components
        .iter()
        .flat_map(|row| row.components.iter())
        .find_map(|c| {
            if let ActionRowComponent::InputText(it) = c {
                if it.custom_id == REPORT_REASON_INPUT {
                    return it.value.clone();
                }
            }
            None
        })
        .unwrap_or_default();

    let template = config.get_template(crate::templates::OPTION_PLAYER_REPORT);

    let mut rendered_description = template.description.clone();
    rendered_description = rendered_description.replace("{reporter}", &format!("<@{}>", modal.user.id));
    rendered_description = rendered_description.replace("{reported_user}", &format!("<@{}>", target_user_id));
    rendered_description = rendered_description.replace("{reason}", &reason);

    let embed = serenity::builder::CreateEmbed::new()
        .title(&template.title)
        .description(&rendered_description)
        .color(template.color);

    let embed = template.fields.iter().fold(embed, |e, field| {
        let value = field
            .value
            .replace("{reporter}", &format!("<@{}>", modal.user.id))
            .replace("{reported_user}", &format!("<@{}>", target_user_id))
            .replace("{reason}", &reason);
        e.field(&field.name, &value, field.inline)
    });

    if let Some(channel_id) = config.log_channel_id.or(config.message_channel_id) {
        let channel = serenity::all::ChannelId::new(channel_id);
        let msg = serenity::builder::CreateMessage::new().embed(embed);
        match channel.send_message(&ctx.http, msg).await {
            Ok(_) => {
                let resp = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Your report has been submitted. Thank you.")
                        .ephemeral(true),
                );
                let _ = modal.create_response(&ctx.http, resp).await;
            }
            Err(e) => {
                println!("[report] ERROR send: {}", e);
                let resp = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Failed to submit report. Please try again later.")
                        .ephemeral(true),
                );
                let _ = modal.create_response(&ctx.http, resp).await;
            }
        }
    } else {
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("No report channel has been configured. Ask an admin to run `/setup`.")
                .ephemeral(true),
        );
        let _ = modal.create_response(&ctx.http, resp).await;
    }
}
