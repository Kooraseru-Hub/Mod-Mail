use serenity::{
    all::{ComponentInteractionDataKind, ActionRowComponent, InputTextStyle, CommandInteraction},
    builder::{CreateInputText, CreateModal, CreateActionRow, CreateInteractionResponse,
              CreateInteractionResponseMessage, CreateCommand},
    model::{
        application::{ComponentInteraction, ModalInteraction},
        channel::Message,
        id::{GuildId, UserId},
    },
    prelude::Context,
};
use crate::config::{GuildConfig, MessageMethod};
use crate::message;

pub const GUILD_SELECT_ID: &str = "mm_guild_select";
pub const CANCEL_ID: &str = "mm_cancel";

const OPTION_PREFIX: &str = "mm_option:";
const CREATE_PREFIX: &str = "mm_create:";
const MODAL_GENERAL_PREFIX: &str = "mm_modal_general:";
const MODAL_REPORT_PREFIX: &str = "mm_modal_report:";

const INPUT_TARGET_USER: &str = "mm_input_target";
const INPUT_REASON: &str = "mm_input_reason";
const INPUT_EVIDENCE: &str = "mm_input_evidence";

pub fn is_modmail_component(id: &str) -> bool {
    id == GUILD_SELECT_ID
        || id == CANCEL_ID
        || id.starts_with(OPTION_PREFIX)
        || id.starts_with(CREATE_PREFIX)
}

pub fn is_modmail_modal(id: &str) -> bool {
    id.starts_with(MODAL_GENERAL_PREFIX)
        || id.starts_with(MODAL_REPORT_PREFIX)
}

struct GuildInfo {
    guild_id: u64,
    guild_name: String,
    config: GuildConfig,
}

async fn find_user_guilds(ctx: &Context, user_id: UserId) -> Vec<GuildInfo> {
    let mut guilds = Vec::new();
    let guild_dir = std::path::Path::new("data/guilds");
    let entries = match std::fs::read_dir(guild_dir) {
        Ok(e) => e,
        Err(_) => return guilds,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let guild_id: u64 = match path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.parse().ok())
        {
            Some(id) => id,
            None => continue,
        };

        let config = GuildConfig::load(guild_id);

        if matches!(config.message_method, MessageMethod::None | MessageMethod::Interaction) {
            continue;
        }

        let gid = GuildId::new(guild_id);
        if ctx.http.get_member(gid, user_id).await.is_err() {
            continue;
        }

        let guild_name = ctx
            .http
            .get_guild(gid)
            .await
            .map(|g| g.name.clone())
            .unwrap_or_else(|_| format!("Server {}", guild_id));

        guilds.push(GuildInfo {
            guild_id,
            guild_name,
            config,
        });
    }

    guilds
}

pub async fn run(ctx: &Context, msg: &Message) {
    println!("[modmail] DM from {} (channel {})", msg.author.name, msg.channel_id);

    let guilds = find_user_guilds(ctx, msg.author.id).await;

    if guilds.is_empty() {
        let payload = build_no_servers_message();
        let delivery = message::DeliveryMethod::DirectMessage(msg.channel_id);
        let _ = message::send_message(ctx, msg, payload, delivery).await;
        return;
    }

    let payload = if guilds.len() == 1 {
        let g = &guilds[0];
        build_option_select_message(g.guild_id, &g.guild_name, &g.config)
    } else {
        build_guild_select_message(&guilds)
    };

    let delivery = message::DeliveryMethod::DirectMessage(msg.channel_id);
    match message::send_message(ctx, msg, payload, delivery).await {
        Ok(()) => println!("[modmail] Menu sent OK"),
        Err(e) => println!("[modmail] ERROR send_message: {}", e),
    }
}

pub fn register_message() -> CreateCommand {
    CreateCommand::new("message")
        .description("Send a message to the moderation team")
        .dm_permission(false)
}

