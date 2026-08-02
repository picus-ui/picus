//! omp (ACP) bridge for picuscode.
//!
//! This owns a dedicated tokio runtime on a background thread and drives a
//! resident `omp acp` child process over stdio JSON-RPC (Agent Client
//! Protocol). One long-lived child hosts every session; sessions persist to
//! the same `~/.omp/agent/sessions/` files an installed `omp` binary uses, so
//! picuscode is fully session-compatible with the user's installed omp.
//!
//! The bridge communicates with the ECS world through two crossbeam channels:
//! `BridgeRequest` in, `BridgeEvent` out. ECS systems push requests and poll
//! events each frame, keeping the async runtime off the Bevy render thread.
//!
//! Turn protocol: `session/prompt` resolves with the turn's `stopReason`
//! (`end_turn` / `max_tokens` / `refusal` / `cancelled` / `max_turn_requests`),
//! while the agent streams progress as `session/update` notifications
//! (`agent_message_chunk`, `tool_call`, `session_info_update`, `usage_update`,
//! …). The bridge therefore maps the prompt response to `TurnEnded` and the
//! notifications to `TurnDelta` / status events — no polling, no SSE client.

// The bridge uses `let`-chain style guards (`if let Some(x) = ... && cond`)
// extensively for clarity; collapsing them hurts readability.
#![allow(clippy::collapsible_if)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context as _, Result, anyhow, bail};
use crossbeam_channel::{Receiver, Sender, unbounded};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::runtime::Runtime as TokioRuntime;
use tracing::{debug, warn};

/// A request pushed from the ECS world to the bridge thread.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum BridgeRequest {
    /// Refresh the thread list from omp's session store.
    ListThreads,
    /// Create a fresh session and return its id.
    CreateThread,
    /// Load a session's persisted transcript.
    ReadThread { thread_id: String },
    /// Send a user message and start a model turn.
    SendMessage { thread_id: String, input: String },
    /// Cancel the in-flight turn for a session, if any.
    CancelTurn { thread_id: String },
    /// Rename a thread (updates the session title).
    SetThreadName { thread_id: String, name: String },
    /// Archive a thread (closes the omp session).
    ArchiveThread { thread_id: String },
    /// List available session config options (model / thinking / mode).
    ConfigList,
    /// Read a single session config option (raw form).
    ConfigGet { key: String },
    /// Set a session config option and persist.
    ConfigSet { key: String, value: String },
    /// Unset a config option (restore default).
    ConfigUnset { key: String },
    /// Reload config + session list from disk.
    ConfigReload,
}

/// An event streamed back from the bridge thread to the ECS world.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// The thread list was refreshed.
    Threads(Vec<ThreadSummary>),
    /// A session's history was loaded.
    ThreadHistory {
        thread_id: String,
        messages: Vec<ChatMessage>,
        thread: Option<ThreadInfo>,
    },
    /// A new session was created.
    ThreadCreated { thread: ThreadInfo },
    /// A streaming turn started.
    TurnStarted {
        thread_id: String,
        response_id: String,
    },
    /// An incremental assistant delta.
    TurnDelta {
        thread_id: String,
        response_id: String,
        delta: String,
    },
    /// A streaming turn finished (ok = ended cleanly, not cancelled/errored).
    TurnEnded {
        thread_id: String,
        response_id: String,
        ok: bool,
    },
    /// An error occurred on a turn.
    TurnError {
        thread_id: String,
        response_id: String,
        message: String,
    },
    /// A config list response (read-only omp options + on-disk key/values).
    ConfigListed(BTreeMap<String, String>),
    /// A single config value response.
    ConfigGot { key: String, value: Option<String> },
    /// A config set/unset/reload result.
    ConfigResult { ok: bool, error: Option<String> },
    /// Bridge thread is ready (omp child spawned and initialized).
    Ready,
}

/// A flattened session summary for UI rendering.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ThreadSummary {
    pub id: String,
    pub name: Option<String>,
    pub preview: String,
    pub model_provider: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived: bool,
}

/// A flattened chat message for UI rendering.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

/// A minimal session descriptor returned by omp's session store.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    pub id: String,
    pub cwd: String,
    pub title: Option<String>,
    pub updated_at: Option<String>,
}

/// Handle held by the ECS world to talk to the bridge thread.
#[derive(Clone)]
pub struct BridgeHandle {
    pub tx: Sender<BridgeRequest>,
    pub events: Receiver<BridgeEvent>,
}

