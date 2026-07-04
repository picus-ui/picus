//! picuscode: a Codex-desktop-style example verifying the three P0 features.
//!
//! - **P0-1 multi-window**: a primary chat window plus a secondary "About"
//!   window, both driven by `MasonryRuntime`. The secondary window's `UiRoot`
//!   carries a `UiWindow` binding so it synthesizes into its own window.
//! - **P0-2 Markdown**: a static `UiMarkdown` welcome card renders headings,
//!   lists, inline emphasis, and a syntax-highlighted Rust code block.
//! - **P0-3 streaming markdown**: a simulated CodeWhale reply appends
//!   `ResponseDelta` events into a `UiStreamingMarkdown` each frame, flushing
//!   completed paragraphs into the cached prefix so only the in-progress tail is
//!   re-parsed.

use std::sync::Arc;

use bevy_window::Window;
use picus_core::{
    AppPicusExt, PicusPlugin, ProjectionCtx, StyleClass, UiEventQueue, UiMarkdown, UiRoot,
    UiStreamingMarkdown, UiView, UiWindow, WorldSceneExt, apply_widget_style,
    bevy_app::{App, PostStartup, PreUpdate, Startup},
    bevy_ecs::{
        entity::Entity,
        hierarchy::ChildOf,
        prelude::*,
    },
    button, emit_ui_action, text_input,
    masonry_core::layout::{Dim, Length},
    resolve_style,
    scene::{CommandsSceneExt, bsn},
    xilem::{
        InsertNewline,
        style::Style as _,
        view::{FlexExt as _, flex_col, flex_row, label, sized_box},
        winit::{error::EventLoopError},
    },
};
use shared_utils::init_logging;

/// Actions emitted by picuscode UI controls.
///
/// Button helpers emit the bare action variant, so each control maps to one
/// distinct variant here.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PicusCodeAction {
    Send,
    ComposerChanged(String),
    OpenAbout,
    CloseAbout,
}

/// Small local mirror of the CodeWhale app-server thread request boundary.
///
/// CodeWhale exposes `ThreadRequest::Message { thread_id, input }`; keeping
/// the example's mock turn client at this shape lets the future integration
/// replace only the transport layer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CodeWhaleThreadRequest {
    Message { thread_id: String, input: String },
}

/// Small local mirror of the CodeWhale streaming frames this example consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CodeWhaleEventFrame {
    /// Maps to CodeWhale `EventFrame::ResponseStart`.
    Start { response_id: String },
    /// Maps to CodeWhale `EventFrame::ResponseDelta`.
    Delta { response_id: String, delta: String },
    /// Maps to CodeWhale `EventFrame::ResponseEnd`.
    End { response_id: String },
}

/// One in-flight mock turn, using the same lifecycle as CodeWhale streaming.
#[derive(Debug)]
struct CodeWhaleMockTurn {
    response_id: String,
    tokens: Vec<String>,
    next_token: usize,
    started: bool,
}

impl CodeWhaleMockTurn {
    fn new(request: CodeWhaleThreadRequest, turn_index: u64) -> Self {
        let CodeWhaleThreadRequest::Message { thread_id, input } = request;
        Self {
            response_id: format!("{thread_id}:mock-turn-{turn_index}"),
            tokens: simulated_reply_tokens(&input),
            next_token: 0,
            started: false,
        }
    }

    fn next_frame(&mut self) -> Option<CodeWhaleEventFrame> {
        if !self.started {
            self.started = true;
            return Some(CodeWhaleEventFrame::Start {
                response_id: self.response_id.clone(),
            });
        }

        if let Some(delta) = self.tokens.get(self.next_token).cloned() {
            self.next_token += 1;
            return Some(CodeWhaleEventFrame::Delta {
                response_id: self.response_id.clone(),
                delta,
            });
        }

        Some(CodeWhaleEventFrame::End {
            response_id: self.response_id.clone(),
        })
    }
}

/// Per-session chat state stored as a resource.
#[derive(Resource, Debug)]
struct ChatState {
    /// Stable demo thread id matching CodeWhale's thread message API.
    thread_id: String,
    /// Incrementing mock turn counter used to build CodeWhale-like response ids.
    next_turn_index: u64,
    /// The mock CodeWhale turn currently producing streaming frames.
    current_turn: Option<CodeWhaleMockTurn>,
    /// Current CodeWhale response id, when a stream is active.
    active_response_id: Option<String>,
    /// Whether a streaming turn is currently active.
    streaming: bool,
    /// Current composer draft text.
    draft: String,
    /// Whether the About window is open.
    about_open: bool,
    /// Entity of the About window (when open).
    about_window: Option<Entity>,
    /// Entity of the About `UiRoot` (when open).
    about_root: Option<Entity>,
    /// Entity of the transcript column (for appending new messages).
    transcript_column: Entity,
    /// Entity of the current turn's `UiStreamingMarkdown` component holder.
    streaming_entity: Entity,
}