pub async fn run_message(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.get(),
        None => {
            let resp = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("This command can only be used in a server.")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, resp).await;
            return;
        }
    };

    let config = GuildConfig::load(guild_id);

    if matches!(config.message_method, crate::config::MessageMethod::DMs | crate::config::MessageMethod::None) {
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Slash command messaging is not enabled in this server.")
                .ephemeral(true),
        );
        let _ = command.create_response(&ctx.http, resp).await;
        return;
    }

    if let Err(e) = command.defer_ephemeral(&ctx.http).await {
        println!("[modmail:message] ERROR defer: {}", e);
        return;
    }

    let guild_name = ctx
        .http
        .get_guild(GuildId::new(guild_id))
        .await
        .map(|g| g.name.clone())
        .unwrap_or_else(|_| format!("Server {}", guild_id));

    let payload = build_option_select_message(guild_id, &guild_name, &config);

    let _ = message::send_components_v2_interaction_response(
        ctx,
        command.application_id.get(),
        &command.token,
        &payload,
    )
    .await;
}

pub async fn handle_component(ctx: &Context, component: &ComponentInteraction) {
    let id = component.data.custom_id.as_str();

    if id == GUILD_SELECT_ID {
        handle_guild_select(ctx, component).await;
    } else if id == CANCEL_ID {
        handle_cancel(ctx, component).await;
    } else if let Some(guild_id_str) = id.strip_prefix(OPTION_PREFIX) {
        handle_option_select(ctx, component, guild_id_str).await;
    } else if let Some(rest) = id.strip_prefix(CREATE_PREFIX) {
        handle_create(ctx, component, rest).await;
    }
}

pub async fn handle_modal(ctx: &Context, modal: &ModalInteraction) {
    let id = modal.data.custom_id.as_str();

    if let Some(guild_id_str) = id.strip_prefix(MODAL_GENERAL_PREFIX) {
        handle_general_modal(ctx, modal, guild_id_str).await;
    } else if let Some(guild_id_str) = id.strip_prefix(MODAL_REPORT_PREFIX) {
        handle_report_modal(ctx, modal, guild_id_str).await;
    }
}

async fn handle_guild_select(ctx: &Context, component: &ComponentInteraction) {
    let guild_id_str = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => return,
    };

    let guild_id: u64 = match guild_id_str.parse() {
        Ok(id) => id,
        Err(_) => return,
    };

    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        println!("[modmail:guild_select] ERROR acknowledge: {}", e);
        return;
    }

    let config = GuildConfig::load(guild_id);
    let guild_name = ctx
        .http
        .get_guild(GuildId::new(guild_id))
        .await
        .map(|g| g.name.clone())
        .unwrap_or_else(|_| format!("Server {}", guild_id));

    let payload = build_option_select_message(guild_id, &guild_name, &config);

    let _ = message::send_components_v2_interaction_response(
        ctx,
        component.application_id.get(),
        &component.token,
        &payload,
    )
    .await;
}

async fn handle_option_select(ctx: &Context, component: &ComponentInteraction, guild_id_str: &str) {
    let guild_id: u64 = match guild_id_str.parse() {
        Ok(id) => id,
        Err(_) => return,
    };

    let selected = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => return,
    };

    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        println!("[modmail:option_select] ERROR acknowledge: {}", e);
        return;
    }

    let config = GuildConfig::load(guild_id);
    let guild_name = ctx
        .http
        .get_guild(GuildId::new(guild_id))
        .await
        .map(|g| g.name.clone())
        .unwrap_or_else(|_| format!("Server {}", guild_id));

    let payload = build_option_selected_message(guild_id, &guild_name, &config, &selected);

    let _ = message::send_components_v2_interaction_response(
        ctx,
        component.application_id.get(),
        &component.token,
        &payload,
    )
    .await;
}

