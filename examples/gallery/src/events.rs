//! Gallery event handling for interactive showcase controls.

use picus::app::bevy_ecs::{
    hierarchy::ChildOf, hierarchy::Children, message::MessageReader, prelude::*,
};
use picus::app::rfd;
use picus::clipboard::Clipboard;
use picus::prelude::{
    AppI18n, BuiltinUiAction, NavigationViewItem, OverlayPlacement, StyleClass, ToastKind,
    UiAction, UiComboBoxChanged, UiDialog, UiLabel, UiNavigationItem, UiNavigationSelectionChanged,
    UiNavigationView, UiPopover, UiRadioGroupChanged, UiSearchChanged, UiToast,
    WindowBackdropMaterial, set_theme_backdrop_material, spawn_in_overlay_root,
    spawn_manual_overlay_at, spawn_popover_in_overlay_root,
};

use crate::pages::rebuild_icon_grid;
use crate::state::{
    GalleryBackdropPicker, GalleryButtonAction, GalleryIconSearch, GalleryLocaleCombo, GalleryPage,
    GalleryRuntime,
};

#[derive(Resource, Default)]
pub struct PendingGalleryActions {
    navigation: Vec<UiAction<UiNavigationSelectionChanged>>,
    builtin: Vec<UiAction<BuiltinUiAction>>,
    radio: Vec<UiAction<UiRadioGroupChanged>>,
    combo: Vec<UiAction<UiComboBoxChanged>>,
    search: Vec<UiAction<UiSearchChanged>>,
}

pub fn collect_gallery_actions(
    mut navigation: MessageReader<UiAction<UiNavigationSelectionChanged>>,
    mut builtin: MessageReader<UiAction<BuiltinUiAction>>,
    mut radio: MessageReader<UiAction<UiRadioGroupChanged>>,
    mut combo: MessageReader<UiAction<UiComboBoxChanged>>,
    mut search: MessageReader<UiAction<UiSearchChanged>>,
    mut pending: ResMut<PendingGalleryActions>,
) {
    pending.navigation.extend(navigation.read().cloned());
    pending.builtin.extend(builtin.read().cloned());
    pending.radio.extend(radio.read().cloned());
    pending.combo.extend(combo.read().cloned());
    pending.search.extend(search.read().cloned());
}

/// Execute the gallery interactions that have visible effects.
pub fn apply_gallery_actions(world: &mut World) {
    let Some(mut rt) = world.get_resource::<GalleryRuntime>().cloned() else {
        return;
    };

    let pending = std::mem::take(&mut *world.resource_mut::<PendingGalleryActions>());

    for event in pending.search {
        if world.get::<GalleryIconSearch>(event.source).is_some()
            || world
                .get::<GalleryIconSearch>(event.action.search)
                .is_some()
        {
            rebuild_icon_grid(world, &event.action.value);
            continue;
        }
        if event.source != rt.search_input && event.action.search != rt.search_input {
            continue;
        }
        apply_nav_search_filter(world, &mut rt, &event.action.value);
    }

    for event in pending.navigation {
        // Only the shell nav view drives gallery page routing. Embedded samples
        // on the NavigationView page manage their own selection.
        if event.action.nav != rt.nav_view {
            continue;
        }
        if event.action.is_settings_selected {
            // Settings is a framework leaf after menu pages; keep selection without
            // remapping into GalleryPage content slots.
            continue;
        }
        set_gallery_page(world, &mut rt, event.action.selected);
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
                GalleryButtonAction::Dialog {
                    title,
                    body,
                    dismiss_label,
                } => {
                    spawn_dialog(world, &title, &body, &dismiss_label);
                }
                GalleryButtonAction::Info { message } => {
                    spawn_toast(world, &message, ToastKind::Info, 2.0);
                }
                GalleryButtonAction::ClipboardCopy { text } => {
                    apply_clipboard_copy(world, &text);
                }
                GalleryButtonAction::ClipboardRead => {
                    apply_clipboard_read(world);
                }
                GalleryButtonAction::PickFile => {
                    apply_pick_file(world);
                }
                GalleryButtonAction::PickFolder => {
                    apply_pick_folder(world);
                }
            }
        } else if let Some(marker) = world
            .get::<crate::pages::AnchoredFlyoutMarker>(event.source)
            .copied()
        {
            spawn_anchored_flyout(world, event.source, marker.placement);
        } else if let Some(pos) = world
            .get::<crate::pages::ManualOverlayMarkerAt>(event.source)
            .copied()
        {
            spawn_manual_popup(world, pos.x, pos.y);
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

    world.insert_resource(rt);
}

