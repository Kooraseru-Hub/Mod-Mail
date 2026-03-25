use serenity::{
    all::{ActionRowComponent, ButtonStyle, CommandInteraction, ComponentInteraction,
          ComponentInteractionDataKind, InputTextStyle, ModalInteraction},
    builder::{
        CreateActionRow, CreateButton, CreateCommand, CreateInputText,
        CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal,
        CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    },
    model::Permissions,
    prelude::Context,
};
use crate::config::GuildConfig;
use crate::templates::{self, EmbedField, EmbedTemplate};

// ── Component / Modal IDs ────────────────────────────────────────────────────

pub const CMD_TEMPLATE_EDIT: &str  = "template-edit";

pub const TPL_SELECT: &str         = "tpl_select";
pub const TPL_EDIT_TITLE: &str     = "tpl_edit_title";
pub const TPL_EDIT_DESC: &str      = "tpl_edit_desc";
pub const TPL_EDIT_COLOR: &str     = "tpl_edit_color";
pub const TPL_ADD_FIELD: &str      = "tpl_add_field";
pub const TPL_REMOVE_FIELD: &str   = "tpl_rm_field";
pub const TPL_RESET_ONE: &str      = "tpl_reset_one";
pub const TPL_PREVIEW: &str        = "tpl_preview";
pub const TPL_DONE: &str           = "tpl_done";

pub const MODAL_TPL_TITLE: &str    = "modal_tpl_title";
pub const MODAL_TPL_TITLE_IN: &str = "modal_tpl_title_in";
pub const MODAL_TPL_DESC: &str     = "modal_tpl_desc";
pub const MODAL_TPL_DESC_IN: &str  = "modal_tpl_desc_in";
pub const MODAL_TPL_COLOR: &str    = "modal_tpl_color";
pub const MODAL_TPL_COLOR_IN: &str = "modal_tpl_color_in";
pub const MODAL_TPL_FIELD: &str    = "modal_tpl_field";
pub const MODAL_TPL_FIELD_NAME: &str  = "modal_tpl_field_name";
pub const MODAL_TPL_FIELD_VALUE: &str = "modal_tpl_field_value";
pub const MODAL_TPL_FIELD_INLINE: &str = "modal_tpl_field_inline";

pub const TPL_SET_CHANNEL: &str    = "tpl_set_channel";
pub const TPL_SET_INSTR: &str      = "tpl_set_instr";
pub const MODAL_TPL_CHANNEL: &str  = "modal_tpl_channel";
pub const MODAL_TPL_CHANNEL_IN: &str = "modal_tpl_channel_in";
pub const MODAL_TPL_INSTR: &str    = "modal_tpl_instr";
pub const MODAL_TPL_INSTR_IN: &str = "modal_tpl_instr_in";

pub const TPL_CREATE_NEW: &str     = "tpl_create_new";
pub const TPL_DELETE_FMT: &str     = "tpl_delete_fmt";
pub const MODAL_TPL_NEW: &str      = "modal_tpl_new";
pub const MODAL_TPL_NEW_NAME: &str = "modal_tpl_new_name";

// ── Slash command registration ───────────────────────────────────────────────

pub fn register() -> CreateCommand {
    CreateCommand::new(CMD_TEMPLATE_EDIT)
        .description("Edit option embeds and settings used by Mod Mail and Reports")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .dm_permission(false)
}

// ── /template-edit entry ─────────────────────────────────────────────────────

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.get(),
        None => {
            ephemeral(ctx, command, "This command can only be used in a server.").await;
            return;
        }
    };

    let storage = crate::storage::get(ctx).await;
    let config = GuildConfig::load(&*storage, guild_id).await;
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(build_select_embed())
            .components(build_select_components(&config))
            .ephemeral(true),
    );
    if let Err(e) = command.create_response(&ctx.http, resp).await {
        println!("[tpl-edit] ERROR: {}", e);
    }
}

// ── Component handler ────────────────────────────────────────────────────────

