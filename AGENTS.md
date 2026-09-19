# AGENTS.md

Enforceable process rules and cross-cutting contracts for humans and automated
agents. Architecture and API guides live in Rustdoc (`cargo doc`) and [`README.md`](README.md).

## 1. Role and scope

- Keep rules short and executable.
- Nested `AGENTS.md` files may add subsystem-local hard rules without modifying
  third-party submodule contents.
- When a contract changes, update this file and the corresponding Rustdoc / README.

## 2. Dependency and crate boundaries

- Applications depend on the **`picus` facade only**, not `picus_core`.
- Upstream GUI stack: **`xilem`** facade (pinned until crates.io release includes imaging/layout).
  `picus_widget::masonry_core` inlined from `xilem::masonry`; reactive core and winit from `xilem::core` / `xilem::winit`.
  Paint adapter: `picus_imaging` (desktop only; no wasm).
- `picus_widget` is lookless: no production brand colour palettes; test skins belong in `picus_theme_test`.
- Macro expansions may only touch `picus::__macro_support` (doc-hidden).

## 3. Default application path

1. Install `PicusPlugin` on a Bevy `App`.
2. Load theme RON and/or select a variant **explicitly** (`AppPicusExt`).
3. Register business payloads with `add_ui_action::<T>()`.
4. Derive `UiComponent` (or `#[ui_view]` for zero-state projected regions)
   and register custom components via `register_ui_components!(app, ...)`.
5. Consume interactions with `MessageReader<UiAction<T>>`; exclusive systems
   should receive app-owned pending state collected by a normal message reader.
6. Start with `AppPicusExt::run_picus`.

## 4. Must-follow contracts

### BSN / authoring
- Public UI authoring components and nested authoring values are
  `Default + Clone` unless documented as runtime-only (e.g. `UiEmit`, hooks).
- Prefer `bsn!` / `bsn_list!` for static trees; `UiComponentTemplate::expand`
  remains authoritative for Picus-owned template parts.

### Projection
- Projection dependencies (components/resources) must be registered (via
  `#[ui_component(resources(...))]` or derive metadata).
- Avoid no-op mutable writes on projection-visible state so change detection stays meaningful.
- Style color transitions (`CurrentColorStyle`) must not be projection dependencies; apply interpolated colors on the retained tree.

### Actions / messages
- Applications **do not** drain or hold the internal action queue.
- Public surface: `UiAction<T>`, `UiActionSender<T>`, `UiEmit`, `add_ui_action`, `PicusUiSet`.
- PreUpdate order: `Input → RetainedRouting → DispatchActions`.
- Input-driven actions are visible to same-frame `Update` readers; Update-time sender emissions are next-frame.

### Styling / theme
- **Missing theme or missing rules ⇒ no visible framework defaults** (not an error).
- Framework **never** auto-selects dark/light.
- Partial themes are legal; only structural RON/token errors fail.
- Production colours come from stylesheet RON, not widget defaults.

### Runtime / input
- Per-window `MasonryRuntime` / `WindowRuntime`; primary window auto-attaches.
- Pointer coordinates from the event window’s physical cursor position.
- Click injection sends move before down/up; resize uses logical dimensions.
- Paint/present errors are captured; only successful `present()` marks painted.
- Font registration broadcasts to all windows and replays on attach.
- Continuous ~60Hz visual animation must not default to dirtying the full-window
  base present path — use `PaintIsolation::AnimEntry` (Stock: Spinner, indeterminate ProgressBar, focused TextArea caret).
- Pure anim-only frames skip Picus projection/style/overlay (`PicusUiSet::HeavyEcs`).

### Overlays / scroll
- Overlay projectors stay transparent until positioned.
- Outside-click dismissal checks the top overlay hit path / bound widget IDs.
- Nested wheel routing starts at the deepest hit target.

### picuscode / omp
- Integration example only; tests must not touch the user's real `~/.omp/`. Use temp dirs for session/config state.

## 5. Forbidden

- Public `UiEventQueue` / typed drain / process-global app action API.
- Framework default dark theme on empty configuration.
- Root-level `pub use picus_core::*` dump on the facade.
- inventory / linkme for UI component registration.
- Closure-on-Component application APIs.
- Application code calling `picus::__macro_support` directly.

## 6. Subsystem rules

- `crates/picus_core/AGENTS.md`
- `crates/picus_surface/AGENTS.md`
- `examples/picuscode/AGENTS.md`
- `thirdparty/AGENTS.md`
