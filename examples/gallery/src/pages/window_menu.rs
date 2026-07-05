//! MenuBar and window menu component examples.
//!
//! Corresponds to Fluent UI's CommandBar and ContextualMenu components.

use crate::helpers::{card, grid, note, toast_button};
use bevy_ecs::{hierarchy::ChildOf, prelude::*};
use picus::{
    ToastKind, UiMenuBar, UiMenuBarItem, UiMenuItem, UiTitleBar,
    scene::{CommandsSceneExt, bsn, template_value},
};

/// MenuBar and window menu component examples.
///
/// Picus supports horizontal menu bars with dropdown panels.
/// Each menu bar item contains a list of menu items that open in an overlay panel.
pub fn spawn_window_menu_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let menu = card(commands, g, "MenuBar");
    let menu_bar = commands
        .spawn_scene(bsn! {
            UiMenuBar
            ChildOf(menu)
        })
        .id();
    commands.spawn_scene(bsn! {
        template_value(UiMenuBarItem::new(
            "File",
            vec![
                UiMenuItem::new("New", "new"),
                UiMenuItem::new("Open...", "open"),
                UiMenuItem::new("Save", "save"),
                UiMenuItem::new("Exit", "exit"),
            ],
        ))
        ChildOf(menu_bar)
    });
    commands.spawn_scene(bsn! {
        template_value(UiMenuBarItem::new(
            "Edit",
            vec![
                UiMenuItem::new("Undo", "undo"),
                UiMenuItem::new("Redo", "redo"),
                UiMenuItem::new("Preferences", "prefs"),
            ],
        ))
        ChildOf(menu_bar)
    });
    commands.spawn_scene(bsn! {
        template_value(UiMenuBarItem::new(
            "Help",
            vec![UiMenuItem::new("About Picus Gallery", "about")],
        ))
        ChildOf(menu_bar)
    });
    note(
        commands,
        menu,
        "MenuBar supports nested items and dropdown overlay panels.",
    );

    // Picus provides a UiTitleBar component that draws a custom title bar with
    // minimize/maximize/close buttons emitting TitleBarAction events.
    let chrome = card(commands, g, "Custom title bar");
    commands.spawn_scene(bsn! {
        template_value(UiTitleBar {
            title: "Picus Gallery — custom title bar".to_string(),
            ..Default::default()
        })
        ChildOf(chrome)
    });
    note(
        commands,
        chrome,
        "UiTitleBar draws a custom window chrome with minimize/maximize/close controls.",
    );

    toast_button(
        commands,
        menu,
        "Warning Toast",
        "Window/Menu placeholder warning.",
        ToastKind::Warning,
        3.2,
    )
}