/// Marker for the root of the primary chat window.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ChatRootView;

/// Marker for the title bar row.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ChatTitleBarView;

/// Marker for the transcript column (messages stack here).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TranscriptColumnView;

/// Marker for the composer row.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ComposerView;

/// Marker for the status line.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StatusLineView;

/// Marker for the secondary About window root.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AboutRootView;

/// A static welcome markdown blob shown at the top of the transcript.
const WELCOME_MARKDOWN: &str = "\
# picuscode

A minimal **Codex-desktop**-style demo verifying the three P0 features:

- Multi-window runtime (primary chat + secondary About window)
- Markdown rendering with syntax highlighting
- Streaming markdown for live LLM-style output

Try typing a message below and pressing **Send** to watch a simulated \
streaming reply render token-by-token.

```rust
fn main() {
    let greeting = \"Hello from picus!\";
    println!(\"{greeting}\");
}
```
";

/// A canned multi-token assistant reply used to simulate CodeWhale streaming.
fn simulated_reply_tokens(input: &str) -> Vec<String> {
    let reply = format!(
        "\
Sure — treating your prompt as a CodeWhale `ThreadRequest::Message` input:

> {input}

Here is how the three P0 pieces fit together:

1. The *multi-window runtime* keys a `WindowRuntime` per Bevy window entity,
   so each OS window owns its own `RenderRoot` and view tree.
2. **Markdown** rendering parses with `pulldown-cmark` and highlights fenced
   code blocks via `syntect`.
3. **Streaming markdown** caches the completed prefix and only re-parses the
   in-progress tail each frame - that keeps per-frame cost flat as the reply
   grows.

```rust
// Streaming append is just:
streaming.append(token);
streaming.flush_completed();
```

This mock emits `ResponseStart`, `ResponseDelta`, and `ResponseEnd`, matching
the CodeWhale app-server event names the real adapter will consume.

> Tip: open the About window from the title bar to see a second OS window.
"
    );
    reply
        .split_whitespace()
        .map(|w| format!("{w} "))
        .collect()
}

fn setup_chat_world(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        UiRoot
        ChatRootView
        StyleClass(vec!["picuscode.root".to_string()])
        Children [
            (
                ChatTitleBarView
                StyleClass(vec!["picuscode.titlebar".to_string()])
            ),
            (
                TranscriptColumnView
                StyleClass(vec!["picuscode.transcript".to_string()])
            ),
            (
                ComposerView
                StyleClass(vec!["picuscode.composer".to_string()])
            ),
            (
                StatusLineView
                StyleClass(vec!["picuscode.status".to_string()])
            ),
        ]
    });
}

fn spawn_about_window(world: &mut World) -> (Entity, Entity) {
    let window = Window {
        title: "About picuscode".to_string(),
        resolution: bevy_window::WindowResolution::new(480, 360),
        ..Default::default()
    };
    let window_entity = world.spawn(window).id();

    let about_entity = world
        .spawn_scene(bsn! {
            UiRoot
            UiWindow(window_entity)
            AboutRootView
            StyleClass(vec!["picuscode.about.root".to_string()])
            Children [
                (UiMarkdown { source: { WELCOME_MARKDOWN.to_string() } }),
            ]
        })
        .expect("about BSN scene should spawn")
        .id();

    (about_entity, window_entity)
}

picus_core::impl_ui_component_template!(ChatRootView, project_chat_root);
picus_core::impl_ui_component_template!(ChatTitleBarView, project_title_bar);
picus_core::impl_ui_component_template!(TranscriptColumnView, project_transcript_column);
picus_core::impl_ui_component_template!(ComposerView, project_composer);
picus_core::impl_ui_component_template!(StatusLineView, project_status_line);
picus_core::impl_ui_component_template!(AboutRootView, project_about_root);

