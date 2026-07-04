//! Projection functions for picuscode view markers.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use picus::{
    ProjectionCtx, UiView, apply_label_style, apply_widget_style,
    bevy_ecs::hierarchy::Children,
    button_with_child, emit_ui_action, icon::icon, icons::PicusIcon, text_input,
    masonry_core::{
        layout::{Dim, Length},
        properties::Dimensions,
    },
    resolve_style, resolve_style_for_classes,
    xilem::{
        Color,
        InsertNewline,
        style::Style as _,
        view::{
            CrossAxisAlignment, FlexExt as _, MainAxisAlignment, flex_col, flex_item, flex_row,
            label, sized_box,
        },
    },
};

use crate::action::PicusCodeAction;
use crate::state::*;

pub fn project_chat_root(_: &ChatRootView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let children = child_entities(&ctx)
        .into_iter()
        .zip(ctx.children)
        .map(|(entity, child)| {
            let grow = resolve_style(ctx.world, entity).layout.flex_grow;
            if grow > 0.0 {
                flex_item(child, grow).into_any_flex()
            } else {
                child.into_any_flex()
            }
        })
        .collect::<Vec<_>>();
    Arc::new(
        sized_box(apply_widget_style(
            flex_col(children)
                .width(Dim::Stretch)
                .height(Dim::Stretch)
                .gap(Length::px(style.layout.gap)),
            &style,
        ))
        .dims(
            Dimensions::AUTO
                .with_width(Dim::Stretch)
                .with_height(Dim::Stretch),
        ),
    )
}