async fn handle_create(ctx: &Context, component: &ComponentInteraction, rest: &str) {
    let (guild_id_str, option) = match rest.split_once(':') {
        Some(pair) => pair,
        None => return,
    };

    let guild_id: u64 = match guild_id_str.parse() {
        Ok(id) => id,
        Err(_) => return,
    };

    let modal = match option {
        "player_report" => CreateInteractionResponse::Modal(
            CreateModal::new(
                format!("{}{}", MODAL_REPORT_PREFIX, guild_id_str),
                "Report Player",
            )
            .components(vec![
                CreateActionRow::InputText(
                    CreateInputText::new(
                        InputTextStyle::Short,
                        "Reported Player (Username or ID)",
                        INPUT_TARGET_USER,
                    )
                    .placeholder("e.g. Player123 or 123456789")
                    .required(true)
                    .max_length(100),
                ),
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "Reason", INPUT_REASON)
                        .placeholder("e.g. Cheating, Harassment, Hate Speech, Exploiting")
                        .required(true)
                        .max_length(200),
                ),
                CreateActionRow::InputText(
                    CreateInputText::new(
                        InputTextStyle::Paragraph,
                        "Explanation & Evidence",
                        INPUT_EVIDENCE,
                    )
                    .placeholder(
                        "Provide details and any links to evidence (screenshots, clips, etc.)",
                    )
                    .required(true)
                    .min_length(10)
                    .max_length(2000),
                ),
            ]),
        ),
        _ => {
            let config = GuildConfig::load(guild_id);
            let template = config.get_template(option);
            let label = crate::templates::display_name(option);
            let modal_title = if label.len() > 45 {
                format!("{}...", &label[..42])
            } else {
                label
            };

            let mut inputs: Vec<CreateActionRow> = template
                .fields
                .iter()
                .filter(|f| !is_auto_fill_only(&f.value))
                .enumerate()
                .take(5)
                .map(|(i, field)| {
                    let placeholder = if is_template_placeholder(&field.value) {
                        format!("Enter {}", field.name.to_lowercase())
                    } else if field.value.len() > 100 {
                        format!("{}...", &field.value[..97])
                    } else {
                        field.value.clone()
                    };
                    let style = if use_paragraph_style(&field.name) {
                        InputTextStyle::Paragraph
                    } else {
                        InputTextStyle::Short
                    };
                    CreateActionRow::InputText(
                        CreateInputText::new(style, &field.name, format!("mm_field_{}", i))
                            .placeholder(&placeholder)
                            .required(true)
                            .max_length(1024),
                    )
                })
                .collect();

            if inputs.is_empty() {
                inputs.push(CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Paragraph, "Your Message", "mm_field_0")
                        .placeholder("Describe your issue in detail. You can include links.")
                        .required(true)
                        .min_length(10)
                        .max_length(2000),
                ));
            }

            CreateInteractionResponse::Modal(
                CreateModal::new(
                    format!("{}{}:{}", MODAL_GENERAL_PREFIX, guild_id_str, option),
                    modal_title,
                )
                .components(inputs),
            )
        }
    };

    if let Err(e) = component.create_response(&ctx.http, modal).await {
        println!("[modmail:create] ERROR: {}", e);
    }
}

async fn handle_cancel(ctx: &Context, component: &ComponentInteraction) {
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        println!("[modmail:cancel] ERROR acknowledge: {}", e);
        return;
    }

    let payload = serde_json::json!({
        "flags": 32768,
        "components": [{
            "type": 17,
            "components": [{
                "type": 10,
                "content": "## Mod Mail Cancelled\nYou can start a new session by sending another message."
            }],
            "accent_color": 15548997
        }]
    });

    let _ = message::send_components_v2_interaction_response(
        ctx,
        component.application_id.get(),
        &component.token,
        &payload,
    )
    .await;
}