pub async fn handle_component(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id.get(),
        None => return,
    };

    let storage = crate::storage::get(ctx).await;
    let id = component.data.custom_id.as_str();

    if id == TPL_SELECT {
        let selected = match &component.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                values.first().cloned().unwrap_or_default()
            }
            _ => return,
        };

        let config = GuildConfig::load(&*storage, guild_id).await;
        let template = config.get_template(&selected);

        let resp = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(build_editor_embed(&selected, &template, &config))
                .components(build_editor_components(&selected, &config)),
        );
        let _ = component.create_response(&ctx.http, resp).await;
        return;
    }

    if id == TPL_CREATE_NEW {
        let modal = CreateInteractionResponse::Modal(
            CreateModal::new(MODAL_TPL_NEW, "Create New Format")
                .components(vec![CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "Format Name", MODAL_TPL_NEW_NAME)
                        .placeholder("e.g. Bug Report")
                        .required(true)
                        .min_length(1)
                        .max_length(50),
                )]),
        );
        let _ = component.create_response(&ctx.http, modal).await;
        return;
    }

    let (action, tpl_name) = match id.split_once(':') {
        Some(pair) => pair,
        None => return,
    };

    let config = GuildConfig::load(&*storage, guild_id).await;
    let template = config.get_template(tpl_name);

    match action {
        "tpl_edit_title" => {
            let modal = CreateInteractionResponse::Modal(
                CreateModal::new(format!("{}:{}", MODAL_TPL_TITLE, tpl_name), "Edit Title")
                    .components(vec![CreateActionRow::InputText(
                        CreateInputText::new(InputTextStyle::Short, "Title", MODAL_TPL_TITLE_IN)
                            .value(&template.title)
                            .required(true)
                            .max_length(256),
                    )]),
            );
            let _ = component.create_response(&ctx.http, modal).await;
        }

        "tpl_edit_desc" => {
            let modal = CreateInteractionResponse::Modal(
                CreateModal::new(format!("{}:{}", MODAL_TPL_DESC, tpl_name), "Edit Description")
                    .components(vec![CreateActionRow::InputText(
                        CreateInputText::new(InputTextStyle::Paragraph, "Description", MODAL_TPL_DESC_IN)
                            .value(&template.description)
                            .required(true)
                            .max_length(2000),
                    )]),
            );
            let _ = component.create_response(&ctx.http, modal).await;
        }

        "tpl_edit_color" => {
            let modal = CreateInteractionResponse::Modal(
                CreateModal::new(format!("{}:{}", MODAL_TPL_COLOR, tpl_name), "Edit Color")
                    .components(vec![CreateActionRow::InputText(
                        CreateInputText::new(InputTextStyle::Short, "Color (hex, e.g. FF0000)", MODAL_TPL_COLOR_IN)
                            .value(format!("{:06X}", template.color))
                            .required(true)
                            .min_length(6)
                            .max_length(6),
                    )]),
            );
            let _ = component.create_response(&ctx.http, modal).await;
        }

        "tpl_add_field" => {
            let modal = CreateInteractionResponse::Modal(
                CreateModal::new(format!("{}:{}", MODAL_TPL_FIELD, tpl_name), "Add Field")
                    .components(vec![
                        CreateActionRow::InputText(
                            CreateInputText::new(InputTextStyle::Short, "Field Name", MODAL_TPL_FIELD_NAME)
                                .required(true)
                                .max_length(256),
                        ),
                        CreateActionRow::InputText(
                            CreateInputText::new(InputTextStyle::Paragraph, "Field Value", MODAL_TPL_FIELD_VALUE)
                                .placeholder("Use {reporter}, {reported_user}, {reason}, {author}, {content}, {ticket_type}")
                                .required(true)
                                .max_length(1024),
                        ),
                        CreateActionRow::InputText(
                            CreateInputText::new(InputTextStyle::Short, "Inline? (true/false)", MODAL_TPL_FIELD_INLINE)
                                .value("true")
                                .required(true)
                                .max_length(5),
                        ),
                    ]),
            );
            let _ = component.create_response(&ctx.http, modal).await;
        }

        "tpl_rm_field" => {
            if template.fields.is_empty() {
                let _ = component.create_response(&ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("No fields to remove.")
                            .ephemeral(true),
                    )).await;
                return;
            }

            let options: Vec<CreateSelectMenuOption> = template
                .fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    CreateSelectMenuOption::new(
                        format!("#{} — {}", i + 1, f.name),
                        format!("{}:{}:{}", TPL_REMOVE_FIELD, tpl_name, i),
                    )
                })
                .collect();

            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_editor_embed(tpl_name, &template, &config))
                    .components(vec![
                        CreateActionRow::SelectMenu(
                            CreateSelectMenu::new(
                                format!("tpl_rm_select:{}", tpl_name),
                                CreateSelectMenuKind::String { options },
                            )
                            .placeholder("Select field to remove"),
                        ),
                        CreateActionRow::Buttons(vec![
                            CreateButton::new(format!("tpl_rm_cancel:{}", tpl_name))
                                .label("Cancel")
                                .style(ButtonStyle::Secondary),
                        ]),
                    ]),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        "tpl_rm_select" => {
            let selected = match &component.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => {
                    values.first().cloned().unwrap_or_default()
                }
                _ => return,
            };

            // Parse "tpl_rm_field:template_name:index"
            let parts: Vec<&str> = selected.splitn(3, ':').collect();
            if parts.len() == 3 {
                let real_tpl_name = parts[1];
                if let Ok(index) = parts[2].parse::<usize>() {
                    let mut cfg = GuildConfig::load(&*storage, guild_id).await;
                    let mut t = cfg.get_template(real_tpl_name);
                    if index < t.fields.len() {
                        t.fields.remove(index);
                        cfg.set_template(real_tpl_name, &t);
                        let _ = cfg.save(&*storage).await;
                    }

                    let resp = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(build_editor_embed(real_tpl_name, &t, &cfg))
                            .components(build_editor_components(real_tpl_name, &cfg)),
                    );
                    let _ = component.create_response(&ctx.http, resp).await;
                }
            }
        }

        "tpl_rm_cancel" => {
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_editor_embed(tpl_name, &template, &config))
                    .components(build_editor_components(tpl_name, &config)),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        "tpl_reset_one" => {
            let mut cfg = GuildConfig::load(&*storage, guild_id).await;
            cfg.reset_template(tpl_name);
            let _ = cfg.save(&*storage).await;

            let default_tpl = templates::get_default(tpl_name);
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_editor_embed(tpl_name, &default_tpl, &cfg))
                    .components(build_editor_components(tpl_name, &cfg)),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        "tpl_preview" => {
            let _ = component.create_response(&ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(
                            serenity::builder::CreateEmbed::new()
                                .title(format!("Preview: {}", template.title))
                                .description(&template.description)
                                .color(template.color)
                                .fields(
                                    template.fields.iter().map(|f| {
                                        (f.name.as_str(), f.value.as_str(), f.inline)
                                    })
                                )
                        )
                        .ephemeral(true),
                )).await;
        }

        "tpl_set_channel" => {
            let opt_cfg = config.get_option_config(tpl_name);
            let current = opt_cfg
                .channel_id
                .map(|id| id.to_string())
                .unwrap_or_default();
            let modal = CreateInteractionResponse::Modal(
                CreateModal::new(
                    format!("{}:{}", MODAL_TPL_CHANNEL, tpl_name),
                    "Set Channel Override",
                )
                .components(vec![CreateActionRow::InputText(
                    CreateInputText::new(
                        InputTextStyle::Short,
                        "Channel ID (leave empty to use default)",
                        MODAL_TPL_CHANNEL_IN,
                    )
                    .value(current)
                    .required(false)
                    .max_length(20),
                )]),
            );
            let _ = component.create_response(&ctx.http, modal).await;
        }

        "tpl_set_instr" => {
            let opt_cfg = config.get_option_config(tpl_name);
            let current = opt_cfg.instructions.unwrap_or_default();
            let modal = CreateInteractionResponse::Modal(
                CreateModal::new(
                    format!("{}:{}", MODAL_TPL_INSTR, tpl_name),
                    "Set Instructions",
                )
                .components(vec![CreateActionRow::InputText(
                    CreateInputText::new(
                        InputTextStyle::Paragraph,
                        "Instructions shown to user in DMs",
                        MODAL_TPL_INSTR_IN,
                    )
                    .value(current)
                    .required(false)
                    .max_length(1000),
                )]),
            );
            let _ = component.create_response(&ctx.http, modal).await;
        }

        "tpl_done" => {
            let display = templates::display_name(tpl_name);
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(
                        serenity::builder::CreateEmbed::new()
                            .title("Format Editor")
                            .description(format!("Format **{}** saved.", display))
                            .color(0x57F287),
                    )
                    .components(vec![]),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        "tpl_delete_fmt" => {
            let mut cfg = GuildConfig::load(&*storage, guild_id).await;
            cfg.remove_custom_format(tpl_name);
            let _ = cfg.save(&*storage).await;

            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_select_embed())
                    .components(build_select_components(&cfg)),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        _ => {}
    }
}

