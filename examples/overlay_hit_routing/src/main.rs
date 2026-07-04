use std::sync::Arc;

use picus::{
    AppPicusExt, BuiltinUiAction, PicusPlugin, ProjectionCtx, UiButton, UiComboBox, UiComboOption,
    UiEventQueue, UiFlexColumn, UiLabel, UiRoot, UiThemePicker, UiView,
    bevy_app::{App, PreUpdate, Startup},
    bevy_ecs::prelude::*,
    run_app_with_window_options,
    scene::{CommandsSceneExt, bsn},
    spawn_in_overlay_root,
    xilem::{
        view::{label, transformed},
        winit::{dpi::LogicalSize, error::EventLoopError},
    },
};
use shared_utils::init_logging;

#[derive(Component, Debug, Clone)]
struct UiToast {
    message: String,
}

#[derive(Component, Debug, Clone, Copy, Default)]
struct SpawnToastButton;

fn project_ui_toast(toast: &UiToast, _ctx: ProjectionCtx<'_>) -> UiView {
    Arc::new(transformed(label(toast.message.clone())).translate((520.0, 40.0)))
}

picus::impl_ui_component_template!(UiToast, project_ui_toast);

fn setup_overlay_hit_routing_world(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        UiRoot
        UiFlexColumn
        Children [
            UiThemePicker,
            UiLabel {
                text: {
                    "Open the dropdown, spawn a toast, then click the toast.\n\
                     Expected: dropdown closes immediately; toast stays visible."
                        .to_string()
                },
            },
            UiComboBox {
                options: {
                    vec![
                        UiComboOption::new("alpha", "Alpha"),
                        UiComboOption::new("beta", "Beta"),
                        UiComboOption::new("gamma", "Gamma"),
                    ]
                },
                placeholder: { "Open dropdown".to_string() },
            },
            (
                UiButton {
                    label: { "Spawn Toast".to_string() },
                }
                SpawnToastButton
            ),
        ]
    });
}

fn drain_overlay_hit_routing_events(world: &mut World) {
    let button_events = world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<BuiltinUiAction>();

    if button_events.is_empty() {
        return;
    }

    for event in button_events {
        if !matches!(event.action, BuiltinUiAction::Clicked) {
            continue;
        }

        if world.get::<SpawnToastButton>(event.entity).is_none() {
            continue;
        }

        let has_toast = {
            let mut query = world.query_filtered::<Entity, With<UiToast>>();
            query.iter(world).next().is_some()
        };

        if has_toast {
            continue;
        }

        spawn_in_overlay_root(
            world,
            (UiToast {
                message: "🍞 Toast: I am outside OverlayStack logic.".to_string(),
            },),
        );
    }
}

fn build_overlay_hit_routing_app() -> App {
    init_logging();

    let mut app = App::new();
    app.add_plugins(PicusPlugin)
        .load_style_sheet_ron(include_str!("../assets/themes/overlay_hit_routing.ron"))
        .register_ui_component::<UiToast>()
        .add_systems(Startup, setup_overlay_hit_routing_world)
        .add_systems(PreUpdate, drain_overlay_hit_routing_events);

    app
}

fn main() -> Result<(), EventLoopError> {
    run_app_with_window_options(
        build_overlay_hit_routing_app(),
        "Overlay Hit Routing",
        |opts| opts.with_initial_inner_size(LogicalSize::new(960.0, 640.0)),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_overlay_hit_routing_theme_ron_parses() {
        picus::parse_stylesheet_ron(include_str!("../assets/themes/overlay_hit_routing.ron"))
            .expect("embedded overlay_hit_routing stylesheet should parse");
    }
}