fn project_chat_root(_: &ChatRootView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let children = ctx
        .children
        .into_iter()
        .map(|child| child.into_any_flex())
        .collect::<Vec<_>>();
    Arc::new(apply_widget_style(
        picus_core::xilem::view::flex_col(children)
            .width(Dim::Stretch)
            .height(Dim::Stretch)
            .gap(Length::px(style.layout.gap)),
        &style,
    ))
}

fn project_title_bar(_: &ChatTitleBarView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let title = label("picuscode").text_size(16.0);
    let about_btn = button(ctx.entity, PicusCodeAction::OpenAbout, "About");
    Arc::new(apply_widget_style(
        flex_row(vec![
            sized_box(title).flex(1.0).into_any_flex(),
            about_btn.into_any_flex(),
        ])
        .gap(Length::px(8.0)),
        &style,
    ))
}

fn project_transcript_column(_: &TranscriptColumnView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let children = ctx
        .children
        .into_iter()
        .map(|child| child.into_any_flex())
        .collect::<Vec<_>>();
    Arc::new(apply_widget_style(
        picus_core::xilem::view::sized_box(flex_col(children).gap(Length::px(12.0)))
            .width(Dim::Stretch)
            .height(Dim::Stretch),
        &style,
    ))
}

fn project_composer(_: &ComposerView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let draft = ctx
        .world
        .get_resource::<ChatState>()
        .map(|state| state.draft.clone())
        .unwrap_or_default();
    let input_entity = ctx.entity;
    let enter_entity = ctx.entity;
    let input = text_input(
        input_entity,
        draft,
        PicusCodeAction::ComposerChanged,
    )
    .placeholder("Message CodeWhale...")
    .insert_newline(InsertNewline::OnShiftEnter)
    .on_enter(move |_| {
        emit_ui_action(enter_entity, PicusCodeAction::Send);
    });
    let send_btn = button(ctx.entity, PicusCodeAction::Send, "Send");
    Arc::new(apply_widget_style(
        flex_row(vec![
            input.flex(1.0).into_any_flex(),
            send_btn.into_any_flex(),
        ])
        .gap(Length::px(8.0)),
        &style,
    ))
}

fn project_status_line(_: &StatusLineView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let text = ctx
        .world
        .get_resource::<ChatState>()
        .map(|state| {
            if state.streaming {
                match &state.active_response_id {
                    Some(response_id) => format!("Streaming {response_id}"),
                    None => "Starting CodeWhale turn".to_string(),
                }
            } else if state.about_open {
                "About window open".to_string()
            } else {
                "Ready".to_string()
            }
        })
        .unwrap_or_else(|| "Ready".to_string());
    Arc::new(apply_widget_style(label(text).text_size(12.0), &style))
}

fn project_about_root(_: &AboutRootView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let close_btn = button(ctx.entity, PicusCodeAction::CloseAbout, "Close");
    let children = ctx
        .children
        .into_iter()
        .map(|child| child.into_any_flex())
        .collect::<Vec<_>>();
    let mut all = children;
    all.push(close_btn.into_any_flex());
    Arc::new(apply_widget_style(
        flex_col(all)
            .width(Dim::Stretch)
            .height(Dim::Stretch)
            .gap(Length::px(12.0)),
        &style,
    ))
}

/// System: spawn the initial welcome markdown + streaming reply holder as
/// children of the transcript column, then seed `ChatState`.
fn seed_transcript(world: &mut World) {
    let transcript_column = {
        let mut query = world.query_filtered::<Entity, With<TranscriptColumnView>>();
        query
            .iter(world)
            .next()
            .expect("transcript column should exist after setup")
    };

    let welcome = world
        .spawn((
            UiMarkdown::new(WELCOME_MARKDOWN),
            ChildOf(transcript_column),
        ))
        .id();

    let _ = welcome;

    world.insert_resource(ChatState {
        thread_id: "picuscode-demo-thread".to_string(),
        next_turn_index: 1,
        current_turn: None,
        active_response_id: None,
        streaming: false,
        draft: String::new(),
        about_open: false,
        about_window: None,
        about_root: None,
        transcript_column,
        streaming_entity: Entity::PLACEHOLDER,
    });
}

