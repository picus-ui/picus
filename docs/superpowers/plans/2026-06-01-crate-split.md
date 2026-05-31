# Crate 模块拆分实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `picus_ui_runtime`、`picus_masonry`、`xilem_masonry` 三个旧 crate 全面重组为 `picus_widget`（widget 层）和 `picus_view`（视图层），外部 UI 依赖仅保留 `masonry_core` 与 `xilem_core`。

**Architecture:** 严格单向三层依赖：`masonry_core` → `picus_widget` → `picus_view` → `picus_core`。`picus_widget` 对 `xilem_core` 无感知；`picus_view` 内部以 Cargo.toml 别名 `masonry` 继续引用 `picus_widget`，源文件无需修改；`picus_core` 将全部 `xilem_masonry::` 导入替换为 `picus_view::`。

**Tech Stack:** Rust 2024 edition, Cargo workspace, git mv for history-preserving rename.

---

## 文件变更总览

**重命名（git mv）：**
- `crates/picus_ui_runtime/` → `crates/picus_widget/`
- `crates/xilem_masonry/` → `crates/picus_view/`

**删除：**
- `crates/picus_masonry/`

**修改：**
- `Cargo.toml`（workspace）
- `crates/picus_widget/Cargo.toml`
- `crates/picus_view/Cargo.toml`
- `crates/picus_view/src/lib.rs`
- `crates/picus_masonry/Cargo.toml`（过渡期临时更新，随后删除）
- `crates/picus_core/Cargo.toml`
- `crates/picus_core/src/`（27 个 .rs 文件）
- `examples/shared_utils/src/lib.rs`
- `examples/game_2048/src/main.rs`

---

## Task 1：将 `picus_ui_runtime` 重命名为 `picus_widget`

**Files:**
- Rename: `crates/picus_ui_runtime/` → `crates/picus_widget/`
- Modify: `crates/picus_widget/Cargo.toml`
- Modify: `Cargo.toml`（workspace members + dep）
- Modify: `crates/picus_masonry/Cargo.toml`（临时更新，dep 指向新名）

- [ ] **Step 1: git mv 重命名目录**

```bash
git mv crates/picus_ui_runtime crates/picus_widget
```

- [ ] **Step 2: 更新 `crates/picus_widget/Cargo.toml` 中的包名**

将第 2 行 `name = "picus_ui_runtime"` 改为：

```toml
name = "picus_widget"
```

- [ ] **Step 3: 更新 workspace `Cargo.toml`**

将成员路径从 `"crates/picus_ui_runtime"` 改为 `"crates/picus_widget"`：

```toml
# 第 7 行原文：
"crates/picus_ui_runtime",
# 改为：
"crates/picus_widget",
```

将 workspace dep 从 `picus_ui_runtime` 改为 `picus_widget`（第 81 行附近）：

```toml
# 原文：
picus_ui_runtime = { path = "crates/picus_ui_runtime" }
# 改为：
picus_widget = { path = "crates/picus_widget" }
```

- [ ] **Step 4: 临时更新 `crates/picus_masonry/Cargo.toml`**

（`picus_masonry` 在 Task 3 中才删除，需先让它能通过编译）

```toml
# 原文：
picus_ui_runtime.workspace = true
# 改为：
picus_widget.workspace = true
```

同时更新 `src/lib.rs` 中的引用（`picus_masonry/src/lib.rs`）：

```rust
// 原文：
pub use picus_ui_runtime::retained::*;
// 改为：
pub use picus_widget::retained::*;
```

- [ ] **Step 5: 验证编译通过**

```bash
cd /c/Users/Summp/source/repos/picus
cargo check -p picus_widget 2>&1 | tail -5
```

期望输出：`Finished` 或仅有警告，无 error。

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor: rename picus_ui_runtime to picus_widget"
```

---

## Task 2：将 `xilem_masonry` 重命名为 `picus_view`，更新其 lib.rs

**Files:**
- Rename: `crates/xilem_masonry/` → `crates/picus_view/`
- Modify: `crates/picus_view/Cargo.toml`
- Modify: `crates/picus_view/src/lib.rs`
- Modify: `Cargo.toml`（workspace members + dep）

- [ ] **Step 1: git mv 重命名目录**

```bash
git mv crates/xilem_masonry crates/picus_view
```

- [ ] **Step 2: 替换 `crates/picus_view/Cargo.toml` 全部内容**

```toml
[package]
name = "picus_view"
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"

[lib]
test = false
doctest = false