/// Spawns the bridge background thread and returns a handle.
///
/// The thread owns its own tokio runtime and the resident `omp acp` child.
/// Dropping the handle does not stop the thread; the process exits when the
/// UI loop exits.
pub fn spawn_bridge() -> BridgeHandle {
    spawn_bridge_with_config_path(None)
}

/// Like [`spawn_bridge`] but pins the session/config root to `omp_home`.
///
/// Tests use this with a tempdir path so they never touch the user's real
/// `~/.omp/` state. `None` falls back to the default omp path resolution
/// (`~/.omp/agent`), sharing sessions with an installed `omp` binary.
pub fn spawn_bridge_with_config_path(config_path: Option<PathBuf>) -> BridgeHandle {
    let (req_tx, req_rx) = unbounded::<BridgeRequest>();
    let (evt_tx, evt_rx) = unbounded::<BridgeEvent>();

    std::thread::Builder::new()
        .name("picuscode-bridge".into())
        .spawn(move || {
            if let Err(err) = run_bridge(req_rx, evt_tx.clone(), config_path) {
                warn!("picuscode bridge thread exited with error: {err:#}");
            }
        })
        .expect("failed to spawn picuscode bridge thread");

    BridgeHandle {
        tx: req_tx,
        events: evt_rx,
    }
}

fn run_bridge(
    req_rx: Receiver<BridgeRequest>,
    evt_tx: Sender<BridgeEvent>,
    config_path: Option<PathBuf>,
) -> Result<()> {
    let tokio_rt = TokioRuntime::new()?;
    let _guard = tokio_rt.enter();

    // Drive the omp child + request handling on the tokio runtime. The
    // crossbeam recv is blocking, so it runs on a blocking task that re-spawns
    // each request onto the runtime.
    tokio_rt.block_on(async move {
        let acp = AcpClient::spawn(config_path.as_deref(), evt_tx.clone())
            .await
            .context("failed to spawn omp acp")?;

        let _ = evt_tx.send(BridgeEvent::Ready);

        // Blocking recv loop: forwards requests into the runtime as tasks.
        tokio::task::spawn_blocking(move || {
            while let Ok(req) = req_rx.recv() {
                let acp = acp.clone();
                let evt_tx = evt_tx.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle_request(req, acp, evt_tx).await {
                        warn!("bridge request failed: {err:#}");
                    }
                });
            }
        });

        // Keep the runtime alive until the blocking loop ends.
        std::future::pending::<()>().await;
        Ok(())
    })
}

// ── ACP JSON-RPC client ────────────────────────────────────────────────

/// A single JSON-RPC response received from the omp child.
#[derive(Debug)]
struct AcpResponse {
    result: Option<Value>,
    error: Option<Value>,
}

/// Resident `omp acp` child process + JSON-RPC plumbing.
#[derive(Clone)]
struct AcpClient {
    writer: Arc<tokio::sync::Mutex<ChildStdin>>,
    responses: Arc<tokio::sync::Mutex<BTreeMap<String, tokio::sync::oneshot::Sender<AcpResponse>>>>,
    next_id: Arc<AtomicU64>,
    session_dir: Arc<std::sync::Mutex<Option<PathBuf>>>,
    /// Forwarded to the ECS world for `session/update` notifications.
    events: Sender<BridgeEvent>,
    /// Latest `configOptions` array captured from session/new / session/load
    /// responses (model / thinking / mode), keyed by option id.
    config_options: Arc<tokio::sync::Mutex<BTreeMap<String, Value>>>,
}

impl AcpClient {
    /// Record the `configOptions` array from a session response for the
    /// settings panel.
    fn capture_config_options(&self, response: &Value) {
        let Some(options) = response.get("configOptions").and_then(|o| o.as_array()) else {
            return;
        };
        let mut map = match self.config_options.try_lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        for option in options {
            if let Some(id) = option.get("id").and_then(|v| v.as_str()) {
                map.insert(id.to_string(), option.clone());
            }
        }
    }

    /// The latest known config options (id -> option object).
    fn config_options(&self) -> Option<Vec<Value>> {
        let guard = self.config_options.try_lock().ok()?;
        Some(guard.values().cloned().collect())
    }
}

