//! Settings window wiring for picuscode.
//!
//! The settings view markers and projection live in [`crate::ui`] and
//! [`crate::state`]; this module is reserved for future settings-specific
//! helpers (key validation, provider dropdown population, test-connection,
//! etc.) and is intentionally minimal in Phase 1 so the panel can read and
//! write omp's config through the bridge (model / thinking / mode options).
