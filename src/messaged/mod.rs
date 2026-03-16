//! Mod mail ticket creation handler
//!
//! Handles incoming DMs, component interactions (dropdown, cancel, create),
//! and modal submissions. Each public `handle_*` function maps to one interaction type.

use serenity::{
    all::{ComponentInteractionDataKind, ActionRowComponent, InputTextStyle},
    builder::{CreateInputText, CreateModal, CreateActionRow, CreateInteractionResponse},
    model::{
        application::{ComponentInteraction, ModalInteraction},
        channel::Message,
    },
    prelude::Context,
};
use crate::message;

pub const DROPDOWN_ID: &str = "p_266055779117699181";
pub const CANCEL_ID:   &str = "p_266056950012186771";
pub const CREATE_ID:   &str = "p_266056692725190689";
pub const MODAL_ID:    &str = "mod_mail_modal";
pub const MODAL_CONTENT_ID: &str = "mod_mail_content";

// ── DM entry point ──────────────────────────────────────────────────────────

/// Called when a user sends any DM to the bot.
/// Loads the mod mail embed and sends it to the user's DM channel.
pub async fn run(ctx: &Context, msg: &Message) {
    println!("[modmail] DM from {} (channel {})", msg.author.name, msg.channel_id);

    match message::load_message_from_file("src/messaged/embed.json") {
        Ok(payload) => {
            println!("[modmail] Embed loaded, dispatching to channel {}", msg.channel_id);
            let delivery = message::DeliveryMethod::DirectMessage(msg.channel_id);
            match message::send_message(ctx, msg, payload, delivery).await {
                Ok(())  => println!("[modmail] Embed sent OK"),
                Err(e)  => println!("[modmail] ERROR send_message: {}", e),
            }
        }
        Err(e) => println!("[modmail] ERROR load embed.json: {}", e),
    }
}

// ── Component interaction handlers ──────────────────────────────────────────

/// Handles the ticket-type dropdown selection.
/// Defers the update, then patches the message to enable the Create button.
pub async fn handle_select(ctx: &Context, component: &ComponentInteraction) {
    let selected = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.first().map(String::as_str).unwrap_or("(none)")
        }
        _ => "(none)",
    };
    println!("[modmail:select] Value='{}' interaction={}", selected, component.id);

    if let Err(e) = component.create_response(&ctx.http, CreateInteractionResponse::Acknowledge).await {
        println!("[modmail:select] ERROR acknowledge: {}", e);
        return;
    }

    match message::load_message_from_file("src/messaged/embed.json") {
        Ok(mut payload) => {
            enable_create_button(&mut payload);
            println!("[modmail:select] Patching message — enabling Create button");
            match message::send_components_v2_interaction_response(
                ctx,
                component.application_id.get(),
                &component.token,
                &payload,
            ).await {
                Ok(())  => println!("[modmail:select] Patch OK"),
                Err(e)  => println!("[modmail:select] ERROR patch: {}", e),
            }
        }
        Err(e) => println!("[modmail:select] ERROR load embed.json: {}", e),
    }
}

/// Handles the Cancel button.
/// Defers the update, then patches the message to disable all interactive components.
pub async fn handle_cancel(ctx: &Context, component: &ComponentInteraction) {
    println!("[modmail:cancel] Clicked interaction={}", component.id);

    if let Err(e) = component.create_response(&ctx.http, CreateInteractionResponse::Acknowledge).await {
        println!("[modmail:cancel] ERROR acknowledge: {}", e);
        return;
    }

    match message::load_message_from_file("src/messaged/embed.json") {
        Ok(mut payload) => {
            disable_all_components(&mut payload);
            println!("[modmail:cancel] Patching message — disabling all components");
            match message::send_components_v2_interaction_response(
                ctx,
                component.application_id.get(),
                &component.token,
                &payload,
            ).await {
                Ok(())  => println!("[modmail:cancel] Patch OK"),
                Err(e)  => println!("[modmail:cancel] ERROR patch: {}", e),
            }
        }
        Err(e) => println!("[modmail:cancel] ERROR load embed.json: {}", e),
    }
}

/// Handles the Create Message button.
/// Responds with a modal so the user can type their message.
pub async fn handle_create(ctx: &Context, component: &ComponentInteraction) {
    println!("[modmail:create] Clicked interaction={}", component.id);

    let modal = CreateInteractionResponse::Modal(
        CreateModal::new(MODAL_ID, "Create Mod Mail Ticket")
            .components(vec![
                CreateActionRow::InputText(
                    CreateInputText::new(
                        InputTextStyle::Paragraph,
                        "Your Message",
                        MODAL_CONTENT_ID,
                    )
                    .placeholder("Describe your issue in detail...")
                    .required(true)
                    .min_length(10)
                    .max_length(2000),
                ),
            ]),
    );

    match component.create_response(&ctx.http, modal).await {
        Ok(())  => println!("[modmail:create] Modal sent OK"),
        Err(e)  => println!("[modmail:create] ERROR create_response: {}", e),
    }
}

// ── Modal handler ────────────────────────────────────────────────────────────

/// Handles submission of the mod mail modal.
/// Acknowledges the modal and logs the content. Ticket creation is TODO.
pub async fn handle_modal(ctx: &Context, modal: &ModalInteraction) {
    let content = modal
        .data
        .components
        .iter()
        .flat_map(|row| row.components.iter())
        .find_map(|c| {
            if let ActionRowComponent::InputText(it) = c {
                if it.custom_id == MODAL_CONTENT_ID {
                    return it.value.as_deref();
                }
            }
            None
        })
        .unwrap_or("");

    println!(
        "[modmail:modal] Submission from {} (id={}) content={:?}",
        modal.user.name, modal.user.id, content
    );

    if let Err(e) = modal.defer(&ctx.http).await {
        println!("[modmail:modal] ERROR defer: {}", e);
        return;
    }

    // TODO: create ticket channel, relay content to staff
    println!("[modmail:modal] Ticket creation not yet implemented");
}

// ── Payload helpers ──────────────────────────────────────────────────────────

/// Enables the Create Message button in a Components V2 payload.
fn enable_create_button(payload: &mut serde_json::Value) {
    mutate_leaf_components(payload, |comp| {
        if comp.get("custom_id").and_then(|id| id.as_str()) == Some(CREATE_ID) {
            comp["disabled"] = serde_json::json!(false);
        }
    });
}

/// Disables every button (type 2) and select menu (type 3) in a Components V2 payload.
fn disable_all_components(payload: &mut serde_json::Value) {
    mutate_leaf_components(payload, |comp| {
        let t = comp.get("type").and_then(|t| t.as_u64()).unwrap_or(0);
        if matches!(t, 2 | 3) {
            comp["disabled"] = serde_json::json!(true);
        }
    });
}

/// Walks the known embed structure (container → rows → leaf components)
/// and calls `f` on each leaf component (buttons, selects).
///
/// Structure: payload.components[container].components[row].components[leaf]
fn mutate_leaf_components<F>(payload: &mut serde_json::Value, mut f: F)
where
    F: FnMut(&mut serde_json::Value),
{
    let Some(containers) = payload
        .get_mut("components")
        .and_then(|c| c.as_array_mut())
    else {
        return;
    };

    for container in containers.iter_mut() {
        let Some(rows) = container
            .get_mut("components")
            .and_then(|c| c.as_array_mut())
        else {
            continue;
        };

        for row in rows.iter_mut() {
            let Some(leaves) = row
                .get_mut("components")
                .and_then(|c| c.as_array_mut())
            else {
                continue;
            };

            for leaf in leaves.iter_mut() {
                f(leaf);
            }
        }
    }
}
