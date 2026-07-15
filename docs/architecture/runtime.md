# Runtime architecture

Picus keeps Bevy as the application scheduler and owns one retained runtime per
window. `MasonryRuntime` is a non-send resource containing the window runtimes;
each `WindowRuntime` owns the retained view state, action sink, hit-test state,
and paint bookkeeping for one Bevy window. The primary window is attached
automatically. Additional windows attach when their `Window` entity is seen.

The normal frame flow is:

```text
PreUpdate   input, retained routing, action dispatch
Update      application systems and state changes
PostUpdate  projection invalidation, synthesis, retained rebuild, IME sync
Last        paint and present for each attached window
```

Paint errors are captured as diagnostics. A frame is marked painted only after
`present()` succeeds, so a failed presentation cannot make later lifecycle
logic assume that pixels reached the window. Window size sent to the retained
runtime is logical size; pointer hit testing uses the event window's physical
cursor position.

Fonts registered through `AppPicusExt` are queued in `XilemFontBridge` and
broadcast to every attached window. A window that attaches later replays all
already registered font bytes. Theme backdrops are explicit: the application
can select one with `theme_backdrop`, while an explicit window setting takes
precedence over a stylesheet backdrop.

See [multi-window](../guide/multi-window.md), [i18n and fonts](../guide/i18n-fonts-icons.md),
and [styling and themes](../guide/styling-themes.md) for application-facing
configuration.

### 四条时间线 (four timelines)

The long-term frame architecture separates four independent timelines so that
animation clock, scene build, and present freshness are no longer one OR-coupled
path. Full plan: [plans/frame-pipeline.md](../plans/frame-pipeline.md).

| Timeline | Role | Trigger | Drop policy |
|----------|------|---------|-------------|
| **A Input/Shell** | Pointer, keyboard, move/resize message pump | Events | Do not drop messages |
| **B Anim clock** | Advance `t`, opacity, cursor blink timers | Logical clock (may be 60–120 Hz) | State may jump |
| **C Scene build** | Rewrite + build/encode scene (today: full window; target: painter-order entries) | Only when corresponding content changes | Uncommitted work may merge |
| **D Present** | Submit the latest ready composite | Display path | Mailbox drops stale; FIFO may only backpressure |

**Today (Phase 0)** these timelines still share `WindowRuntime::paint_frame`: anim
tick, full-window redraw/encode, and present remain on one path. Continuous
widgets (e.g. Spinner) that request every anim tick can still force full-window
encode+present. A **transitional** pure-animation present throttle (~30 Hz
default; override `PICUS_ANIM_PRESENT_HZ` with a positive Hz, or
`0` / `off` / `none` / `false` to disable) reduces DWM drag ghosting without
changing content/resize present rates. That throttle is **not** the end state;
it is removed only after layered anim encode gates pass (G10).

**Observability:** set `PICUS_FRAME_TIMING=1` for per-window phase averages and a
monotonic `frame_id` (`input_dispatch_ms`, `anim_tick_ms`,
`scene_build_base_ms` / `scene_build_anim_ms`, `surface_acquire_ms`,
`encode_*_ms`, `composite_ms`, `present_submit_ms`, `presented` /
`anim_tick_only`). These are **CPU submit-path** times — not displayed-frame
latency.

**Phase instrumentation today (Phase 0 honesty):** `anim_tick_ms` includes
rewrite that Masonry performs inside `AnimFrame`. `scene_build_base_ms` is only
the subsequent root `redraw()` call. Present-path averages (`encode_*`,
`composite`, `present_submit`, …) are over **content paint attempts**
(`frames − anim_tick_only`), not diluted by throttled anim-only zeros. Process
log `frames` counts **per-window paint attempts**; ECS averages use `bevy_frames`.
Idle pure-`Skipped` paint does not assign `frame_id`s but still flushes process
summaries on a ~1s wall clock.

Windows baselines require PresentMon/ETW; protocol and result template:
[perf/frame-pipeline-baseline.md](../perf/frame-pipeline-baseline.md).

### Frame pipeline evolution

Implementation plan and success metrics G1–G10:
[plans/frame-pipeline.md](../plans/frame-pipeline.md).