/// Build hierarchical nav items, optionally filtered by a case-insensitive query
/// against each page's label and description.
///
/// Returns `(items, leaf_to_page)` where `leaf_to_page[i]` is the
/// [`GalleryPage::ALL`] index for selectable leaf `i`.
pub fn build_gallery_nav_items_filtered(query: &str) -> (Vec<NavigationViewItem>, Vec<usize>) {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return (
            build_unfiltered_nav_items(),
            (0..GalleryPage::ALL.len()).collect(),
        );
    }

    let mut leaf_to_page = Vec::new();
    let mut items = Vec::new();

    for category in GalleryPage::CATEGORIES {
        let range = category.first_page_index..category.first_page_index + category.page_count;
        let mut children = Vec::new();
        for (offset, page) in GalleryPage::ALL[range].iter().enumerate() {
            let page_index = category.first_page_index + offset;
            let label = page.label();
            let description = page.description();
            if label.to_lowercase().contains(&q) || description.to_lowercase().contains(&q) {
                leaf_to_page.push(page_index);
                children.push(NavigationViewItem::new(label).with_icon(page.icon()));
            }
        }
        if !children.is_empty() {
            items.push(
                NavigationViewItem::new(category.label)
                    .with_children(children)
                    .expanded(),
            );
        }
    }

    (items, leaf_to_page)
}

fn build_unfiltered_nav_items() -> Vec<NavigationViewItem> {
    GalleryPage::CATEGORIES
        .iter()
        .enumerate()
        .map(|(category_index, category)| {
            let children = GalleryPage::ALL
                [category.first_page_index..category.first_page_index + category.page_count]
                .iter()
                .map(|page| NavigationViewItem::new(page.label()).with_icon(page.icon()))
                .collect::<Vec<_>>();
            // Expand only the category that owns the default selection. Fully
            // expanding every category mounted ~40 nav leaves into the retained
            // tree on every frame.
            let item = NavigationViewItem::new(category.label).with_children(children);
            if category_index == 0 {
                item.expanded()
            } else {
                item
            }
        })
        .collect()
}

fn apply_nav_search_filter(world: &mut World, rt: &mut GalleryRuntime, query: &str) {
    let (items, leaf_to_page) = build_gallery_nav_items_filtered(query);
    let filtering = !query.trim().is_empty();

    // Content children map 1:1 to leaf indices. Put matching pages first so the
    // filtered leaf index still selects the correct content child. When nothing
    // matches, leave the current page visible and clear the sidebar list.
    if !leaf_to_page.is_empty() {
        let content_order = content_order_for_pages(&rt.content_pages, &leaf_to_page);
        reorder_nav_content_children(world, rt.nav_view, &content_order);
    }

    let selected_leaf = if leaf_to_page.is_empty() {
        0
    } else {
        leaf_to_page
            .iter()
            .position(|&page| page == rt.current_page)
            .unwrap_or(0)
    };

    if let Some(&page) = leaf_to_page.get(selected_leaf) {
        rt.current_page = page;
    }

    rt.leaf_to_page = leaf_to_page;

    if let Some(mut nav) = world.get_mut::<UiNavigationView>(rt.nav_view) {
        nav.items = items;
        // Hide Settings while filtering so its leaf index cannot land on a
        // leftover content child after the filtered prefix.
        nav.is_settings_visible = !filtering;
        let max_leaf = nav.leaf_count().saturating_sub(1);
        nav.selected = selected_leaf.min(max_leaf);
    }
}