[dependencies]
masonry = { package = "picus_widget", path = "../picus_widget" }
xilem_core = { git = "https://github.com/linebender/xilem.git", rev = "72c6517ad035f352c70f939f4754a3f79fca23fd" }
smallvec = "1.15.1"
tokio = { version = "1.52.3", features = ["rt", "rt-multi-thread", "time", "sync"] }
tracing = "0.1"
```

注意：内部别名 `masonry` 保留，指向 `picus_widget`，因此 `src/view/*.rs` 中的全部 `masonry::` 引用无需任何修改。

- [ ] **Step 3: 更新 `crates/picus_view/src/lib.rs`**

将原来的：
```rust
pub extern crate masonry;
pub use xilem_core as core;
```
改为：
```rust
pub use picus_widget;
pub use picus_widget::masonry_core;
pub use xilem_core as core;
```

同时将文件头部注释改为：
```rust
//! Picus-native Xilem view layer, targeting [`picus_widget`].
```

完整修改后的 `lib.rs`：
```rust
// Copyright 2026 Summp
// SPDX-License-Identifier: Apache-2.0

//! Picus-native Xilem view layer, targeting [`picus_widget`].

#![forbid(unsafe_code)]
#![allow(
    clippy::all,
    reason = "Vendored upstream view adapter code is kept close to the source while Picus integration tests cover its behavior."
)]
#![expect(
    missing_debug_implementations,
    reason = "Vendored upstream view types are intentionally light on Debug impls."
)]
#![expect(clippy::missing_assert_message, reason = "Vendored upstream behavior.")]

pub use picus_widget;
pub use picus_widget::masonry_core;
pub use xilem_core as core;

pub mod style;
pub mod view;

mod any_view;
mod masonry_root;
mod one_of;
mod pod;
mod view_ctx;
mod widget_view;

pub use any_view::AnyWidgetView;
pub use masonry_root::{InitialRootWidget, MasonryRoot};
pub use pod::Pod;
pub use view_ctx::ViewCtx;
pub use widget_view::{WidgetView, WidgetViewSequence};

// TODO - Remove these re-exports and fix the places in the crate that use them
pub(crate) use masonry::parley::Alignment as TextAlign;
pub(crate) use masonry::peniko::Color;
pub(crate) use masonry::widgets::InsertNewline;
```

- [ ] **Step 4: 更新 workspace `Cargo.toml`**

成员路径（第 9 行附近）：
```toml
# 原文：
"crates/xilem_masonry",
# 改为：
"crates/picus_view",
```

workspace dep（第 52 行附近）：
```toml
# 原文：
xilem_masonry = { path = "crates/xilem_masonry" }
# 改为：
picus_view = { path = "crates/picus_view" }
```

- [ ] **Step 5: 验证编译通过**

```bash
cargo check -p picus_view 2>&1 | tail -5
```

期望输出：`Finished` 或仅有警告，无 error。

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor: rename xilem_masonry to picus_view, wire to picus_widget"
```

---

## Task 3：删除 `picus_masonry`

**Files:**
- Delete: `crates/picus_masonry/`
- Modify: `Cargo.toml`（workspace members 移除）

- [ ] **Step 1: 从 workspace members 中移除**

编辑根 `Cargo.toml`，删除成员行：
```toml
# 删除这行：
"crates/picus_masonry",
```

- [ ] **Step 2: 删除目录**

```bash
git rm -r crates/picus_masonry
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: remove picus_masonry facade crate"
```

---

## Task 4：更新 `picus_core` 依赖声明

**Files:**
- Modify: `crates/picus_core/Cargo.toml`

- [ ] **Step 1: 替换 `xilem_masonry` 依赖为 `picus_view`**

在 `crates/picus_core/Cargo.toml` 中：
```toml
# 原文：
xilem_masonry.workspace = true
# 改为：
picus_view.workspace = true
```

- [ ] **Step 2: 验证 Cargo 解析（不要求代码通过，只检查依赖树）**

```bash
cargo tree -p picus_core 2>&1 | grep "picus_view\|picus_widget" | head -10
```

期望：能看到 `picus_view` 和 `picus_widget` 出现在依赖树中。

- [ ] **Step 3: Commit**

```bash
git add crates/picus_core/Cargo.toml
git commit -m "refactor(picus_core): replace xilem_masonry dep with picus_view"
```

---

## Task 5：批量替换 `picus_core/src` 中的导入路径

**Files:** `crates/picus_core/src/` 下的 27 个 .rs 文件（见下方列表）

两步替换策略：
1. 先替换 `xilem_masonry::masonry::` → `picus_view::picus_widget::`（3 处）
2. 再替换 `xilem_masonry` → `picus_view`（其余所有处）

- [ ] **Step 1: 替换 `xilem_masonry::masonry::` 引用（3 个文件）**

```bash
cd /c/Users/Summp/source/repos/picus
sed -i 's/xilem_masonry::masonry::/picus_view::picus_widget::/g' \
  crates/picus_core/src/projection/elements.rs \
  crates/picus_core/src/styling.rs \
  crates/picus_core/src/xilem.rs
```

验证替换结果：
```bash
grep -n "picus_view::picus_widget::" \
  crates/picus_core/src/projection/elements.rs \
  crates/picus_core/src/styling.rs \
  crates/picus_core/src/xilem.rs
```

期望：每个文件各显示 1 行带 `picus_view::picus_widget::` 的内容。

- [ ] **Step 2: 替换剩余 `xilem_masonry` 引用（全部 27 个文件）**

```bash
find crates/picus_core/src -name "*.rs" -exec \
  sed -i 's/xilem_masonry/picus_view/g' {} +
```

- [ ] **Step 3: 验证 `picus_core/src` 中不再有旧引用**

```bash
grep -rn "xilem_masonry" crates/picus_core/src/
```

期望：无任何输出。

- [ ] **Step 4: 验证 `picus_core` 编译通过**

```bash
cargo check -p picus_core 2>&1 | tail -10
```

期望：`Finished` 或仅有警告，无 error。

如果有编译错误，逐条检查错误信息，定位到具体文件和行号，手动修正路径（最常见的问题是某个深层 re-export 路径需要调整）。

- [ ] **Step 5: Commit**

```bash
git add crates/picus_core/src/
git commit -m "refactor(picus_core): replace xilem_masonry imports with picus_view"
```

---

## Task 6：更新 examples

**Files:**
- Modify: `examples/shared_utils/src/lib.rs`
- Modify: `examples/game_2048/src/main.rs`

- [ ] **Step 1: 更新 `shared_utils/src/lib.rs` 日志过滤字符串**

文件第 5 行的 `DEFAULT_LOG_FILTER` 常量，用 sed 批量替换：

```bash
sed -i \
  -e 's/picus_ui_runtime/picus_widget/g' \
  -e 's/picus_masonry=info,//g' \
  -e 's/xilem_masonry/picus_view/g' \
  examples/shared_utils/src/lib.rs
```

验证结果：
```bash
grep "DEFAULT_LOG_FILTER" examples/shared_utils/src/lib.rs
```

期望：字符串中出现 `picus_widget` 和 `picus_view`，不再有 `picus_ui_runtime`、`picus_masonry`、`xilem_masonry`。

- [ ] **Step 2: 更新 `game_2048/src/main.rs`**

game_2048 通过 `use picus_core::{..., xilem_masonry::{...}, ...}` 访问视图层。将其改为 `picus_view`:

```bash
sed -i 's/xilem_masonry/picus_view/g' examples/game_2048/src/main.rs
```

验证：
```bash
grep -n "picus_view\|xilem_masonry" examples/game_2048/src/main.rs
```

期望：看到 `picus_view::{...}`，无 `xilem_masonry`。

- [ ] **Step 3: Commit**

```bash
git add examples/shared_utils/src/lib.rs examples/game_2048/src/main.rs
git commit -m "refactor(examples): update log filter and imports for picus_view rename"
```

---

## Task 7：全量构建验证

- [ ] **Step 1: 确认 workspace 中无旧 crate 引用**

```bash
grep -rn "xilem_masonry\|picus_masonry\|picus_ui_runtime" \
  Cargo.toml crates/ examples/ \
  --include="*.toml" --include="*.rs" 2>/dev/null
```

期望：无任何输出。

- [ ] **Step 2: 完整 workspace 构建**

```bash
cargo build --workspace 2>&1 | tail -20
```

期望：`Finished` 无 error。

- [ ] **Step 3: 构建所有 examples**

```bash
cargo build --examples 2>&1 | tail -20
```

期望：`Finished` 无 error。

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: full-workspace build verified after crate split reorganization"
```

---

## 自检：规格覆盖

| 规格要求 | 对应任务 |
|---|---|
| `picus_ui_runtime` → `picus_widget` | Task 1 |
| `picus_masonry` 删除 | Task 3 |
| `xilem_masonry` → `picus_view` | Task 2 |
| `picus_view/lib.rs` 暴露 `picus_widget` + `masonry_core` | Task 2 Step 3 |
| 外部 masonry dep 仅 `masonry_core` | Task 2 Step 2（Cargo.toml alias） |
| `picus_core` 使用 `picus_view` | Task 4 + Task 5 |
| `picus_surface`/`picus_activation` 不改动 | 无对应任务（正确） |
| workspace 全量构建通过 | Task 7 |
