use bevy_ecs::prelude::*;

use crate::{ProjectionCtx, UiView, components::UiComponentTemplate};

/// A single item in the navigation view sidebar.
///
/// Each item has a display label and an optional icon glyph
/// from the bundled Lucide icon font.
#[derive(Debug, Clone, Default)]
pub struct NavigationViewItem {
    /// Human-readable label shown in the sidebar.
    pub label: String,
    /// Optional Lucide icon glyph (single Unicode character from the Lucide font).
    pub icon: Option<char>,
}

impl NavigationViewItem {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
        }
    }

    #[must_use]
    pub fn with_icon(mut self, icon: char) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// Sidebar navigation container with items and a content area.
///
/// The sidebar is rendered as a vertical list of navigation items (with optional
/// Lucide icon glyphs). The content area displays the ECS child at [`selected`]
/// index — analogous to a [`UiTabBar`](crate::UiTabBar) with hidden headers but
/// with a separate navigation panel.
///
/// # Styling classes
///
/// The projector resolves these class names from the style system:
/// - `"nav.sidebar"` — sidebar panel
/// - `"nav.item"` — each navigation button (base)
/// - `"nav.item.active"` — the active navigation button
/// - `"nav.content"` — content area wrapper
#[derive(Component, Debug, Clone)]
pub struct UiNavigationView {
    /// Navigation items displayed in the sidebar.
    pub items: Vec<NavigationViewItem>,
    /// Index of the currently selected item.
    pub selected: usize,
}

impl UiNavigationView {
    #[must_use]
    pub fn new(items: impl IntoIterator<Item = NavigationViewItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
            selected: 0,
        }
    }

    #[must_use]
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }
}

impl Default for UiNavigationView {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Emitted when the selected item in a [`UiNavigationView`] changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiNavigationSelectionChanged {
    /// The navigation view entity.
    pub nav: Entity,
    /// The newly selected index.
    pub selected: usize,
}

impl UiComponentTemplate for UiNavigationView {
    fn project(component: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        crate::projection::widgets::project_navigation_view(component, ctx)
    }
}