fn content_order_for_pages(all_pages: &[Entity], leaf_to_page: &[usize]) -> Vec<Entity> {
    let mut order = Vec::with_capacity(all_pages.len());
    let mut used = vec![false; all_pages.len()];
    for &idx in leaf_to_page {
        if idx < all_pages.len() && !used[idx] {
            order.push(all_pages[idx]);
            used[idx] = true;
        }
    }
    for (i, &entity) in all_pages.iter().enumerate() {
        if !used[i] {
            order.push(entity);
        }
    }
    order
}

/// Reorder the shell nav view's content page children while preserving template
/// item entities (menu / footer / settings).
fn reorder_nav_content_children(world: &mut World, nav: Entity, content_order: &[Entity]) {
    let children: Vec<Entity> = world
        .get::<Children>(nav)
        .map(|c| c.iter().collect())
        .unwrap_or_default();

    let content_set: std::collections::HashSet<Entity> = content_order.iter().copied().collect();
    let mut item_children = Vec::new();
    for child in children {
        if world.get::<UiNavigationItem>(child).is_some() {
            item_children.push(child);
        } else if !content_set.contains(&child) {
            // Keep any unexpected non-content / non-item children in place.
            item_children.push(child);
        }
    }

    let mut new_order = item_children;
    new_order.extend_from_slice(content_order);
    world.entity_mut(nav).replace_children(&new_order);
}

fn set_gallery_page(world: &mut World, rt: &mut GalleryRuntime, leaf: usize) {
    if let Some(&page) = rt.leaf_to_page.get(leaf) {
        rt.current_page = page;
    }
    if let Some(mut nav_view) = world.get_mut::<UiNavigationView>(rt.nav_view) {
        // Unified leaf index spans menu leaves (+ footer/settings when present).
        nav_view.selected = leaf.min(nav_view.leaf_count().saturating_sub(1));
    }
}

fn spawn_dialog(world: &mut World, title: &str, body: &str, dismiss_label: &str) {
    let mut dialog = UiDialog::new(title, body).with_fixed_width(460.0);
    dialog.dismiss_label = dismiss_label.to_string();
    spawn_in_overlay_root(world, (dialog,));
}

/// WinUI Flyout: anchored light-dismiss panel via UiPopover.
fn spawn_anchored_flyout(world: &mut World, anchor: Entity, placement: OverlayPlacement) {
    let popover = UiPopover::new(anchor)
        .with_placement(placement)
        .with_auto_flip_placement(true)
        .with_fixed_size(280.0, 120.0);
    let entity = spawn_popover_in_overlay_root(
        world,
        (StyleClass(vec!["overlay.menu.panel".to_string()]),),
        popover,
    );
    world.spawn((
        UiLabel::new(format!(
            "Anchored flyout\n(WinUI Flyout ≈ UiPopover)\nplacement: {placement:?}"
        )),
        ChildOf(entity),
    ));
}

