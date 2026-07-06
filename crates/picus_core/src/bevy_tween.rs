use std::{marker::PhantomData, time::Duration};

use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::{Component, Mutable},
    entity::Entity,
    prelude::*,
    schedule::{IntoScheduleConfigs, ScheduleLabel, SystemSet},
    system::ScheduleSystem,
};
use bevy_time::Time;

pub mod interpolation {
    #[derive(
        bevy_ecs::component::Component, Debug, Clone, Copy, Default, PartialEq, serde::Deserialize,
    )]
    pub enum EaseKind {
        #[default]
        Linear,
        QuadraticInOut,
        ElasticOut,
        /// Cubic bezier curve with control points (x1, y1, x2, y2).
        /// Maps to CSS `cubic-bezier(x1, y1, x2, y2)`.
        CubicBezier(f32, f32, f32, f32),
        /// Preset: Fluent `curveDecelerateMax` = cubic-bezier(0.1, 0.9, 0.2, 1)
        DecelerateMax,
        /// Preset: Fluent `curveDecelerateMin` = cubic-bezier(0.33, 0, 0.1, 1)
        DecelerateMin,
        /// Preset: Fluent `curveEasyEase` = cubic-bezier(0.33, 0, 0.67, 1)
        EasyEase,
    }

    /// Find `t` such that `cubic_bezier(t, x1, x2) == target_x` using Newton-Raphson.
    fn cubic_root(x1: f32, x2: f32, target_x: f32) -> f32 {
        const EPSILON: f32 = 1.0 / 512.0;
        const MAX_ITER: u32 = 8;
        let mut t = target_x; // initial guess
        for _ in 0..MAX_ITER {
            let x = cubic_bezier_sample(t, x1, x2);
            let dx = 3.0 * (1.0 - t).powi(2) * x1
                + 3.0 * (1.0 - t) * t * (x2 - x1) * 2.0
                + 3.0 * t.powi(2) * (1.0 - x2);
            if dx.abs() < EPSILON {
                break;
            }
            t -= (x - target_x) / dx;
            t = t.clamp(0.0, 1.0);
        }
        t.clamp(0.0, 1.0)
    }

    /// Evaluate cubic bezier at `t` for X or Y coordinate with control points (a, b).
    fn cubic_bezier_sample(t: f32, a: f32, b: f32) -> f32 {
        3.0 * (1.0 - t).powi(2) * t * a + 3.0 * (1.0 - t) * t.powi(2) * b + t.powi(3)
    }

    impl EaseKind {
        #[must_use]
        pub fn sample(self, t: f32) -> f32 {
            let t = t.clamp(0.0, 1.0);
            match self {
                Self::Linear => t,
                Self::QuadraticInOut => {
                    if t < 0.5 {
                        2.0 * t * t
                    } else {
                        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                    }
                }
                Self::ElasticOut => {
                    if t <= 0.0 {
                        0.0
                    } else if t >= 1.0 {
                        1.0
                    } else {
                        let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                        2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                    }
                }
                Self::CubicBezier(x1, y1, x2, y2) => {
                    let t_param = cubic_root(x1, x2, t);
                    cubic_bezier_sample(t_param, y1, y2)
                }
                Self::DecelerateMax => {
                    let t_param = cubic_root(0.1, 0.2, t);
                    cubic_bezier_sample(t_param, 0.9, 1.0)
                }
                Self::DecelerateMin => {
                    let t_param = cubic_root(0.33, 0.1, t);
                    cubic_bezier_sample(t_param, 0.0, 1.0)
                }
                Self::EasyEase => {
                    let t_param = cubic_root(0.33, 0.67, t);
                    cubic_bezier_sample(t_param, 0.0, 1.0)
                }
            }
        }
    }
}

pub mod interpolate {
    pub trait Interpolator: Send + Sync + 'static {
        type Item: Send + Sync + 'static;

        fn interpolate(&self, target: &mut Self::Item, ratio: f32, previous_value: f32);
    }
}

pub mod bevy_time_runner {
    use super::*;

