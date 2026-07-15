//! Lightweight frame-phase timing for diagnosing CPU-bound UI frames.
//!
//! Enable with environment variable `PICUS_FRAME_TIMING=1` (or `true` / `yes` / `on`).
//! When enabled, Picus records **per-window** phase durations under a monotonic
//! `frame_id` and logs a summary about once per second at `info` level.
//!
//! ## Units and averages
//!
//! - **Process rollup** (`picus frame timing (process)`): `frames` is the count of
//!   **per-window paint attempts that entered work** (not Bevy system invocations).
//!   Multi-window sessions therefore inflate `frames` by ~window count.
//!   ECS phases (`input_dispatch_ms`, `synth_ms`, `rebuild_ms`) are averaged over
//!   **`bevy_frames`** (each `begin_frame` / synthesis entry), not paint attempts.
//!   Process `paint_ms` / `redraw_ms` / `present_ms` use **content paint attempts**
//!   (`frames − anim_tick_only`) so throttled anim-only zeros do not dilute
//!   encode/present cost (same rule as per-window present-path means). When there
//!   are no content paints, `present_ms` is `0` and paint/redraw fall back to all
//!   entered-work attempts so pure anim-tick cost remains visible.
//! - **Per-window line**: `anim_tick_ms` is averaged over **all** entered-work paint
//!   attempts for that window. Present-path phases (`scene_build_*`,
//!   `surface_acquire`, `encode_*`, `composite`, `present_submit`) are averaged
//!   only over **content paint attempts** (`frames - anim_tick_only`) so throttled
//!   anim-only zeros do not dilute encode/present cost. When that denominator is
//!   zero, those fields log as `0.000`.
//! - Pure idle (`Skipped`) does not assign `frame_id`s; summaries still flush on a
//!   ~1s wall clock from `begin_frame` so Button-idle campaigns still get process
//!   lines (often with `frames=0` for paint).
//!
//! ## Phases (G1 skeleton)
//!
//! | Field | Timeline | Meaning (instrumentation today) |
//! |-------|----------|----------------------------------|
//! | `input_dispatch_ms` | A | PreUpdate Masonry input injection (process-wide; per Bevy frame) |
//! | `anim_tick_ms` | B | `WindowEvent::AnimFrame` handling (**includes rewrite** that runs inside AnimFrame) |
//! | `scene_build_base_ms` | C | Root `render_root.redraw()` only (full window today); **not** AnimFrame rewrite |
//! | `scene_build_anim_ms` | C | Isolated anim-layer scene build (**0** until layered) |
//! | `surface_acquire_ms` | D | Swapchain texture acquire |
//! | `encode_base_ms` | C/D | Vello encode of base content (full window today) |
//! | `encode_anim_ms` | C/D | Vello encode of anim layers (**0** until layered) |
//! | `composite_ms` | D | Blit/composite into the swapchain texture |
//! | `present_submit_ms` | D | CPU wall time of `present()` |
//! | `presented` / `anim_tick_only` | — | Counters for successful present vs anim-only ticks |
//!
//! Until rewrite is timed separately, timeline **B** (`anim_tick_ms`) absorbs rewrite
//! that Masonry performs during `AnimFrame`. Timeline **C** `scene_build_base_ms` is
//! only the subsequent root `redraw()` call. Do not treat B/C as cleanly split yet.
//!
//! ## CPU submit ≠ display time
//!
//! `present_submit_ms` (and every other phase above) is **CPU-side wall time**.
//! It is **not** displayed-frame latency, DWM composition time, or vsync-aligned
//! frame time. On Windows, use PresentMon/ETW for actual display-path metrics;
//! see `docs/perf/frame-pipeline-baseline.md` and `docs/plans/frame-pipeline.md`.
//!
//! Example log line:
//!
//! ```text
//! picus frame timing: window=1v0 frame_id=120..179 frames=60 presented=30 \
//!   anim_tick_only=28 anim_tick_ms=0.11 scene_build_base_ms=1.80 \
//!   scene_build_anim_ms=0.00 surface_acquire_ms=0.25 encode_base_ms=5.10 \
//!   encode_anim_ms=0.00 composite_ms=0.18 present_submit_ms=0.09 \
//!   paint_reasons=anim_frame|anim_no_present
//! ```
//!
//! Spans are always available via `tracing` (`picus_core::perf` target) so
//! `RUST_LOG=picus_core::perf=trace` works without the env flag.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;