/// System: drain UI actions, start streaming turns, and append simulated
/// tokens to the active streaming markdown each frame.
fn handle_picuscode_actions(world: &mut World) {
    let actions = world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<PicusCodeAction>();

    let mut to_send = false;
    let mut to_open_about = false;
    let mut to_close_about = false;
    let mut latest_draft = None;

    for event in actions {
        match event.action {
            PicusCodeAction::Send => to_send = true,
            PicusCodeAction::ComposerChanged(value) => latest_draft = Some(value),
            PicusCodeAction::OpenAbout => to_open_about = true,
            PicusCodeAction::CloseAbout => to_close_about = true,
        }
    }

    if let Some(draft) = latest_draft {
        world.resource_mut::<ChatState>().draft = draft;
    }

    if to_close_about {
        let close_targets = {
            let mut state = world.resource_mut::<ChatState>();
            let about_root = state.about_root.take();
            let about_window = state.about_window.take();
            if about_window.is_some() {
                state.about_open = false;
            }
            about_window.map(|window| (about_root, window))
        };

        if let Some((about_root, about_window)) = close_targets {
            if let Some(root) = about_root
                && let Ok(entity) = world.get_entity_mut(root)
            {
                entity.despawn();
            }
            if let Ok(entity) = world.get_entity_mut(about_window) {
                entity.despawn();
            }
        }
    }

    if to_open_about {
        let should_open = {
            let mut state = world.resource_mut::<ChatState>();
            if state.about_open {
                false
            } else {
                state.about_open = true;
                true
            }
        };

        if should_open {
            let (about_root, about_window) = spawn_about_window(world);
            let mut state = world.resource_mut::<ChatState>();
            state.about_root = Some(about_root);
            state.about_window = Some(about_window);
        }
    }

    if to_send {
        let (can_send, draft, thread_id, turn_index, transcript) = {
            let state = world.resource::<ChatState>();
            (
                !state.streaming && !state.draft.is_empty(),
                state.draft.clone(),
                state.thread_id.clone(),
                state.next_turn_index,
                state.transcript_column,
            )
        };
        if can_send {
            world.spawn((
                picus_core::UiMarkdown::new(format!("**You:** {draft}")),
                ChildOf(transcript),
            ));

            let streaming_entity = world
                .spawn((UiStreamingMarkdown::new(), ChildOf(transcript)))
                .id();

            {
                let mut state = world.resource_mut::<ChatState>();
                state.streaming = true;
                state.active_response_id = None;
                state.current_turn = Some(CodeWhaleMockTurn::new(
                    CodeWhaleThreadRequest::Message {
                        thread_id,
                        input: draft.clone(),
                    },
                    turn_index,
                ));
                state.next_turn_index += 1;
                state.draft.clear();
                state.streaming_entity = streaming_entity;
            }
        }
    }

    stream_next_token(world);
}

fn streaming_entity_id(world: &World) -> Entity {
    world
        .resource::<ChatState>()
        .streaming_entity
}

fn stream_next_token(world: &mut World) {
    let should_stream = world.resource::<ChatState>().streaming;
    if !should_stream {
        return;
    }

    let entity = streaming_entity_id(world);

    let frame = {
        let mut state = world.resource_mut::<ChatState>();
        state
            .current_turn
            .as_mut()
            .and_then(CodeWhaleMockTurn::next_frame)
    };

    let Some(frame) = frame else {
        return;
    };

    match frame {
        CodeWhaleEventFrame::Start { response_id } => {
            world.resource_mut::<ChatState>().active_response_id = Some(response_id);
        }
        CodeWhaleEventFrame::Delta { response_id, delta } => {
            let accepts_delta = world
                .resource::<ChatState>()
                .active_response_id
                .as_deref()
                .is_some_and(|active| active == response_id);
            if accepts_delta
                && let Some(mut streaming) = world.get_mut::<UiStreamingMarkdown>(entity)
            {
                let should_flush = delta.ends_with("\n\n")
                    || delta.ends_with("``` ")
                    || delta.ends_with(": ");
                streaming.append(&delta);
                if should_flush {
                    streaming.flush_completed();
                }
            }
        }
        CodeWhaleEventFrame::End { response_id } => {
            let should_finish = {
                let mut state = world.resource_mut::<ChatState>();
                if state.active_response_id.as_deref() == Some(response_id.as_str()) {
                    state.streaming = false;
                    state.current_turn = None;
                    state.active_response_id = None;
                    true
                } else {
                    false
                }
            };

            if should_finish
                && let Some(mut streaming) = world.get_mut::<UiStreamingMarkdown>(entity)
            {
                streaming.finish();
            }
        }
    }
}

