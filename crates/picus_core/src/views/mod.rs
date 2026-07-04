//! View helpers exported by `picus_core`.
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
mod ecs_button_view;
mod ecs_button_with_child_view;
mod ecs_component_views;
mod ecs_drag_thumb_view;
mod entity_scope_view;
mod opaque_hitbox_view;
mod scroll_portal_view;

pub use ecs_button_view::ecs_button as button;
pub use ecs_button_view::{EcsButtonView, ecs_button};
pub use ecs_button_with_child_view::ecs_button_with_child as button_with_child;
pub use ecs_button_with_child_view::{EcsButtonWithChildView, ecs_button_with_child};
pub use ecs_component_views::ecs_checkbox as checkbox;
pub(crate) use ecs_component_views::ecs_radio_button;
pub use ecs_component_views::ecs_slider as slider;
pub use ecs_component_views::ecs_switch as switch;
pub use ecs_component_views::ecs_text_input as text_input;
pub use ecs_component_views::{
    EcsSliderView, EcsSwitchView, ecs_checkbox, ecs_slider, ecs_switch, ecs_text_input,
};
pub use ecs_drag_thumb_view::{EcsDragThumbView, ecs_drag_thumb};
pub use entity_scope_view::entity_scope;
pub use opaque_hitbox_view::{OpaqueHitboxView, opaque_hitbox, opaque_hitbox_for_entity};
pub use scroll_portal_view::{ScrollPortalView, scroll_portal};