async fn handle_general_modal(ctx: &Context, modal: &ModalInteraction, rest: &str) {
    let (guild_id_str, option_key) = match rest.split_once(':') {
        Some((gid, opt)) => (gid, opt),
        None => (rest, "general_support"),
    };

    let guild_id: u64 = match guild_id_str.parse() {
        Ok(id) => id,
        Err(_) => return,
    };

    if let Err(e) = modal.defer(&ctx.http).await {
        println!("[modmail:modal] ERROR defer: {}", e);
        return;
    }

    let config = GuildConfig::load(guild_id);
    let template = config.get_template(option_key);
    let display = crate::templates::display_name(option_key);
    let author_mention = format!("<@{}>", modal.user.id);

    let channel_id = match config.option_channel(option_key) {
        Some(id) => id,
        None => {
            let _ = send_followup_v2(
                ctx,
                modal.application_id.get(),
                &modal.token,
                "## Error\nNo channel has been configured for this server. Ask an admin to run `/setup`.",
                15548997,
            )
            .await;
            return;
        }
    };

    let channel = serenity::all::ChannelId::new(channel_id);

    let user_input_count = template
        .fields
        .iter()
        .filter(|f| !is_auto_fill_only(&f.value))
        .take(5)
        .count();

    let user_inputs: Vec<String> = (0..user_input_count)
        .map(|i| {
            extract_input(&modal.data.components, &format!("mm_field_{}", i))
                .unwrap_or_default()
        })
        .collect();

    let description = template
        .description
        .replace("{author}", &author_mention)
        .replace("{ticket_type}", &display)
        .replace("{reporter}", &author_mention);

    let mut embed = serenity::builder::CreateEmbed::new()
        .title(&template.title)
        .description(&description)
        .color(template.color);

    let mut input_idx = 0usize;
    for field in &template.fields {
        if is_auto_fill_only(&field.value) {
            let value = field
                .value
                .replace("{author}", &author_mention)
                .replace("{ticket_type}", &display)
                .replace("{reporter}", &author_mention);
            embed = embed.field(&field.name, &value, field.inline);
        } else if input_idx < user_inputs.len() {
            embed = embed.field(&field.name, &user_inputs[input_idx], field.inline);
            input_idx += 1;
        }
    }

    let msg = serenity::builder::CreateMessage::new().embed(embed);
    match channel.send_message(&ctx.http, msg).await {
        Ok(_) => {
            let _ = send_followup_v2(
                ctx,
                modal.application_id.get(),
                &modal.token,
                "## Ticket Submitted\nYour message has been sent to the moderation team. They will respond as soon as possible.",
                5763719,
            )
            .await;
        }
        Err(e) => {
            println!("[modmail:modal] ERROR send: {}", e);
            let _ = send_followup_v2(
                ctx,
                modal.application_id.get(),
                &modal.token,
                "## Error\nFailed to submit your ticket. Please try again later.",
                15548997,
            )
            .await;
        }
    }
}

async fn handle_report_modal(ctx: &Context, modal: &ModalInteraction, guild_id_str: &str) {
    let guild_id: u64 = match guild_id_str.parse() {
        Ok(id) => id,
        Err(_) => return,
    };

    let target_user = extract_input(&modal.data.components, INPUT_TARGET_USER).unwrap_or_default();
    let reason = extract_input(&modal.data.components, INPUT_REASON).unwrap_or_default();
    let evidence = extract_input(&modal.data.components, INPUT_EVIDENCE).unwrap_or_default();

    if let Err(e) = modal.defer(&ctx.http).await {
        println!("[modmail:modal] ERROR defer: {}", e);
        return;
    }

    let config = GuildConfig::load(guild_id);
    let template = config.get_template(crate::templates::OPTION_PLAYER_REPORT);

    let channel_id = match config.option_channel("player_report") {
        Some(id) => id,
        None => {
            let _ = send_followup_v2(
                ctx,
                modal.application_id.get(),
                &modal.token,
                "## Error\nNo channel has been configured for this server. Ask an admin to run `/setup`.",
                15548997,
            )
            .await;
            return;
        }
    };

    let channel = serenity::all::ChannelId::new(channel_id);

    let embed = serenity::builder::CreateEmbed::new()
        .title(&template.title)
        .description(
            &template
                .description
                .replace("{reporter}", &format!("<@{}>", modal.user.id))
                .replace("{reported_user}", &target_user)
                .replace("{reason}", &reason),
        )
        .color(template.color);

    let embed = template.fields.iter().fold(embed, |e, field| {
        let value = field
            .value
            .replace("{reporter}", &format!("<@{}>", modal.user.id))
            .replace("{reported_user}", &target_user)
            .replace("{reason}", &reason);
        e.field(&field.name, &value, field.inline)
    });

    let embed = embed.field("Evidence / Explanation", &evidence, false);

    let msg = serenity::builder::CreateMessage::new().embed(embed);
    match channel.send_message(&ctx.http, msg).await {
        Ok(_) => {
            let _ = send_followup_v2(
                ctx,
                modal.application_id.get(),
                &modal.token,
                "## Report Submitted\nYour player report has been sent to the moderation team. Thank you.",
                5763719,
            )
            .await;
        }
        Err(e) => {
            println!("[modmail:modal] ERROR send: {}", e);
            let _ = send_followup_v2(
                ctx,
                modal.application_id.get(),
                &modal.token,
                "## Error\nFailed to submit your report. Please try again later.",
                15548997,
            )
            .await;
        }
    }
}