fn build_picuscode_app() -> App {
    init_logging();

    let mut app = App::new();
    app.add_plugins(PicusPlugin)
        .load_style_sheet_ron(include_str!("../assets/themes/picuscode.ron"))
        .register_ui_component::<ChatRootView>()
        .register_ui_component::<ChatTitleBarView>()
        .register_ui_component::<TranscriptColumnView>()
        .register_ui_component::<ComposerView>()
        .register_ui_component::<StatusLineView>()
        .register_ui_component::<AboutRootView>()
        .add_systems(Startup, setup_chat_world)
        .add_systems(PostStartup, seed_transcript)
        .add_systems(PreUpdate, handle_picuscode_actions);

    app
}

fn main() -> Result<(), EventLoopError> {
    picus_core::run_app_with_window_options(
        build_picuscode_app(),
        "picuscode",
        |options| {
            options.with_initial_inner_size(picus_core::xilem::winit::dpi::LogicalSize::new(
                720.0,
                640.0,
            ))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_picuscode_theme_ron_parses() {
        picus_core::parse_stylesheet_ron(include_str!("../assets/themes/picuscode.ron"))
            .expect("embedded picuscode stylesheet should parse");
    }

    #[test]
    fn mock_turn_uses_codewhale_streaming_lifecycle() {
        let mut turn = CodeWhaleMockTurn::new(
            CodeWhaleThreadRequest::Message {
                thread_id: "thread-a".to_string(),
                input: "hello".to_string(),
            },
            7,
        );

        assert_eq!(
            turn.next_frame(),
            Some(CodeWhaleEventFrame::Start {
                response_id: "thread-a:mock-turn-7".to_string(),
            })
        );

        let Some(CodeWhaleEventFrame::Delta { response_id, delta }) = turn.next_frame() else {
            panic!("mock turn should emit at least one response delta");
        };
        assert_eq!(response_id, "thread-a:mock-turn-7");
        assert!(!delta.is_empty());

        while let Some(frame) = turn.next_frame() {
            if frame
                == (CodeWhaleEventFrame::End {
                    response_id: "thread-a:mock-turn-7".to_string(),
                })
            {
                return;
            }
        }

        panic!("mock turn should finish with ResponseEnd");
    }

    #[test]
    fn stream_next_token_finishes_active_turn() {
        let mut world = World::new();
        world.insert_resource(UiEventQueue::default());
        let transcript = world.spawn_empty().id();
        let streaming_entity = world
            .spawn((UiStreamingMarkdown::new(), ChildOf(transcript)))
            .id();
        world.insert_resource(ChatState {
            thread_id: "thread-a".to_string(),
            next_turn_index: 2,
            current_turn: Some(CodeWhaleMockTurn::new(
                CodeWhaleThreadRequest::Message {
                    thread_id: "thread-a".to_string(),
                    input: "hello".to_string(),
                },
                1,
            )),
            active_response_id: None,
            streaming: true,
            draft: String::new(),
            about_open: false,
            about_window: None,
            about_root: None,
            transcript_column: transcript,
            streaming_entity,
        });

        for _ in 0..512 {
            stream_next_token(&mut world);
            if !world.resource::<ChatState>().streaming {
                break;
            }
        }

        let state = world.resource::<ChatState>();
        assert!(!state.streaming);
        assert!(state.current_turn.is_none());
        assert!(state.active_response_id.is_none());
    }

    #[test]
    fn send_click_action_starts_codewhale_turn() {
        let mut world = World::new();
        world.insert_resource(UiEventQueue::default());
        let composer = world.spawn_empty().id();
        let transcript = world.spawn_empty().id();
        world.insert_resource(ChatState {
            thread_id: "thread-a".to_string(),
            next_turn_index: 1,
            current_turn: None,
            active_response_id: None,
            streaming: false,
            draft: "hello from click".to_string(),
            about_open: false,
            about_window: None,
            about_root: None,
            transcript_column: transcript,
            streaming_entity: Entity::PLACEHOLDER,
        });

        world
            .resource::<UiEventQueue>()
            .push_typed(composer, PicusCodeAction::Send);

        handle_picuscode_actions(&mut world);

        let state = world.resource::<ChatState>();
        assert!(state.streaming);
        assert_eq!(state.draft, "");
        assert_eq!(state.next_turn_index, 2);
        assert!(state.current_turn.is_some());
        assert!(state.active_response_id.is_some());
        assert_ne!(state.streaming_entity, Entity::PLACEHOLDER);
        assert!(world.get::<UiStreamingMarkdown>(state.streaming_entity).is_some());
    }
}
