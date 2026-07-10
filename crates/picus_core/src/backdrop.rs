//! Native top-level window backdrop materials.

use bevy_ecs::prelude::*;
use bevy_window::{CompositeAlphaMode, RawHandleWrapper, Window, WindowWrapper};
use picus_surface::{
    NativeWindowBackdropError, NativeWindowBackdropMaterial,
    set_native_window_backdrop_material,
};

use crate::styling::{StyleSheet, resolve_theme_backdrop_material};

/// Native desktop backdrop material for a top-level [`Window`].
///
/// Attach this component to a Bevy window entity to request the corresponding
/// platform material. On Windows, Picus maps these values to DWM system
/// backdrops. Other platforms keep running and treat the request as unsupported.
///
/// On Windows, backdrop windows must be created as transparent so Picus' wgpu
/// surface can reveal the compositor material behind the app content. Use
/// [`Self::configure_window`] before the native window is created, or use
/// [`crate::BevyWindowOptions::with_backdrop_material`] for the primary window.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum WindowBackdropMaterial {
    /// No native backdrop material.
    #[default]
    None,
    /// Let the operating system choose the backdrop.
    Auto,
    /// Windows Mica system backdrop for long-lived app windows.
    Mica,
    /// Windows Desktop Acrylic system backdrop for transient surfaces.
    Acrylic,
    /// Windows tabbed/Mica Alt system backdrop.
    MicaAlt,
}

impl WindowBackdropMaterial {
    /// Stable lowercase name used by theme files and public tooling.
    #[must_use]
    pub const fn theme_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Auto => "auto",
            Self::Mica => "mica",
            Self::Acrylic => "acrylic",
            Self::MicaAlt => "mica-alt",
        }
    }

    /// Parse a backdrop name used by a theme file.
    pub fn from_theme_name(name: &str) -> Result<Self, String> {
        match name.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "auto" => Ok(Self::Auto),
            "mica" => Ok(Self::Mica),
            "acrylic" => Ok(Self::Acrylic),
            "mica-alt" | "mica_alt" | "micaalt" => Ok(Self::MicaAlt),
            _ => Err(format!(
                "unknown window backdrop `{name}`; expected none, auto, mica, acrylic, or mica-alt"
            )),
        }
    }

    /// Returns `true` when this material needs a transparent window surface.
    #[must_use]
    pub const fn requires_transparent_surface(self) -> bool {
        cfg!(windows) && !matches!(self, Self::None)
    }

    /// Apply the window-creation flags needed for this backdrop.
    ///
    /// Call this before Bevy/winit creates the native window. Changing these
    /// flags after creation is platform-limited, especially `transparent`.
    pub fn configure_window(self, window: &mut Window) {
        if self.requires_transparent_surface() {
            if !window.transparent {
                window.transparent = true;
            }
            if window.composite_alpha_mode != CompositeAlphaMode::PostMultiplied {
                window.composite_alpha_mode = CompositeAlphaMode::PostMultiplied;
            }
        }
    }

    fn needs_window_configuration(self, window: &Window) -> bool {
        self.requires_transparent_surface()
            && (!window.transparent
                || window.composite_alpha_mode != CompositeAlphaMode::PostMultiplied)
    }

    const fn native(self) -> NativeWindowBackdropMaterial {
        match self {
            Self::None => NativeWindowBackdropMaterial::None,
            Self::Auto => NativeWindowBackdropMaterial::Auto,
            Self::Mica => NativeWindowBackdropMaterial::Mica,
            Self::Acrylic => NativeWindowBackdropMaterial::Acrylic,
            Self::MicaAlt => NativeWindowBackdropMaterial::MicaAlt,
        }
    }
}