pub fn project_title_bar(_: &ChatTitleBarView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let snapshot = HeaderSnapshot::from_state(&ctx);
    let title = text_view(&ctx, ["picuscode.title"], "picuscode");
    let subtitle = text_view(&ctx, ["picuscode.subtitle"], snapshot.subtitle);
    let brand = flex_col(vec![title.into_any_flex(), subtitle.into_any_flex()])
        .gap(Length::px(1.0));

    let chips = flex_row(vec![
        chip_view(&ctx, snapshot.provider_chip, ChipTone::Neutral).into_any_flex(),
        chip_view(&ctx, snapshot.model_chip, ChipTone::Accent).into_any_flex(),
        chip_view(&ctx, snapshot.stream_chip, ChipTone::Success).into_any_flex(),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(Length::px(6.0));

    let new_btn = toolbar_button(&ctx, PicusCodeAction::NewThread, "New", PicusIcon::Plus);
    let settings_btn = toolbar_button(
        &ctx,
        PicusCodeAction::OpenSettings,
        "Settings",
        PicusIcon::Settings,
    );
    let about_btn = toolbar_button(&ctx, PicusCodeAction::OpenAbout, "About", PicusIcon::Info);
    Arc::new(apply_widget_style(
        flex_row(vec![
            sized_box(brand).flex(1.0).into_any_flex(),
            chips.into_any_flex(),
            new_btn.into_any_flex(),
            settings_btn.into_any_flex(),
            about_btn.into_any_flex(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(Length::px(8.0)),
        &style,
    ))
}

pub fn project_chat_body(_: &ChatBodyView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let children = child_entities(&ctx)
        .into_iter()
        .zip(ctx.children)
        .map(|(entity, child)| {
            let grow = resolve_style(ctx.world, entity).layout.flex_grow;
            if grow > 0.0 {
                flex_item(child, grow).into_any_flex()
            } else {
                child.into_any_flex()
            }
        })
        .collect::<Vec<_>>();
    Arc::new(apply_widget_style(
        flex_row(children)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(Length::px(style.layout.gap)),
        &style,
    ))
}

pub fn project_sidebar_column(_: &SidebarColumnView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let state = ctx.world.get_resource::<PicusState>();
    let active_thread = state.and_then(|s| s.active_thread.clone());
    let threads = state.map(|s| s.threads.clone()).unwrap_or_default();

    let mut items: Vec<_> = Vec::with_capacity(threads.len() + 3);
    let thread_count = threads.len();
    let active_count = threads.iter().filter(|t| !t.archived).count();

    let header = flex_row(vec![
        text_view(&ctx, ["picuscode.sidebar.heading"], "Threads").into_any_flex(),
        sized_box(chip_view(
            &ctx,
            format!("{active_count}/{thread_count}"),
            ChipTone::Neutral,
        ))
        .flex(1.0)
        .into_any_flex(),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(Length::px(8.0));
    items.push(header.into_any_flex());

    items.push(
        text_view(&ctx, ["picuscode.sidebar.caption"], "Recent CodeWhale state")
            .into_any_flex(),
    );

    if threads.is_empty() {
        items.push(empty_sidebar_state(&ctx).into_any_flex());
    }
    for t in threads {
        let is_active = active_thread.as_deref() == Some(t.id.as_str());
        let name = t
            .name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| fallback_thread_title(&t.preview, &t.id));
        let preview = if t.preview.trim().is_empty() {
            "No preview yet".to_string()
        } else {
            truncate_preview(&t.preview, 72)
        };
        let item_style = if is_active {
            resolve_style_for_classes(
                ctx.world,
                ["picuscode.thread.item", "picuscode.thread.item.active"],
            )
        } else {
            resolve_style_for_classes(ctx.world, ["picuscode.thread.item"])
        };
        let item_title_style = if is_active {
            resolve_style_for_classes(
                ctx.world,
                ["picuscode.thread.title", "picuscode.thread.title.active"],
            )
        } else {
            resolve_style_for_classes(ctx.world, ["picuscode.thread.title"])
        };
        let meta = format!(
            "{}  ·  {}",
            clean_provider(&t.model_provider),
            format_timestamp(t.updated_at)
        );
        let status = if t.archived {
            chip_view(&ctx, "archived", ChipTone::Danger)
        } else if is_active {
            chip_view(&ctx, "active", ChipTone::Accent)
        } else {
            chip_view(&ctx, "ready", ChipTone::Neutral)
        };
        let content = flex_col(vec![
            flex_row(vec![
                Arc::new(apply_label_style(label(name), &item_title_style)).into_any_flex(),
                sized_box(status).into_any_flex(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(Length::px(8.0))
            .into_any_flex(),
            text_view(&ctx, ["picuscode.thread.preview"], preview).into_any_flex(),
            text_view(&ctx, ["picuscode.thread.meta"], meta).into_any_flex(),
        ])
        .gap(Length::px(4.0));
        let btn = button_with_child(
            ctx.entity,
            PicusCodeAction::SelectThread(t.id.clone()),
            apply_widget_style(content, &item_style),
        );
        items.push(btn.into_any_flex());
    }

    Arc::new(apply_widget_style(
        sized_box(flex_col(items).gap(Length::px(4.0)))
            .width(Length::px(220.0))
            .height(Dim::Stretch),
        &style,
    ))
}

pub fn project_transcript_column(_: &TranscriptColumnView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let state = ctx.world.get_resource::<PicusState>();
    let summary = TranscriptSummary::from_state(state);
    let mut rows = Vec::with_capacity(ctx.children.len() + 4);
    rows.push(transcript_header(&ctx, &summary).into_any_flex());

    if summary.active_thread.is_none() || (summary.message_count == 0 && ctx.children.is_empty()) {
        rows.push(transcript_empty_state(&ctx, &summary).into_any_flex());
    }

    rows.extend(ctx
        .children
        .into_iter()
        .map(|child| child.into_any_flex()));

    Arc::new(apply_widget_style(
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(Length::px(style.layout.gap)),
        &style,
    ))
}

pub fn project_composer(_: &ComposerView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let draft = ctx
        .world
        .get_resource::<PicusState>()
        .map(|s| s.draft.clone())
        .unwrap_or_default();
    let streaming = ctx
        .world
        .get_resource::<PicusState>()
        .is_some_and(|s| s.streaming);
    let draft_count = draft_len(&draft);
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
    let action_btn = if streaming {
        toolbar_button(
            &ctx,
            PicusCodeAction::CancelTurn,
            "Stop",
            PicusIcon::StopCircle,
        )
    } else {
        toolbar_button(&ctx, PicusCodeAction::Send, "Send", PicusIcon::Send)
    };
    let selected = ctx
        .world
        .get_resource::<PicusState>()
        .and_then(|s| s.active_thread.as_deref().map(str::to_owned))
        .is_some();
    let helper = if streaming {
        "Assistant response is streaming"
    } else if selected {
        "Ready to send"
    } else {
        "No thread selected"
    };
    Arc::new(apply_widget_style(
        flex_col(vec![
            flex_row(vec![
                input.flex(1.0).into_any_flex(),
                action_btn.into_any_flex(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(Length::px(8.0))
            .into_any_flex(),
            flex_row(vec![
                text_view(&ctx, ["picuscode.composer.helper"], helper).into_any_flex(),
                sized_box(draft_meter(&ctx, draft_count)).flex(1.0).into_any_flex(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(Length::px(8.0))
            .into_any_flex(),
        ])
        .gap(Length::px(style.layout.gap)),
        &style,
    ))
}

pub fn project_status_line(_: &StatusLineView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let state = ctx.world.get_resource::<PicusState>();
    let status = state
        .map(|s| s.status.clone())
        .unwrap_or_else(|| "Ready".to_string());
    let summary = state
        .map(|s| {
            let provider = config_summary_value(s, "provider", "provider unset");
            let model = config_summary_value(s, "model", "model unset");
            format!(
                "{} threads · {} messages · {provider} / {model}",
                s.threads.len(),
                s.messages.len()
            )
        })
        .unwrap_or_else(|| "Bridge starting".to_string());
    Arc::new(apply_widget_style(
        flex_row(vec![
            text_view(&ctx, ["picuscode.status.primary"], status).into_any_flex(),
            sized_box(text_view(&ctx, ["picuscode.status.secondary"], summary))
                .flex(1.0)
                .into_any_flex(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(Length::px(8.0)),
        &style,
    ))
}

pub fn project_about_root(_: &AboutRootView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let close_btn = toolbar_button(&ctx, PicusCodeAction::CloseAbout, "Close", PicusIcon::X);
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

pub fn project_settings_root(_: &SettingsRootView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let close_btn = toolbar_button(&ctx, PicusCodeAction::CloseSettings, "Close", PicusIcon::X);
    let reload_btn = toolbar_button(
        &ctx,
        PicusCodeAction::ReloadConfig,
        "Reload",
        PicusIcon::RefreshCw,
    );
    let children = ctx
        .children
        .into_iter()
        .map(|child| child.into_any_flex())
        .collect::<Vec<_>>();
    let mut all = children;
    all.push(
        flex_row(vec![
            sized_box(reload_btn).flex(1.0).into_any_flex(),
            close_btn.into_any_flex(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(Length::px(8.0))
        .into_any_flex(),
    );
    Arc::new(apply_widget_style(
        flex_col(all)
            .width(Dim::Stretch)
            .height(Dim::Stretch)
            .gap(Length::px(12.0)),
        &style,
    ))
}

pub fn project_settings_form(_: &SettingsFormView, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let state = ctx.world.get_resource::<PicusState>();

    let mut rows: Vec<_> = Vec::new();
    rows.push(settings_header(&ctx, state).into_any_flex());

    let values = state.map(|s| s.config_values.clone()).unwrap_or_default();
    rows.push(
        settings_section(
            &ctx,
            "Connection",
            &values,
            &[
                ("provider", "Provider"),
                ("model", "Model"),
                ("api_key", "API Key"),
                ("base_url", "Base URL"),
            ],
        )
        .into_any_flex(),
    );
    rows.push(
        settings_section(
            &ctx,
            "Runtime",
            &values,
            &[("auth_mode", "Auth Mode"), ("telemetry", "Telemetry")],
        )
        .into_any_flex(),
    );
    rows.push(
        settings_section(
            &ctx,
            "Safety",
            &values,
            &[
                ("approval_policy", "Approval Policy"),
                ("sandbox_mode", "Sandbox Mode"),
            ],
        )
        .into_any_flex(),
    );

    if let Some(s) = state
        && let Some(status) = &s.config_status
    {
        rows.push(
            text_view(&ctx, ["picuscode.settings.status"], status.as_str())
                .into_any_flex(),
        );
    }

    Arc::new(apply_widget_style(
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(Length::px(style.layout.gap)),
        &style,
    ))
}

fn child_entities(ctx: &ProjectionCtx<'_>) -> Vec<picus::bevy_ecs::entity::Entity> {
    ctx.world
        .get::<Children>(ctx.entity)
        .map(|children| children.iter().copied().collect::<Vec<_>>())
        .unwrap_or_default()
}

fn text_view<const N: usize>(
    ctx: &ProjectionCtx<'_>,
    classes: [&'static str; N],
    text: impl Into<String>,
) -> UiView {
    let style = resolve_style_for_classes(ctx.world, classes);
    Arc::new(apply_label_style(label(text.into()), &style))
}

fn toolbar_button(
    ctx: &ProjectionCtx<'_>,
    action: PicusCodeAction,
    text: &'static str,
    glyph: PicusIcon,
) -> UiView {
    let style = resolve_style_for_classes(ctx.world, ["picuscode.toolbar.button"]);
    let text_style = resolve_style_for_classes(ctx.world, ["picuscode.toolbar.button.text"]);
    let icon_color = text_style.colors.text.unwrap_or(Color::WHITE);
    let content = flex_row(vec![
        icon(glyph, 16.0, icon_color).into_any_flex(),
        Arc::new(apply_label_style(label(text), &text_style)).into_any_flex(),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::Center)
    .gap(Length::px(6.0));
    Arc::new(apply_widget_style(
        button_with_child(ctx.entity, action, content),
        &style,
    ))
}

#[derive(Clone, Copy)]
enum ChipTone {
    Neutral,
    Accent,
    Success,
    Danger,
}

fn chip_view(ctx: &ProjectionCtx<'_>, text: impl Into<String>, tone: ChipTone) -> UiView {
    let style = match tone {
        ChipTone::Neutral => resolve_style_for_classes(ctx.world, ["picuscode.chip"]),
        ChipTone::Accent => {
            resolve_style_for_classes(ctx.world, ["picuscode.chip", "picuscode.chip.accent"])
        }
        ChipTone::Success => {
            resolve_style_for_classes(ctx.world, ["picuscode.chip", "picuscode.chip.success"])
        }
        ChipTone::Danger => {
            resolve_style_for_classes(ctx.world, ["picuscode.chip", "picuscode.chip.danger"])
        }
    };
    Arc::new(apply_widget_style(
        apply_label_style(label(text.into()), &style),
        &style,
    ))
}

fn empty_sidebar_state(ctx: &ProjectionCtx<'_>) -> UiView {
    let style = resolve_style_for_classes(ctx.world, ["picuscode.empty.panel"]);
    let action = toolbar_button(ctx, PicusCodeAction::NewThread, "New thread", PicusIcon::Plus);
    Arc::new(apply_widget_style(
        flex_col(vec![
            text_view(ctx, ["picuscode.empty.title"], "No threads").into_any_flex(),
            text_view(ctx, ["picuscode.empty.body"], "Create a thread to start chatting.")
                .into_any_flex(),
            action.into_any_flex(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(Length::px(8.0)),
        &style,
    ))
}

#[derive(Default)]
struct HeaderSnapshot {
    subtitle: String,
    provider_chip: String,
    model_chip: String,
    stream_chip: String,
}

impl HeaderSnapshot {
    fn from_state(ctx: &ProjectionCtx<'_>) -> Self {
        let Some(state) = ctx.world.get_resource::<PicusState>() else {
            return Self {
                subtitle: "Bridge starting".to_string(),
                provider_chip: "provider pending".to_string(),
                model_chip: "model pending".to_string(),
                stream_chip: "idle".to_string(),
            };
        };
        let active = state
            .active_thread
            .as_deref()
            .and_then(|id| state.threads.iter().find(|thread| thread.id == id));
        let subtitle = active
            .map(|thread| fallback_thread_title(thread.name.as_deref().unwrap_or(&thread.preview), &thread.id))
            .unwrap_or_else(|| "No active thread".to_string());
        let provider_chip = config_summary_value(state, "provider", "provider unset");
        let model_chip = config_summary_value(state, "model", "model unset");
        let stream_chip = if state.streaming { "streaming" } else { "idle" }.to_string();
        Self {
            subtitle,
            provider_chip,
            model_chip,
            stream_chip,
        }
    }
}

#[derive(Default)]
struct TranscriptSummary {
    active_thread: Option<String>,
    title: String,
    subtitle: String,
    message_count: usize,
    user_count: usize,
    assistant_count: usize,
    streaming: bool,
}

impl TranscriptSummary {
    fn from_state(state: Option<&PicusState>) -> Self {
        let Some(state) = state else {
            return Self {
                title: "Starting bridge".to_string(),
                subtitle: "Waiting for CodeWhale state".to_string(),
                ..Default::default()
            };
        };
        let active_thread = state.active_thread.clone();
        let active = active_thread
            .as_deref()
            .and_then(|id| state.threads.iter().find(|thread| thread.id == id));
        let title = active
            .map(|thread| thread.name.as_deref().unwrap_or(&thread.preview))
            .filter(|title| !title.trim().is_empty())
            .map(|title| truncate_preview(title, 54))
            .unwrap_or_else(|| {
                active_thread
                    .as_deref()
                    .map(|id| fallback_thread_title("", id))
                    .unwrap_or_else(|| "No active thread".to_string())
            });
        let subtitle = active
            .map(|thread| {
                format!(
                    "{} · updated {}",
                    clean_provider(&thread.model_provider),
                    format_timestamp(thread.updated_at)
                )
            })
            .unwrap_or_else(|| "Thread list is shared with CodeWhale".to_string());
        let user_count = state
            .messages
            .iter()
            .filter(|message| message.role == "user")
            .count();
        let assistant_count = state
            .messages
            .iter()
            .filter(|message| message.role == "assistant")
            .count();
        Self {
            active_thread,
            title,
            subtitle,
            message_count: state.messages.len(),
            user_count,
            assistant_count,
            streaming: state.streaming,
        }
    }
}

fn transcript_header(ctx: &ProjectionCtx<'_>, summary: &TranscriptSummary) -> UiView {
    let style = resolve_style_for_classes(ctx.world, ["picuscode.transcript.header"]);
    let state_chip = if summary.streaming {
        chip_view(ctx, "streaming", ChipTone::Success)
    } else {
        chip_view(ctx, format!("{} messages", summary.message_count), ChipTone::Neutral)
    };
    Arc::new(apply_widget_style(
        flex_row(vec![
            flex_col(vec![
                text_view(ctx, ["picuscode.transcript.title"], summary.title.clone())
                    .into_any_flex(),
                text_view(ctx, ["picuscode.transcript.subtitle"], summary.subtitle.clone())
                    .into_any_flex(),
            ])
            .gap(Length::px(2.0))
            .into_any_flex(),
            sized_box(flex_row(vec![
                chip_view(ctx, format!("{} user", summary.user_count), ChipTone::Accent)
                    .into_any_flex(),
                chip_view(
                    ctx,
                    format!("{} assistant", summary.assistant_count),
                    ChipTone::Neutral,
                )
                .into_any_flex(),
                state_chip.into_any_flex(),
            ]))
            .flex(1.0)
            .into_any_flex(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(Length::px(12.0)),
        &style,
    ))
}

fn transcript_empty_state(ctx: &ProjectionCtx<'_>, summary: &TranscriptSummary) -> UiView {
    let style = resolve_style_for_classes(ctx.world, ["picuscode.empty.panel"]);
    let (title, body) = if summary.active_thread.is_none() {
        ("Ready when you are", "Select an existing thread or create a fresh one.")
    } else {
        ("Fresh thread", "Send the first message from the composer.")
    };
    let new_btn = toolbar_button(ctx, PicusCodeAction::NewThread, "New thread", PicusIcon::Plus);
    Arc::new(apply_widget_style(
        flex_col(vec![
            text_view(ctx, ["picuscode.empty.title"], title).into_any_flex(),
            text_view(ctx, ["picuscode.empty.body"], body).into_any_flex(),
            new_btn.into_any_flex(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(Length::px(10.0)),
        &style,
    ))
}

fn draft_meter(ctx: &ProjectionCtx<'_>, count: usize) -> UiView {
    let tone = if count > 0 {
        ChipTone::Accent
    } else {
        ChipTone::Neutral
    };
    chip_view(ctx, format!("{count} chars"), tone)
}

fn settings_header(ctx: &ProjectionCtx<'_>, state: Option<&PicusState>) -> UiView {
    let style = resolve_style_for_classes(ctx.world, ["picuscode.settings.header"]);
    let subtitle = state
        .map(|s| {
            format!(
                "{} keys loaded · {}",
                s.config_values.len(),
                s.config_status.as_deref().unwrap_or("ready")
            )
        })
        .unwrap_or_else(|| "Config bridge starting".to_string());
    Arc::new(apply_widget_style(
        flex_col(vec![
            text_view(ctx, ["picuscode.settings.title"], "CodeWhale Settings").into_any_flex(),
            text_view(ctx, ["picuscode.settings.subtitle"], subtitle).into_any_flex(),
        ])
        .gap(Length::px(2.0)),
        &style,
    ))
}

fn settings_section(
    ctx: &ProjectionCtx<'_>,
    title: &'static str,
    values: &std::collections::BTreeMap<String, String>,
    fields: &[(&'static str, &'static str)],
) -> UiView {
    let style = resolve_style_for_classes(ctx.world, ["picuscode.settings.section"]);
    let mut rows =
        vec![text_view(ctx, ["picuscode.settings.section.title"], title).into_any_flex()];
    for (key, display) in fields {
        let current = values.get(*key).cloned().unwrap_or_default();
        let key_string = (*key).to_string();
        let row = flex_row(vec![
            sized_box(text_view(ctx, ["picuscode.settings.label"], *display))
                .width(Length::px(132.0))
                .into_any_flex(),
            text_input(ctx.entity, current, move |v| {
                PicusCodeAction::SetConfig(key_string.clone(), v)
            })
            .flex(1.0)
            .into_any_flex(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(Length::px(8.0));
        rows.push(row.into_any_flex());
    }
    Arc::new(apply_widget_style(
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(Length::px(9.0)),
        &style,
    ))
}

fn config_summary_value(state: &PicusState, key: &str, fallback: &str) -> String {
    state
        .config_values
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(truncate_config_value)
        .unwrap_or_else(|| fallback.to_string())
}

fn truncate_config_value(value: &str) -> String {
    if value.chars().count() <= 28 {
        value.to_string()
    } else {
        truncate_preview(value, 25)
    }
}

fn fallback_thread_title(seed: &str, id: &str) -> String {
    let seed = seed.trim();
    if !seed.is_empty() {
        return truncate_preview(seed, 34);
    }
    let short_id: String = id.chars().take(8).collect();
    format!("Thread {short_id}")
}

fn clean_provider(provider: &str) -> String {
    let provider = provider.trim();
    if provider.is_empty() {
        "unknown provider".to_string()
    } else {
        truncate_preview(provider, 22)
    }
}

fn format_timestamp(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "unknown time".to_string())
}

fn draft_len(draft: &str) -> usize {
    draft.chars().count()
}

fn truncate_preview(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_truncation_is_ascii_and_trimmed() {
        assert_eq!(truncate_preview("  hello world  ", 20), "hello world");
        assert_eq!(truncate_preview("abcdefghijklmnopqrstuvwxyz", 5), "abcde...");
    }

    #[test]
    fn thread_title_falls_back_to_short_id() {
        assert_eq!(fallback_thread_title("", "abcdef123456"), "Thread abcdef12");
        assert_eq!(
            fallback_thread_title("A useful remembered prompt", "abcdef123456"),
            "A useful remembered prompt"
        );
    }

    #[test]
    fn timestamp_format_is_stable() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00 UTC");
    }
}
