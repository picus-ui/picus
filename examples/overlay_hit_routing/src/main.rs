use std::sync::Arc;

use picus::{
    AppPicusExt, BevyWindowOptions, BuiltinUiAction, PicusPlugin, ProjectionCtx, UiAction, UiButton,
    UiComboBox, UiComboOption, UiComponent, UiComponentTemplate, UiFlexColumn, UiLabel, UiRoot,
    UiThemePicker, UiView, bevy_app::{App, Startup, Update},
    bevy_ecs::{message::MessageReader, prelude::*},
    register_ui_components,
    scene::{CommandsSceneExt, bsn},
    spawn_in_overlay_root,
    xilem::{
        view::{label, transformed},
        winit::{dpi::LogicalSize, error::EventLoopError},
    },
};
use shared_utils::init_logging;

#[derive(Component, Debug, Clone, UiComponent)]
#[ui_component(runtime_only)]
struct DemoToast {
    message: String,
}

#[derive(Component, Debug, Clone, Copy, Default)]
struct SpawnToastButton;

#[derive(Resource, Default)]
struct PendingToastSpawn(bool);

impl UiComponentTemplate for DemoToast {
    fn project(toast: &Self, _ctx: ProjectionCtx<'_>) -> UiView {
        Arc::new(transformed(label(toast.message.clone())).translate((520.0, 40.0)))
    }
}

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

fn mark_toast_spawn(
    mut reader: MessageReader<UiAction<BuiltinUiAction>>,
    spawn_buttons: Query<(), With<SpawnToastButton>>,
    mut pending: ResMut<PendingToastSpawn>,
) {
    for UiAction { source, action } in reader.read() {
        if matches!(action, BuiltinUiAction::Clicked) && spawn_buttons.get(*source).is_ok() {
            pending.0 = true;
        }
    }
}

fn spawn_toasts_from_clicks(world: &mut World) {
    let Some(mut pending) = world.get_resource_mut::<PendingToastSpawn>() else {
        return;
    };
    if !pending.0 {
        return;
    }
    pending.0 = false;
    drop(pending);

    let has_toast = {
        let mut query = world.query_filtered::<Entity, With<DemoToast>>();
        query.iter(world).next().is_some()
    };
    if has_toast {
        return;
    }

    spawn_in_overlay_root(
        world,
        (DemoToast {
            message: "🍞 Toast: I am outside OverlayStack logic.".to_string(),
        },),
    );
}

fn main() -> Result<(), EventLoopError> {
    init_logging();

    let mut app = App::new();
    app.add_plugins(PicusPlugin)
        .load_style_sheet_ron(include_str!("../assets/themes/overlay_hit_routing.ron"))
        .init_resource::<PendingToastSpawn>()
        .add_systems(Startup, setup_overlay_hit_routing_world)
        .add_systems(Update, (mark_toast_spawn, spawn_toasts_from_clicks).chain());

    register_ui_components!(&mut app, DemoToast);

    app.run_picus(
        "Overlay Hit Routing",
        BevyWindowOptions::default().with_initial_inner_size(LogicalSize::new(960.0, 640.0)),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_overlay_hit_routing_theme_ron_parses() {
        let sheet =
            picus::parse_stylesheet_ron(include_str!("../assets/themes/overlay_hit_routing.ron"))
                .expect("embedded overlay_hit_routing stylesheet should parse");
        assert_eq!(sheet.default_variant.as_deref(), None);
    }
}