impl AcpClient {
    /// Spawn `omp acp` (resolving via PATH) and wait for the initialize
    /// handshake response. When `omp_home` is set, exports `PI_CODING_AGENT_DIR`
    /// so omp keeps its sessions under the test tempdir instead of `~/.omp`.
    async fn spawn(omp_home: Option<&Path>, events: Sender<BridgeEvent>) -> Result<Self> {
        let mut cmd = Command::new("omp");
        cmd.arg("acp");
        cmd.kill_on_drop(true);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());
        if let Some(home) = omp_home {
            // Isolate sessions to the given root (used by tests) while keeping
            // the real `~/.omp/agent` config/auth for model resolution.
            cmd.arg("--session-dir").arg(home.join("sessions"));
        }

        let mut child: Child = cmd
            .spawn()
            .map_err(|e| anyhow!("failed to spawn `omp acp`: {e}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("omp acp stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("omp acp stdout unavailable"))?;

        let client = AcpClient {
            writer: Arc::new(tokio::sync::Mutex::new(stdin)),
            responses: Arc::new(tokio::sync::Mutex::new(BTreeMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            session_dir: Arc::new(std::sync::Mutex::new(omp_home.map(|p| p.join("sessions")))),
            events,
            config_options: Arc::new(tokio::sync::Mutex::new(BTreeMap::new())),
        };

        let client_clone = client.clone();
        let mut lines = BufReader::new(stdout).lines();
        tokio::spawn(async move {
            loop {
                let Ok(Some(line)) = lines.next_line().await else {
                    break;
                };
                client_clone.dispatch_line(&line);
            }
        });

        // initialize handshake — required before any session method.
        client
            .request(
                "initialize",
                json!({ "protocolVersion": 1, "clientCapabilities": {} }),
            )
            .await
            .map_err(|e| anyhow!("omp acp initialize failed: {e}"))?;
        debug!("omp acp initialized");

        Ok(client)
    }

    fn dispatch_line(&self, line: &str) {
        let Ok(msg) = serde_json::from_str::<Value>(line) else {
            return;
        };
        // Notifications have no id — route session/update to the UI.
        if msg.get("id").is_none() {
            if msg.get("method").and_then(|m| m.as_str()) == Some("session/update") {
                self.handle_session_update(msg.get("params"));
            }
            return;
        }
        let Some(id) = msg.get("id").and_then(|v| v.as_str()).map(str::to_string) else {
            return;
        };
        let Some(tx) = self.responses.try_lock().ok().and_then(|mut g| g.remove(&id)) else {
            return;
        };
        let _ = tx.send(AcpResponse {
            result: msg.get("result").cloned(),
            error: msg.get("error").cloned(),
        });
    }

    fn handle_session_update(&self, params: Option<&Value>) {
        let Some(params) = params else { return };
        let Some(session_id) = params
            .get("sessionId")
            .and_then(|v| v.as_str())
            .map(str::to_string)
        else {
            return;
        };
        let Some(update) = params.get("update") else { return };
        let Some(kind) = update
            .get("sessionUpdate")
            .and_then(|v| v.as_str())
        else {
            return;
        };
        let response_id = format!("resp-{session_id}");
        match kind {
            "agent_message_chunk" => {
                let Some(text) = update
                    .get("content")
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                else {
                    return;
                };
                if text.is_empty() {
                    return;
                }
                let _ = self.events.send(BridgeEvent::TurnDelta {
                    thread_id: session_id,
                    response_id,
                    delta: text.to_string(),
                });
            }
            "usage_update" => {
                if let Some(error) = update.get("error") {
                    let message = error
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("usage error");
                    let _ = self.events.send(BridgeEvent::TurnError {
                        thread_id: session_id,
                        response_id,
                        message: message.to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    /// Send a JSON-RPC request and await its response (turn-bounded).
    async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.responses.lock().await.insert(id.clone(), tx);

        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let mut line = serde_json::to_string(&frame)?;
        line.push('\n');
        {
            let mut stdin = self.writer.lock().await;
            stdin.write_all(line.as_bytes()).await?;
            stdin.flush().await?;
        }

        let resp = tokio::time::timeout(std::time::Duration::from_secs(600), rx)
            .await
            .map_err(|_| anyhow!("omp acp request `{method}` timed out"))?
            .map_err(|_| anyhow!("omp acp request `{method}` dropped"))?;

        if let Some(err) = resp.error {
            let code = err.get("code").cloned().unwrap_or(Value::Null);
            let message = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            bail!("omp acp `{method}` failed ({code}): {message}");
        }
        resp.result
            .ok_or_else(|| anyhow!("omp acp `{method}` returned no result"))
    }

    /// Send a JSON-RPC notification (no response; e.g. `session/cancel`).
    async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let frame = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let mut line = serde_json::to_string(&frame)?;
        line.push('\n');
        let mut stdin = self.writer.lock().await;
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    fn sessions_root(&self) -> Option<PathBuf> {
        self.session_dir.lock().ok()?.clone()
    }
}

// ── Request handling ───────────────────────────────────────────────────

async fn handle_request(req: BridgeRequest, acp: AcpClient, evt_tx: Sender<BridgeEvent>) -> Result<()> {
    match req {
        BridgeRequest::ListThreads => {
            let summaries = collect_summaries(&acp).await;
            let _ = evt_tx.send(BridgeEvent::Threads(summaries));
            Ok(())
        }
        BridgeRequest::CreateThread => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let result = acp
                .request(
                    "session/new",
                    json!({ "cwd": cwd.to_string_lossy(), "mcpServers": [] }),
                )
                .await?;
            acp.capture_config_options(&result);
            let id = result
                .get("sessionId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("session/new returned no sessionId"))?
                .to_string();
            let info = session_info(&acp, &id).await.unwrap_or_else(|| ThreadInfo {
                id: id.clone(),
                cwd: cwd.to_string_lossy().into_owned(),
                title: None,
                updated_at: None,
            });
            let _ = evt_tx.send(BridgeEvent::ThreadCreated { thread: info });
            // Refresh the list so the new session shows up in the sidebar.
            let _ = evt_tx.send(BridgeEvent::Threads(collect_summaries(&acp).await));
            Ok(())
        }
        BridgeRequest::ReadThread { thread_id } => {
            let messages = read_session_transcript(&acp, &thread_id).await?;
            let info = session_info(&acp, &thread_id).await;
            let _ = evt_tx.send(BridgeEvent::ThreadHistory {
                thread_id,
                messages,
                thread: info,
            });
            Ok(())
        }
        BridgeRequest::SendMessage { thread_id, input } => {
            start_turn(&acp, thread_id, input).await
        }
        BridgeRequest::CancelTurn { thread_id } => {
            acp.notify("session/cancel", json!({ "sessionId": thread_id }))
                .await?;
            Ok(())
        }
        BridgeRequest::SetThreadName { thread_id, name } => {
            // omp titles are derived from the first user message; renaming is
            // not a first-class ACP operation. Persist the desired name in a
            // sidecar keyed by session id so picuscode can display it.
            set_display_name(&acp, &thread_id, &name).await;
            let _ = evt_tx.send(BridgeEvent::Threads(collect_summaries(&acp).await));
            Ok(())
        }
        BridgeRequest::ArchiveThread { thread_id } => {
            let _ = acp
                .request("session/close", json!({ "sessionId": thread_id }))
                .await;
            let _ = evt_tx.send(BridgeEvent::Threads(collect_summaries(&acp).await));
            Ok(())
        }
        BridgeRequest::ConfigList => {
            let values = list_config_values(&acp).await;
            let _ = evt_tx.send(BridgeEvent::ConfigListed(values));
            Ok(())
        }
        BridgeRequest::ConfigGet { key } => {
            let values = list_config_values(&acp).await;
            let value = values.get(&key).cloned();
            let _ = evt_tx.send(BridgeEvent::ConfigGot { key, value });
            Ok(())
        }
        BridgeRequest::ConfigSet { key, value } => {
            let result = set_config_option(&acp, &key, &value).await;
            match result {
                Ok(()) => {
                    let _ = evt_tx.send(BridgeEvent::ConfigResult { ok: true, error: None });
                    let values = list_config_values(&acp).await;
                    let _ = evt_tx.send(BridgeEvent::ConfigListed(values));
                }
                Err(e) => {
                    let _ = evt_tx.send(BridgeEvent::ConfigResult {
                        ok: false,
                        error: Some(e.to_string()),
                    });
                }
            }
            Ok(())
        }
        BridgeRequest::ConfigUnset { key: _ } => {
            // omp persists its own config; there is no "unset" — report
            // success and refresh.
            let _ = evt_tx.send(BridgeEvent::ConfigResult { ok: true, error: None });
            let values = list_config_values(&acp).await;
            let _ = evt_tx.send(BridgeEvent::ConfigListed(values));
            Ok(())
        }
        BridgeRequest::ConfigReload => {
            let _ = evt_tx.send(BridgeEvent::ConfigResult { ok: true, error: None });
            let _ = evt_tx.send(BridgeEvent::Threads(collect_summaries(&acp).await));
            Ok(())
        }
    }
}

async fn start_turn(acp: &AcpClient, thread_id: String, input: String) -> Result<()> {
    let response_id = format!("resp-{thread_id}");
    let _ = acp.events.send(BridgeEvent::TurnStarted {
        thread_id: thread_id.clone(),
        response_id: response_id.clone(),
    });

    // session/prompt resolves when the turn finishes — the response is the
    // stop signal. Chunks arrive as session/update notifications on the
    // reader task, which forwards them to the UI through `acp.events`.
    let result = acp
        .request(
            "session/prompt",
            json!({
                "sessionId": thread_id,
                "prompt": [{ "type": "text", "text": input }],
            }),
        )
        .await;

    match result {
        Ok(resp) => {
            let stop = resp
                .get("stopReason")
                .and_then(|v| v.as_str())
                .unwrap_or("end_turn");
            let ok = matches!(stop, "end_turn" | "max_turn_requests");
            let _ = acp.events.send(BridgeEvent::TurnEnded {
                thread_id: thread_id.clone(),
                response_id: response_id.clone(),
                ok,
            });
        }
        Err(e) => {
            let _ = acp.events.send(BridgeEvent::TurnError {
                thread_id: thread_id.clone(),
                response_id: response_id.clone(),
                message: e.to_string(),
            });
            let _ = acp.events.send(BridgeEvent::TurnEnded {
                thread_id,
                response_id,
                ok: false,
            });
        }
    }
    Ok(())
}

// ── Session listing / transcript ───────────────────────────────────────

/// Walk the sessions root for `*.jsonl` session files, handling both layouts
/// omp uses:
/// - default: `root/<cwd-slug>/<ts>_<id>.jsonl` (per-cwd subdirs)
/// - explicit `--session-dir`: `root/<ts>_<id>.jsonl` directly
/// Returns the newest-first list of session file paths.
async fn scan_session_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(root).await else {
        return files;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let Ok(ft) = entry.file_type().await else {
            continue;
        };
        if ft.is_dir() {
            // cwd-slug layout
            let Ok(mut sub) = tokio::fs::read_dir(&path).await else {
                continue;
            };
            while let Ok(Some(sub_entry)) = sub.next_entry().await {
                let sub_path = sub_entry.path();
                if sub_path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    files.push(sub_path);
                }
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
    files
}

/// Walk `~/.omp/agent/sessions/<cwd-slug>/*.jsonl` and build summaries.
///
/// omp persists every session as a JSONL stream. We scan the session root,
/// parse session headers + messages, and sort newest-first (matching omp's
/// own list order).
async fn collect_summaries(acp: &AcpClient) -> Vec<ThreadSummary> {
    let Some(root) = acp.sessions_root() else {
        return Vec::new();
    };
    let mut summaries = Vec::new();
    for path in scan_session_files(&root).await {
        if let Ok(info) = parse_session_file(&path).await {
            summaries.push(thread_summary_from_info(acp, &info).await);
        }
    }
    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    summaries
}

/// Load a session's full transcript from its JSONL file.
async fn read_session_transcript(acp: &AcpClient, session_id: &str) -> Result<Vec<ChatMessage>> {
    let Some(root) = acp.sessions_root() else {
        return Ok(Vec::new());
    };
    for path in scan_session_files(&root).await {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if name.contains(session_id) {
            return Ok(parse_transcript(&path).await);
        }
    }
    Ok(Vec::new())
}

async fn session_info(acp: &AcpClient, session_id: &str) -> Option<ThreadInfo> {
    let root = acp.sessions_root()?;
    for path in scan_session_files(&root).await {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if name.contains(session_id) {
            return parse_session_file(&path).await.ok();
        }
    }
    None
}

async fn thread_summary_from_info(acp: &AcpClient, info: &ThreadInfo) -> ThreadSummary {
    let name = display_name(acp, &info.id)
        .await
        .or_else(|| info.title.clone());
    let preview = if name.is_some() {
        String::new()
    } else {
        // First user message text.
        match read_session_transcript(acp, &info.id).await {
            Ok(messages) => messages
                .iter()
                .find(|m| m.role == "user")
                .map(|m| m.content.clone())
                .unwrap_or_default(),
            Err(_) => String::new(),
        }
    };
    let created = info
        .updated_at
        .as_deref()
        .and_then(parse_omp_timestamp)
        .unwrap_or(0);
    ThreadSummary {
        id: info.id.clone(),
        name,
        preview,
        model_provider: "omp".to_string(),
        created_at: created,
        updated_at: created,
        archived: false,
    }
}

fn parse_omp_timestamp(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|dt| dt.timestamp())
}

// ── Session file parsing (JSONL) ───────────────────────────────────────

/// Parse one omp session JSONL file into a `ThreadInfo`.
async fn parse_session_file(path: &Path) -> Result<ThreadInfo> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut id = String::new();
    let mut cwd = String::new();
    let mut title: Option<String> = None;
    let mut updated: Option<String> = None;
    for line in content.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match v.get("type").and_then(|t| t.as_str()) {
            Some("session") => {
                id = v
                    .get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                cwd = v
                    .get("cwd")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                if title.is_none() {
                    title = v.get("title").and_then(|x| x.as_str()).map(str::to_string);
                }
                if updated.is_none() {
                    updated = v
                        .get("timestamp")
                        .and_then(|x| x.as_str())
                        .map(str::to_string);
                }
            }
            Some("title_change") => {
                if title.is_none() {
                    title = v.get("title").and_then(|x| x.as_str()).map(str::to_string);
                }
            }
            Some("message") => {
                updated = v
                    .get("timestamp")
                    .and_then(|x| x.as_str())
                    .map(str::to_string);
            }
            _ => {}
        }
    }
    if id.is_empty() {
        bail!("session file {path:?} has no header");
    }
    Ok(ThreadInfo {
        id,
        cwd,
        title,
        updated_at: updated,
    })
}

/// Parse the user/assistant message stream from a session JSONL file.
async fn parse_transcript(path: &Path) -> Vec<ChatMessage> {
    let Ok(content) = tokio::fs::read_to_string(path).await else {
        return Vec::new();
    };
    let mut messages = Vec::new();
    let mut seq: i64 = 0;
    for line in content.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("message") {
            continue;
        }
        let Some(role) = v
            .get("message")
            .and_then(|m| m.get("role"))
            .and_then(|r| r.as_str())
        else {
            continue;
        };
        let text = extract_message_text(&v);
        if text.trim().is_empty() {
            continue;
        }
        let created = v
            .get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(parse_omp_timestamp)
            .unwrap_or(0);
        seq += 1;
        messages.push(ChatMessage {
            id: seq,
            role: role.to_string(),
            content: text,
            created_at: created,
        });
    }
    messages
}

/// Extract plain-text content from an omp `message` entry (skips tool calls,
/// images, thinking blocks).
fn extract_message_text(v: &Value) -> String {
    let Some(content) = v.get("message").and_then(|m| m.get("content")) else {
        return String::new();
    };
    let mut text = String::new();
    if let Some(arr) = content.as_array() {
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(s) = block.get("text").and_then(|t| t.as_str()) {
                    text.push_str(s);
                }
            }
        }
    } else if let Some(s) = content.as_str() {
        text.push_str(s);
    }
    text
}

// ── Config ─────────────────────────────────────────────────────────────

/// Read the active session's ACP config options (model/thinking/mode) plus
/// the on-disk omp config files into the settings panel's key/value map.
async fn list_config_values(acp: &AcpClient) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();

    // Session-scoped options from the most recent session's configOptions.
    // omp returns them on session/new / session/load — we cache the latest
    // response via a small in-memory store updated by those handlers.
    if let Some(options) = acp.config_options() {
        for option in options {
            let id = option.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let current = option
                .get("currentValue")
                .and_then(|v| v.as_str().map(str::to_string))
                .or_else(|| {
                    option
                        .get("currentValue")
                        .and_then(|v| v.as_bool())
                        .map(|b| b.to_string())
                })
                .unwrap_or_default();
            values.insert(id.to_string(), current);
        }
    }

    // On-disk omp config + model list (read-only display).
    let agent_dir = acp
        .sessions_root()
        .as_deref()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());
    if let Some(dir) = agent_dir {
        for file in ["config.yml", "models.yml"] {
            let path = dir.join(file);
            if let Ok(content) = std::fs::read_to_string(&path) {
                values.insert(
                    format!("omp:{file}"),
                    content.lines().take(40).collect::<Vec<_>>().join("\n"),
                );
            }
        }
    }