#[cfg(test)]
thread_local! {
    /// When true, [`frame_timing_enabled`] returns true regardless of env
    /// (unit tests only; never set from production code).
    static FORCE_FRAME_TIMING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether process-level frame timing aggregation is enabled.
pub fn frame_timing_enabled() -> bool {
    #[cfg(test)]
    if FORCE_FRAME_TIMING.with(|c| c.get()) {
        return true;
    }

    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("PICUS_FRAME_TIMING")
            .map(|value| {
                let value = value.trim();
                value == "1"
                    || value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("yes")
                    || value.eq_ignore_ascii_case("on")
            })
            .unwrap_or(false)
    })
}

/// CPU-side phase timings for one window paint attempt.
///
/// Layered anim encode paths are not wired yet; `scene_build_anim` and
/// `encode_anim` stay zero until isolation lands. See frame-pipeline plan.
///
/// # Instrumentation notes
///
/// - `anim_tick` covers `WindowEvent::AnimFrame` (including rewrite Masonry runs
///   inside that event).
/// - `scene_build_base` is only the root `redraw()` that follows when a content
///   present is attempted — not AnimFrame rewrite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PaintPhaseTimings {
    pub anim_tick: Duration,
    pub scene_build_base: Duration,
    pub scene_build_anim: Duration,
    pub surface_acquire: Duration,
    pub encode_base: Duration,
    pub encode_anim: Duration,
    pub composite: Duration,
    pub present_submit: Duration,
}

impl PaintPhaseTimings {
    /// Sum of anim-tick + scene-build work (legacy `redraw_duration` aggregate).
    #[must_use]
    pub fn redraw_total(&self) -> Duration {
        self.anim_tick + self.scene_build_base + self.scene_build_anim
    }

    /// Sum of surface/present work (legacy `present_duration` aggregate).
    #[must_use]
    pub fn present_total(&self) -> Duration {
        self.surface_acquire
            + self.encode_base
            + self.encode_anim
            + self.composite
            + self.present_submit
    }
}

/// Aggregated phase timings for recent frames (process + per-window).
#[derive(Resource, Debug, Default)]
pub struct FrameTiming {
    window_started: Option<Instant>,
    /// Monotonic id assigned to each per-window paint attempt that enters work.
    next_frame_id: u64,
    /// Count of Bevy-frame entries via [`Self::begin_frame`] (ECS average denominator).
    bevy_frames: u32,
    /// Process-wide PreUpdate input injection time this summary window.
    input_dispatch_ns: u128,
    /// Per-window paint attempts that entered work (multi-window: one per window).
    frames: u32,
    painted_frames: u32,
    anim_tick_only_frames: u32,
    synth_dirty_frames: u32,
    synth_ns: u128,
    rebuild_ns: u128,
    paint_ns: u128,
    paint_redraw_ns: u128,
    paint_present_ns: u128,
    synth_nodes_sum: u64,
    synth_cache_hits_sum: u64,
    /// Bitmask of paint reasons observed this window (see [`PaintReason`]).
    paint_reasons: u32,
    /// Compact labels of dirty synthesis reasons seen this window.
    synth_reason_labels: Vec<&'static str>,
    by_window: HashMap<Entity, WindowTimingAgg>,
}

#[derive(Debug, Default)]
struct WindowTimingAgg {
    frames: u32,
    painted_frames: u32,
    anim_tick_only_frames: u32,
    first_frame_id: Option<u64>,
    last_frame_id: u64,
    anim_tick_ns: u128,
    scene_build_base_ns: u128,
    scene_build_anim_ns: u128,
    surface_acquire_ns: u128,
    encode_base_ns: u128,
    encode_anim_ns: u128,
    composite_ns: u128,
    present_submit_ns: u128,
    paint_reasons: u32,
}

