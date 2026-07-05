use crate::helpers::{card, class, grid, note};
use bevy_ecs::{hierarchy::ChildOf, prelude::*};
use picus::{
    UiButton, UiCheckbox, UiFlexColumn, UiGroupBox, UiLabel, UiListView, UiMultilineTextInput,
    UiSplitPane, UiTabBar, UiTextInput,
    scene::{CommandsSceneExt, bsn, template_value},
};

/// Grouping, SplitPane, TabBar, and Popover component examples.
///
/// SplitPane, Pivot/Tabs, and Popover mirror Fluent patterns; GroupBox is a
/// Picus-owned grouping helper styled locally by the gallery.
pub fn spawn_panels_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let group_box = card(commands, g, "Grouping / Cards");
    let inner = commands
        .spawn_scene(bsn! {
            template_value(UiGroupBox::new("Nested group"))
            template_value(class("gallery.group_box"))
            ChildOf(group_box)
        })
        .id();
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Labels and controls can be grouped."))
        ChildOf(inner)
    });
    commands.spawn_scene(bsn! {
        template_value(UiCheckbox::new("Inside a group", true))
        ChildOf(inner)
    });

    let split = card(commands, g, "SplitPane");
    let pane = commands
        .spawn_scene(bsn! {
            template_value(UiSplitPane::new(0.42))
            ChildOf(split)
        })
        .id();
    let left = commands
        .spawn_scene(bsn! {
            UiFlexColumn
            template_value(class("gallery.split_panel"))
            ChildOf(pane)
        })
        .id();
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Left panel"))
        ChildOf(left)
    });
    commands.spawn_scene(bsn! {
        template_value(
            UiListView::new(["One", "Two", "Three"]).with_selected(0)
        )
        ChildOf(left)
    });
    let right = commands
        .spawn_scene(bsn! {
            UiFlexColumn
            template_value(class("gallery.split_panel"))
            ChildOf(pane)
        })
        .id();
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Right panel"))
        ChildOf(right)
    });
    commands.spawn_scene(bsn! {
        template_value(UiTextInput::new("Resizable split content"))
        ChildOf(right)
    });

    let tabs = card(commands, g, "Tabs");
    let tab_bar = commands
        .spawn_scene(bsn! {
            template_value(UiTabBar::new(["Details", "Settings", "Logs"]))
            ChildOf(tabs)
        })
        .id();
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Details tab content"))
        ChildOf(tab_bar)
    });
    commands.spawn_scene(bsn! {
        template_value(UiCheckbox::new("Enable option", true))
        ChildOf(tab_bar)
    });
    commands.spawn_scene(bsn! {
        template_value(UiMultilineTextInput::new("Log line 1\nLog line 2"))
        ChildOf(tab_bar)
    });

    let popover = card(commands, g, "Popover");
    let pop_btn = commands
        .spawn_scene(bsn! {
            template_value(UiButton::new("Open popover dialog"))
            ChildOf(popover)
        })
        .id();
    note(
        commands,
        popover,
        "Picus popovers are used by combo boxes, menus, pickers, and tooltips.",
    );

    pop_btn
}