// ── Modal handler ────────────────────────────────────────────────────────────

pub async fn handle_modal(ctx: &Context, modal: &ModalInteraction) {
    let guild_id = match modal.guild_id {
        Some(id) => id.get(),
        None => return,
    };

    let storage = crate::storage::get(ctx).await;

    if modal.data.custom_id == MODAL_TPL_NEW {
        let name = extract_value(&modal.data.components, MODAL_TPL_NEW_NAME).unwrap_or_default();
        if name.is_empty() {
            return;
        }
        let key = name.to_lowercase().replace(' ', "_");
        let mut config = GuildConfig::load(&*storage, guild_id).await;
        config.add_custom_format(&key);
        let default_tpl = templates::default_general_support();
        let mut new_tpl = default_tpl;
        new_tpl.title = name.clone();
        new_tpl.description = format!("A new {} ticket has been created.", name);
        config.set_template(&key, &new_tpl);
        let _ = config.save(&*storage).await;

        let resp = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(build_editor_embed(&key, &new_tpl, &config))
                .components(build_editor_components(&key, &config)),
        );
        let _ = modal.create_response(&ctx.http, resp).await;
        return;
    }

    let (base_id, tpl_name) = match modal.data.custom_id.split_once(':') {
        Some(pair) => pair,
        None => return,
    };

    let mut config = GuildConfig::load(&*storage, guild_id).await;
    let mut template = config.get_template(tpl_name);

    match base_id {
        MODAL_TPL_TITLE => {
            if let Some(val) = extract_value(&modal.data.components, MODAL_TPL_TITLE_IN) {
                template.title = val;
            }
        }
        MODAL_TPL_DESC => {
            if let Some(val) = extract_value(&modal.data.components, MODAL_TPL_DESC_IN) {
                template.description = val;
            }
        }
        MODAL_TPL_COLOR => {
            if let Some(val) = extract_value(&modal.data.components, MODAL_TPL_COLOR_IN) {
                let hex = val.trim_start_matches('#');
                if let Ok(c) = u32::from_str_radix(hex, 16) {
                    template.color = c;
                }
            }
        }
        MODAL_TPL_FIELD => {
            let name = extract_value(&modal.data.components, MODAL_TPL_FIELD_NAME).unwrap_or_default();
            let value = extract_value(&modal.data.components, MODAL_TPL_FIELD_VALUE).unwrap_or_default();
            let inline_str = extract_value(&modal.data.components, MODAL_TPL_FIELD_INLINE).unwrap_or_default();
            let inline = inline_str.trim().eq_ignore_ascii_case("true");

            if !name.is_empty() && !value.is_empty() {
                template.fields.push(EmbedField { name, value, inline });
            }
        }
        MODAL_TPL_CHANNEL => {
            let val = extract_value(&modal.data.components, MODAL_TPL_CHANNEL_IN).unwrap_or_default();
            let mut opt_cfg = config.get_option_config(tpl_name);
            opt_cfg.channel_id = val.trim().parse::<u64>().ok();
            config.set_option_config(tpl_name, &opt_cfg);
            let _ = config.save(&*storage).await;

            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_editor_embed(tpl_name, &template, &config))
                    .components(build_editor_components(tpl_name, &config)),
            );
            if let Err(e) = modal.create_response(&ctx.http, resp).await {
                println!("[tpl-edit] ERROR modal response: {}", e);
            }
            return;
        }
        MODAL_TPL_INSTR => {
            let val = extract_value(&modal.data.components, MODAL_TPL_INSTR_IN).unwrap_or_default();
            let mut opt_cfg = config.get_option_config(tpl_name);
            opt_cfg.instructions = if val.trim().is_empty() {
                None
            } else {
                Some(val)
            };
            config.set_option_config(tpl_name, &opt_cfg);
            let _ = config.save(&*storage).await;

            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_editor_embed(tpl_name, &template, &config))
                    .components(build_editor_components(tpl_name, &config)),
            );
            if let Err(e) = modal.create_response(&ctx.http, resp).await {
                println!("[tpl-edit] ERROR modal response: {}", e);
            }
            return;
        }
        _ => return,
    }

    config.set_template(tpl_name, &template);
    let _ = config.save(&*storage).await;

    let resp = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(build_editor_embed(tpl_name, &template, &config))
            .components(build_editor_components(tpl_name, &config)),
    );
    if let Err(e) = modal.create_response(&ctx.http, resp).await {
        println!("[tpl-edit] ERROR modal response: {}", e);
    }
}