    #[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TimeSpan {
        pub start: Duration,
        pub end: Duration,
    }

    impl TryFrom<std::ops::Range<Duration>> for TimeSpan {
        type Error = &'static str;

        fn try_from(range: std::ops::Range<Duration>) -> Result<Self, Self::Error> {
            if range.end < range.start {
                return Err("time span end must be greater than or equal to start");
            }

            Ok(Self {
                start: range.start,
                end: range.end,
            })
        }
    }

    impl TimeSpan {
        #[must_use]
        pub fn duration(&self) -> Duration {
            self.end.saturating_sub(self.start)
        }
    }

    #[derive(Component, Debug, Clone, Copy, PartialEq)]
    pub struct TimeRunner {
        elapsed: Duration,
        duration: Duration,
        finished: bool,
    }

    impl TimeRunner {
        #[must_use]
        pub const fn new(duration: Duration) -> Self {
            Self {
                elapsed: Duration::ZERO,
                duration,
                finished: false,
            }
        }

        pub fn advance(&mut self, delta: Duration) -> f32 {
            if self.finished {
                return 1.0;
            }

            self.elapsed = self.elapsed.saturating_add(delta);
            let duration_secs = self.duration.as_secs_f32();
            let ratio = if duration_secs <= f32::EPSILON {
                1.0
            } else {
                self.elapsed.as_secs_f32() / duration_secs
            }
            .clamp(0.0, 1.0);

            self.finished = ratio >= 1.0;
            ratio
        }

        #[must_use]
        pub const fn is_finished(&self) -> bool {
            self.finished
        }
    }

    #[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TimeContext<T: Send + Sync + 'static = ()>(PhantomData<T>);

    impl<T: Send + Sync + 'static> Default for TimeContext<T> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }
}

pub mod component_tween {
    use super::*;

    #[derive(Component, Debug, Clone)]
    pub struct ComponentTween<I: interpolate::Interpolator> {
        pub target: Entity,
        pub interpolator: I,
    }

    impl<I: interpolate::Interpolator> ComponentTween<I> {
        #[must_use]
        pub const fn new_target(target: Entity, interpolator: I) -> Self {
            Self {
                target,
                interpolator,
            }
        }
    }

    #[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
    pub struct TweenInterpolationValue(pub f32);

    #[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
    pub struct TweenPreviousValue(pub f32);
}

pub use bevy_time_runner::{TimeContext, TimeRunner, TimeSpan};
pub use component_tween::{ComponentTween, TweenInterpolationValue, TweenPreviousValue};
pub use interpolate::Interpolator;
pub use interpolation::EaseKind;

pub mod tween {
    pub use super::component_tween::*;
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TweenSystemSet {
    UpdateInterpolationValue,
    ApplyTween,
}

pub struct TweenCorePlugin<T: Send + Sync + 'static = ()>(PhantomData<T>);

impl<T: Send + Sync + 'static> Default for TweenCorePlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Send + Sync + 'static> Plugin for TweenCorePlugin<T> {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            bevy_app::Update,
            (
                TweenSystemSet::UpdateInterpolationValue,
                TweenSystemSet::ApplyTween.after(TweenSystemSet::UpdateInterpolationValue),
            ),
        );
    }
}

#[derive(Default)]
pub struct DefaultTweenPlugins<T: Send + Sync + 'static = ()> {
    _marker: PhantomData<T>,
}

impl<T: Send + Sync + 'static> DefaultTweenPlugins<T> {
    #[must_use]
    pub fn in_schedule<S: ScheduleLabel>(_schedule: S) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: Send + Sync + 'static> Plugin for DefaultTweenPlugins<T> {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TweenCorePlugin<T>>() {
            app.add_plugins(TweenCorePlugin::<T>::default());
        }
    }
}

pub trait BevyTweenRegisterSystems {
    fn add_tween_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self;
}

impl BevyTweenRegisterSystems for App {
    fn add_tween_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.add_systems(schedule, systems.in_set(TweenSystemSet::ApplyTween))
    }
}