fn build_no_servers_message() -> serde_json::Value {
    serde_json::json!({
        "flags": 32768,
        "components": [{
            "type": 17,
            "components": [{
                "type": 10,
                "content": "## Mod Mail\nYou are not in any server that has Mod Mail configured, or DM-based mod mail is disabled for all your servers."
            }],
            "accent_color": 15548997
        }]
    })
}

fn build_guild_select_message(guilds: &[GuildInfo]) -> serde_json::Value {
    let options: Vec<serde_json::Value> = guilds
        .iter()
        .map(|g| {
            serde_json::json!({
                "label": g.guild_name,
                "value": g.guild_id.to_string(),
                "description": format!("Contact the staff of {}", g.guild_name)
            })
        })
        .collect();

    serde_json::json!({
        "flags": 32768,
        "components": [{
            "type": 17,
            "components": [
                {
                    "type": 10,
                    "content": "## WELCOME TO MOD MAIL\nMod Mail allows you to contact a server's moderation team privately."
                },
                {"type": 14},
                {
                    "type": 10,
                    "content": "## SELECT A SERVER\nChoose which server you'd like to contact."
                },
                {"type": 14},
                {
                    "type": 1,
                    "components": [{
                        "type": 3,
                        "custom_id": GUILD_SELECT_ID,
                        "placeholder": "Select a Server",
                        "options": options
                    }]
                },
                {
                    "type": 1,
                    "components": [{
                        "style": 4,
                        "type": 2,
                        "label": "Cancel",
                        "custom_id": CANCEL_ID
                    }]
                }
            ],
            "accent_color": 10181046
        }]
    })
}

fn build_format_options(config: &GuildConfig, selected: Option<&str>) -> Vec<serde_json::Value> {
    config
        .all_format_names()
        .iter()
        .map(|name| {
            let label = crate::templates::display_name(name);
            let desc = match name.as_str() {
                "general_support" => "Use for anything that is not a rule violation.".to_string(),
                "player_report" => "Report another user's behavior.".to_string(),
                _ => {
                    let d = config.get_template(name).description.clone();
                    if d.len() > 100 {
                        format!("{}...", &d[..97])
                    } else {
                        d
                    }
                }
            };
            let mut opt = serde_json::json!({
                "label": label,
                "value": name,
                "description": desc
            });
            if let Some(sel) = selected {
                if sel == name {
                    opt["default"] = serde_json::json!(true);
                }
            }
            opt
        })
        .collect()
}

