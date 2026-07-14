# Crate boundaries

| Crate | Role |
|-------|------|
| `picus` | **Only** application dependency. Grouped modules + macros. |
| `picus_macros` | Proc-macros (`UiComponent`). Re-exported by `picus`. |
| `picus_core` | Implementation: projection, styling, overlays, plugin, runner. |
| `picus_widget` | Lookless retained widgets/properties (no production brand colours). |
| `picus_view` | Xilem-compatible view adapter on `picus_widget`. |
| `picus_surface` | wgpu/Vello surface for Bevy windows. |
| `picus_theme_test` | Test-only dark property sets; not for apps. |

## Forbidden

- Reintroduce upstream `masonry` / `xilem` **application** crates as dependencies.
- Depend on `picus_core` from application code (use `picus`).
- Ship production colour palettes from `picus_widget`.
