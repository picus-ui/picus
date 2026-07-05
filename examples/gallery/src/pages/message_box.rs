use crate::helpers::{card, dialog_button, grid, note};
use bevy_ecs::{hierarchy::ChildOf, prelude::*};
use picus::{
    UiButton,
    scene::{CommandsSceneExt, bsn, template_value},
};

/// Dialog and message box component examples.
///
/// Corresponds to Fluent UI's Dialog and MessageBar components.
pub fn spawn_message_box_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let dialog = card(commands, g, "Dialog / MessageBox");
    let error_btn = commands
        .spawn_scene(bsn! {
            template_value(UiButton::new("Show Error Toast"))
            ChildOf(dialog)
        })
        .id();
    dialog_button(
        commands,
        dialog,
        "Info Dialog",
        "Info",
        "This is an informational dialog spawned from the MessageBox page.",
    );
    dialog_button(
        commands,
        dialog,
        "Warning Dialog",
        "Warning",
        "This is a warning dialog spawned from the MessageBox page.",
    );
    dialog_button(
        commands,
        dialog,
        "Error Dialog",
        "Error",
        "This is an error dialog spawned from the MessageBox page.",
    );
    note(
        commands,
        dialog,
        "UiDialog provides modal dialogs with title, body, and dismiss actions.",
    );

    let prompt = card(commands, g, "Prompt");
    dialog_button(
        commands,
        prompt,
        "Prompt Placeholder",
        "Prompt Placeholder",
        "Picus UiDialog does not yet expose an input slot, so the prompt sample is represented here.",
    );

    let native = card(commands, g, "Native Message Hook");
    dialog_button(
        commands,
        native,
        "Native Message Hook",
        "Native Hook Placeholder",
        "Platform-native message hooks are not part of the public Picus runtime API.",
    );

    error_btn
}
