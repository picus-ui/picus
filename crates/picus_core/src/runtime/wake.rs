//! Event-loop wake policy for UI clocks that are not display-rate animations.
//!
//! Display-rate AnimEntry widgets (Spinner / indeterminate bar) still write
//! [`RequestRedraw`] every tick. Caret blink is an AnimEntry but discrete and
//! uses the slow wait. Overlay auto-dismiss and hover debounce also arm that
//! wait. The default reactive wait is idle (`Duration::MAX`) unless a deadline
//! is armed.

use std::time::{Duration, Instant};

use bevy_ecs::prelude::*;
use bevy_window::RequestRedraw;
use bevy_winit::{UpdateMode, WinitSettings};

/// Marker: Picus owns [`WinitSettings`] wait timeouts for this app.
///
/// Inserted only when `run_picus` installed the default reactive settings.
/// Applications that insert their own [`WinitSettings`] are left untouched.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct PicusManagedWinitSettings;

/// Earliest wall-clock time another Bevy frame is needed for a slow UI clock
/// (cursor blink, debounce, auto-dismiss) without an immediate redraw.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct UiScheduledWake {
    next: Option<Instant>,
}

/// Host coalescing interval for discrete anim clocks (caret blink, portal
/// auto-hide). Matches half a caret cycle.
pub(crate) const SLOW_ANIM_CLOCK_WAKE: Duration = Duration::from_millis(500);

/// When true, Picus skips projection/style/overlay work for this Bevy frame.
///
/// Armed after a pure anim-only present (Spinner G2 or caret blink). Cleared
/// if input, rewrite, a style tween, or `StyleDirty` appears before PostUpdate.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PicusHeavyEcsGate {
    pub skip_heavy_ecs: bool,
}

/// Run condition: full Picus ECS rebuild/style/overlay work is needed.
pub(crate) fn heavy_ecs_enabled(gate: Res<PicusHeavyEcsGate>) -> bool {
    !gate.skip_heavy_ecs
}

impl UiScheduledWake {
    pub(crate) fn request_at(&mut self, when: Instant) {
        self.next = Some(self.next.map_or(when, |next| next.min(when)));
    }

    pub(crate) fn request_after(&mut self, delay: Duration) {
        self.request_at(Instant::now() + delay);
    }

