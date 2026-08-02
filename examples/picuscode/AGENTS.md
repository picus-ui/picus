# picuscode example — agent rules

Integration example for Oh My Pi (omp) + Picus. Not a product binary for end users.

## Hard rules

- **Do not** read, write, or delete the developer's real `~/.omp/` (or
  equivalent user config home) from tests or example default paths.
- Prefer fixture directories under the example or temp dirs for agent/session state.
- omp bridge (ACP over stdio) and streaming Markdown are intentional advanced
  surfaces; keep application entry on the standard DX path:
  `PicusPlugin` + `add_ui_action` + `register_ui_components!` + `run_picus`.
