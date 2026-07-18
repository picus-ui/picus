//! System integration pages (clipboard and storage pickers).
//!
//! WinUI mapping:
//! - **Clipboard** → `picus::clipboard` (`Clipboard` resource + event helpers)
//! - **StoragePickers** → `picus::app::rfd` (`FileDialog` open/folder pickers)

use crate::helpers::{card, grid, note};
use crate::state::GalleryButtonAction;
use bevy_ecs::{hierarchy::ChildOf, prelude::*};
use picus::prelude::UiButton;
use picus::scene::{bsn, template_value, CommandsSceneExt};

const SAMPLE_CLIPBOARD_TEXT: &str = "Hello from Picus Gallery clipboard demo";

/// WinUI `Clipboard` sample: set/get system clipboard text via the facade.
pub fn spawn_clipboard_page(commands: &mut Commands, parent: Entity) {
    let g = grid(commands, parent, 2);

    let set_get = card(commands, g, "Set and get text");
    let copy_btn = commands
        .spawn_scene(bsn! {
            template_value(UiButton::new("Copy sample text"))
            ChildOf(set_get)
        })
        .id();
    commands
        .entity(copy_btn)
        .insert(GalleryButtonAction::ClipboardCopy {
            text: SAMPLE_CLIPBOARD_TEXT.to_string(),
        });

    let read_btn = commands
        .spawn_scene(bsn! {
            template_value(UiButton::new("Read clipboard"))
            ChildOf(set_get)
        })
        .id();
    commands
        .entity(read_btn)
        .insert(GalleryButtonAction::ClipboardRead);

    note(
        commands,
        set_get,
        "Uses picus::clipboard::Clipboard (system clipboard via arboard). Unavailable in headless environments.",
    );

    let mapping = card(commands, g, "WinUI name mapping");
    note(
        commands,
        mapping,
        "WinUI UniqueId Clipboard → picus::clipboard::{Clipboard, ClipboardEvent, ClipboardKind, ClipboardText}. PicusPlugin registers the Clipboard resource and handle_clipboard_events.",
    );
}

/// WinUI `StoragePickers` sample: native file/folder dialogs via `rfd`.
pub fn spawn_storage_pickers_page(commands: &mut Commands, parent: Entity) {
    let g = grid(commands, parent, 2);

    let open = card(commands, g, "Open file picker");
    let file_btn = commands
        .spawn_scene(bsn! {
            template_value(UiButton::new("Pick a file\u{2026}"))
            ChildOf(open)
        })
        .id();
    commands
        .entity(file_btn)
        .insert(GalleryButtonAction::PickFile);
    note(
        commands,
        open,
        "picus::app::rfd::FileDialog::new().pick_file() — returns an optional PathBuf shown as a toast.",
    );

    let folder = card(commands, g, "Folder picker");
    let folder_btn = commands
        .spawn_scene(bsn! {
            template_value(UiButton::new("Pick a folder\u{2026}"))
            ChildOf(folder)
        })
        .id();
    commands
        .entity(folder_btn)
        .insert(GalleryButtonAction::PickFolder);
    note(
        commands,
        folder,
        "picus::app::rfd::FileDialog::new().pick_folder() — folder pickers use the same rfd re-export as MessageDialog.",
    );

    let mapping = card(commands, g, "WinUI name mapping");
    note(
        commands,
        mapping,
        "WinUI StoragePickers (FileOpenPicker / FolderPicker) map to the rfd crate re-exported at picus::app::rfd (also in prelude via app::*). Dialogs are modal and block until dismissed.",
    );
}