    pub(crate) fn take_if_due(&mut self, now: Instant) -> bool {
        let Some(next) = self.next else {
            return false;
        };
        if now >= next {
            self.next = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn clear(&mut self) {
        self.next = None;
    }

    pub(crate) fn wait_from(&self, now: Instant) -> Duration {
        match self.next {
            Some(next) => next.saturating_duration_since(now),
            None => Duration::MAX,
        }
    }
}

/// Default reactive settings: sleep until input, `RequestRedraw`, or a scheduled
/// slow-clock wait. Display-rate animation uses `RequestRedraw`, not this wait.
pub(crate) fn idle_winit_settings() -> WinitSettings {
    WinitSettings {
        focused_mode: UpdateMode::reactive(Duration::MAX),
        unfocused_mode: UpdateMode::reactive_low_power(Duration::MAX),
    }
}

pub(crate) fn apply_managed_winit_wait(settings: &mut WinitSettings, wait: Duration) {
    settings.focused_mode = UpdateMode::reactive(wait);
    settings.unfocused_mode = UpdateMode::reactive_low_power(wait);
}

/// After paint: either request an immediate frame, arm a slow-clock wait, or
/// idle. `immediate` covers content presents and AnimEntry ticks (Spinner).
pub(crate) fn settle_frame_wake(
    demand_any: bool,
    immediate: bool,
    slow_anim_clock: bool,
    managed: bool,
    wake: &mut UiScheduledWake,
    redraw: &mut impl FnMut(),
    winit_settings: Option<&mut WinitSettings>,
) {
    let now = Instant::now();
    if immediate {
        wake.clear();
        if demand_any {
            redraw();
        }
    } else if slow_anim_clock {
        wake.request_after(SLOW_ANIM_CLOCK_WAKE);
        if wake.take_if_due(now) {
            redraw();
        }
    } else if wake.take_if_due(now) {
        redraw();
    }

    if managed && let Some(settings) = winit_settings {
        apply_managed_winit_wait(settings, wake.wait_from(Instant::now()));
    }
}

/// Clear the heavy-ECS skip if input or Masonry content dirt arrived this frame.
pub(crate) fn refresh_heavy_ecs_gate_after_input(
    mut gate: ResMut<PicusHeavyEcsGate>,
    runtime: Option<NonSend<super::MasonryRuntime>>,
) {
    if !gate.skip_heavy_ecs {
        return;
    }
    let Some(runtime) = runtime else {
        return;
    };
    if runtime
        .windows
        .values()
        .any(super::WindowRuntime::has_pending_content_work)
    {
        gate.skip_heavy_ecs = false;
    }
}

/// Clear the skip if a style tween or style dirty set appeared in Update.
pub(crate) fn refresh_heavy_ecs_gate_after_update(
    mut gate: ResMut<PicusHeavyEcsGate>,
    dirty: Query<(), With<crate::styling::StyleDirty>>,
    runners: Query<(), With<crate::bevy_tween::TimeRunner>>,
) {
    if !gate.skip_heavy_ecs {
        return;
    }
    if !dirty.is_empty() || !runners.is_empty() {
        gate.skip_heavy_ecs = false;
    }
}

/// Keep style tweens advancing: they do not go through Masonry `request_anim_frame`.
pub(crate) fn request_redraw_for_style_tweens(
    runners: Query<&crate::bevy_tween::TimeRunner>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    if runners.iter().any(|runner| !runner.is_finished()) {
        redraw.write(RequestRedraw);
    }
}

/// Arm a later wake for hover debounce and overlay auto-dismiss so those
/// timers do not depend on a 120 Hz reactive timeout.
pub(crate) fn schedule_pending_ui_clocks(
    pending_hover: Query<(
        &crate::styling::HoverDebounce,
        &crate::styling::PendingHoverState,
    )>,
    auto_dismiss: Query<&crate::ecs::AutoDismiss>,
    time: Res<bevy_time::Time>,
    mut wake: ResMut<UiScheduledWake>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    let now_secs = time.elapsed_secs_f64();
    for (debounce, pending) in &pending_hover {
        let remaining = pending.remaining_until_active(now_secs, debounce.enter_delay_secs);
        if remaining.is_zero() {
            redraw.write(RequestRedraw);
        } else {
            wake.request_after(remaining);
        }
    }
    for dismiss in &auto_dismiss {
        let remaining = dismiss.timer.remaining();
        if remaining.is_zero() {
            redraw.write(RequestRedraw);
        } else {
            wake.request_after(remaining);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_wake_keeps_earliest_deadline() {
        let mut wake = UiScheduledWake::default();
        let t0 = Instant::now();
        wake.request_at(t0 + Duration::from_millis(800));
        wake.request_at(t0 + Duration::from_millis(200));
        assert_eq!(wake.wait_from(t0), Duration::from_millis(200));
        assert!(!wake.take_if_due(t0));
        assert!(wake.take_if_due(t0 + Duration::from_millis(200)));
        assert!(wake.next.is_none());
    }

    #[test]
    fn settle_immediate_clears_slow_clock() {
        let mut wake = UiScheduledWake::default();
        wake.request_after(Duration::from_secs(5));
        let mut drew = false;
        settle_frame_wake(
            true,
            true,
            true,
            false,
            &mut wake,
            &mut || drew = true,
            None,
        );
        assert!(drew);
        assert!(wake.next.is_none());
    }

    #[test]
    fn settle_slow_clock_arms_wait_without_redraw() {
        let mut wake = UiScheduledWake::default();
        let mut drew = false;
        settle_frame_wake(
            true,
            false,
            true,
            false,
            &mut wake,
            &mut || drew = true,
            None,
        );
        assert!(!drew);
        assert!(wake.next.is_some());
        assert!(wake.wait_from(Instant::now()) <= SLOW_ANIM_CLOCK_WAKE);
    }
}
