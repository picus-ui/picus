# Public facade modules

Applications depend on `picus`, not `picus_core`. The facade is grouped so an
application can import the stable surface without inheriting implementation
details:

| Module | Use |
|--------|-----|
| `picus::prelude` | Normal application imports and proc/macro helpers |
| `picus::app` | `PicusPlugin`, `AppPicusExt`, Bevy app/ECS/window re-exports |
| `picus::components` | Authoring components, templates, and view helpers |
| `picus::events` | `UiAction`, `UiActionSender`, `UiEmit`, built-in action types |
| `picus::projection` | `ProjectionCtx`, `UiView`, xilem-compatible view types |
| `picus::styling` | Stylesheets, variants, selectors, inline style helpers |
| `picus::overlay` | Dialog, popover, toast, and overlay helpers |
| `picus::runtime` | Window/runtime types and ordinary runtime helpers |
| `picus::runtime::advanced` | Projector and registration APIs for framework integrations |
| `picus::scene` | `bsn!`, scene spawning, and template values |
| `picus::i18n` | Localization and font-facing application APIs |

The crate root intentionally exposes only the proc macros and macro support
boundary. It does not re-export the `picus_core` API, the internal action queue,
typed drains, global emitters, or raw retained-widget helpers. The normal
registration path is `register_ui_components!(app, ...)`; advanced registration
is opt-in and requires the `AdvancedAppPicusExt` trait from
`picus::runtime::advanced`.

### Continuous-anim paint isolation (retained)

Continuous ~60 Hz visual animation uses a **painter-slot** contract
(`PaintIsolation::{Inline, AnimEntry}`), not a global top layer:

| Surface | Role |
|---------|------|
| `picus::components` (`UiSpinner`, `UiProgressBar`) | App-facing controls; isolation defaults already correct |
| `picus_widget::PaintIsolation` | Retained/advanced enum + `apply` (not on `picus::prelude`) |
| Picus host discovery | Closed allowlist today; custom `AnimEntry.apply` alone is not enough |

Guide: [guide/paint-isolation.md](../guide/paint-isolation.md). AGENTS hard rule:
continuous anim must not default to dirtying the full-window base present path.

This split keeps the application contract small while allowing custom controls,
projectors, and platform integrations to use the lower-level APIs explicitly.
