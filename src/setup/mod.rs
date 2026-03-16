use serenity::{
    all::{
        ButtonStyle, ChannelType, CommandInteraction, ComponentInteraction,
        ComponentInteractionDataKind, InputTextStyle, ModalInteraction,
        ActionRowComponent,
    },
    builder::{
        CreateActionRow, CreateButton, CreateCommand, CreateInputText,
        CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal,
        CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    },
    model::Permissions,
    prelude::Context,
};
use crate::config::{GuildConfig, MessageMethod};

// ── Component IDs ────────────────────────────────────────────────────────────

pub const SETUP_BOT_NAME_BTN: &str    = "setup_bot_name";
pub const SETUP_AVATAR_BTN: &str      = "setup_avatar";
pub const SETUP_CHANNEL_SELECT: &str   = "setup_channel";
pub const SETUP_METHOD_SELECT: &str    = "setup_method";
pub const SETUP_REPORTS_SELECT: &str   = "setup_reports";
pub const SETUP_SAVE_BTN: &str         = "setup_save";
pub const SETUP_CANCEL_BTN: &str       = "setup_cancel";
pub const SETUP_RESET_BTN: &str        = "setup_reset";
pub const SETUP_LOG_CHANNEL: &str      = "setup_log_channel";
pub const SETUP_EDIT_TPL_BTN: &str     = "setup_edit_tpl";

pub const MODAL_BOT_NAME: &str         = "modal_bot_name";
pub const MODAL_BOT_NAME_INPUT: &str   = "modal_bot_name_input";
pub const MODAL_AVATAR: &str           = "modal_avatar";
pub const MODAL_AVATAR_INPUT: &str     = "modal_avatar_input";

// ── Command registration ─────────────────────────────────────────────────────

pub fn register() -> CreateCommand {
    CreateCommand::new("setup")
        .description("Configure the Mod Mail bot for this server")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .dm_permission(false)
}

pub fn register_reset() -> CreateCommand {
    CreateCommand::new("reset-defaults")
        .description("Reset all templates and settings to defaults")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .dm_permission(false)
}

// ── /setup entry point ───────────────────────────────────────────────────────

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.get(),
        None => {
            respond_ephemeral(ctx, command, "This command can only be used in a server.").await;
            return;
        }
    };

    let config = GuildConfig::load(guild_id);
    let embed = build_setup_embed(&config);

    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(build_setup_components(&config))
            .ephemeral(true),
    );

    if let Err(e) = command.create_response(&ctx.http, resp).await {
        println!("[setup] ERROR create_response: {}", e);
    }
}

// ── /reset-defaults entry point ──────────────────────────────────────────────

pub async fn run_reset(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.get(),
        None => {
            respond_ephemeral(ctx, command, "This command can only be used in a server.").await;
            return;
        }
    };

    let mut config = GuildConfig::load(guild_id);
    config.reset_all_templates();
    config.bot_name = "Mod Mail".to_string();
    config.bot_avatar_url = None;
    config.message_channel_id = None;
    config.log_channel_id = None;
    config.message_method = MessageMethod::default();
    config.reports_enabled = false;

    match config.save() {
        Ok(()) => {
            respond_ephemeral(ctx, command, "All settings and templates have been reset to defaults.").await;
        }
        Err(e) => {
            respond_ephemeral(ctx, command, &format!("Failed to save: {}", e)).await;
        }
    }
}

// ── Component handlers ───────────────────────────────────────────────────────

