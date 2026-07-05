use crate::helpers::{card, grid, placeholder};
use bevy_ecs::{hierarchy::ChildOf, prelude::*};
use picus::{
    HasTooltip, UiButton, UiComboBox, UiComboOption, UiContextMenuItem, UiContextMenuTrigger,
    UiMultilineTextInput, UiNumericUpDown, UiPasswordInput, UiSlider, UiTextInput,
    scene::{CommandsSceneExt, bsn, template_value},
};

/// Text input, password, multiline text, combo box, and tooltip component examples.
///
/// Corresponds to Fluent UI's TextField, ComboBox, and Tooltip components.
pub fn spawn_inputs_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 3);

    let text = card(commands, g, "TextBox");
    commands.spawn_scene(bsn! {
        template_value(UiTextInput::new("").with_placeholder("Type your name..."))
        ChildOf(text)
    });
    commands.spawn_scene(bsn! {
        template_value(UiTextInput::new("This is my name"))
        ChildOf(text)
    });
    commands.spawn_scene(bsn! {
        template_value(UiTextInput::new("Read/write ECS text"))
        ChildOf(text)
    });

    let password = card(commands, g, "PasswordBox");
    commands.spawn_scene(bsn! {
        template_value(UiPasswordInput::new("").with_placeholder("Password"))
        ChildOf(password)
    });
    commands.spawn_scene(bsn! {
        template_value(UiPasswordInput::new("secret").with_mask('*'))
        ChildOf(password)
    });
    commands.spawn_scene(bsn! {
        template_value(UiPasswordInput::new("disabled placeholder").read_only(true))
        ChildOf(password)
    });

    let multiline = card(commands, g, "MultiLineTextBox");
    commands.spawn_scene(bsn! {
        template_value(UiMultilineTextInput::new(
            "The quick brown fox jumps over the lazy dog.\n\n- Wrap supported\n- Selection is provided by Masonry text input\n- ECS value sync is enabled",
        ).with_placeholder("Notes"))
        ChildOf(multiline)
    });

    let combo = card(commands, g, "Combo / Numeric");
    let mut language = UiComboBox::new(vec![
        UiComboOption::new("rust", "Rust"),
        UiComboOption::new("csharp", "C#"),
        UiComboOption::new("swift", "Swift"),
        UiComboOption::new("kotlin", "Kotlin"),
    ])
    .with_placeholder("Pick a language");
    language.selected = 0;
    commands.spawn_scene(bsn! {
        template_value(language)
        ChildOf(combo)
    });
    commands.spawn_scene(bsn! {
        template_value(UiSlider::new(0.0, 100.0, 42.5).with_step(0.5))
        ChildOf(combo)
    });
    commands.spawn_scene(bsn! {
        template_value(
            UiNumericUpDown::new(0.0, 100.0, 25.0)
                .with_step(5.0)
                .with_suffix(" px")
        )
        ChildOf(combo)
    });
    commands.spawn_scene(bsn! {
        template_value(
            UiNumericUpDown::new(0.0, 1.0, 0.30)
                .with_step(0.05)
                .with_precision(2)
                .with_suffix(" s")
        )
        ChildOf(combo)
    });

    let tooltip = card(commands, g, "ToolTip / Context");
    commands.spawn_scene(bsn! {
        template_value(UiButton::new("Hover for tooltip"))
        template_value(HasTooltip::new("Tooltip overlay anchored to this button."))
        ChildOf(tooltip)
    });
    // Right-click on the button below to open a context menu.
    let ctx_btn = commands
        .spawn_scene(bsn! {
            template_value(UiButton::new("Right-click for context menu"))
            ChildOf(tooltip)
        })
        .id();
    commands.entity(ctx_btn).insert(UiContextMenuTrigger::new([
        UiContextMenuItem::new("Cut"),
        UiContextMenuItem::new("Copy"),
        UiContextMenuItem::new("Paste"),
        UiContextMenuItem::new("Separator").with_separator(),
        UiContextMenuItem::new("Select All"),
    ]));

    let drag_drop = card(commands, g, "Drag and Drop");
    placeholder(
        commands,
        drag_drop,
        "Window drag/drop",
        "Picus input bridge does not expose platform file-drop IDataObject/XDND events to ECS yet.",
    );

    commands
        .spawn_scene(bsn! {
            template_value(UiButton::new("Show persistent toast"))
            ChildOf(text)
        })
        .id()
}