/// Applies native backdrop flags to a Bevy [`Window`] before creation.
pub fn configure_window_for_backdrop(window: &mut Window, material: WindowBackdropMaterial) {
    material.configure_window(window);
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedWindowBackdropMaterial(WindowBackdropMaterial);

/// Marks windows whose backdrop component is owned by the active theme.
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct ThemeManagedWindowBackdrop;

/// Synchronize the active theme backdrop to windows without an explicit
/// application-owned [`WindowBackdropMaterial`].
pub(crate) fn sync_theme_window_backdrops(
    mut commands: Commands,
    stylesheet: Res<StyleSheet>,
    mut windows: Query<(
        Entity,
        &mut Window,
        Option<&WindowBackdropMaterial>,
        Option<&ThemeManagedWindowBackdrop>,
    )>,
) {
    let Some(material) = resolve_theme_backdrop_material(&stylesheet) else {
        return;
    };

    for (entity, mut window, current, managed) in &mut windows {
        if current.is_some() && managed.is_none() {
            continue;
        }
        if current.is_some_and(|current| *current == material) && managed.is_some() {
            continue;
        }

        material.configure_window(&mut window);
        commands
            .entity(entity)
            .insert((material, ThemeManagedWindowBackdrop));
    }
}

/// Synchronize requested native backdrop materials to attached winit windows.
pub(crate) fn apply_window_backdrop_materials(
    mut commands: Commands,
    mut window_query: Query<(
        Entity,
        &WindowBackdropMaterial,
        &mut Window,
        Option<&AppliedWindowBackdropMaterial>,
    )>,
) {
    for (entity, material, mut window, applied) in &mut window_query {
        if material.needs_window_configuration(&window) {
            material.configure_window(&mut window);
        }

        if applied.is_some_and(|applied| applied.0 == *material) {
            continue;
        }

        let Some(result) = with_window_raw_handle(entity, |raw_handle| {
            set_native_window_backdrop_material(&raw_handle, material.native())
        }) else {
            continue;
        };

        match result {
            Ok(()) => {
                commands
                    .entity(entity)
                    .insert(AppliedWindowBackdropMaterial(*material));
            }
            Err(error) => {
                log_backdrop_error(entity, *material, error);
                commands
                    .entity(entity)
                    .insert(AppliedWindowBackdropMaterial(*material));
            }
        }
    }
}

fn with_window_raw_handle<T>(
    entity: Entity,
    apply: impl FnOnce(RawHandleWrapper) -> T,
) -> Option<T> {
    bevy_winit::WINIT_WINDOWS.with(|winit_windows| {
        let winit_windows = winit_windows.borrow();
        let window: &WindowWrapper<crate::xilem::winit::window::Window> =
            winit_windows.get_window(entity)?;
        match RawHandleWrapper::new(window) {
            Ok(raw_handle) => Some(apply(raw_handle)),
            Err(error) => {
                tracing::error!(
                    "failed to create raw window handle for backdrop material on window {:?}: {error}",
                    entity
                );
                None
            }
        }
    })
}

fn log_backdrop_error(
    entity: Entity,
    material: WindowBackdropMaterial,
    error: NativeWindowBackdropError,
) {
    match error {
        NativeWindowBackdropError::UnsupportedPlatform => {
            tracing::debug!(
                "window backdrop material {:?} is unsupported on this platform for window {:?}",
                material,
                entity
            );
        }
        NativeWindowBackdropError::UnsupportedWindowHandle => {
            tracing::warn!(
                "window backdrop material {:?} requires a supported native window handle for window {:?}",
                material,
                entity
            );
        }
        NativeWindowBackdropError::WindowsHresult(hr) => {
            tracing::warn!(
                "failed to apply window backdrop material {:?} to window {:?}: HRESULT {hr:#010x}",
                material,
                entity
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StyleSheet, StyleValue, ThemeBackdrop};
    use bevy_app::{App, Update};
    use std::collections::HashMap;

    #[test]
    fn backdrop_material_configures_transparent_window() {
        let mut window = Window::default();

        WindowBackdropMaterial::Mica.configure_window(&mut window);

        assert_eq!(window.transparent, cfg!(windows));
        assert_eq!(
            window.composite_alpha_mode,
            if cfg!(windows) {
                CompositeAlphaMode::PostMultiplied
            } else {
                CompositeAlphaMode::Auto
            }
        );
    }

    #[test]
    fn no_backdrop_keeps_default_window_opacity() {
        let mut window = Window::default();

        WindowBackdropMaterial::None.configure_window(&mut window);

        assert!(!window.transparent);
        assert_eq!(window.composite_alpha_mode, CompositeAlphaMode::Auto);
    }

    #[test]
    fn theme_backdrop_manages_unconfigured_windows() {
        let mut app = App::new();
        app.insert_resource(StyleSheet {
            backdrop: Some(ThemeBackdrop {
                material: StyleValue::value(WindowBackdropMaterial::Mica),
                styles: HashMap::new(),
            }),
            ..StyleSheet::default()
        })
        .add_systems(Update, sync_theme_window_backdrops);
        let window = app.world_mut().spawn(Window::default()).id();

        app.update();

        assert_eq!(
            app.world().get::<WindowBackdropMaterial>(window),
            Some(&WindowBackdropMaterial::Mica)
        );
        assert!(app.world().get::<ThemeManagedWindowBackdrop>(window).is_some());
    }

    #[test]
    fn explicit_window_backdrop_takes_precedence_over_theme() {
        let mut app = App::new();
        app.insert_resource(StyleSheet {
            backdrop: Some(ThemeBackdrop {
                material: StyleValue::value(WindowBackdropMaterial::Mica),
                styles: HashMap::new(),
            }),
            ..StyleSheet::default()
        })
        .add_systems(Update, sync_theme_window_backdrops);
        let window = app
            .world_mut()
            .spawn((Window::default(), WindowBackdropMaterial::Acrylic))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<WindowBackdropMaterial>(window),
            Some(&WindowBackdropMaterial::Acrylic)
        );
        assert!(app.world().get::<ThemeManagedWindowBackdrop>(window).is_none());
    }
}
