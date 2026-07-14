//! Gallery event handling for interactive showcase controls.

use picus::app::bevy_ecs::{message::MessageReader, prelude::*};
use picus::prelude::{
    AppI18n, BuiltinUiAction, OverlayPlacement, ToastKind, UiAction, UiComboBoxChanged, UiDialog,
    UiNavigationSelectionChanged, UiNavigationView, UiRadioGroupChanged, UiToast,
    WindowBackdropMaterial, set_theme_backdrop_material, spawn_in_overlay_root,
    spawn_manual_overlay_at,
};

use crate::state::{
    GalleryBackdropPicker, GalleryButtonAction, GalleryLocaleCombo, GalleryRuntime,
};

#[derive(Resource, Default)]
pub struct PendingGalleryActions {
    navigation: Vec<UiAction<UiNavigationSelectionChanged>>,
    builtin: Vec<UiAction<BuiltinUiAction>>,
    radio: Vec<UiAction<UiRadioGroupChanged>>,
    combo: Vec<UiAction<UiComboBoxChanged>>,
}

pub fn collect_gallery_actions(
    mut navigation: MessageReader<UiAction<UiNavigationSelectionChanged>>,
    mut builtin: MessageReader<UiAction<BuiltinUiAction>>,
    mut radio: MessageReader<UiAction<UiRadioGroupChanged>>,
    mut combo: MessageReader<UiAction<UiComboBoxChanged>>,
    mut pending: ResMut<PendingGalleryActions>,
) {
    pending.navigation.extend(navigation.read().cloned());
    pending.builtin.extend(builtin.read().cloned());
    pending.radio.extend(radio.read().cloned());
    pending.combo.extend(combo.read().cloned());
}

/// Execute the gallery interactions that have visible effects.
pub fn apply_gallery_actions(world: &mut World) {
    let Some(rt) = world.get_resource::<GalleryRuntime>().cloned() else {
        return;
    };

    let pending = std::mem::take(&mut *world.resource_mut::<PendingGalleryActions>());

    for event in pending.navigation {
        if event.action.is_settings_selected {
            // Settings is a framework leaf after menu pages; keep selection without
            // remapping into GalleryPage content slots.
            continue;
        }
        set_gallery_page(world, &rt, event.action.selected);
    }

    for event in pending.builtin {
        if !matches!(event.action, BuiltinUiAction::Clicked) {
            continue;
        }

        if let Some(action) = world.get::<GalleryButtonAction>(event.source).cloned() {
            match action {
                GalleryButtonAction::Toast {
                    message,
                    kind,
                    duration,
                } => spawn_toast(world, &message, kind, duration),
                GalleryButtonAction::Dialog { title, body } => {
                    spawn_dialog(world, &title, &body);
                }
                GalleryButtonAction::Info { message } => {
                    spawn_toast(world, &message, ToastKind::Info, 2.0);
                }
            }
        } else if world
            .get::<crate::pages::ManualOverlayMarker>(event.source)
            .is_some()
        {
            spawn_manual_overlay_at(
                world,
                UiDialog::new(
                    "Manual overlay",
                    "This popover was positioned at a fixed (x, y) pixel coordinate via spawn_manual_overlay_at.",
                )
                .with_fixed_width(360.0),
                120.0,
                80.0,
            );
        }
    }

    for event in pending.radio {
        if world
            .get::<GalleryBackdropPicker>(event.action.group)
            .is_some()
        {
            let material = match event.action.selected {
                0 => WindowBackdropMaterial::None,
                2 => WindowBackdropMaterial::Acrylic,
                _ => WindowBackdropMaterial::Mica,
            };
            set_theme_backdrop_material(world, material);
        }
    }

    for event in pending.combo {
        if world.get::<GalleryLocaleCombo>(event.source).is_some()
            && let Ok(locale) = event
                .action
                .value
                .parse::<unic_langid::LanguageIdentifier>()
        {
            world.resource_mut::<AppI18n>().set_active_locale(locale);
        }
    }
}

fn set_gallery_page(world: &mut World, rt: &GalleryRuntime, page: usize) {
    if let Some(mut nav_view) = world.get_mut::<UiNavigationView>(rt.nav_view) {
        // Unified leaf index spans menu leaves (+ footer/settings when present).
        nav_view.selected = page.min(nav_view.leaf_count().saturating_sub(1));
    }
}

fn spawn_dialog(world: &mut World, title: &str, body: &str) {
    spawn_in_overlay_root(world, (UiDialog::new(title, body).with_fixed_width(460.0),));
}

fn spawn_toast(world: &mut World, message: &str, kind: ToastKind, duration: f32) {
    spawn_in_overlay_root(
        world,
        (UiToast::new(message)
            .with_kind(kind)
            .with_duration(duration)
            .with_min_width(320.0)
            .with_max_width(480.0)
            .with_placement(OverlayPlacement::BottomEnd),),
    );
}
