//! Public facade for building Picus applications.
//!
//! Depend on this crate only. Implementation details live in `picus_core` and
//! are not part of the stable application surface.
//!
//! # Quick path
//!
//! 1. Create a Bevy `App`, add [`app::PicusPlugin`].
//! 2. **Explicitly** load a stylesheet ([`app::AppPicusExt::load_style_sheet_ron`]) and/or
//!    select a variant ([`app::AppPicusExt::style_variant`]). Picus never auto-picks dark/light.
//! 3. Register business actions with [`app::AppPicusExt::add_ui_action`].
//! 4. Implement [`components::UiComponentTemplate`] for custom regions; derive [`UiComponent`] and
//!    register them once with [`register_ui_components!`].
//! 5. Handle interactions with `MessageReader<UiAction<T>>` (not an internal queue).
//! 6. Run with [`app::AppPicusExt::run_picus`].
//!
//! # Counter Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use picus::prelude::*;
//! use picus::{
//!     app::{bevy_app::{App, Startup, Update}, bevy_ecs::{message::MessageReader, prelude::*}},
//!     projection::xilem::{view::label, winit::{dpi::LogicalSize, error::EventLoopError}},
//!     scene::{CommandsSceneExt, bsn, template_value},
//! };
//!
//! #[derive(Clone, Debug)]
//! enum CounterAction {
//!     Increment,
//! }
//!
//! #[derive(Resource, Default)]
//! struct Counter(i32);
//!
//! #[derive(Component, Clone, Default, UiComponent)]
//! #[ui_component(resources(Counter))]
//! struct CounterRoot;
//!
//! impl UiComponentTemplate for CounterRoot {
//!     fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
//!         let n = ctx.world.resource::<Counter>().0;
//!         Arc::new(label(format!("Count: {n}")))
//!     }
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn_scene(bsn! {
//!         UiRoot
//!         Children [
//!             CounterRoot,
//!             (UiButton { label: { "+".into() } } template_value(UiEmit::new(CounterAction::Increment))),
//!         ]
//!     });
//! }
//!
//! fn on_counter(
//!     mut reader: MessageReader<UiAction<CounterAction>>,
//!     mut counter: ResMut<Counter>,
//! ) {
//!     for UiAction { action, .. } in reader.read() {
//!         if matches!(action, CounterAction::Increment) {
//!             counter.0 += 1;
//!         }
//!     }
//! }
//!
//! fn main() -> Result<(), EventLoopError> {
//!     let mut app = App::new();
//!     app.add_plugins(PicusPlugin)
//!         .load_style_sheet_ron(r#"(default_variant: "dark")"#)
//!         .insert_resource(Counter::default())
//!         .add_ui_action::<CounterAction>()
//!         .add_systems(Startup, setup)
//!         .add_systems(Update, on_counter);
//!     register_ui_components!(&mut app, CounterRoot);
//!     app.run_picus(
//!         "Counter",
//!         BevyWindowOptions::default().with_initial_inner_size(LogicalSize::new(360.0, 220.0)),
//!     )
//! }
//! ```
//!
//! # Architecture & Frame Stages
//!
//! Picus integrates Bevy's ECS scheduler with a retained Masonry Core widget runtime:
//!
//! | Stage | Work |
//! |-------|------|
//! | `PreUpdate` | Input injection, retained message routing, **action dispatch** (`PicusUiSet`) |
//! | `Update` | Application systems, state changes, overlay lifecycle, style/theme transitions |
//! | `PostUpdate` | Projection invalidation, UI synthesis, retained rebuild, IME sync |
//! | `Last` | Vello paint and presentation for each attached window |
//!
//! # Authoring Guidelines
//!
//! - **When to split a component**: Prefer a single container component that maps children
//!   or builds a small view tree when the piece is not reused, has no independent style type,
//!   and does not need its own projection resources. Split into a [`UiComponent`] when the subtree
//!   is reused, has distinct styles/classes, or registers its own resource dependencies.
//! - **Fine-grained vs Container Map**: Prefer **container map** for short-lived or purely derived
//!   lists (less registration noise). Prefer **fine-grained entities** when items need hit testing identity,
//!   per-row [`UiEmit`], focus, or stylesheet type/class selectors.
//! - **Exclusive systems**: Prefer ordinary `MessageReader` systems. When a mutation must run in an
//!   exclusive system, collect messages into an app-owned pending resource in a normal system, then pass
//!   that resource to the exclusive system. The internal action queue is never exposed to applications.
#![forbid(unsafe_code)]

/// Application setup, plugins, runners, and Bevy re-exports.
///
/// This module provides the central [`PicusPlugin`], the [`AppPicusExt`] extension trait for
/// Bevy [`bevy_app::App`], desktop window configuration through [`BevyWindowOptions`],
/// and window backdrop settings ([`WindowBackdropMaterial`], [`WindowBackdropColorScheme`]).
///
/// # Example
///
/// ```no_run
/// use picus::app::{bevy_app::App, PicusPlugin, AppPicusExt};
///
/// let mut app = App::new();
/// app.add_plugins(PicusPlugin)
///     .load_style_sheet_ron(r#"(default_variant: "dark")"#);
/// ```
pub mod app {
    pub use picus_core::{
        bevy_app, bevy_asset, bevy_ecs, bevy_input, bevy_math, bevy_scene, bevy_tasks, bevy_text,
        bevy_tween, bevy_window, configure_window_for_backdrop, rfd, AppPicusExt,
        BevyWindowOptions, PicusBuiltinsPlugin, PicusPlugin, PicusUiSet, SyncAssetSource,
        SyncTextSource, WindowBackdropColorScheme, WindowBackdropMaterial, WindowSize,
    };
}


/// ECS authoring components, helper views, and component registration contracts.
///
/// # Authoring Contracts
///
/// - Public UI authoring components and nested values implement `Default + Clone` so they
///   are template-ready for Bevy Scene Notation (`bsn!` / `bsn_list!`).
/// - Custom components derive [`UiComponent`] and implement [`UiComponentTemplate`].
/// - Event-hook components such as [`UiEmit`] are runtime-only; attach them with `template_value(...)`
///   in BSN or spawn them from systems.
///
/// # Composite Layout Components
///
/// - [`UiFormRow`]: Label column + child control(s) in a horizontal row.
/// - [`UiContentShell`]: Optional title + vertical content stack.
///
/// # Continuous Animation & Paint Isolation
///
/// Controls with continuous visual animation ([`UiSpinner`], indeterminate [`UiProgressBar`])
/// project to retained widgets with `PaintIsolation::AnimEntry`, reserving an External painter slot
/// so high-frequency ticks skip full-window base scene rebuilds.
pub mod components {
    pub use picus_core::avatar_sizes;
    pub use picus_core::icon::{
        fluent_icon, icon, icon_glyph, icon_glyph_with_font_stack, icon_source, picus_icon,
    };
    pub use picus_core::{
        checkbox, slider, switch, text_input, AppBreakpoints, AutoDismiss, AvatarShape,
        BuiltinUiAction, ButtonAppearance, ButtonIconPosition, ButtonShape, ButtonSize,
        ColorPickerChannel, FluentIcon, HasTooltip, IconGlyph, LocalizeText, MessageBarKind,
        NavigationBackButtonVisible, NavigationDisplayMode, NavigationItemRegion,
        NavigationPaneDisplayMode, NavigationViewItem, NavigationViewItemKind, PicusIcon,
        RatingColor, RatingSize, ScrollAxis, SplitDirection, TitleBarAction, TitleBarIcon,
        TitleBarState, ToastKind, TypographyPreset, UiAnyView, UiAvatar, UiBadge, UiBreadcrumb,
        UiBreadcrumbItem, UiButton, UiCanvas, UiCanvasCommand, UiCanvasPathCommand,
        UiCanvasPosition, UiCard, UiCheckbox, UiCheckboxChanged, UiColorPicker,
        UiColorPickerChanged, UiColorPickerPanel, UiComboBox, UiComboBoxChanged, UiComboOption,
        UiComponentTemplate, UiContentShell, UiContextMenu, UiContextMenuItem,
        UiContextMenuItemSelected, UiContextMenuTrigger, UiDataCell, UiDataColumn, UiDataRow,
        UiDataTable, UiDataTableSelectionChanged, UiDataTableSort, UiDataTableSortChanged,
        UiDatePicker, UiDatePickerChanged, UiDatePickerPanel, UiDialog, UiDivider, UiDropdownItem,
        UiDropdownMenu, UiDropdownPlacement, UiEmit, UiExpander, UiExpanderChanged, UiFlexColumn,
        UiFlexRow, UiFormRow, UiGradientStop, UiGrid, UiGridAutoFlow, UiGridCell, UiGridLength,
        UiGridLengthParseError, UiGroupBox, UiImage, UiImageAlignmentX, UiImageAlignmentY,
        UiImageViewBox, UiImageViewBoxUnits, UiInteractionEvent, UiLabel, UiLink, UiLinkAction,
        UiListSelectionMode, UiListView, UiListViewSelectionChanged, UiMarkdown, UiMenuBar,
        UiMenuBarItem, UiMenuItem, UiMenuItemPanel, UiMenuItemSelected, UiMessageBar,
        UiMultilineTextInput, UiMultilineTextInputChanged, UiNavigationBackRequested,
        UiNavigationDisplayModeChanged, UiNavigationItem, UiNavigationItemExpandedChanged,
        UiNavigationItemInvoked, UiNavigationPaneChanged, UiNavigationSelectionChanged,
        UiNavigationSettingsItem, UiNavigationView, UiNumericUpDown, UiNumericUpDownChanged,
        UiOverlayRoot, UiPasswordInput, UiPasswordInputChanged, UiPointerEvent, UiPointerHitEvent,
        UiPointerPhase, UiPopover, UiProgressBar, UiRadioGroup, UiRadioGroupChanged, UiRating,
        UiRatingChanged, UiResponsiveGrid, UiResponsiveRow, UiRoot, UiScrollView,
        UiScrollViewChanged, UiSearch, UiSearchChanged, UiSlider, UiSliderChanged, UiSortDirection,
        UiSpinner, UiSplitPane, UiStreamingMarkdown, UiSwitch, UiSwitchChanged, UiTabBar,
        UiTabChanged, UiTable, UiText, UiTextInput, UiTextInputChanged, UiThemePicker,
        UiThemePickerChanged, UiThemePickerMenu, UiThemePickerOption, UiTimePicker,
        UiTimePickerChanged, UiTimePickerPanel, UiTitleBar, UiToast, UiToolbar, UiTooltip,
        UiTreeNode, UiTreeNodeToggled, UiView, UiVisibleResponsive, UiWindow,
        WindowBackdropColorScheme, WindowBackdropMaterial, NAV_COMPACT_MODE_THRESHOLD,
        NAV_EXPANDED_MODE_THRESHOLD, NAV_PANE_COMPACT_WIDTH, NAV_PANE_EXPANDED_WIDTH,
    };
}

/// Low-level projection helpers for custom [`UiComponentTemplate`] implementations.
///
/// Projection maps ECS authoring components to retained views. A `UiProjectorRegistry`
/// stores the projector for each registered component, and root entities anchor each projected tree.
///
/// # Invalidation and Change Detection
///
/// - Projection invalidation tracks components and resources registered as dependencies.
/// - Declare resource dependencies with `#[ui_component(resources(MyResource))]` or
///   via `UiComponentTemplate::register_projection_dependencies`.
/// - Avoid no-op mutable writes to projection-visible state: Bevy change detection drives
///   invalidation, so unchanged writes cause needless rebuilds.
/// - [`CurrentColorStyle`](crate::styling::CurrentColorStyle) is **not** a projection dependency;
///   smooth color transitions patch retained properties in place.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use picus::projection::{ProjectionCtx, UiView, xilem::view::label};
/// use picus::components::UiComponentTemplate;
/// use picus::app::bevy_ecs::prelude::*;
///
/// #[derive(Component, Clone, Default)]
/// struct Greeting;
///
/// impl UiComponentTemplate for Greeting {
///     fn project(_: &Self, _ctx: ProjectionCtx<'_>) -> UiView {
///         Arc::new(label("Hello, Picus!"))
///     }
/// }
/// ```
pub mod projection {
    pub use picus_core::{
        checkbox, slider, switch, text_input, ButtonView, ButtonWithChildView, CheckboxView,
        ProjectionCtx, SliderView, SwitchView, UiView,
    };
    pub use picus_core::{picus_view, xilem};
}

/// Styling, themes, and selector APIs.
///
/// Picus features a complete styling pipeline inspired by CSS, using RON stylesheets:
///
/// # Theme Contract
///
/// 1. **No theme / no variant** → controls show **no** framework default visible fill or text
///    colour (transparent / empty). This is not an error.
/// 2. The framework **never** auto-selects dark or light.
/// 3. **Partial themes are legal**: missing component or property rules stay empty.
/// 4. Errors are for structural issues only (malformed RON, invalid token).
///
/// # Style Layers
///
/// - **Layer 0**: No theme = no visible defaults.
/// - **Layer 1**: Loaded stylesheet / variant rules.
/// - **Layer 2**: Inline / builder styles ([`InlineStyle`], [`styled`]).
/// - **Layer 3**: Class + app RON override.
/// - **Layer 4**: Full multi-brand stylesheet.
///
/// # Example
///
/// ```
/// use picus::styling::{InlineStyle, StyleClass};
/// use picus::classes;
///
/// // Create style classes for an entity:
/// let class: StyleClass = classes!("card", "card.elevated");
///
/// // Create an inline style override:
/// let inline = InlineStyle::new().padding(8.0).text_size(14.0);
/// ```
pub mod styling {
    pub use picus_core::{
        apply_active_stylesheet_ron, apply_direct_text_input_style, apply_direct_widget_style,
        apply_label_style, apply_text_input_style, apply_widget_style,
        clear_theme_backdrop_material_override, mark_style_dirty, parse_stylesheet_ron,
        register_builtin_style_type_aliases, resolve_style, resolve_style_for_classes,
        resolve_style_for_classes_with_state, resolve_style_for_entity_classes,
        resolve_theme_backdrop_material, set_active_style_variant_by_name,
        set_theme_backdrop_material, styled, ActiveStyleVariant, BackdropStyle, BaseStyleSheet,
        ColorStyle, ComputedStyle, CurrentColorStyle, InlineStyle, InteractionState, LayoutStyle,
        PseudoClass, Selector, StyleClass, StyleDirty, StylePseudoState, StyleRule, StyleSetter,
        StyleSheet, StyleTransition, SyncAssetSource, SyncTextSource, TargetColorStyle, TextStyle,
        ThemeBackdrop, ThemeBackdropOverride, TokenValue, WINDOW_BACKDROP_TOKEN,
    };
}

/// Application-facing UI actions and Bevy message integration.
///
/// # Architecture
///
/// ```text
/// Retained widgets / projection callbacks
///         │  push type-erased payload
///         ▼
/// InternalUiEventQueue  (internal, app-owned)
///         │  sole consumer: dispatch_ui_actions
///         ▼
/// UiActionRegistry (TypeId → handlers)
///         │
///         ├─ built-in handlers (widget/overlay mutations)
///         └─ application handlers → Messages<UiAction<T>>
///                                       │
///                                       ▼
///                          MessageReader<UiAction<T>>
/// ```
///
/// # Scheduling
///
/// - Input-driven actions become [`UiAction`] messages **before** ordinary `Update` systems in
///   the same frame. The fixed `PreUpdate` order is `Input → RetainedRouting → DispatchActions`.
/// - Emissions from `Update` via [`UiActionSender`] are **next-frame** visible.
/// - Applications consume completed actions using `MessageReader<UiAction<T>>`.
///
/// # Example
///
/// ```
/// use picus::app::bevy_app::App;
/// use picus::app::bevy_ecs::prelude::*;
/// use picus::events::{UiAction, UiActionSender};
/// use picus::app::AppPicusExt;
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// enum AppAction {
///     Submit,
/// }
///
/// let mut app = App::new();
/// app.add_ui_action::<AppAction>();
///
/// fn read_actions(mut reader: MessageReader<UiAction<AppAction>>) {
///     for action in reader.read() {
///         assert_eq!(action.action, AppAction::Submit);
///     }
/// }
/// ```
pub mod events {
    pub use picus_core::{
        format_accelerator_text, AcceleratorActivated, AcceleratorModifiers, AcceleratorScope,
        AcceleratorTextOverride, AccessibleAction, CurrentAcceleratorModifiers,
        KeyboardAccelerator, UiAction, UiActionSender, UiEmit,
    };
}

/// Overlay helpers and overlay lifecycle systems.
///
/// Provides dialogs, popovers, tooltips, dropdowns, and toast notifications.
///
/// # Contracts
///
/// - **Positioning**: Overlay projectors stay **transparent until positioned**.
/// - **Outside-click dismissal**: Checks the top overlay hit path and its bound widget IDs.
/// - **Scroll routing**: Nested wheel routing starts at the deepest hit target.
/// - Built-in overlay interactions use internal payloads dispatched during `PreUpdate`.
pub mod overlay {
    pub use picus_core::{
        dismiss_overlays_on_click, ensure_overlay_root, ensure_overlay_root_entity,
        handle_global_overlay_clicks, handle_tooltip_hovers, spawn_in_overlay_root,
        spawn_manual_overlay_at, spawn_popover_in_overlay_root, sync_dropdown_positions,
        sync_overlay_positions, sync_overlay_stack_lifecycle, tick_auto_dismiss, tick_toasts,
        OverlayComputedPosition, OverlayConfig, OverlayMouseButtonCursor, OverlayPlacement,
        OverlayPointerRoutingState, OverlayStack, OverlayState,
    };
}

/// Runtime synthesis and rendering integration.
///
/// # Per-Window Runtime & Timelines
///
/// Picus manages one retained runtime per window via [`MasonryRuntime`] and [`WindowRuntime`].
/// Frame execution separates four timelines:
///
/// - **Timeline A (Input/Shell)**: Pointer, keyboard, move/resize message pump.
/// - **Timeline B (Anim clock)**: Advance `t`, opacity, cursor blink timers.
/// - **Timeline C (Scene build)**: Rewrite + per-entry encode (pure anim can skip base scene).
/// - **Timeline D (Present)**: Submit latest ready composite to the swapchain.
///
/// # Input & Multi-Window
///
/// - Pointer coordinates are read from the event window's physical cursor position and converted
///   to logical coordinates by the matching [`WindowRuntime`].
/// - Click injection sends move before down/up to ensure hover state is current.
/// - The primary window auto-attaches; additional windows attach when their `Window` entity is spawned.
/// - Action sinks are app-owned: all windows of one `App` share one internal queue.
///
/// # Observability
///
/// Set `PICUS_FRAME_TIMING=1` to log per-window frame phase durations.
/// Unset `PICUS_ANIM_PRESENT_HZ` for the default unthrottled anim path, or set a positive Hz
/// as an explicit diagnostic cap.
pub mod runtime {
    pub use picus_core::masonry_core;
    pub use picus_core::{
        collect_bevy_font_assets, inject_bevy_input_into_masonry, rebuild_masonry_runtime,
        synthesize_ui, track_window_size, MasonryRuntime, ProjectionCtx, SynthesizedUiViews,
        UiDirtyReason, UiProjectionDirtyDebug, UiProjectionInvalidation, UiView, WindowRuntime,
        XilemFontBridge,
    };

    /// Low-level registration and projector APIs for advanced / framework use.
    pub mod advanced {
        pub use picus_core::{
            expand_builtin_ui_component_templates, find_template_part, gather_ui_roots,
            register_builtin_projectors, register_builtin_ui_components,
            route_masonry_view_messages, sync_fonts_to_xilem, synthesize_roots,
            synthesize_roots_with_stats, synthesize_world, AdvancedAppPicusExt, UiProjector,
            UiProjectorRegistry,
        };
    }
}

/// Internationalization and font helpers.
///
/// # i18n
///
/// - Register Fluent bundles with [`crate::app::AppPicusExt::register_i18n_bundle`].
/// - Resolve display strings through [`resolve_localized_text`] and [`crate::components::LocalizeText`].
/// - Missing localization keys fall back to authoring strings without failing the frame.
///
/// # Fonts
///
/// - Register fonts using [`crate::app::AppPicusExt::register_xilem_font`].
/// - Font registrations broadcast to all attached windows and replay for newly attached windows.
pub mod i18n {
    pub use picus_core::{resolve_localized_text, AppI18n};
}

/// Icon definitions and bundled icon font data.
pub mod icons {
    pub use picus_core::icons::*;
}

/// Validation helpers.
pub mod validation {
    pub use picus_core::validation::*;
}

/// System clipboard resource and ECS clipboard event helpers.
///
/// Prefer the [`Clipboard`] resource for simple get/set text access. Attach
/// [`ClipboardEvent`] for copy/cut/paste flows processed by
/// [`handle_clipboard_events`] (registered by [`crate::app::PicusPlugin`]).
pub mod clipboard {
    pub use picus_core::{
        handle_clipboard_events, Clipboard, ClipboardEvent, ClipboardKind, ClipboardText,
    };
}

/// BSN scene authoring helpers.
///
/// Supports Bevy Scene Notation as a Rust-embedded UI description language.
///
/// # Authoring Contract
///
/// - Public UI authoring components and nested values are `Default + Clone`.
/// - Use `bsn!` and `bsn_list!` to construct static trees without manual `ChildOf` wiring.
/// - Use `template_value(...)` for runtime-only values like [`crate::components::UiEmit`].
///
/// # Example
///
/// ```
/// use picus::app::bevy_ecs::prelude::*;
/// use picus::prelude::*;
///
/// fn setup(mut commands: Commands) {
///     commands.spawn_scene(bsn! {
///         UiRoot
///         UiFlexColumn
///         Children [
///             UiLabel {
///                 text: { "Hello".to_string() },
///             },
///         ]
///     });
/// }
/// ```
pub mod scene {
    pub use picus_core::scene::*;
}

/// Common imports for Picus applications.
pub mod prelude {
    pub use crate::{
        app::*, clipboard::*, components::*, events::*, i18n::*, icons::*, overlay::*,
        projection::*, runtime::*, scene::*, styling::*,
    };
    pub use crate::{classes, register_ui_components, ui_view, UiComponent};
    pub use picus_core::bevy_ecs::hierarchy::{ChildOf, Children};
    pub use picus_core::bevy_ecs::message::MessageReader;
}

// ---------------------------------------------------------------------------
// Root-level macros and hidden macro support (no `picus_core::*` dump).
// ---------------------------------------------------------------------------

pub use picus_macros::{ui_view, UiComponent};

/// Construct a [`StyleClass`](crate::styling::StyleClass) from string literals or expressions.
///
/// # Example
///
/// ```
/// use picus::classes;
/// use picus::styling::StyleClass;
///
/// let class: StyleClass = classes!("btn", "btn.primary");
/// assert_eq!(class.0, vec!["btn".to_string(), "btn.primary".to_string()]);
/// ```
#[macro_export]
macro_rules! classes {
    ($($class:expr),* $(,)?) => {
        $crate::styling::StyleClass(
            ::std::vec![
                $(::std::string::ToString::to_string(&$class)),*
            ],
        )
    };
}

/// Register one or more `#[derive(UiComponent)]` types on a mutable Bevy `App`.
///
/// This is the primary component registration entry point. It registers projection
/// templates, resource dependencies, and style aliases in one call.
///
/// # Example
///
/// ```rust,ignore
/// use picus::prelude::*;
/// use picus::app::bevy_app::App;
///
/// #[derive(Component, Clone, Default, UiComponent)]
/// struct MyView;
///
/// impl UiComponentTemplate for MyView {
///     fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
///         std::sync::Arc::new(picus::projection::xilem::view::label("Hello"))
///     }
/// }
///
/// let mut app = App::new();
/// register_ui_components!(&mut app, MyView);
/// ```
#[macro_export]
macro_rules! register_ui_components {
    ($app:expr $(, $ty:ty)* $(,)?) => {{
        $(
            <$ty as $crate::__macro_support::UiComponentRegistration>::register($app);
        )*
    }};
}

/// Hidden support surface used only by macro expansions.
#[doc(hidden)]
pub mod __macro_support {
    use bevy_app::App;
    use bevy_ecs::prelude::{Component, Resource};
    use picus_core::{AdvancedAppPicusExt, UiComponentTemplate};

    /// Implemented by `#[derive(UiComponent)]`.
    pub trait UiComponentRegistration {
        fn register(app: &mut App);
    }

    pub fn register_ui_component<T: UiComponentTemplate>(app: &mut App) {
        app.register_ui_component::<T>();
    }

    pub fn register_projection_resource<R: Resource>(app: &mut App) {
        app.register_projection_resource::<R>();
    }

    pub fn register_style_selector_type<T: Component>(app: &mut App, name: &str) {
        app.register_style_selector_type::<T>(name);
    }
}