impl WindowTimingAgg {
    /// Paint attempts that ran content/scene/present work (not pure anim-tick skips).
    fn content_paint_frames(&self) -> u32 {
        self.frames.saturating_sub(self.anim_tick_only_frames)
    }
}

/// Why a paint pass ran (for idle continuous-redraw diagnosis).
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum PaintReason {
    FirstPaint = 1 << 0,
    NeedsRedraw = 1 << 1,
    NeedsAnimFrame = 1 << 2,
    RenderRootNeedsAnim = 1 << 3,
    NeedsRewritePasses = 1 << 4,
    Skipped = 1 << 5,
    /// Animation ticked but no widget requested a pixel update (no present).
    AnimTickNoPresent = 1 << 6,
}

impl FrameTiming {
    /// Called once per Bevy frame from the synthesis entry path.
    ///
    /// Starts the ~1s summary window and flushes on wall clock even when no
    /// window entered paint work (idle Button baselines still get process logs).
    pub fn begin_frame(&mut self) {
        if !frame_timing_enabled() {
            return;
        }
        if self.window_started.is_none() {
            self.window_started = Some(Instant::now());
        }
        self.bevy_frames = self.bevy_frames.saturating_add(1);
        self.maybe_flush();
    }

    /// Record PreUpdate Masonry input injection cost (timeline A).
    pub fn record_input_dispatch(&mut self, duration: Duration) {
        if !frame_timing_enabled() {
            return;
        }
        if self.window_started.is_none() {
            self.window_started = Some(Instant::now());
        }
        self.input_dispatch_ns += duration.as_nanos();
        self.maybe_flush();
    }

