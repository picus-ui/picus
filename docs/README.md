# Picus documentation

This directory is the human-readable source of truth for architecture, application
guides, and subsystem deep-dives. Hard process rules for agents live in
repository `AGENTS.md` files (root and nested where needed).

## Map

| Path | Topic |
|------|--------|
| [architecture/overview.md](architecture/overview.md) | Bevy + Masonry + projection overview |
| [architecture/crates.md](architecture/crates.md) | Crate boundaries |
| [guide/app.md](guide/app.md) | How to write a Picus app (`AppPicusExt`, theme, `UiAction`, BSN) |
| [guide/events-messages.md](guide/events-messages.md) | `UiAction`, `UiActionSender`, `UiEmit`, scheduling |
| [guide/macros.md](guide/macros.md) | `#[derive(UiComponent)]`, `register_ui_components!`, `classes!` |
| [guide/styling-themes.md](guide/styling-themes.md) | No-theme contract, RON, variants, backdrop |
| [examples/index.md](examples/index.md) | What each example teaches |
| [contributing/codewhale-submodule.md](contributing/codewhale-submodule.md) | CodeWhale fork sync |
| [plans/app-dx.md](plans/app-dx.md) | Application DX plan (in progress / landed) |

Start with [guide/app.md](guide/app.md) and the `timer` or `calculator` example.
