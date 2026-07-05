//! Canvas shapes and brush color swatch component examples.
//!
//! Corresponds to Fluent UI's shape primitives and theme color swatch patterns.

use crate::helpers::{card, class, grid, placeholder, sample_canvas};
use bevy_ecs::{hierarchy::ChildOf, prelude::*};
use picus::{
    UiCanvas, UiCanvasCommand, UiGradientStop, UiLabel,
    scene::{CommandsSceneExt, bsn, template_value},
    xilem::Color,
};

/// Canvas shapes and brush/swatch color component examples.
///
/// Demonstrates available canvas drawing commands (rectangles, circles, paths)
/// and color swatch labels that map to theme palette tokens.
pub fn spawn_shapes_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let primitives = card(commands, g, "Shapes");
    commands.spawn_scene(bsn! {
        template_value(sample_canvas())
        template_value(class("gallery.canvas"))
        ChildOf(primitives)
    });

    let fills = card(commands, g, "Brushes");
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Red"))
        template_value(class("gallery.swatch.red"))
        ChildOf(fills)
    });
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Green"))
        template_value(class("gallery.swatch.green"))
        ChildOf(fills)
    });
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Blue"))
        template_value(class("gallery.swatch.blue"))
        ChildOf(fills)
    });
    commands.spawn_scene(bsn! {
        template_value(UiLabel::new("Gold"))
        template_value(class("gallery.swatch.gold"))
        ChildOf(fills)
    });

    // Linear and radial gradient brush demos.
    let gradients = card(commands, g, "Gradient brushes");
    commands.spawn_scene(bsn! {
        template_value(
            UiCanvas::new()
                .with_alt_text("Linear gradient sample")
                .with_size(320.0, 120.0)
                .with_command(UiCanvasCommand::FillLinearGradientRect {
                    x: 8.0,
                    y: 8.0,
                    width: 304.0,
                    height: 104.0,
                    start_x: 8.0,
                    start_y: 8.0,
                    end_x: 312.0,
                    end_y: 8.0,
                    stops: vec![
                        UiGradientStop::new(0.0, Color::from_rgb8(0x25, 0x63, 0xEB)),
                        UiGradientStop::new(0.5, Color::from_rgb8(0x7C, 0x3A, 0xED)),
                        UiGradientStop::new(1.0, Color::from_rgb8(0xDB, 0x27, 0x77)),
                    ],
                })
        )
        template_value(class("gallery.canvas"))
        ChildOf(gradients)
    });
    commands.spawn_scene(bsn! {
        template_value(
            UiCanvas::new()
                .with_alt_text("Radial gradient sample")
                .with_size(320.0, 120.0)
                .with_command(UiCanvasCommand::FillRadialGradientCircle {
                    cx: 160.0,
                    cy: 60.0,
                    radius: 52.0,
                    inner_radius: 0.0,
                    stops: vec![
                        UiGradientStop::new(0.0, Color::from_rgb8(0xF9, 0x73, 0x16)),
                        UiGradientStop::new(1.0, Color::from_rgb8(0x1E, 0x29, 0x3B)),
                    ],
                })
        )
        template_value(class("gallery.canvas"))
        ChildOf(gradients)
    });

    placeholder(
        commands,
        g,
        "Shape hit testing",
        "Canvas drawing is visual only; per-shape pointer hit testing is not a public component contract.",
    );

    parent
}
