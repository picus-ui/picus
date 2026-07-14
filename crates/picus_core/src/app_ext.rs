use bevy_app::{App, Update};
use bevy_asset::AssetServer;
use bevy_ecs::prelude::{Component, Resource};
use fluent::{FluentResource, concurrent::FluentBundle};
use std::{fs, io, path::Path};
use unic_langid::LanguageIdentifier;

use crate::xilem::winit::error::EventLoopError;
use crate::{
    ActiveStyleSheetAsset, AppI18n, BevyWindowOptions, MasonryRuntime, ProjectionCtx, StyleSheet,
    StyleTypeRegistry, UiProjector, UiProjectorRegistry, UiView, WindowBackdropMaterial,
    XilemFontBridge, apply_active_stylesheet_ron, clear_theme_backdrop_material_override,
    components::{
        RegisteredUiComponentTypes, UiComponentTemplate, expand_added_ui_component_templates,
    },
    events::{InternalUiEventQueue, register_ui_action_type},
    run_app_with_window_options, set_active_style_variant_by_name,
    set_active_stylesheet_asset_path, set_theme_backdrop_material,
};

/// Synchronous source for binary assets (fonts).
pub enum SyncAssetSource<'a> {
    Bytes(&'a [u8]),
    FilePath(&'a str),
}

/// Synchronous source for textual assets (FTL bundles).
pub enum SyncTextSource<'a> {
    String(&'a str),
    FilePath(&'a str),
}

fn flush_pending_font_registrations(app: &mut App) {
    {
        let world = app.world_mut();
        world.init_resource::<InternalUiEventQueue>();
        world.init_non_send::<MasonryRuntime>();
    }

    if app.world().non_send::<MasonryRuntime>().windows.is_empty() {
        return;
    }

    let pending = app
        .world_mut()
        .resource_mut::<XilemFontBridge>()
        .take_pending_fonts();

    if pending.is_empty() {
        return;
    }

    let mut runtime = app.world_mut().non_send_mut::<MasonryRuntime>();
    for font_bytes in pending {
        runtime.register_fonts_all(font_bytes);
    }
}

/// Low-level registration APIs for framework integrations.
///
/// Application components should use `#[derive(UiComponent)]` plus
/// `register_ui_components!`; this trait is exposed only from
/// `picus::runtime::advanced` by the public facade.
pub trait AdvancedAppPicusExt {
    /// Register a typed projector for a specific component.
    fn register_projector<C: Component>(
        &mut self,
        projector: fn(&C, ProjectionCtx<'_>) -> UiView,
    ) -> &mut Self;

    /// Register an ECS-native UI component template.
    fn register_ui_component<T: UiComponentTemplate>(&mut self) -> &mut Self;

    /// Register a Bevy resource as an input to UI projection.
    fn register_projection_resource<R: Resource>(&mut self) -> &mut Self;

    /// Register a raw projector implementation.
    fn register_raw_projector<P: UiProjector>(&mut self, projector: P) -> &mut Self;

    /// Register a selector type alias usable by `Selector::Type("...")`.
    fn register_style_selector_type<T: Component>(
        &mut self,
        selector_name: impl Into<String>,
    ) -> &mut Self;
}

/// Fluent application setup methods for a Bevy [`App`].
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
///
/// use picus_core::{
///     AdvancedAppPicusExt, AppPicusExt, PicusPlugin, ProjectionCtx, UiComponentTemplate,
///     UiRoot, UiView,
///     bevy_app::{App, Startup},
///     bevy_ecs::prelude::*,
/// };
///
/// #[derive(Component, Clone, Copy)]
/// struct Root;
///
/// #[derive(Debug, Clone, Copy)]
/// enum Action {
///     Clicked,
/// }
///
/// impl UiComponentTemplate for Root {
///     fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
///         Arc::new(ctx.button(Action::Clicked, "Click"))
///     }
/// }
///
/// fn setup(mut commands: Commands) {
///     commands.spawn((UiRoot, Root));
/// }
///
/// let mut app = App::new();
/// app.add_plugins(PicusPlugin)
///     .add_ui_action::<Action>()
///     .register_ui_component::<Root>()
///     .add_systems(Startup, setup);
/// ```
pub trait AppPicusExt {
    /// Register payload type `T` for application UI actions.
    ///
    /// Installs `Messages<UiAction<T>>`, a [`crate::UiActionSender<T>`] resource,
    /// and a dispatcher handler that writes messages from the internal queue.
    fn add_ui_action<T: Clone + Send + Sync + 'static>(&mut self) -> &mut Self;

    /// Load a RON stylesheet asset and bind it as the active runtime style source.
    ///
    /// The file is hot-reloaded through Bevy's asset pipeline. If it declares
    /// `default_variant` and no active variant is already selected, that
    /// registered variant is applied before the active stylesheet is overlaid.
    ///
    /// Picus does **not** auto-select a theme. Without a loaded sheet and/or
    /// selected variant, controls render with no framework-provided visible
    /// fill or text colour.
    fn load_style_sheet(&mut self, asset_path: impl Into<String>) -> &mut Self;

    /// Parse and load an active stylesheet directly from embedded RON text.
    ///
    /// This bypasses filesystem asset loading and applies the stylesheet as the
    /// active tier with the same precedence as file-based active stylesheets. If
    /// it declares `default_variant` and no active variant is already selected,
    /// that registered variant is applied before the active stylesheet is
    /// overlaid.
    fn load_style_sheet_ron(&mut self, ron_text: &str) -> &mut Self;

    /// Select an active style variant by registered name (for example `"dark"`).
    ///
    /// Does nothing permanent if the name is unknown until a matching registered
    /// variant is available; missing styles remain transparent rather than
    /// erroring.
    fn style_variant(&mut self, name: impl Into<String>) -> &mut Self;

    /// Override the theme window backdrop material for this app.
    fn theme_backdrop(&mut self, material: WindowBackdropMaterial) -> &mut Self;

    /// Clear an application theme backdrop override.
    fn clear_theme_backdrop_override(&mut self) -> &mut Self;

    /// Register a font synchronously from bytes or filesystem path.
    ///
    /// Font registration is fail-fast and writes into the active Masonry Core runtime font
    /// database used for text shaping.
    fn register_xilem_font(&mut self, source: SyncAssetSource<'_>) -> &mut Self;

    /// Register a Fluent bundle synchronously from in-memory text or filesystem path.
    ///
    /// Initializes [`AppI18n`] automatically when missing.
    fn register_i18n_bundle(
        &mut self,
        locale: &str,
        source: SyncTextSource<'_>,
        font_stack: Vec<&str>,
    ) -> &mut Self;

    /// Queue raw font bytes for registration in retained text shaping.
    ///
    /// This bridges app-provided fonts into the retained Masonry Core font database.
    fn register_xilem_font_bytes(&mut self, bytes: &[u8]) -> &mut Self;

    /// Read and queue a font file for registration in retained text shaping.
    ///
    /// Typical path for Bevy projects: `assets/fonts/<font-file>.ttf|otf`.
    fn register_xilem_font_path(&mut self, path: impl AsRef<Path>) -> io::Result<&mut Self>;

    /// Run this app with Picus window bootstrap.
    ///
    /// This is the only recommended desktop runner. It installs Bevy window /
    /// input / winit plugins as needed and applies [`BevyWindowOptions`].
    fn run_picus(
        self,
        title: impl Into<String>,
        options: BevyWindowOptions,
    ) -> Result<(), EventLoopError>;
}

impl AdvancedAppPicusExt for App {
    fn register_projector<C: Component>(
        &mut self,
        projector: fn(&C, ProjectionCtx<'_>) -> UiView,
    ) -> &mut Self {
        self.init_resource::<UiProjectorRegistry>();
        self.world_mut()
            .resource_mut::<UiProjectorRegistry>()
            .register_component::<C>(projector);
        self
    }

    fn register_ui_component<T: UiComponentTemplate>(&mut self) -> &mut Self {
        self.init_resource::<RegisteredUiComponentTypes>();
        if !self
            .world_mut()
            .resource_mut::<RegisteredUiComponentTypes>()
            .insert::<T>()
        {
            return self;
        }

        self.init_resource::<UiProjectorRegistry>();
        self.world_mut()
            .resource_mut::<UiProjectorRegistry>()
            .register_component::<T>(T::project);
        T::register_projection_dependencies(
            &mut self.world_mut().resource_mut::<UiProjectorRegistry>(),
        );

        self.init_resource::<StyleTypeRegistry>();
        T::register_style_types(&mut self.world_mut().resource_mut::<StyleTypeRegistry>());

        self.add_systems(Update, expand_added_ui_component_templates::<T>);

        self
    }

    fn register_projection_resource<R: Resource>(&mut self) -> &mut Self {
        self.init_resource::<UiProjectorRegistry>();
        self.world_mut()
            .resource_mut::<UiProjectorRegistry>()
            .register_resource_dependency::<R>();
        self
    }

    fn register_raw_projector<P: UiProjector>(&mut self, projector: P) -> &mut Self {
        self.init_resource::<UiProjectorRegistry>();
        self.world_mut()
            .resource_mut::<UiProjectorRegistry>()
            .register_projector(projector);
        self
    }

    fn register_style_selector_type<T: Component>(
        &mut self,
        selector_name: impl Into<String>,
    ) -> &mut Self {
        self.init_resource::<StyleTypeRegistry>();
        let mut registry = self.world_mut().resource_mut::<StyleTypeRegistry>();
        registry.register_type_aliases::<T>();
        registry.register_type_name::<T>(selector_name);
        self
    }
}

impl AppPicusExt for App {
    fn add_ui_action<T: Clone + Send + Sync + 'static>(&mut self) -> &mut Self {
        register_ui_action_type::<T>(self);
        self
    }

    fn load_style_sheet(&mut self, asset_path: impl Into<String>) -> &mut Self {
        let asset_path = asset_path.into();
        set_active_stylesheet_asset_path(self.world_mut(), asset_path);

        if let Some(path) = self
            .world()
            .get_resource::<ActiveStyleSheetAsset>()
            .and_then(|active| active.path.clone())
            && let Some(asset_server) = self.world().get_resource::<AssetServer>()
        {
            let handle = asset_server.load::<StyleSheet>(path);
            self.world_mut()
                .resource_mut::<ActiveStyleSheetAsset>()
                .handle = Some(handle);
        }

        self
    }

    fn load_style_sheet_ron(&mut self, ron_text: &str) -> &mut Self {
        apply_active_stylesheet_ron(self.world_mut(), ron_text)
            .unwrap_or_else(|error| panic!("failed to parse embedded stylesheet RON: {error}"));
        self
    }

    fn style_variant(&mut self, name: impl Into<String>) -> &mut Self {
        set_active_style_variant_by_name(self.world_mut(), &name.into());
        self
    }

    fn theme_backdrop(&mut self, material: WindowBackdropMaterial) -> &mut Self {
        set_theme_backdrop_material(self.world_mut(), material);
        self
    }

    fn clear_theme_backdrop_override(&mut self) -> &mut Self {
        clear_theme_backdrop_material_override(self.world_mut());
        self
    }

    fn register_xilem_font(&mut self, source: SyncAssetSource<'_>) -> &mut Self {
        let bytes = match source {
            SyncAssetSource::Bytes(data) => data.to_vec(),
            SyncAssetSource::FilePath(path) => fs::read(path)
                .unwrap_or_else(|error| panic!("failed to read font file `{path}`: {error}")),
        };

        self.init_resource::<XilemFontBridge>();
        let queued = self
            .world_mut()
            .resource_mut::<XilemFontBridge>()
            .register_font_bytes(&bytes);

        if queued {
            flush_pending_font_registrations(self);
        }

        self
    }

    fn register_i18n_bundle(
        &mut self,
        locale: &str,
        source: SyncTextSource<'_>,
        font_stack: Vec<&str>,
    ) -> &mut Self {
        let locale_id: LanguageIdentifier = locale
            .parse()
            .unwrap_or_else(|_| panic!("locale `{locale}` should parse"));
        let font_stack = font_stack.into_iter().map(String::from).collect::<Vec<_>>();

        let ftl_text = match source {
            SyncTextSource::String(text) => text.to_string(),
            SyncTextSource::FilePath(path) => fs::read_to_string(path).unwrap_or_else(|error| {
                panic!("failed to read localization file `{path}`: {error}")
            }),
        };

        let resource = FluentResource::try_new(ftl_text).unwrap_or_else(|(_, errors)| {
            panic!("failed to parse Fluent resource for locale `{locale}`: {errors:?}")
        });

        let mut bundle = FluentBundle::new_concurrent(vec![locale_id.clone()]);
        if let Err(errors) = bundle.add_resource(resource) {
            panic!("failed to add Fluent resource for locale `{locale}`: {errors:?}");
        }

        if self.world().get_resource::<AppI18n>().is_none() {
            self.insert_resource(AppI18n::new(locale_id.clone()));
        }

        let mut i18n = self.world_mut().resource_mut::<AppI18n>();
        if i18n.bundles.is_empty() {
            i18n.set_active_locale(locale_id.clone());
        }
        i18n.insert_bundle(locale_id, bundle, font_stack);

        self
    }

    fn register_xilem_font_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.register_xilem_font(SyncAssetSource::Bytes(bytes))
    }

    fn register_xilem_font_path(&mut self, path: impl AsRef<Path>) -> io::Result<&mut Self> {
        let path = path.as_ref();
        let path = path.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("font path `{}` is not valid UTF-8", path.display()),
            )
        })?;

        self.register_xilem_font(SyncAssetSource::FilePath(path));
        Ok(self)
    }

    fn run_picus(
        self,
        title: impl Into<String>,
        options: BevyWindowOptions,
    ) -> Result<(), EventLoopError> {
        run_app_with_window_options(self, title, move |_| options.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PicusPlugin;
    use bevy_app::App;

    #[test]
    fn sync_asset_source_bytes() {
        let data = b"font data";
        let source = SyncAssetSource::Bytes(data);
        match source {
            SyncAssetSource::Bytes(d) => assert_eq!(d, data),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn sync_text_source_string() {
        let text = "hello";
        let source = SyncTextSource::String(text);
        match source {
            SyncTextSource::String(s) => assert_eq!(s, text),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn register_projector_adds_to_registry() {
        use crate::{ProjectionCtx, UiRoot};
        use std::sync::Arc;

        let mut app = App::new();
        app.add_plugins(PicusPlugin).register_projector::<UiRoot>(
            |_: &UiRoot, _: ProjectionCtx<'_>| {
                Arc::new(crate::retained_bridge::button_view(
                    bevy_ecs::entity::Entity::PLACEHOLDER,
                    (),
                    "Placeholder",
                ))
            },
        );
    }
}
