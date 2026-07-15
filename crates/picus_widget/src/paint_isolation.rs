// Copyright 2026 Summp
// SPDX-License-Identifier: Apache-2.0

//! Public paint isolation contract for continuous visual animation.
//!
//! [`PaintIsolation`] declares a **painter-order slot**, not a global top-most
//! layer. Ordinary widgets stay [`PaintIsolation::Inline`] (base / cached
//! scene). Continuous high-frequency animation must use
//! [`PaintIsolation::AnimEntry`] so dirty frames do not rewrite the full-window
//! base present path.
//!
//! # Discovery vs promotion (P3 honesty)
//!
//! - **Promotion decision** is isolation-keyed: only
//!   [`PaintIsolation::AnimEntry`] becomes an anim compositor entry.
//! - **Resolving** isolation for a live widget is still a **closed type
//!   allowlist** in the Picus host (`Spinner`, indeterminate `ProgressBar`).
//!   Calling [`PaintIsolation::apply`] alone makes Masonry reserve an External
//!   placeholder; without host discovery + a host painter the slot stays a
//!   transparent External forever (never an empty Anim).
//!
//! Path forward for third-party retained widgets: open discovery (e.g. trait or
//! TypeId-keyed host painter registry — not inventory/linkme). Until then stock
//! anim widgets are framework-known only.
//!
//! See `docs/guide/paint-isolation.md` and `docs/architecture/runtime.md`.

use crate::core::{PaintCtx, PaintLayerMode};

/// How a widget contributes pixels into the painter-order composite.
///
/// This is a **painter slot** declaration (inline base vs isolated anim entry),
/// not a Z-order “always on top” flag. Order still follows Masonry
/// `VisualLayerPlan` painter order.
///
/// # Defaults
///
/// | Widget | Isolation |
/// |--------|-----------|
/// | Most widgets | [`Inline`](Self::Inline) |
/// | [`Spinner`](crate::widgets::Spinner) | [`AnimEntry`](Self::AnimEntry) |
/// | Indeterminate [`ProgressBar`](crate::widgets::ProgressBar) | [`AnimEntry`](Self::AnimEntry) |
/// | Determinate [`ProgressBar`](crate::widgets::ProgressBar) | [`Inline`](Self::Inline) |
///
/// # Contract
///
/// - [`AnimEntry`](Self::AnimEntry) maps to Masonry
///   [`PaintLayerMode::External`] every paint (mode is **not** sticky — the
///   widget must re-apply each paint pass).
/// - **Promotion** to an anim compositor entry is isolation-keyed
///   (`promotes_to_anim_host`), but the host **discovers** isolation only for
///   known types that implement `paint_isolation()` (closed allowlist today).
///   Unknown External stays transparent External — never an empty Anim.
/// - Continuous ~60 Hz visual animation **must not** default to dirtying the
///   full-window base present path; use [`AnimEntry`](Self::AnimEntry) **and**
///   a host-known painter path (stock: Spinner / indeterminate ProgressBar).
///
/// # Known limitation (custom widgets)
///
/// A third-party widget that only calls [`Self::apply`] with
/// [`Self::AnimEntry`] gets an External placeholder but is **not** promoted to
/// Anim until discovery is opened (trait / TypeId host-painter registry). See
/// module docs and `docs/guide/paint-isolation.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PaintIsolation {
    /// Paint into the base / cached scene segment (default for most widgets).
    #[default]
    Inline,
    /// Reserve an External painter-order slot filled by the Picus anim host
    /// **when** the host discovers this isolation and has a scene painter.
    ///
    /// Required for continuous high-frequency visual animation so anim ticks
    /// encode only the anim entry (G2 selective path) instead of rewriting base.
    AnimEntry,
}

impl PaintIsolation {
    /// Masonry paint-layer mode for this isolation declaration.
    ///
    /// [`Self::AnimEntry`] → [`PaintLayerMode::External`];
    /// [`Self::Inline`] → [`PaintLayerMode::Inline`].
    #[inline]
    pub const fn paint_layer_mode(self) -> PaintLayerMode {
        match self {
            Self::Inline => PaintLayerMode::Inline,
            Self::AnimEntry => PaintLayerMode::External,
        }
    }

    /// Whether the isolation **value** is an anim-host promotion candidate.
    ///
    /// Does not mean the host has discovered this widget type; discovery is a
    /// separate allowlist (see module docs).
    #[inline]
    pub const fn promotes_to_anim_host(self) -> bool {
        matches!(self, Self::AnimEntry)
    }

    /// Apply this isolation to a paint context.
    ///
    /// For [`Self::AnimEntry`], sets [`PaintLayerMode::External`]. Mode is not
    /// sticky — call every paint when isolation is non-default. [`Self::Inline`]
    /// is a no-op (Masonry default is Inline after each pass reset).
    ///
    /// This only affects the Masonry painter slot. Host Anim promotion still
    /// requires discovery of the widget’s isolation + a host scene painter.
    #[inline]
    pub fn apply(self, ctx: &mut PaintCtx<'_>) {
        if self.promotes_to_anim_host() {
            ctx.set_paint_layer_mode(self.paint_layer_mode());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_mode_mapping() {
        assert_eq!(PaintIsolation::default(), PaintIsolation::Inline);
        assert_eq!(
            PaintIsolation::Inline.paint_layer_mode(),
            PaintLayerMode::Inline
        );
        assert_eq!(
            PaintIsolation::AnimEntry.paint_layer_mode(),
            PaintLayerMode::External
        );
        assert!(!PaintIsolation::Inline.promotes_to_anim_host());
        assert!(PaintIsolation::AnimEntry.promotes_to_anim_host());
        // `apply` uses paint_layer_mode + set_paint_layer_mode; AnimEntry must
        // map to External so Masonry reserves an External placeholder.
        // End-to-end apply→External is covered by
        // `runtime::layers` isolation-box / Spinner paint tests.
        assert_eq!(
            PaintIsolation::AnimEntry.paint_layer_mode(),
            PaintLayerMode::External
        );
    }
}
