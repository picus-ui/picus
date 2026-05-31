# Crate 模块拆分方案

**日期：** 2026-06-01  
**状态：** 已批准

## 背景

`crates/xilem_masonry` 是从 `../xilem/xilem_masonry` 直接复制而来的上游代码，通过将 `masonry` 依赖重定向到本地的 `picus_ui_runtime` 来驱动。目标是彻底去除对上游 `masonry`（高层封装）和 `xilem_masonry` 的依赖，仅保留 `masonry_core` 与 `xilem_core` 作为外部 UI 相关依赖，其余全部在 Picus 内部实现。架构参考 MewUI 的分层设计（Core → Platform → Backend 单向依赖）。

## 目标

- 外部 UI 依赖仅剩：`masonry_core`、`xilem_core`
- 三个旧 crate（`picus_ui_runtime`、`picus_masonry`、`xilem_masonry`）全面重组为两个新 crate
- 依赖方向严格单向，无循环
- `picus_core` 及平台层（`picus_surface`、`picus_activation`）最小化改动

## 选定方案：Widget / View / Core 严格三层

### 整体依赖图

```
外部依赖
  masonry_core  ──────────────────────────┐
  xilem_core    ───────────────────────┐  │
                                       │  │
crates/                                │  │
  picus_widget  ◄── masonry_core ──────┘  │
       │                                  │
       ▼                                  │
  picus_view    ◄── xilem_core ───────────┘
       │
       ▼
  picus_core
       │
       ├──► picus_surface
       └──► picus_activation
```

**关键约束：**
- `picus_widget` 对 `xilem_core` 完全无感知
- `picus_view` 不在自身 `Cargo.toml` 直接声明 `masonry_core`，通过 `picus_widget` 的 `pub use masonry_core` 透传获取 masonry 类型
- `picus_core` 只需将 `xilem_masonry` 依赖替换为 `picus_view`

## crate 详细设计

### `picus_widget`（新增）

**来源：** `picus_ui_runtime` 重命名 + `picus_masonry` 内容合并  
**职责：** 在 `masonry_core` 之上实现 Picus 的全部 widget，包含主题、属性系统、层管理。

**Cargo.toml：**
```toml
[package]
name = "picus_widget"

[dependencies]
masonry_core = { workspace = true }
smallvec     = { workspace = true }
tracing      = { workspace = true }
accesskit    = { workspace = true }
```

**模块结构：**
```
src/
  lib.rs
  theme.rs                 ← picus_ui_runtime/src/theme.rs
  properties/
    mod.rs                 ← picus_ui_runtime/src/properties/
    *.rs
  layers/
    mod.rs                 ← picus_ui_runtime/src/layers/
    *.rs
  widgets/
    mod.rs
    *.rs                   ← picus_ui_runtime/src/widgets/ + picus_masonry/src/ 全部合并
```

**公开 API（`lib.rs`）：**
```rust
pub use masonry_core;   // 透传，供 picus_view 使用

pub mod theme;
pub mod properties;
pub mod layers;
pub mod widgets;
```

---

### `picus_view`（新增）

**来源：** `xilem_masonry` 重命名 + 重组  
**职责：** 在 `picus_widget` + `xilem_core` 之上实现响应式视图层，完整替代 `xilem_masonry`。

**Cargo.toml：**
```toml
[package]
name = "picus_view"

[dependencies]
picus_widget = { path = "../picus_widget" }
xilem_core   = { workspace = true }
smallvec     = { workspace = true }
tokio        = { workspace = true, features = ["rt", "rt-multi-thread", "time", "sync"] }
tracing      = { workspace = true }
```

**模块结构：**
```
src/
  lib.rs
  view_ctx.rs
  widget_view.rs
  any_view.rs
  pod.rs
  masonry_root.rs
  one_of.rs
  style.rs
  views/
    mod.rs
    # 布局
    flex.rs  grid.rs  split.rs  zstack.rs  virtual_scroll.rs
    # 输入
    button.rs  text_input.rs  checkbox.rs  radio_button.rs
    radio_group.rs  slider.rs  switch.rs
    # 展示
    label.rs  image.rs  prose.rs  divider.rs
    progress_bar.rs  spinner.rs
    # 功能性
    badge.rs  badged.rs  canvas.rs  portal.rs
    task.rs  transform.rs  sized_box.rs  prop.rs
```

**公开 API（`lib.rs`）：**
```rust
pub use picus_widget;
pub use picus_widget::masonry_core;
pub use xilem_core as core;

pub mod style;
pub mod views;

pub use any_view::AnyWidgetView;
pub use masonry_root::{InitialRootWidget, MasonryRoot};
pub use pod::Pod;
pub use view_ctx::ViewCtx;
pub use widget_view::{WidgetView, WidgetViewSequence};
```

---

### `picus_core`（保留，最小改动）

只需将 `Cargo.toml` 中的 `xilem_masonry` 依赖替换为 `picus_view`，业务逻辑代码不变。

### `picus_surface`、`picus_activation`（保留，不动）

## 迁移对照表

| 源 crate | 动作 | 目标 |
|---|---|---|
| `picus_ui_runtime` | 重命名 + 重组 | `picus_widget` |
| `picus_masonry` | 合并 | `picus_widget/src/widgets/` |
| `xilem_masonry` | 重命名 + 重组 | `picus_view` |
| `picus_core` | 更新依赖声明 | `xilem_masonry` → `picus_view` |
| `picus_surface` | 不动 | — |
| `picus_activation` | 不动 | — |

**删除：** `crates/picus_ui_runtime/`、`crates/picus_masonry/`、`crates/xilem_masonry/`

## Workspace `Cargo.toml` 变更

```toml
# members 中移除：
#   crates/picus_ui_runtime
#   crates/picus_masonry
#   crates/xilem_masonry
# members 中新增：
#   crates/picus_widget
#   crates/picus_view

# 移除旧的 masonry 别名依赖（原来 xilem_masonry 用的）：
# masonry = { package = "picus_ui_runtime", ... }

# 新增 workspace deps：
[workspace.dependencies]
picus_widget = { path = "crates/picus_widget" }
picus_view   = { path = "crates/picus_view" }
```

## 成功标准

- `cargo build -p picus_view` 通过
- `cargo build -p picus_core` 通过
- `cargo build --workspace` 通过
- workspace 中不再有 `masonry`（上游高层封装）或 `xilem_masonry` 的依赖引用
- 所有 examples 编译通过
