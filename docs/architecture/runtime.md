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

**Today (Phase 1)** each window’s paint path runs through an internal
`FrameDriver` (`picus_core::runtime::frame_driver`, not on the app facade).
`paint_masonry_ui` → `WindowRuntime::step_frame`, which uses
`FrameDriver::decide_entry` / `decide_present` (there is no `FrameDriver::step`).
`DirtyBudget` aggregates `FirstPaint`, `InputOrRebuild`, `LayoutRewrite`,
`ResizeMetrics`, `AnimPaint { layer }`, `AnimTick`, `CompositorPlan`,
`ThemeOrFont`, `RetrySurface`. Decision flags record intent; **Phase 1
execution only splits anim-tick vs full-window encode/present** —
`do_rewrite` / `do_encode` / `do_present` stay coupled on the content path until
layered textures (Phase 2). Sticky content dirt (`resize_dirty`, `retry_dirty`,
`theme_or_font_dirty`, …) is cleared **only after successful present**.

**Hard rule (G5):** `ResizeMetrics`, `InputOrRebuild`, `FirstPaint`, and
`RetrySurface` are **never** skipped by the anim present throttle. Continuous
widgets (e.g. Spinner) may still full-window encode this phase; scheduling
semantics are correct so interaction/resize redraws are not blocked.

A **transitional** pure-animation present throttle (~30 Hz default; override
`PICUS_ANIM_PRESENT_HZ` with a positive Hz, or `0` / `off` / `none` / `false` to
disable) reduces DWM drag ghosting. That throttle is **not** the end state; it is
removed only after layered anim encode gates pass (G10). Anim tick and present
are **not** inseparably tied: pure `AnimTick` may skip encode/present while
keeping the event loop awake. Non-G5 content co-occurring with the anim clock
(e.g. `LayoutRewrite` + `AnimTick`) may also be delayed by the interval.

**PresentPolicy (G7):** surface creation negotiates an explicit capability —
`MailboxLatest` (GPU/compositor may replace queued frames) or
`FifoBackpressure` (name covers FIFO / FifoRelaxed / AutoVsync / Immediate
fallbacks that are *not* Mailbox; does not imply every non-mailbox mode is true
FIFO queueing). `LatestReadyQueue` is a **helper** for CPU-side latest-only
coalescing of *unsubmitted* frames (unit-tested; **not yet on the hot present
path** — present remains single in-flight submit). Submitted FIFO frames are
**not** claimed withdrawable. There is no fake unified `drop_stale` boolean
across modes. Runtime logs mode + strategy at surface init. Shared helper:
`picus_surface::select_present_mode` / `PresentPolicy::negotiate`.

**Observability:** set `PICUS_FRAME_TIMING=1` for per-window phase averages and a
monotonic `frame_id` (`input_dispatch_ms`, `anim_tick_ms`,
`scene_build_base_ms` / `scene_build_anim_ms`, `surface_acquire_ms`,
`encode_*_ms`, `composite_ms`, `present_submit_ms`, `presented` /
`anim_tick_only`). These are **CPU submit-path** times — not displayed-frame
latency.

**Phase instrumentation honesty:** `anim_tick_ms` includes rewrite that Masonry
performs inside `AnimFrame`. `scene_build_base_ms` is only the subsequent root
`redraw()` call. Present-path averages (`encode_*`, `composite`,
`present_submit`, …) and process `paint_ms` / `present_ms` are over **content
paint attempts** (`frames − anim_tick_only`), not diluted by throttled
anim-only zeros. Process log `frames` counts **per-window paint attempts**; ECS
averages use `bevy_frames`. Idle pure-`Skipped` paint does not assign `frame_id`s
but still flushes process summaries on a ~1s wall clock.

Windows baselines require PresentMon/ETW; protocol and result template:
[perf/frame-pipeline-baseline.md](../perf/frame-pipeline-baseline.md).

### Frame pipeline evolution

Implementation plan and success metrics G1–G10:
[plans/frame-pipeline.md](../plans/frame-pipeline.md).