fn build_option_select_message(
    guild_id: u64,
    guild_name: &str,
    config: &GuildConfig,
) -> serde_json::Value {
    let option_select_id = format!("{}{}", OPTION_PREFIX, guild_id);

    let options: Vec<serde_json::Value> = build_format_options(config, None);

    serde_json::json!({
        "flags": 32768,
        "components": [{
            "type": 17,
            "components": [
                {
                    "type": 10,
                    "content": format!("## MOD MAIL — {}", guild_name)
                },
                {"type": 14},
                {
                    "type": 10,
                    "content": "## PRIVACY INFORMATION\nAll conversations are private. Only you and the moderation team can see messages in your ticket."
                },
                {"type": 14},
                {
                    "type": 10,
                    "content": "## NEXT STEP\nSelect an option from the dropdown below to continue."
                },
                {"type": 14},
                {
                    "type": 1,
                    "components": [{
                        "type": 3,
                        "custom_id": option_select_id,
                        "placeholder": "Select an Option",
                        "options": options
                    }]
                },
                {
                    "type": 1,
                    "components": [
                        {
                            "style": 4,
                            "type": 2,
                            "label": "Cancel",
                            "custom_id": CANCEL_ID
                        },
                        {
                            "style": 2,
                            "type": 2,
                            "label": "Create Message",
                            "disabled": true,
                            "custom_id": format!("{}{}:none", CREATE_PREFIX, guild_id)
                        }
                    ]
                }
            ],
            "accent_color": 10181046
        }]
    })
}

fn build_option_selected_message(
    guild_id: u64,
    guild_name: &str,
    config: &GuildConfig,
    selected_option: &str,
) -> serde_json::Value {
    let instructions = config
        .option_configs
        .get(selected_option)
        .and_then(|o| o.instructions.as_deref())
        .unwrap_or(match selected_option {
            "general_support" => {
                "Describe your issue or question in detail when you create your message."
            }
            "player_report" => {
                "You'll be asked for the player's information and the reason for your report."
            }
            _ => "Provide your details when you create your message.",
        });

    let selected_label = crate::templates::display_name(selected_option);

    let option_select_id = format!("{}{}", OPTION_PREFIX, guild_id);
    let create_id = format!("{}{}:{}", CREATE_PREFIX, guild_id, selected_option);

    let options: Vec<serde_json::Value> = build_format_options(config, Some(selected_option));

    serde_json::json!({
        "flags": 32768,
        "components": [{
            "type": 17,
            "components": [
                {
                    "type": 10,
                    "content": format!("## MOD MAIL — {}", guild_name)
                },
                {"type": 14},
                {
                    "type": 10,
                    "content": format!("## {}\n{}", selected_label, instructions)
                },
                {"type": 14},
                {
                    "type": 1,
                    "components": [{
                        "type": 3,
                        "custom_id": option_select_id,
                        "placeholder": "Select an Option",
                        "options": options
                    }]
                },
                {
                    "type": 1,
                    "components": [
                        {
                            "style": 4,
                            "type": 2,
                            "label": "Cancel",
                            "custom_id": CANCEL_ID
                        },
                        {
                            "style": 1,
                            "type": 2,
                            "label": "Create Message",
                            "disabled": false,
                            "custom_id": create_id
                        }
                    ]
                }
            ],
            "accent_color": 10181046
        }]
    })
}

async fn send_followup_v2(
    ctx: &Context,
    application_id: u64,
    interaction_token: &str,
    content: &str,
    color: u32,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "flags": 32768,
        "components": [{
            "type": 17,
            "components": [{
                "type": 10,
                "content": content
            }],
            "accent_color": color
        }]
    });

    message::send_components_v2_interaction_response(ctx, application_id, interaction_token, &payload)
        .await
}

fn is_auto_fill_only(value: &str) -> bool {
    matches!(value.trim(), "{author}" | "{ticket_type}" | "{reporter}")
}

fn is_template_placeholder(value: &str) -> bool {
    let t = value.trim();
    t.starts_with('{') && t.ends_with('}') && !t.contains(' ')
}

fn use_paragraph_style(field_name: &str) -> bool {
    let lower = field_name.to_lowercase();
    lower.contains("message")
        || lower.contains("content")
        || lower.contains("evidence")
        || lower.contains("explanation")
        || lower.contains("detail")
        || lower.contains("description")
        || lower.contains("clip")
        || lower.contains("reason")
}

fn extract_input(
    components: &[serenity::model::application::ActionRow],
    custom_id: &str,
) -> Option<String> {
    components
        .iter()
        .flat_map(|row| row.components.iter())
        .find_map(|c| {
            if let ActionRowComponent::InputText(it) = c {
                if it.custom_id == custom_id {
                    return it.value.clone();
                }
            }
            None
        })
}