// ── UI builders ──────────────────────────────────────────────────────────────

fn build_select_embed() -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::new()
        .title("Format Editor")
        .description("Select a format to edit, or create a new one.\n\nFormats define how embeds look and where tickets are sent.\n\nAvailable placeholders:\n`{reporter}` `{reported_user}` `{reason}` `{author}` `{content}` `{ticket_type}`")
        .color(0x5865F2)
}

pub fn build_select_embed_pub() -> serenity::builder::CreateEmbed {
    build_select_embed()
}

fn build_select_components(config: &GuildConfig) -> Vec<CreateActionRow> {
    let names = config.all_format_names();
    let options: Vec<CreateSelectMenuOption> = names
        .iter()
        .map(|name| {
            let is_overridden = config.templates.contains_key(name.as_str());
            let display = templates::display_name(name);
            let label = if is_overridden {
                format!("{} (customized)", display)
            } else {
                format!("{} (default)", display)
            };
            CreateSelectMenuOption::new(label, name.as_str())
        })
        .collect();

    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                TPL_SELECT,
                CreateSelectMenuKind::String { options },
            )
            .placeholder("Choose a format to edit"),
        ),
        CreateActionRow::Buttons(vec![
            CreateButton::new(TPL_CREATE_NEW)
                .label("Create New Format")
                .style(ButtonStyle::Success),
        ]),
    ]
}

