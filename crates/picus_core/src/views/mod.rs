//! Picus action view helpers.
//!
//! This module exposes Picus ECS action helpers. Low-level retained widgets remain available
//! from `picus_view::view` for projection internals, but Picus-facing helpers should route
//! user interaction through [`crate::UiEventQueue`].
//!
//! # Example
//!
//! ```
//! use picus_core::{
//!     button,
//!     bevy_ecs::world::World,
//! };
//!
//! let mut world = World::new();
//! let entity = world.spawn_empty().id();
//!
//! let _button = button(entity, (), "ECS event button");
//! ```
mod button_view;
mod button_with_child_view;
mod component_views;
mod drag_thumb_view;
mod entity_scope_view;
mod opaque_hitbox_view;
mod scroll_portal_view;

pub use button_view::button_view as button;
pub use button_view::{ButtonView, button_view};
pub use button_with_child_view::button_with_child_view as button_with_child;
pub use button_with_child_view::{ButtonWithChildView, button_with_child_view};
pub use component_views::checkbox_view as checkbox;
pub(crate) use component_views::radio_button_view;
pub use component_views::slider_view as slider;
pub use component_views::switch_view as switch;
pub use component_views::text_input_view as text_input;
pub use component_views::{
    CheckboxView, SliderView, SwitchView, checkbox_view, slider_view, switch_view, text_input_view,
};
pub use drag_thumb_view::{DragThumbView, drag_thumb_view};
pub use entity_scope_view::entity_scope;
pub use opaque_hitbox_view::{OpaqueHitboxView, opaque_hitbox, opaque_hitbox_for_entity};
pub use scroll_portal_view::{ScrollPortalView, scroll_portal};