pub async fn handle_component(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id.get(),
        None => return,
    };

    match component.data.custom_id.as_str() {
        SETUP_BOT_NAME_BTN => {
            let config = GuildConfig::load(guild_id);
            let modal = CreateInteractionResponse::Modal(
                CreateModal::new(MODAL_BOT_NAME, "Set Bot Name")
                    .components(vec![CreateActionRow::InputText(
                        CreateInputText::new(InputTextStyle::Short, "Bot Name", MODAL_BOT_NAME_INPUT)
                            .placeholder("Mod Mail")
                            .value(&config.bot_name)
                            .required(true)
                            .min_length(1)
                            .max_length(32),
                    )]),
            );
            if let Err(e) = component.create_response(&ctx.http, modal).await {
                println!("[setup] ERROR modal: {}", e);
            }
        }

        SETUP_AVATAR_BTN => {
            let config = GuildConfig::load(guild_id);
            let mut input = CreateInputText::new(InputTextStyle::Short, "Avatar URL", MODAL_AVATAR_INPUT)
                .placeholder("https://example.com/avatar.png")
                .required(false)
                .max_length(512);
            if let Some(url) = &config.bot_avatar_url {
                input = input.value(url);
            }
            let modal = CreateInteractionResponse::Modal(
                CreateModal::new(MODAL_AVATAR, "Set Bot Avatar")
                    .components(vec![CreateActionRow::InputText(input)]),
            );
            if let Err(e) = component.create_response(&ctx.http, modal).await {
                println!("[setup] ERROR modal: {}", e);
            }
        }

        SETUP_CHANNEL_SELECT => {
            let channels = match &component.data.kind {
                ComponentInteractionDataKind::ChannelSelect { values } => values.clone(),
                _ => vec![],
            };

            let mut config = GuildConfig::load(guild_id);
            config.message_channel_id = channels.first().map(|c| c.get());
            let _ = config.save();

            update_setup_message(ctx, component, &config).await;
        }

        SETUP_METHOD_SELECT => {
            let selected = match &component.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => {
                    values.first().map(String::as_str).unwrap_or("both")
                }
                _ => "both",
            };

            let mut config = GuildConfig::load(guild_id);
            config.message_method = MessageMethod::from_value(selected);
            let _ = config.save();

            update_setup_message(ctx, component, &config).await;
        }

        SETUP_REPORTS_SELECT => {
            let mut config = GuildConfig::load(guild_id);
            config.reports_enabled = !config.reports_enabled;
            let _ = config.save();

            update_setup_message(ctx, component, &config).await;
        }

        SETUP_LOG_CHANNEL => {
            let channels = match &component.data.kind {
                ComponentInteractionDataKind::ChannelSelect { values } => values.clone(),
                _ => vec![],
            };

            let mut config = GuildConfig::load(guild_id);
            config.log_channel_id = channels.first().map(|c| c.get());
            let _ = config.save();

            update_setup_message(ctx, component, &config).await;
        }

        SETUP_EDIT_TPL_BTN => {
            let config = GuildConfig::load(guild_id);
            let resp = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(crate::builder::build_select_embed_pub())
                    .components(crate::builder::build_select_components_pub(&config))
                    .ephemeral(true),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        SETUP_SAVE_BTN => {
            let config = GuildConfig::load(guild_id);
            match config.save() {
                Ok(()) => {
                    let resp = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(
                                serenity::builder::CreateEmbed::new()
                                    .title("Setup Complete")
                                    .description("Configuration has been saved successfully.")
                                    .color(0x57F287),
                            )
                            .components(vec![]),
                    );
                    let _ = component.create_response(&ctx.http, resp).await;
                }
                Err(e) => {
                    let _ = component
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content(format!("Failed to save: {}", e))
                                    .ephemeral(true),
                            ),
                        )
                        .await;
                }
            }
        }

        SETUP_CANCEL_BTN => {
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(
                        serenity::builder::CreateEmbed::new()
                            .title("Setup Cancelled")
                            .description("No changes were saved.")
                            .color(0xED4245),
                    )
                    .components(vec![]),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        SETUP_RESET_BTN => {
            let mut config = GuildConfig::load(guild_id);
            config.reset_all_templates();
            let _ = config.save();

            update_setup_message(ctx, component, &config).await;
        }

        _ => {}
    }
}

// ── Modal handlers ───────────────────────────────────────────────────────────

pub async fn handle_modal(ctx: &Context, modal: &ModalInteraction) {
    let guild_id = match modal.guild_id {
        Some(id) => id.get(),
        None => return,
    };

    match modal.data.custom_id.as_str() {
        MODAL_BOT_NAME => {
            let name = extract_modal_value(&modal.data.components, MODAL_BOT_NAME_INPUT)
                .unwrap_or_default();

            let mut config = GuildConfig::load(guild_id);
            if !name.is_empty() {
                config.bot_name = name;
            }
            let _ = config.save();

            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_setup_embed(&config))
                    .components(build_setup_components(&config)),
            );
            if let Err(e) = modal.create_response(&ctx.http, resp).await {
                println!("[setup] ERROR modal response: {}", e);
            }
        }

        MODAL_AVATAR => {
            let url = extract_modal_value(&modal.data.components, MODAL_AVATAR_INPUT)
                .unwrap_or_default();

            let mut config = GuildConfig::load(guild_id);
            config.bot_avatar_url = if url.is_empty() { None } else { Some(url) };
            let _ = config.save();

            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_setup_embed(&config))
                    .components(build_setup_components(&config)),
            );
            if let Err(e) = modal.create_response(&ctx.http, resp).await {
                println!("[setup] ERROR modal response: {}", e);
            }
        }

        _ => {}
    }
}

// ── UI builders ──────────────────────────────────────────────────────────────