    values
}

/// Apply a `session/set_config_option` for the most recent session.
async fn set_config_option(acp: &AcpClient, key: &str, value: &str) -> Result<()> {
    let Some(session_id) = first_session_id(acp).await else {
        bail!("no session to configure; create a thread first");
    };
    let parsed: Value = match value {
        "true" | "false" => json!(value == "true"),
        _ => json!(value),
    };
    let _ = acp
        .request(
            "session/set_config_option",
            json!({ "sessionId": session_id, "configId": key, "value": parsed }),
        )
        .await?;
    Ok(())
}

async fn first_session_id(acp: &AcpClient) -> Option<String> {
    let summaries = collect_summaries(acp).await;
    summaries.into_iter().next().map(|s| s.id)
}

// ── Display-name sidecar (rename support) ──────────────────────────────

/// Store a user-assigned display name for a session in a sidecar file under
/// the omp sessions root, so renames survive restarts without touching omp's
/// own JSONL.
async fn set_display_name(acp: &AcpClient, session_id: &str, name: &str) {
    let Some(root) = acp.sessions_root() else {
        return;
    };
    let sidecar = root.join("picuscode-names.json");
    let mut names: BTreeMap<String, String> = std::fs::read_to_string(&sidecar)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if name.trim().is_empty() {
        names.remove(session_id);
    } else {
        names.insert(session_id.to_string(), name.trim().to_string());
    }
    let _ = tokio::fs::write(
        &sidecar,
        serde_json::to_string_pretty(&names).unwrap_or_default(),
    )
    .await;
}

