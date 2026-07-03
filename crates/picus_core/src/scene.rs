//! Rust-embedded BSN authoring support for Picus UI trees.
//!
//! Picus uses Bevy Scene Notation as an optional ECS tree description language.
//! The spawned entities remain ordinary Picus ECS UI components and continue
//! through the normal `UiComponentTemplate` expansion and projection pipeline.

/// Create a Bevy scene with BSN using Picus' re-exported Bevy scene runtime.
#[macro_export]
macro_rules! bsn {
    ($($tokens:tt)*) => {{
        use $crate::bevy_scene;
        $crate::bevy_scene::bsn! { $($tokens)* }
    }};
}

/// Create a Bevy scene list with BSN using Picus' re-exported Bevy scene runtime.
#[macro_export]
macro_rules! bsn_list {
    ($($tokens:tt)*) => {{
        use $crate::bevy_scene;
        $crate::bevy_scene::bsn_list! { $($tokens)* }
    }};
}

pub use bevy_scene::{
    CommandsSceneExt, EntityCommandsSceneExt, EntityWorldMutSceneExt, PatchFromTemplate,
    PatchTemplate, Scene, SceneComponent, SceneList, SpawnListSystem, SpawnSystem, WorldSceneExt,
    on, template_value,
};

pub use crate::{bsn, bsn_list};