pub fn build_select_components_pub(config: &GuildConfig) -> Vec<CreateActionRow> {
    build_select_components(config)
}

fn build_editor_embed(tpl_name: &str, template: &EmbedTemplate, config: &GuildConfig) -> serenity::builder::CreateEmbed {
    let display = templates::display_name(tpl_name);
    let fields_display = if template.fields.is_empty() {
        "*(none)*".to_string()
    } else {
        template
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                format!(
                    "**#{}** `{}` → `{}` (inline: {})",
                    i + 1,
                    f.name,
                    if f.value.len() > 40 {
                        format!("{}...", &f.value[..40])
                    } else {
                        f.value.clone()
                    },
                    f.inline
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let opt_cfg = config.get_option_config(tpl_name);
    let channel_display = opt_cfg
        .channel_id
        .map(|id| format!("<#{}>", id))
        .unwrap_or_else(|| "*Using default*".to_string());
    let instr_display = opt_cfg
        .instructions
        .as_deref()
        .unwrap_or("*Not set*");

    serenity::builder::CreateEmbed::new()
        .title(format!("Editing: {}", display))
        .color(template.color)
        .field("Title", &template.title, false)
        .field("Description", &template.description, false)
        .field("Color", format!("`#{:06X}`", template.color), true)
        .field("Channel Override", &channel_display, true)
        .field("Fields", &fields_display, false)
        .field("Instructions", instr_display, false)
}

fn build_editor_components(tpl_name: &str, config: &GuildConfig) -> Vec<CreateActionRow> {
    let is_custom = config.custom_formats.contains(&tpl_name.to_string());
    let mut rows = vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("tpl_edit_title:{}", tpl_name))
                .label("Edit Title")
                .style(ButtonStyle::Primary),
            CreateButton::new(format!("tpl_edit_desc:{}", tpl_name))
                .label("Edit Description")
                .style(ButtonStyle::Primary),
            CreateButton::new(format!("tpl_edit_color:{}", tpl_name))
                .label("Edit Color")
                .style(ButtonStyle::Primary),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("tpl_add_field:{}", tpl_name))
                .label("Add Field")
                .style(ButtonStyle::Success),
            CreateButton::new(format!("tpl_rm_field:{}", tpl_name))
                .label("Remove Field")
                .style(ButtonStyle::Danger),
            CreateButton::new(format!("tpl_preview:{}", tpl_name))
                .label("Preview")
                .style(ButtonStyle::Secondary),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("tpl_set_channel:{}", tpl_name))
                .label("Set Channel")
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("tpl_set_instr:{}", tpl_name))
                .label("Set Instructions")
                .style(ButtonStyle::Secondary),
        ]),
    ];

    let mut last_row = vec![
        CreateButton::new(format!("tpl_reset_one:{}", tpl_name))
            .label("Reset to Default")
            .style(ButtonStyle::Danger),
    ];
    if is_custom {
        last_row.push(
            CreateButton::new(format!("tpl_delete_fmt:{}", tpl_name))
                .label("Delete Format")
                .style(ButtonStyle::Danger),
        );
    }
    last_row.push(
        CreateButton::new(format!("tpl_done:{}", tpl_name))
            .label("Done")
            .style(ButtonStyle::Success),
    );
    rows.push(CreateActionRow::Buttons(last_row));
    rows
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    let _ = command.create_response(&ctx.http, resp).await;
}

fn extract_value(
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

/// Returns true if this custom_id belongs to the template editor
pub fn is_template_editor_component(id: &str) -> bool {
    id == TPL_SELECT
        || id == TPL_CREATE_NEW
        || id.starts_with("tpl_edit_")
        || id.starts_with("tpl_add_")
        || id.starts_with("tpl_rm_")
        || id.starts_with("tpl_reset_")
        || id.starts_with("tpl_preview:")
        || id.starts_with("tpl_done:")
        || id.starts_with("tpl_set_")
        || id.starts_with("tpl_delete_")
}

/// Returns true if this modal custom_id belongs to the template editor
pub fn is_template_editor_modal(id: &str) -> bool {
    id == MODAL_TPL_NEW
        || id.starts_with(MODAL_TPL_TITLE)
        || id.starts_with(MODAL_TPL_DESC)
        || id.starts_with(MODAL_TPL_COLOR)
        || id.starts_with(MODAL_TPL_FIELD)
        || id.starts_with(MODAL_TPL_CHANNEL)
        || id.starts_with(MODAL_TPL_INSTR)
}