/// WinUI Popup: explicit pixel origin via spawn_manual_overlay_at.
///
/// Uses a lightweight panel (not [`UiDialog`]) so Popup demos stay distinct from
/// ContentDialog / modal dialog chrome on the Dialog page.
fn spawn_manual_popup(world: &mut World, x: f64, y: f64) {
    let entity = spawn_manual_overlay_at(
        world,
        (
            StyleClass(vec!["overlay.menu.panel".to_string()]),
            // UiPopover provides the projecting panel surface; ManualOverlayPosition
            // (from spawn_manual_overlay_at) keeps the explicit (x, y) origin.
            UiPopover::new(Entity::PLACEHOLDER)
                .with_placement(OverlayPlacement::TopStart)
                .with_auto_flip_placement(false)
                .with_fixed_size(300.0, 96.0),
        ),
        x,
        y,
    );
    world.spawn((
        UiLabel::new(format!(
            "Popup at ({x:.0}, {y:.0})\nWinUI Popup ≈ spawn_manual_overlay_at"
        )),
        ChildOf(entity),
    ));
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

/// Max Unicode characters shown in gallery toast previews (clipboard / paths).
const TOAST_PREVIEW_CHARS: usize = 120;

/// Truncate long toast content with a trailing ellipsis (Unicode-safe).
fn toast_preview(text: &str) -> String {
    if text.chars().count() > TOAST_PREVIEW_CHARS {
        let truncated: String = text.chars().take(TOAST_PREVIEW_CHARS).collect();
        format!("{truncated}\u{2026}")
    } else {
        text.to_string()
    }
}

fn apply_clipboard_copy(world: &mut World, text: &str) {
    // `Clipboard::set_text` swallows arboard errors; verify with get_text before Success.
    let outcome = match world.get_resource::<Clipboard>() {
        None => Err((
            "Clipboard resource is not available.".to_string(),
            ToastKind::Error,
        )),
        Some(clipboard) if !clipboard.is_available() => Err((
            "System clipboard is unavailable in this environment.".to_string(),
            ToastKind::Warning,
        )),
        Some(clipboard) => {
            clipboard.set_text(text);
            match clipboard.get_text() {
                Some(written) if written == text => Ok(()),
                Some(_) | None => Err((
                    "Clipboard write could not be verified.".to_string(),
                    ToastKind::Warning,
                )),
            }
        }
    };
    match outcome {
        Ok(()) => spawn_toast(
            world,
            &format!("Copied to clipboard: {}", toast_preview(text)),
            ToastKind::Success,
            2.4,
        ),
        Err((message, kind)) => spawn_toast(world, &message, kind, 3.0),
    }
}

fn apply_clipboard_read(world: &mut World) {
    let outcome = match world.get_resource::<Clipboard>() {
        None => Err((
            "Clipboard resource is not available.".to_string(),
            ToastKind::Error,
            3.0,
        )),
        Some(clipboard) if !clipboard.is_available() => Err((
            "System clipboard is unavailable in this environment.".to_string(),
            ToastKind::Warning,
            3.0,
        )),
        Some(clipboard) => match clipboard.get_text() {
            Some(text) if !text.is_empty() => {
                Ok(format!("Clipboard: {}", toast_preview(&text)))
            }
            Some(_) => Err(("Clipboard is empty.".to_string(), ToastKind::Info, 2.0)),
            None => Err((
                "Could not read clipboard text.".to_string(),
                ToastKind::Warning,
                2.4,
            )),
        },
    };
    match outcome {
        Ok(message) => spawn_toast(world, &message, ToastKind::Info, 3.2),
        Err((message, kind, duration)) => spawn_toast(world, &message, kind, duration),
    }
}

fn apply_pick_file(world: &mut World) {
    // Native modal dialogs intentionally block until dismissed.
    match rfd::FileDialog::new()
        .set_title("Pick a file")
        .pick_file()
    {
        Some(path) => {
            let display = toast_preview(&path.display().to_string());
            spawn_toast(
                world,
                &format!("Selected file: {display}"),
                ToastKind::Success,
                4.0,
            );
        }
        None => spawn_toast(world, "File pick cancelled.", ToastKind::Info, 2.0),
    }
}

fn apply_pick_folder(world: &mut World) {
    match rfd::FileDialog::new()
        .set_title("Pick a folder")
        .pick_folder()
    {
        Some(path) => {
            let display = toast_preview(&path.display().to_string());
            spawn_toast(
                world,
                &format!("Selected folder: {display}"),
                ToastKind::Success,
                4.0,
            );
        }
        None => spawn_toast(world, "Folder pick cancelled.", ToastKind::Info, 2.0),
    }
}