pub fn component_tween_system<I>() -> impl FnMut(&mut World) + Send + Sync + 'static
where
    I: Interpolator + Clone,
    I::Item: Component<Mutability = Mutable>,
{
    move |world: &mut World| {
        let delta = world
            .get_resource::<Time>()
            .map(Time::delta)
            .unwrap_or(Duration::ZERO);

        let mut updates = Vec::new();
        {
            let mut query = world.query::<(
                Entity,
                &mut TimeRunner,
                Option<&TimeSpan>,
                Option<&EaseKind>,
                &ComponentTween<I>,
                Option<&TweenPreviousValue>,
            )>();

            for (runner_entity, mut runner, span, ease, tween, previous) in query.iter_mut(world) {
                let raw_ratio = runner.advance(delta);
                let span_ratio = span.map_or(raw_ratio, |span| {
                    let span_duration_secs = span.duration().as_secs_f32();
                    if span_duration_secs <= f32::EPSILON {
                        1.0
                    } else {
                        raw_ratio.clamp(0.0, 1.0)
                    }
                });
                let eased_ratio = ease.copied().unwrap_or_default().sample(span_ratio);
                updates.push((
                    runner_entity,
                    tween.target,
                    tween.interpolator.clone(),
                    eased_ratio,
                    previous.map_or(0.0, |previous| previous.0),
                    runner.is_finished(),
                ));
            }
        }

        for (runner_entity, target, interpolator, ratio, previous, finished) in updates {
            if let Ok(mut target_entity) = world.get_entity_mut(target)
                && let Some(mut item) = target_entity.get_mut::<I::Item>()
            {
                interpolator.interpolate(&mut item, ratio, previous);
            }

            if let Ok(mut runner_entity) = world.get_entity_mut(runner_entity) {
                if finished {
                    runner_entity.remove::<(
                        TimeRunner,
                        TimeSpan,
                        EaseKind,
                        ComponentTween<I>,
                        TimeContext<()>,
                        TweenInterpolationValue,
                        TweenPreviousValue,
                    )>();
                } else {
                    runner_entity
                        .insert((TweenInterpolationValue(ratio), TweenPreviousValue(ratio)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // -----------------------------------------------------------------------
    // EaseKind::sample
    // -----------------------------------------------------------------------

    #[test]
    fn ease_linear_is_identity() {
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!(
                (EaseKind::Linear.sample(t) - t).abs() < 0.001,
                "linear sample({t})"
            );
        }
    }

    #[test]
    fn ease_quadratic_in_out_symmetry() {
        let mid = EaseKind::QuadraticInOut.sample(0.5);
        assert!((mid - 0.5).abs() < 0.001);
        let low = EaseKind::QuadraticInOut.sample(0.25);
        let high = EaseKind::QuadraticInOut.sample(0.75);
        assert!((low - (1.0 - high)).abs() < 0.001);
    }

    #[test]
    fn ease_elastic_out_bounds() {
        let ease = EaseKind::ElasticOut;
        assert!((ease.sample(0.0)).abs() < f32::EPSILON);
        assert!((ease.sample(1.0) - 1.0).abs() < f32::EPSILON);
        // Elastic overshoots above 1.0 in the middle
        assert!(ease.sample(0.5) >= 0.0);
    }

    #[test]
    fn ease_cubic_bezier_matches_linear_at_default() {
        // A zero-curve bezier: (0,0,1,1) should be linear
        let ease = EaseKind::CubicBezier(0.0, 0.0, 1.0, 1.0);
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((ease.sample(t) - t).abs() < 0.02, "cubic sample({t})");
        }
    }

    #[test]
    fn ease_decelerate_max_smooth() {
        let ease = EaseKind::DecelerateMax;
        assert!((ease.sample(0.0)).abs() < 0.01);
        assert!((ease.sample(1.0) - 1.0).abs() < 0.01);
        // Should be concave (decelerating) — first half faster than linear
        assert!(ease.sample(0.5) > 0.5);
    }

    #[test]
    fn ease_decelerate_min_smooth() {
        let ease = EaseKind::DecelerateMin;
        assert!((ease.sample(0.0)).abs() < 0.01);
        assert!((ease.sample(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn ease_easy_ease_smooth() {
        let ease = EaseKind::EasyEase;
        assert!((ease.sample(0.0)).abs() < 0.01);
        assert!((ease.sample(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn ease_clamps_input_to_unit() {
        assert!((EaseKind::Linear.sample(-0.5)).abs() < f32::EPSILON);
        assert!((EaseKind::Linear.sample(1.5) - 1.0).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // TimeSpan
    // -----------------------------------------------------------------------

    #[test]
    fn time_span_try_from_valid() {
        let span = TimeSpan::try_from(Duration::ZERO..Duration::from_secs(5)).unwrap();
        assert_eq!(span.duration(), Duration::from_secs(5));
    }

    #[test]
    fn time_span_try_from_zero_duration() {
        let span = TimeSpan::try_from(Duration::from_secs(3)..Duration::from_secs(3)).unwrap();
        assert_eq!(span.duration(), Duration::ZERO);
    }

    #[test]
    fn time_span_try_from_invalid_reversed() {
        let result = TimeSpan::try_from(Duration::from_secs(5)..Duration::from_secs(2));
        assert!(result.is_err());
    }

    #[test]
    fn time_span_duration_saturating() {
        let span = TimeSpan {
            start: Duration::from_secs(10),
            end: Duration::from_secs(3),
        };
        // saturating_sub returns zero when start > end
        assert_eq!(span.duration(), Duration::ZERO);
    }

    // -----------------------------------------------------------------------
    // TimeRunner
    // -----------------------------------------------------------------------

    #[test]
    fn time_runner_initial_state() {
        let mut runner = TimeRunner::new(Duration::from_secs(1));
        assert!(!runner.is_finished());
        assert!((runner.advance(Duration::ZERO)).abs() < f32::EPSILON);
    }

    #[test]
    fn time_runner_halfway() {
        let mut runner = TimeRunner::new(Duration::from_secs(2));
        let ratio = runner.advance(Duration::from_secs(1));
        assert!((ratio - 0.5).abs() < 0.001);
        assert!(!runner.is_finished());
    }

    #[test]
    fn time_runner_completes() {
        let mut runner = TimeRunner::new(Duration::from_secs(1));
        let ratio = runner.advance(Duration::from_secs(2));
        assert!((ratio - 1.0).abs() < f32::EPSILON);
        assert!(runner.is_finished());
    }

    #[test]
    fn time_runner_returns_one_after_finish() {
        let mut runner = TimeRunner::new(Duration::from_secs(1));
        let _ = runner.advance(Duration::from_secs(2));
        assert!(runner.is_finished());
        let ratio = runner.advance(Duration::from_secs(1));
        assert!((ratio - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn time_runner_zero_duration_immediate() {
        let mut runner = TimeRunner::new(Duration::ZERO);
        let ratio = runner.advance(Duration::ZERO);
        assert!((ratio - 1.0).abs() < f32::EPSILON);
        assert!(runner.is_finished());
    }

    #[test]
    fn time_runner_advance_saturating_add() {
        let mut runner = TimeRunner::new(Duration::from_secs(1));
        let ratio = runner.advance(Duration::MAX);
        assert!((ratio - 1.0).abs() < f32::EPSILON);
        assert!(runner.is_finished());
    }

    // -----------------------------------------------------------------------
    // ComponentTween
    // -----------------------------------------------------------------------

    #[test]
    fn component_tween_new_target() {
        let entity = Entity::from_bits(42);
        let interpolator = SimpleInterpolator;
        let tween = ComponentTween::new_target(entity, interpolator);
        assert_eq!(tween.target, entity);
    }

    // Simple interpolator for test use
    #[derive(Clone)]
    struct SimpleInterpolator;

    impl Interpolator for SimpleInterpolator {
        type Item = f32;

        fn interpolate(&self, target: &mut f32, ratio: f32, _previous_value: f32) {
            *target = ratio;
        }
    }

    #[test]
    fn interpolation_value_component() {
        let val = TweenInterpolationValue(0.42);
        assert!((val.0 - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn tween_previous_value_component() {
        let val = TweenPreviousValue(0.5);
        assert!((val.0 - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn ease_kind_default_is_linear() {
        assert_eq!(EaseKind::default(), EaseKind::Linear);
    }

    // -----------------------------------------------------------------------
    // TimeSpan + TimeRunner integration
    // -----------------------------------------------------------------------

    #[test]
    fn time_span_affects_ratio() {
        // TimeSpan should make the runner advance at full speed within the span
        let mut runner = TimeRunner::new(Duration::from_secs(10));
        let span = TimeSpan {
            start: Duration::from_secs(2),
            end: Duration::from_secs(4),
        };
        // After 2s of wall time, the span progress is 0.5 (2s / 4s duration)
        let raw = runner.advance(Duration::from_secs(2));
        let span_dur = span.duration().as_secs_f32();
        let span_ratio = raw.clamp(0.0, 1.0);
        assert!((span_ratio - 0.2).abs() < 0.001); // 2s / 10s = 0.2
        assert_eq!(span_dur, 2.0);
    }
}