    pub fn record_synthesis(
        &mut self,
        duration: Duration,
        dirty: bool,
        node_count: usize,
        reason_labels: &[&'static str],
    ) {
        self.record_synthesis_with_cache(duration, dirty, node_count, 0, reason_labels);
    }

    pub fn record_synthesis_with_cache(
        &mut self,
        duration: Duration,
        dirty: bool,
        node_count: usize,
        cache_hits: usize,
        reason_labels: &[&'static str],
    ) {
        if !frame_timing_enabled() {
            return;
        }
        self.synth_ns += duration.as_nanos();
        if dirty {
            self.synth_dirty_frames += 1;
            self.synth_nodes_sum += node_count as u64;
            self.synth_cache_hits_sum += cache_hits as u64;
            for label in reason_labels {
                if !self.synth_reason_labels.contains(label) {
                    self.synth_reason_labels.push(*label);
                }
            }
        }
        self.maybe_flush();
    }

    pub fn record_rebuild(&mut self, duration: Duration) {
        if !frame_timing_enabled() {
            return;
        }
        self.rebuild_ns += duration.as_nanos();
        self.maybe_flush();
    }

    /// Record one per-window paint attempt with phase breakdown.
    ///
    /// Assigns a monotonic `frame_id`. Process-level `frames` counts **per-window
    /// paint attempts**, not Bevy `paint_masonry_ui` invocations.
    pub fn record_window_paint(
        &mut self,
        window: Entity,
        phases: PaintPhaseTimings,
        painted: bool,
        anim_tick_only: bool,
        reasons: u32,
    ) {
        if !frame_timing_enabled() {
            return;
        }
        self.accumulate_window_paint(window, phases, painted, anim_tick_only, reasons);
        self.maybe_flush();
    }

    fn accumulate_window_paint(
        &mut self,
        window: Entity,
        phases: PaintPhaseTimings,
        painted: bool,
        anim_tick_only: bool,
        reasons: u32,
    ) {
        if self.window_started.is_none() {
            self.window_started = Some(Instant::now());
        }

        let frame_id = self.next_frame_id;
        self.next_frame_id = self.next_frame_id.saturating_add(1);

        let redraw = phases.redraw_total();
        let present = phases.present_total();
        let total = redraw + present;

        self.frames += 1;
        self.paint_ns += total.as_nanos();
        self.paint_redraw_ns += redraw.as_nanos();
        self.paint_present_ns += present.as_nanos();
        if painted {
            self.painted_frames += 1;
        }
        if anim_tick_only {
            self.anim_tick_only_frames += 1;
        }
        self.paint_reasons |= reasons;

        let entry = self.by_window.entry(window).or_default();
        if entry.first_frame_id.is_none() {
            entry.first_frame_id = Some(frame_id);
        }
        entry.last_frame_id = frame_id;
        entry.frames += 1;
        if painted {
            entry.painted_frames += 1;
        }
        if anim_tick_only {
            entry.anim_tick_only_frames += 1;
        }
        entry.anim_tick_ns += phases.anim_tick.as_nanos();
        entry.scene_build_base_ns += phases.scene_build_base.as_nanos();
        entry.scene_build_anim_ns += phases.scene_build_anim.as_nanos();
        entry.surface_acquire_ns += phases.surface_acquire.as_nanos();
        entry.encode_base_ns += phases.encode_base.as_nanos();
        entry.encode_anim_ns += phases.encode_anim.as_nanos();
        entry.composite_ns += phases.composite.as_nanos();
        entry.present_submit_ns += phases.present_submit.as_nanos();
        entry.paint_reasons |= reasons;
    }

    fn maybe_flush(&mut self) {
        let Some(started) = self.window_started else {
            return;
        };
        if started.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.flush_summary();
    }

    fn flush_summary(&mut self) {
        // ECS phases use Bevy-frame denominator so multi-window paint attempts
        // do not dilute synth/input averages.
        let bevy_frames = self.bevy_frames.max(1) as f64;
        let content_paint_frames = self.frames.saturating_sub(self.anim_tick_only_frames);
        // Present-path process means: same content-paint rule as per-window lines.
        // When only anim-tick-only attempts exist, fall back to all entered-work
        // frames for paint/redraw so pure-anim cost is still visible; present=0.
        let paint_denom = if content_paint_frames > 0 {
            f64::from(content_paint_frames)
        } else {
            f64::from(self.frames.max(1))
        };
        let input_dispatch_ms = (self.input_dispatch_ns as f64 / bevy_frames) / 1_000_000.0;
        let synth_ms = (self.synth_ns as f64 / bevy_frames) / 1_000_000.0;
        let rebuild_ms = (self.rebuild_ns as f64 / bevy_frames) / 1_000_000.0;
        let paint_ms = (self.paint_ns as f64 / paint_denom) / 1_000_000.0;
        let redraw_ms = (self.paint_redraw_ns as f64 / paint_denom) / 1_000_000.0;
        let present_ms = if content_paint_frames == 0 {
            0.0
        } else {
            (self.paint_present_ns as f64 / f64::from(content_paint_frames)) / 1_000_000.0
        };
        let avg_nodes = if self.synth_dirty_frames == 0 {
            0.0
        } else {
            self.synth_nodes_sum as f64 / f64::from(self.synth_dirty_frames)
        };
        let avg_cache_hits = if self.synth_dirty_frames == 0 {
            0.0
        } else {
            self.synth_cache_hits_sum as f64 / f64::from(self.synth_dirty_frames)
        };
        let reasons = format_paint_reasons(self.paint_reasons);
        let synth_reasons = if self.synth_reason_labels.is_empty() {
            "none".to_string()
        } else {
            self.synth_reason_labels.join("|")
        };

        // Process-wide rollup (ECS + multi-window paint attempt totals).
        tracing::info!(
            target: "picus_core::perf",
            bevy_frames = self.bevy_frames,
            frames = self.frames,
            content_paint_frames,
            presented = self.painted_frames,
            anim_tick_only = self.anim_tick_only_frames,
            synth_dirty = self.synth_dirty_frames,
            input_dispatch_ms = format_args!("{input_dispatch_ms:.3}"),
            synth_ms = format_args!("{synth_ms:.3}"),
            rebuild_ms = format_args!("{rebuild_ms:.3}"),
            paint_ms = format_args!("{paint_ms:.3}"),
            redraw_ms = format_args!("{redraw_ms:.3}"),
            present_ms = format_args!("{present_ms:.3}"),
            avg_synth_nodes = format_args!("{avg_nodes:.0}"),
            avg_cache_hits = format_args!("{avg_cache_hits:.0}"),
            paint_reasons = %reasons,
            synth_reasons = %synth_reasons,
            note = "frames=per-window paint attempts; paint/present avgs over content_paint_frames; ECS over bevy_frames; CPU only (PresentMon/ETW for display)",
            "picus frame timing (process)"
        );

        // Per-window phase breakdown with frame_id range.
        // input_dispatch is process-wide only (omitted here to avoid misread).
        for (window, agg) in &self.by_window {
            let w_frames = agg.frames.max(1) as f64;
            let content_frames = agg.content_paint_frames().max(1) as f64;
            let content_denom = agg.content_paint_frames();
            let first = agg.first_frame_id.unwrap_or(agg.last_frame_id);
            let anim_tick_ms = (agg.anim_tick_ns as f64 / w_frames) / 1_000_000.0;
            // Present-path averages: skip anim_tick_only zeros so encode/present
            // report the cost of frames that actually built/presented content.
            let scene_build_base_ms = if content_denom == 0 {
                0.0
            } else {
                (agg.scene_build_base_ns as f64 / content_frames) / 1_000_000.0
            };
            let scene_build_anim_ms = if content_denom == 0 {
                0.0
            } else {
                (agg.scene_build_anim_ns as f64 / content_frames) / 1_000_000.0
            };
            let surface_acquire_ms = if content_denom == 0 {
                0.0
            } else {
                (agg.surface_acquire_ns as f64 / content_frames) / 1_000_000.0
            };
            let encode_base_ms = if content_denom == 0 {
                0.0
            } else {
                (agg.encode_base_ns as f64 / content_frames) / 1_000_000.0
            };
            let encode_anim_ms = if content_denom == 0 {
                0.0
            } else {
                (agg.encode_anim_ns as f64 / content_frames) / 1_000_000.0
            };
            let composite_ms = if content_denom == 0 {
                0.0
            } else {
                (agg.composite_ns as f64 / content_frames) / 1_000_000.0
            };
            let present_submit_ms = if content_denom == 0 {
                0.0
            } else {
                (agg.present_submit_ns as f64 / content_frames) / 1_000_000.0
            };
            let w_reasons = format_paint_reasons(agg.paint_reasons);

            tracing::info!(
                target: "picus_core::perf",
                window = %format_args!("{window:?}"),
                frame_id_first = first,
                frame_id_last = agg.last_frame_id,
                frames = agg.frames,
                presented = agg.painted_frames,
                anim_tick_only = agg.anim_tick_only_frames,
                content_paint_frames = content_denom,
                anim_tick_ms = format_args!("{anim_tick_ms:.3}"),
                scene_build_base_ms = format_args!("{scene_build_base_ms:.3}"),
                scene_build_anim_ms = format_args!("{scene_build_anim_ms:.3}"),
                surface_acquire_ms = format_args!("{surface_acquire_ms:.3}"),
                encode_base_ms = format_args!("{encode_base_ms:.3}"),
                encode_anim_ms = format_args!("{encode_anim_ms:.3}"),
                composite_ms = format_args!("{composite_ms:.3}"),
                present_submit_ms = format_args!("{present_submit_ms:.3}"),
                paint_reasons = %w_reasons,
                note = "present-path avgs over content_paint_frames; present_submit_ms is CPU not display",
                "picus frame timing"
            );
        }

        let next_frame_id = self.next_frame_id;
        *self = Self {
            window_started: Some(Instant::now()),
            next_frame_id,
            ..Self::default()
        };
    }
}

fn format_paint_reasons(mask: u32) -> String {
    let mut parts = Vec::new();
    if mask & PaintReason::FirstPaint as u32 != 0 {
        parts.push("first");
    }
    if mask & PaintReason::NeedsRedraw as u32 != 0 {
        parts.push("redraw");
    }
    if mask & PaintReason::NeedsAnimFrame as u32 != 0 {
        parts.push("anim_frame");
    }
    if mask & PaintReason::RenderRootNeedsAnim as u32 != 0 {
        parts.push("needs_anim");
    }
    if mask & PaintReason::NeedsRewritePasses as u32 != 0 {
        parts.push("rewrite");
    }
    if mask & PaintReason::Skipped as u32 != 0 {
        parts.push("skipped");
    }
    if mask & PaintReason::AnimTickNoPresent as u32 != 0 {
        parts.push("anim_no_present");
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join("|")
    }
}

/// RAII timer that records elapsed nanos into a callback.
pub struct PhaseTimer {
    start: Instant,
}

impl PhaseTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ForceTimingGuard;

    impl ForceTimingGuard {
        fn enter() -> Self {
            FORCE_FRAME_TIMING.with(|c| c.set(true));
            Self
        }
    }

    impl Drop for ForceTimingGuard {
        fn drop(&mut self) {
            FORCE_FRAME_TIMING.with(|c| c.set(false));
        }
    }

    fn test_entity(index: u32) -> Entity {
        Entity::from_bits(u64::from(index) | (1u64 << 32))
    }

    #[test]
    fn paint_phase_totals_sum_components() {
        let phases = PaintPhaseTimings {
            anim_tick: Duration::from_millis(1),
            scene_build_base: Duration::from_millis(2),
            scene_build_anim: Duration::from_millis(3),
            surface_acquire: Duration::from_millis(4),
            encode_base: Duration::from_millis(5),
            encode_anim: Duration::from_millis(6),
            composite: Duration::from_millis(7),
            present_submit: Duration::from_millis(8),
        };
        assert_eq!(phases.redraw_total(), Duration::from_millis(6));
        assert_eq!(phases.present_total(), Duration::from_millis(30));
    }

    #[test]
    fn format_paint_reasons_lists_known_bits() {
        let mask = PaintReason::NeedsAnimFrame as u32 | PaintReason::AnimTickNoPresent as u32;
        assert_eq!(format_paint_reasons(mask), "anim_frame|anim_no_present");
    }

    #[test]
    fn frame_timing_defaults_have_zero_frame_id() {
        let timing = FrameTiming::default();
        assert_eq!(timing.next_frame_id, 0);
        assert!(timing.by_window.is_empty());
        let _ = frame_timing_enabled();
    }

    #[test]
    fn record_window_paint_assigns_monotonic_frame_ids_and_counters() {
        let _guard = ForceTimingGuard::enter();
        let mut timing = FrameTiming::default();
        let window = test_entity(1);

        // Presented success path.
        timing.record_window_paint(
            window,
            PaintPhaseTimings {
                anim_tick: Duration::from_micros(100),
                scene_build_base: Duration::from_millis(2),
                encode_base: Duration::from_millis(5),
                present_submit: Duration::from_micros(50),
                ..PaintPhaseTimings::default()
            },
            true,
            false,
            PaintReason::NeedsRedraw as u32,
        );
        // Anim-tick only (no pixel present).
        timing.record_window_paint(
            window,
            PaintPhaseTimings {
                anim_tick: Duration::from_micros(80),
                ..PaintPhaseTimings::default()
            },
            false,
            true,
            PaintReason::AnimTickNoPresent as u32,
        );
        // Throttle-style skip (also anim_tick_only).
        timing.record_window_paint(
            window,
            PaintPhaseTimings {
                anim_tick: Duration::from_micros(90),
                ..PaintPhaseTimings::default()
            },
            false,
            true,
            PaintReason::NeedsAnimFrame as u32 | PaintReason::AnimTickNoPresent as u32,
        );
        // Present retry / failed (content path, not painted, not anim_tick_only).
        timing.record_window_paint(
            window,
            PaintPhaseTimings {
                anim_tick: Duration::from_micros(70),
                scene_build_base: Duration::from_millis(1),
                surface_acquire: Duration::from_micros(200),
                encode_base: Duration::from_millis(3),
                ..PaintPhaseTimings::default()
            },
            false,
            false,
            PaintReason::NeedsRedraw as u32,
        );

        assert_eq!(timing.next_frame_id, 4);
        assert_eq!(timing.frames, 4);
        assert_eq!(timing.painted_frames, 1);
        assert_eq!(timing.anim_tick_only_frames, 2);

        let agg = timing.by_window.get(&window).expect("window agg");
        assert_eq!(agg.frames, 4);
        assert_eq!(agg.painted_frames, 1);
        assert_eq!(agg.anim_tick_only_frames, 2);
        assert_eq!(agg.content_paint_frames(), 2);
        assert_eq!(agg.first_frame_id, Some(0));
        assert_eq!(agg.last_frame_id, 3);
        // Encode only accumulated on the two content paints (5ms + 3ms).
        assert_eq!(agg.encode_base_ns, Duration::from_millis(8).as_nanos());
        // Anim tick only attempts contribute zero encode.
        assert!(agg.anim_tick_ns > 0);
    }

    #[test]
    fn content_paint_average_excludes_anim_tick_only_zeros() {
        let _guard = ForceTimingGuard::enter();
        let mut timing = FrameTiming::default();
        let window = test_entity(2);

        timing.record_window_paint(
            window,
            PaintPhaseTimings {
                encode_base: Duration::from_millis(10),
                ..PaintPhaseTimings::default()
            },
            true,
            false,
            PaintReason::NeedsRedraw as u32,
        );
        timing.record_window_paint(
            window,
            PaintPhaseTimings::default(),
            false,
            true,
            PaintReason::AnimTickNoPresent as u32,
        );

        let agg = timing.by_window.get(&window).expect("agg");
        assert_eq!(agg.frames, 2);
        assert_eq!(agg.content_paint_frames(), 1);
        // Mean over content paints = 10ms, not 5ms diluted over both frames.
        let mean_ms =
            (agg.encode_base_ns as f64 / f64::from(agg.content_paint_frames())) / 1_000_000.0;
        assert!((mean_ms - 10.0).abs() < 1e-9);
    }

    #[test]
    fn painted_and_anim_tick_only_are_independent_counters() {
        let _guard = ForceTimingGuard::enter();
        let mut timing = FrameTiming::default();
        let window = test_entity(3);

        // A content paint that did not successfully present (retry/failed).
        timing.record_window_paint(
            window,
            PaintPhaseTimings {
                encode_base: Duration::from_millis(1),
                ..PaintPhaseTimings::default()
            },
            false,
            false,
            PaintReason::NeedsRedraw as u32,
        );
        assert_eq!(timing.painted_frames, 0);
        assert_eq!(timing.anim_tick_only_frames, 0);
        assert_eq!(
            timing
                .by_window
                .get(&window)
                .expect("agg")
                .content_paint_frames(),
            1
        );
    }

    #[test]
    fn process_content_paint_denom_excludes_anim_tick_only() {
        let _guard = ForceTimingGuard::enter();
        let mut timing = FrameTiming::default();
        let window = test_entity(4);

        // 10ms present path on one content paint.
        timing.record_window_paint(
            window,
            PaintPhaseTimings {
                present_submit: Duration::from_millis(10),
                encode_base: Duration::from_millis(0),
                ..PaintPhaseTimings::default()
            },
            true,
            false,
            PaintReason::NeedsRedraw as u32,
        );
        // Tick-only zeros would dilute 10ms → 5ms if averaged over all frames.
        timing.record_window_paint(
            window,
            PaintPhaseTimings::default(),
            false,
            true,
            PaintReason::AnimTickNoPresent as u32,
        );

        assert_eq!(timing.frames, 2);
        assert_eq!(timing.anim_tick_only_frames, 1);
        let content = timing.frames.saturating_sub(timing.anim_tick_only_frames);
        assert_eq!(content, 1);
        let present_mean_ms =
            (timing.paint_present_ns as f64 / f64::from(content)) / 1_000_000.0;
        assert!((present_mean_ms - 10.0).abs() < 1e-9);
    }
}