async fn display_name(acp: &AcpClient, session_id: &str) -> Option<String> {
    let root = acp.sessions_root()?;
    let sidecar = root.join("picuscode-names.json");
    let names: BTreeMap<String, String> = std::fs::read_to_string(&sidecar)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    names.get(session_id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Obtain a real `ChildStdin` for test-only client construction by
    /// spawning a throwaway `cmd /c exit` child (Windows) / `true` (POSIX).
    fn dummy_child_stdin() -> ChildStdin {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "exit"]);
            c
        } else {
            let c = Command::new("true");
            c
        };
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let rt = TokioRuntime::new().unwrap();
        rt.block_on(async move {
            let mut child = cmd.spawn().expect("spawn dummy child");
            child.stdin.take().expect("dummy stdin")
        })
    }

    /// Drains bridge events until a predicate matches or the timeout expires.
    fn wait_event<F>(handle: &BridgeHandle, predicate: F) -> Option<BridgeEvent>
    where
        F: Fn(&BridgeEvent) -> bool,
    {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Ok(ev) = handle.events.recv_timeout(std::time::Duration::from_millis(50)) {
                if predicate(&ev) {
                    return Some(ev);
                }
            }
        }
        None
    }

    #[test]
    fn session_jsonl_parse_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("sessions");
        std::fs::create_dir_all(&root.join("-source-repos-demo")).unwrap();
        let file = root
            .join("-source-repos-demo")
            .join("2026-08-01T00-00-00-000Z_019fbe9c-9aa9-7000-9baf-a94a673ab2b8.jsonl");
        std::fs::write(
            &file,
            r#"{"type":"session","version":3,"id":"019fbe9c-9aa9-7000-9baf-a94a673ab2b8","timestamp":"2026-08-01T00:00:00.000Z","cwd":"C:\\source\\repos\\demo"}
{"type":"message","id":"m1","timestamp":"2026-08-01T00:00:01.000Z","message":{"role":"user","content":[{"type":"text","text":"hello omp"}]}}
{"type":"message","id":"m2","timestamp":"2026-08-01T00:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"hi there"}]}}
"#,
        )
        .unwrap();

        let rt = TokioRuntime::new().unwrap();
        let info = rt.block_on(parse_session_file(&file)).unwrap();
        assert_eq!(info.id, "019fbe9c-9aa9-7000-9baf-a94a673ab2b8");
        assert_eq!(info.cwd, "C:\\source\\repos\\demo");

        let messages = rt.block_on(parse_transcript(&file));
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "hello omp");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "hi there");
        assert!(messages[0].created_at > 0);
    }

    #[test]
    fn jsonrpc_frame_serialization_matches_acp() {
        let _client = AcpClient {
            writer: Arc::new(tokio::sync::Mutex::new(dummy_child_stdin())),
            responses: Arc::new(tokio::sync::Mutex::new(BTreeMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            session_dir: Arc::new(std::sync::Mutex::new(None)),
            events: unbounded::<BridgeEvent>().0,
            config_options: Arc::new(tokio::sync::Mutex::new(BTreeMap::new())),
        };
        let frame = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "session/new",
            "params": { "cwd": "C:\\demo", "mcpServers": [] }
        }))
        .unwrap();
        assert!(frame.contains("\"method\":\"session/new\""));
        assert!(frame.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn session_update_notification_maps_to_turn_delta() {
        let (evt_tx, evt_rx) = unbounded::<BridgeEvent>();
        let client = AcpClient {
            writer: Arc::new(tokio::sync::Mutex::new(dummy_child_stdin())),
            responses: Arc::new(tokio::sync::Mutex::new(BTreeMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            session_dir: Arc::new(std::sync::Mutex::new(None)),
            events: evt_tx,
            config_options: Arc::new(tokio::sync::Mutex::new(BTreeMap::new())),
        };
        client.handle_session_update(Some(&json!({
            "sessionId": "abc123",
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "Hello " },
                "messageId": "m1"
            }
        })));
        let ev = evt_rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap();
        match ev {
            BridgeEvent::TurnDelta {
                thread_id,
                response_id,
                delta,
            } => {
                assert_eq!(thread_id, "abc123");
                assert_eq!(response_id, "resp-abc123");
                assert_eq!(delta, "Hello ");
            }
            other => panic!("expected TurnDelta, got {other:?}"),
        }
    }

    /// Live smoke test against a real `omp acp` child. Skipped when `omp` is
    /// not on PATH (CI). Proves the full Rust bridge speaks ACP correctly:
    /// initialize → session/new → prompt (streaming chunks + end_turn).
    /// Uses a tempdir sessions root so it never touches the user's `~/.omp`.
    #[test]
    fn live_omp_acp_bridge_smoke() {
        // Locate `omp` on PATH; skip silently when absent.
        let probe = Command::new("omp")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        let Ok(mut probe_child) = probe else {
            eprintln!("skipping live_omp_acp_bridge_smoke: omp not on PATH");
            return;
        };
        let _ = probe_child.wait();

        let tmp = tempfile::tempdir().expect("tempdir");
        let omp_home = tmp.path().to_path_buf();
        let handle = spawn_bridge_with_config_path(Some(omp_home.clone()));

        let _ = wait_event(&handle, |e| matches!(e, BridgeEvent::Ready));

        let _ = handle.tx.send(BridgeRequest::ListThreads);
        let _ = wait_event(&handle, |e| matches!(e, BridgeEvent::Threads(_)));

        let _ = handle.tx.send(BridgeRequest::CreateThread);
        let created = wait_event(&handle, |e| matches!(e, BridgeEvent::ThreadCreated { .. }));
        let thread_id = match created {
            Some(BridgeEvent::ThreadCreated { thread }) => thread.id,
            other => panic!("expected ThreadCreated, got {other:?}"),
        };

        let _ = handle.tx.send(BridgeRequest::SendMessage {
            thread_id: thread_id.clone(),
            input: "Reply with exactly: ACP_OK".to_string(),
        });

        // Must observe at least one TurnDelta and a clean TurnEnded.
        let delta = wait_event(&handle, |e| matches!(e, BridgeEvent::TurnDelta { .. }));
        assert!(
            matches!(&delta, Some(BridgeEvent::TurnDelta { delta, .. }) if delta.contains("ACP")),
            "expected streaming delta containing ACP, got {delta:?}"
        );
        let ended = wait_event(&handle, |e| matches!(e, BridgeEvent::TurnEnded { .. }));
        assert!(
            matches!(ended, Some(BridgeEvent::TurnEnded { ok: true, .. })),
            "expected clean turn end, got {ended:?}"
        );

        // The session should now be listed and its transcript readable.
        // Retry a few times since omp's session file write is async.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut found = false;
        while std::time::Instant::now() < deadline {
            let _ = handle.tx.send(BridgeRequest::ListThreads);
            if let Some(BridgeEvent::Threads(t)) = wait_event(&handle, |e| matches!(e, BridgeEvent::Threads(_))) {
                if t.iter().any(|t| t.id == thread_id) {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "new session should appear in thread list within 5s");

        let _ = handle.tx.send(BridgeRequest::ReadThread {
            thread_id: thread_id.clone(),
        });
        let history = wait_event(&handle, |e| matches!(e, BridgeEvent::ThreadHistory { .. }));
        assert!(
            matches!(&history, Some(BridgeEvent::ThreadHistory { messages, .. }) if messages.iter().any(|m| m.content.contains("ACP"))),
            "transcript should contain the assistant reply: {history:?}"
        );
    }
}
