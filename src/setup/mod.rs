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
pub const SETUP_CHANNEL_SELECT: &str   = "setup_channel";
pub const SETUP_METHOD_SELECT: &str    = "setup_method";
pub const SETUP_REPORTS_SELECT: &str   = "setup_reports";
pub const SETUP_SAVE_BTN: &str         = "setup_save";
pub const SETUP_CANCEL_BTN: &str       = "setup_cancel";
pub const SETUP_RESET_BTN: &str        = "setup_reset";
pub const SETUP_LOG_CHANNEL: &str      = "setup_log_channel";
pub const SETUP_EDIT_TPL_BTN: &str     = "setup_edit_tpl";

pub const SETTINGS_SELECT: &str        = "settings_select";
pub const SETTINGS_BACK_BTN: &str      = "settings_back";

pub const MODAL_BOT_NAME: &str         = "modal_bot_name";
pub const MODAL_BOT_NAME_INPUT: &str   = "modal_bot_name_input";

// ── Command registration ─────────────────────────────────────────────────────

pub fn register() -> CreateCommand {
    CreateCommand::new("setup")
        .description("Configure the Mod Mail bot for this server")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .dm_permission(false)
}

pub fn register_reset() -> CreateCommand {
    CreateCommand::new("reset")
        .description("Reset all formats and settings to defaults")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .dm_permission(false)
}

pub fn register_settings() -> CreateCommand {
    CreateCommand::new("settings")
        .description("Manage bot settings: Formats, Channels, and more")
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

    let storage = crate::storage::get(ctx).await;
    let config = GuildConfig::load(&*storage, guild_id).await;
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

    let storage = crate::storage::get(ctx).await;
    let mut config = GuildConfig::load(&*storage, guild_id).await;
    config.reset_all_templates();
    config.bot_name = "Mod Mail".to_string();
    config.message_channel_id = None;
    config.log_channel_id = None;
    config.message_method = MessageMethod::default();
    config.reports_enabled = false;

    match config.save(&*storage).await {
        Ok(()) => {
            respond_ephemeral(ctx, command, "All settings and templates have been reset to defaults.").await;
        }
        Err(e) => {
            respond_ephemeral(ctx, command, &format!("Failed to save: {}", e)).await;
        }
    }
}

pub async fn run_settings(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.get(),
        None => {
            respond_ephemeral(ctx, command, "This command can only be used in a server.").await;
            return;
        }
    };

    let storage = crate::storage::get(ctx).await;
    let _config = GuildConfig::load(&*storage, guild_id).await;

    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(build_settings_embed())
            .components(build_settings_components())
            .ephemeral(true),
    );
    if let Err(e) = command.create_response(&ctx.http, resp).await {
        println!("[settings] ERROR create_response: {}", e);
    }
}

// ── Component handlers ───────────────────────────────────────────────────────