fn build_setup_embed(config: &GuildConfig) -> serenity::builder::CreateEmbed {
    let channel_display = config
        .message_channel_id
        .map(|id| format!("<#{}>", id))
        .unwrap_or_else(|| "*Not set*".to_string());

    let log_channel_display = config
        .log_channel_id
        .map(|id| format!("<#{}>", id))
        .unwrap_or_else(|| "*Not set*".to_string());

    let avatar_display = config
        .bot_avatar_url
        .as_deref()
        .unwrap_or("*Not set*");

    let templates_overridden = if config.templates.is_empty() {
        "None (using defaults)".to_string()
    } else {
        config.templates.keys().cloned().collect::<Vec<_>>().join(", ")
    };

    serenity::builder::CreateEmbed::new()
        .title("Mod Mail Setup")
        .description("Configure your Mod Mail bot settings below. Each dropdown/button updates a setting. Press **Save** when done.")
        .color(0x5865F2)
        .field("Bot Name", &config.bot_name, true)
        .field("Avatar URL", avatar_display, true)
        .field("Message Channel", &channel_display, false)
        .field("Log / Output Channel", &log_channel_display, false)
        .field("Message Method", config.message_method.to_string(), true)
        .field("Player Reports", if config.reports_enabled { "Enabled" } else { "Disabled" }, true)
        .field("Template Overrides", &templates_overridden, false)
}

fn build_setup_components(config: &GuildConfig) -> Vec<CreateActionRow> {
    // Discord allows max 5 action rows. We use:
    // Row 1: Bot Name + Avatar + Reports toggle buttons
    // Row 2: Message channel select
    // Row 3: Log / output channel select
    // Row 4: Message method + reports select combined as string menu
    // Row 5: Save / Cancel / Reset / Edit Templates
    vec![
        // Row 1: Bot Name + Avatar buttons
        CreateActionRow::Buttons(vec![
            CreateButton::new(SETUP_BOT_NAME_BTN)
                .label(format!("Bot Name: {}", config.bot_name))
                .style(ButtonStyle::Secondary),
            CreateButton::new(SETUP_AVATAR_BTN)
                .label("Set Avatar URL")
                .style(ButtonStyle::Secondary),
            CreateButton::new(SETUP_REPORTS_SELECT)
                .label(if config.reports_enabled { "Reports: ON" } else { "Reports: OFF" })
                .style(if config.reports_enabled { ButtonStyle::Success } else { ButtonStyle::Secondary }),
        ]),
        // Row 2: Message channel select
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                SETUP_CHANNEL_SELECT,
                CreateSelectMenuKind::Channel {
                    channel_types: Some(vec![ChannelType::Text]),
                    default_channels: config.message_channel_id.map(|id| {
                        vec![serenity::all::ChannelId::new(id)]
                    }),
                },
            )
            .placeholder("Select message input channel"),
        ),
        // Row 3: Log / output channel select
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                SETUP_LOG_CHANNEL,
                CreateSelectMenuKind::Channel {
                    channel_types: Some(vec![ChannelType::Text]),
                    default_channels: config.log_channel_id.map(|id| {
                        vec![serenity::all::ChannelId::new(id)]
                    }),
                },
            )
            .placeholder("Select log / output channel (reports & tickets go here)"),
        ),
        // Row 4: Message method select
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                SETUP_METHOD_SELECT,
                CreateSelectMenuKind::String {
                    options: vec![
                        CreateSelectMenuOption::new("DMs Only", "dms")
                            .default_selection(config.message_method == MessageMethod::DMs),
                        CreateSelectMenuOption::new("Slash Command Only", "interaction")
                            .default_selection(config.message_method == MessageMethod::Interaction),
                        CreateSelectMenuOption::new("Both DMs & Slash Commands", "both")
                            .default_selection(config.message_method == MessageMethod::Both),
                        CreateSelectMenuOption::new("Disabled", "none")
                            .default_selection(config.message_method == MessageMethod::None),
                    ],
                },
            )
            .placeholder("Message method"),
        ),
        // Row 5: Save / Cancel / Reset / Edit Templates
        CreateActionRow::Buttons(vec![
            CreateButton::new(SETUP_SAVE_BTN)
                .label("Save")
                .style(ButtonStyle::Success),
            CreateButton::new(SETUP_CANCEL_BTN)
                .label("Cancel")
                .style(ButtonStyle::Danger),
            CreateButton::new(SETUP_RESET_BTN)
                .label("Reset Templates")
                .style(ButtonStyle::Secondary),
            CreateButton::new(SETUP_EDIT_TPL_BTN)
                .label("Edit Templates")
                .style(ButtonStyle::Primary),
        ]),
    ]
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn update_setup_message(ctx: &Context, component: &ComponentInteraction, config: &GuildConfig) {
    let resp = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(build_setup_embed(config))
            .components(build_setup_components(config)),
    );
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        println!("[setup] ERROR update: {}", e);
    }
}

async fn respond_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = command.create_response(&ctx.http, resp).await {
        println!("[setup] ERROR respond: {}", e);
    }
}

fn extract_modal_value(
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
