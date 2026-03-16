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

// ── Slash command registration ───────────────────────────────────────────────

pub fn register() -> CreateCommand {
    CreateCommand::new(CMD_TEMPLATE_EDIT)
        .description("Edit embed templates used by Mod Mail and Reports")
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

    let config = GuildConfig::load(guild_id);
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

    let id = component.data.custom_id.as_str();

    if id == TPL_SELECT {
        let selected = match &component.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                values.first().cloned().unwrap_or_default()
            }
            _ => return,
        };

        let config = GuildConfig::load(guild_id);
        let template = config.get_template(&selected);

        let resp = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(build_editor_embed(&selected, &template))
                .components(build_editor_components(&selected)),
        );
        let _ = component.create_response(&ctx.http, resp).await;
        return;
    }

    // All other buttons carry the template name as a suffix: "tpl_edit_title:player_report"
    let (action, tpl_name) = match id.split_once(':') {
        Some(pair) => pair,
        None => return,
    };

    let config = GuildConfig::load(guild_id);
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
                    .embed(build_editor_embed(tpl_name, &template))
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
                    let mut cfg = GuildConfig::load(guild_id);
                    let mut t = cfg.get_template(real_tpl_name);
                    if index < t.fields.len() {
                        t.fields.remove(index);
                        cfg.set_template(real_tpl_name, &t);
                        let _ = cfg.save();
                    }

                    let resp = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(build_editor_embed(real_tpl_name, &t))
                            .components(build_editor_components(real_tpl_name)),
                    );
                    let _ = component.create_response(&ctx.http, resp).await;
                }
            }
        }

        "tpl_rm_cancel" => {
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_editor_embed(tpl_name, &template))
                    .components(build_editor_components(tpl_name)),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        "tpl_reset_one" => {
            let mut cfg = GuildConfig::load(guild_id);
            cfg.reset_template(tpl_name);
            let _ = cfg.save();

            let default_tpl = templates::get_default(tpl_name);
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(build_editor_embed(tpl_name, &default_tpl))
                    .components(build_editor_components(tpl_name)),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }

        "tpl_preview" => {
            let preview = template.to_components_v2();
            let preview_str = serde_json::to_string_pretty(&preview).unwrap_or_default();
            let truncated = if preview_str.len() > 1900 {
                format!("{}...", &preview_str[..1900])
            } else {
                preview_str
            };

            // Show a preview embed alongside the editor
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
                        .content(format!("```json\n{}\n```", truncated))
                        .ephemeral(true),
                )).await;
        }

        "tpl_done" => {
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(
                        serenity::builder::CreateEmbed::new()
                            .title("Template Editor")
                            .description(format!("Template **{}** saved.", tpl_name))
                            .color(0x57F287),
                    )
                    .components(vec![]),
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

    let (base_id, tpl_name) = match modal.data.custom_id.split_once(':') {
        Some(pair) => pair,
        None => return,
    };

    let mut config = GuildConfig::load(guild_id);
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
        _ => return,
    }

    config.set_template(tpl_name, &template);
    let _ = config.save();

    let resp = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(build_editor_embed(tpl_name, &template))
            .components(build_editor_components(tpl_name)),
    );
    if let Err(e) = modal.create_response(&ctx.http, resp).await {
        println!("[tpl-edit] ERROR modal response: {}", e);
    }
}

// ── UI builders ──────────────────────────────────────────────────────────────

fn build_select_embed() -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::new()
        .title("Template Editor")
        .description("Select a template to edit. Templates define how embeds look for reports and mod mail tickets.\n\nAvailable placeholders:\n`{reporter}` `{reported_user}` `{reason}` `{author}` `{content}` `{ticket_type}`")
        .color(0x5865F2)
}

pub fn build_select_embed_pub() -> serenity::builder::CreateEmbed {
    build_select_embed()
}

fn build_select_components(config: &GuildConfig) -> Vec<CreateActionRow> {
    let names = templates::list_template_names();
    let options: Vec<CreateSelectMenuOption> = names
        .iter()
        .map(|name| {
            let is_overridden = config.templates.contains_key(*name);
            let label = if is_overridden {
                format!("{} (customized)", name)
            } else {
                format!("{} (default)", name)
            };
            CreateSelectMenuOption::new(label, *name)
        })
        .collect();

    vec![CreateActionRow::SelectMenu(
        CreateSelectMenu::new(
            TPL_SELECT,
            CreateSelectMenuKind::String { options },
        )
        .placeholder("Choose a template to edit"),
    )]
}

pub fn build_select_components_pub(config: &GuildConfig) -> Vec<CreateActionRow> {
    build_select_components(config)
}

fn build_editor_embed(tpl_name: &str, template: &EmbedTemplate) -> serenity::builder::CreateEmbed {
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

    serenity::builder::CreateEmbed::new()
        .title(format!("Editing: {}", tpl_name))
        .color(template.color)
        .field("Title", &template.title, false)
        .field("Description", &template.description, false)
        .field("Color", format!("`#{:06X}`", template.color), true)
        .field("Fields", &fields_display, false)
}

fn build_editor_components(tpl_name: &str) -> Vec<CreateActionRow> {
    vec![
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
            CreateButton::new(format!("tpl_reset_one:{}", tpl_name))
                .label("Reset to Default")
                .style(ButtonStyle::Danger),
            CreateButton::new(format!("tpl_done:{}", tpl_name))
                .label("Done")
                .style(ButtonStyle::Success),
        ]),
    ]
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
        || id.starts_with("tpl_edit_")
        || id.starts_with("tpl_add_")
        || id.starts_with("tpl_rm_")
        || id.starts_with("tpl_reset_")
        || id.starts_with("tpl_preview:")
        || id.starts_with("tpl_done:")
}

/// Returns true if this modal custom_id belongs to the template editor
pub fn is_template_editor_modal(id: &str) -> bool {
    id.starts_with(MODAL_TPL_TITLE)
        || id.starts_with(MODAL_TPL_DESC)
        || id.starts_with(MODAL_TPL_COLOR)
        || id.starts_with(MODAL_TPL_FIELD)
}