pub async fn handle_component(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id.get(),
        None => return,
    };

    let storage = crate::storage::get(ctx).await;

    match component.data.custom_id.as_str() {
        SETTINGS_SELECT => {
            let selected = match &component.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => {
                    values.first().map(String::as_str).unwrap_or("")
                }
                _ => return,
            };

            let config = GuildConfig::load(&*storage, guild_id).await;

            match selected {
                "formats" => {
                    let resp = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(crate::builder::build_select_embed_pub())
                            .components(crate::builder::build_select_components_pub(&config)),
                    );
                    let _ = component.create_response(&ctx.http, resp).await;
                }
                "channels" => {
                    let resp = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(build_channels_embed(&config))
                            .components(build_channels_components(&config)),
                    );
                    let _ = component.create_response(&ctx.http, resp).await;
                }
                "general" => {
                    let resp = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(build_general_embed(&config))
                            .components(build_general_components(&config)),
                    );
                    let _ = component.create_response(&ctx.http, resp).await;
                }
                _ => {}
            }
        }

        SETTINGS_BACK_BTN => {
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_settings_embed())
                    .components(build_settings_components()),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        SETUP_BOT_NAME_BTN => {
            let config = GuildConfig::load(&*storage, guild_id).await;
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

        SETUP_CHANNEL_SELECT => {
            let channels = match &component.data.kind {
                ComponentInteractionDataKind::ChannelSelect { values } => values.clone(),
                _ => vec![],
            };

            let mut config = GuildConfig::load(&*storage, guild_id).await;
            config.message_channel_id = channels.first().map(|c| c.get());
            let _ = config.save(&*storage).await;

            update_setup_message(ctx, component, &config).await;
        }

        SETUP_METHOD_SELECT => {
            let selected = match &component.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => {
                    values.first().map(String::as_str).unwrap_or("both")
                }
                _ => "both",
            };

            let mut config = GuildConfig::load(&*storage, guild_id).await;
            config.message_method = MessageMethod::from_value(selected);
            let _ = config.save(&*storage).await;

            update_setup_message(ctx, component, &config).await;
        }

        SETUP_REPORTS_SELECT => {
            let mut config = GuildConfig::load(&*storage, guild_id).await;
            config.reports_enabled = !config.reports_enabled;
            let _ = config.save(&*storage).await;

            update_setup_message(ctx, component, &config).await;
        }

        SETUP_LOG_CHANNEL => {
            let channels = match &component.data.kind {
                ComponentInteractionDataKind::ChannelSelect { values } => values.clone(),
                _ => vec![],
            };

            let mut config = GuildConfig::load(&*storage, guild_id).await;
            config.log_channel_id = channels.first().map(|c| c.get());
            let _ = config.save(&*storage).await;

            update_setup_message(ctx, component, &config).await;
        }

        SETUP_EDIT_TPL_BTN => {
            let config = GuildConfig::load(&*storage, guild_id).await;
            let resp = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(crate::builder::build_select_embed_pub())
                    .components(crate::builder::build_select_components_pub(&config))
                    .ephemeral(true),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        SETUP_SAVE_BTN => {
            let config = GuildConfig::load(&*storage, guild_id).await;
            match config.save(&*storage).await {
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
            let mut config = GuildConfig::load(&*storage, guild_id).await;
            config.reset_all_templates();
            let _ = config.save(&*storage).await;

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

    let storage = crate::storage::get(ctx).await;

    match modal.data.custom_id.as_str() {
        MODAL_BOT_NAME => {
            let name = extract_modal_value(&modal.data.components, MODAL_BOT_NAME_INPUT)
                .unwrap_or_default();

            let mut config = GuildConfig::load(&*storage, guild_id).await;
            if !name.is_empty() {
                config.bot_name = name;
            }
            let _ = config.save(&*storage).await;

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

    let templates_overridden = if config.templates.is_empty() {
        "None (using defaults)".to_string()
    } else {
        config.templates.keys()
            .map(|k| crate::templates::display_name(k))
            .collect::<Vec<_>>()
            .join(", ")
    };

    serenity::builder::CreateEmbed::new()
        .title("Mod Mail Setup")
        .description("Configure your Mod Mail bot settings below. Each dropdown/button updates a setting. Press **Save** when done.")
        .color(0x5865F2)
        .field("Bot Name", &config.bot_name, true)
        .field("Message Channel", &channel_display, false)
        .field("Log / Output Channel", &log_channel_display, false)
        .field("Message Method", config.message_method.to_string(), true)
        .field("Player Reports", if config.reports_enabled { "Enabled" } else { "Disabled" }, true)
        .field("Format Overrides", &templates_overridden, false)
}

fn build_setup_components(config: &GuildConfig) -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(SETUP_BOT_NAME_BTN)
                .label(format!("Bot Name: {}", config.bot_name))
                .style(ButtonStyle::Secondary),
            CreateButton::new(SETUP_REPORTS_SELECT)
                .label(if config.reports_enabled { "Reports: ON" } else { "Reports: OFF" })
                .style(if config.reports_enabled { ButtonStyle::Success } else { ButtonStyle::Secondary }),
        ]),
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
        CreateActionRow::Buttons(vec![
            CreateButton::new(SETUP_SAVE_BTN)
                .label("Save")
                .style(ButtonStyle::Success),
            CreateButton::new(SETUP_CANCEL_BTN)
                .label("Cancel")
                .style(ButtonStyle::Danger),
            CreateButton::new(SETUP_RESET_BTN)
                .label("Reset Formats")
                .style(ButtonStyle::Secondary),
            CreateButton::new(SETUP_EDIT_TPL_BTN)
                .label("Edit Formats")
                .style(ButtonStyle::Primary),
        ]),
    ]
}

fn build_settings_embed() -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::new()
        .title("Settings")
        .description("Select a category to configure.")
        .color(0x5865F2)
}

fn build_settings_components() -> Vec<CreateActionRow> {
    vec![CreateActionRow::SelectMenu(
        CreateSelectMenu::new(
            SETTINGS_SELECT,
            CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Formats", "formats")
                        .description("Edit format templates for tickets and reports"),
                    CreateSelectMenuOption::new("Channels", "channels")
                        .description("Configure message and log channels"),
                    CreateSelectMenuOption::new("General", "general")
                        .description("Bot profile, message method, and reports"),
                ],
            },
        )
        .placeholder("Choose a settings category"),
    )]
}

fn build_channels_embed(config: &GuildConfig) -> serenity::builder::CreateEmbed {
    let channel_display = config
        .message_channel_id
        .map(|id| format!("<#{}>", id))
        .unwrap_or_else(|| "*Not set*".to_string());
    let log_display = config
        .log_channel_id
        .map(|id| format!("<#{}>", id))
        .unwrap_or_else(|| "*Not set*".to_string());

    serenity::builder::CreateEmbed::new()
        .title("Channel Settings")
        .description("Configure where messages and logs are sent.")
        .color(0x5865F2)
        .field("Message Channel", &channel_display, false)
        .field("Log / Output Channel", &log_display, false)
}

fn build_channels_components(config: &GuildConfig) -> Vec<CreateActionRow> {
    vec![
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
            .placeholder("Select log / output channel"),
        ),
        CreateActionRow::Buttons(vec![
            CreateButton::new(SETTINGS_BACK_BTN)
                .label("Back")
                .style(ButtonStyle::Secondary),
        ]),
    ]
}

fn build_general_embed(config: &GuildConfig) -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::new()
        .title("General Settings")
        .description("Configure bot profile, message method, and reports.")
        .color(0x5865F2)
        .field("Bot Name", &config.bot_name, true)
        .field("Message Method", config.message_method.to_string(), true)
        .field("Player Reports", if config.reports_enabled { "Enabled" } else { "Disabled" }, true)
}

fn build_general_components(config: &GuildConfig) -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(SETUP_BOT_NAME_BTN)
                .label(format!("Bot Name: {}", config.bot_name))
                .style(ButtonStyle::Secondary),
            CreateButton::new(SETUP_REPORTS_SELECT)
                .label(if config.reports_enabled { "Reports: ON" } else { "Reports: OFF" })
                .style(if config.reports_enabled { ButtonStyle::Success } else { ButtonStyle::Secondary }),
        ]),
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
        CreateActionRow::Buttons(vec![
            CreateButton::new(SETTINGS_BACK_BTN)
                .label("Back")
                .style(ButtonStyle::Secondary),
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
