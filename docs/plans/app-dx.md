# Picus 应用层 DX 完整计划

> **状态**：草案（修订）  
> **位置**：`docs/plans/app-dx.md` — 全部改进项的单一事实来源。  
> **用法**：条目扁平列出，**做什么、先做什么由你决定**。完成某项后勾选即可。本文不规定轮次或 PR 切分。

---

## 0. 背景

Picus 内核（Bevy ECS + 投影 + 保留式 Masonry + Fluent 样式）自洽，但**应用作者路径偏重**：自定义 `UiComponentTemplate`、逐个 `register_*`、手写 `UiEventQueue::drain_actions`、主题与窗口配置入口分散。

目标：

- 用 **宏 + App builder 类设施 + Bevy 原生消息** 压低样板  
- **所有 examples 能迁则迁**；复杂例子至少把简单壳/设置/按钮路径迁过去  
- **不改变「无主题则不显示」的视觉契约**；主题配置入口与 builder 对齐  
- 复杂能力（自定义投影、overlay、流式 Markdown）仍完整保留  
- **文档分层**：`AGENTS.md` 只保留可执行契约与指针；架构/教程/参考进 `docs/`；README 做入口  

```text
┌──────────────────────────────────────────────────┐
│  应用层 DX                                         │
│  macros · app builder · theme · Message · 文档分层  │
│  样式糖 · 投影 helper · 组合控件 · 全 examples 迁移  │
└────────────────────────┬─────────────────────────┘
                         │
┌────────────────────────▼─────────────────────────┐
│  内核                                              │
│  PicusPlugin · UiEventQueue · UiComponentTemplate  │
│  BSN · 样式/主题管线 · MasonryRuntime · 投影失效    │
└──────────────────────────────────────────────────┘

文档角色（目标态）：

  README.md     → 定位、安装、Quick start、链到 docs/
  AGENTS.md     → 短：强制规则、禁止项、链到 docs 权威章节
  docs/**       → 完整：架构、应用指南、样式、运行时、示例索引、宏参考
  rustdoc       → API 级细节（与 docs 概念文互补，不重复长教程）
```

---

## 1. 问题对照

| 摩擦 | 说明 |
|------|------|
| 双层描述 | BSN 实体树 + `project` 再拼 view |
| 显式注册 | 漏 `register_ui_component` / `register_projection_resource` 即不刷新 |
| 事件管道 | 每 app 手写 drain + match；与 Bevy `Message` 生态脱节 |
| 主题入口分散 | `load_style_sheet_ron` / `set_active_style_variant_by_name` 与窗口 builder 未统一 |
| 自定义区块膨胀 | 每区块 Component + template + 注册 + class |
| 示例口径不一 | 新读者不知「推荐路径」；迁移未覆盖全库 |
| 文档职责错位 | `AGENTS.md` 塞满业务/运行时叙述（对 agent 过重）；人类可读文档又不足；README Quick start 仍是旧 drain 模式 |

### 成功度量

| ID | 目标 |
|----|------|
| G1 | 业务消息可用 Bevy `MessageReader` / 标准 system 消费（见 §5.2 设计结论） |
| G2 | 宏消除常规 register / 投影依赖 / 常见样板；手写路径仍可用 |
| G3 | 主题**不会**在无配置时偷偷启用；builder 提供清晰的主题加载/选 variant API |
| G4 | 全部 examples 按 §5.8 迁移或标注「已部分迁移」 |
| G5 | **文档分层完成**：`AGENTS.md` 瘦身；`docs/` 成为架构与教程权威源；与 DX 契约一致 |
| G6 | 减少无谓双层写作（helper、组合控件、文档指引） |
| G7 | 新人/agent 能从 README → docs → example 走通，而不必通读现行超长 `AGENTS.md` |

---

## 2. 非目标

- 改成 immediate-mode  
- 去掉 BSN / ECS / 投影失效契约  
- 重写 `picus_widget` 或废弃 Fluent RON 体系  
- **框架在无主题时自动选 dark/light**（保持「无主题 ≈ 无可见样式」）  
- 新增独立 `examples/minimal` 作为「唯一正确示例」  
- 任意闭包挂在 Component 上（Send/生命周期问题）  
- 与 React/SwiftUI 语法级兼容  

---

## 3. 默认决策

| # | 议题 | 决定 |
|---|------|------|
| D1 | 无主题时的外观 | **什么都不显示**（透明/无填充）；应用必须加载 sheet 和/或选 variant。框架**不**默认 dark。 |
| D2 | 主题入口 | 与 app builder / `AppPicusExt` 统一：加载 RON、选 variant、可选 backdrop 覆盖等链式或显式 API；与 `run_app*` 窗口选项兼容。 |
| D3 | 示例策略 | **不**单独做 minimal；**全部 examples 迁移**；复杂例（gallery、picuscode）至少迁移简单壳、设置、通用按钮路径。 |
| D4 | 业务消息 | **与 Bevy `Message` 结合**（见 §5.2）；`UiEventQueue` 保留为控件/保留式回调的入队与类型擦除层，并桥到 `Message`。 |
| D5 | 按钮负载 | 组件侧可挂 `UiEmit<T>`（或等价）；最终对应用以 `Message`（及 source `Entity`）形式可见。 |
| D6 | 宏 | **必做**（见 §5.4 详细设计）。 |
| D7 | 闭包实体 | 不做。 |
| D8 | 文档 | **DX 必做部分**（见 §5.9）。`AGENTS.md` 不承载长篇业务说明；权威叙述在 `docs/`。 |

### 主题优先级（仅在有配置时）

1. 应用显式 `set_active_style_variant_by_name` / builder 已选 variant  
2. 已加载 stylesheet 的 `default_variant`  
3. **无回退**——不自动 dark；未选 variant 则不应用 variant 规则（与现内核一致）

### 样式使用分层（文档）

| 层级 | 用途 |
|------|------|
| 0 | 无主题 = 无可见默认装饰（契约） |
| 1 | 应用通过 builder/API 加载 Fluent bundle 或 RON 并选 variant |
| 2 | Inline / builder 局部样式 |
| 3 | class + 应用 RON override |
| 4 | 完整多品牌 stylesheet |

---

## 4. 目标作者路径（相关能力落地后）

```rust
#[derive(Clone, Debug, Message)]
enum AppMsg {
    Inc,
    Dec,
}

#[derive(Resource, Default)]
struct Count(i32);

#[derive(Component, UiComponent)] // 宏：注册投影 + 依赖
struct CountLabel;

impl UiComponentTemplate for CountLabel {
    fn project(_: &Self, ctx: ProjectionCtx<'_>) -> UiView {
        let n = ctx.world.resource::<Count>().0;
        // styled / classes 等糖
        Arc::new(label(format!("{n}")))
    }
}

fn on_app_msg(mut reader: MessageReader<UiMessage<AppMsg>>, mut count: ResMut<Count>) {
    for UiMessage { action, .. } in reader.read() {
        match action {
            AppMsg::Inc => count.0 += 1,
            AppMsg::Dec => count.0 -= 1,
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        UiRoot
        UiFlexColumn
        Children [
            CountLabel,
            (UiButton { label: { "+".into() } } UiEmit(AppMsg::Inc)),
            (UiButton { label: { "-".into() } } UiEmit(AppMsg::Dec)),
        ]
    });
}

fn main() -> Result<(), EventLoopError> {
    picus::App::new("Counter") // 或 AppPicusExt 链式等价
        .theme_ron(include_str!("../assets/themes/app.ron")) // 显式主题；无此则无可视样式
        // .theme_variant("dark") 若 RON 未带 default_variant
        .register_ui_components::<CountLabel>() // 或由 inventory/自动收集
        .register_projection_resources::<Count>()
        .add_systems(Startup, setup)
        .add_systems(Update, on_app_msg)
        .run()
}
```

示意：业务侧用 **Bevy system + `MessageReader`**，而非每 app 手写 `drain_actions`。

---

## 5. 工作项清单

分类仅便于检索，**无隐含实现顺序**。

---

### 5.1 主题与 App builder 设施

**契约（不可破）**

- [ ] 文档与测试固定：**未加载主题 / 未选 variant 时，控件不出现框架默认可见填充与文字色**（「什么都不显示」）  
- [ ] **删除或永不引入**「框架默认 dark」类行为  

**Builder / 入口（必做方向）**

- [ ] 统一主题相关 API，与 app 启动路径兼容，例如（命名可定）：
  - `load_style_sheet_ron` / `theme_ron` / `theme_asset_path`
  - `style_variant("dark" | "light" | …)`
  - 与现有 `set_active_style_variant_by_name`、`apply_active_stylesheet_ron` 的关系写清（builder 为糖，内核 API 仍在）
  - **（可选同批）** backdrop：`theme_backdrop` / `clear_theme_backdrop_override` 进入同一入口族
- [ ] 与 `run_app` / `run_app_with_window_options` / 未来 `picus::App` builder 组合方式文档化：先配主题再 `run`，避免「窗口起来了但忘了主题」时误以为框架坏了——应表现为空白，文档说明原因  
- [ ] 示例一律**显式**加载主题（现状多数已有 RON；迁移时保持显式，不依赖隐式默认）

**非目标**

- 无配置时自动启用 Fluent dark/light  

---

### 5.2 控件消息与 Bevy `Message`（设计结论 + 工作项）

仓库已在 Bevy **0.19**（`MessageReader` / `MessageWriter`，见 accelerator 等）。应用侧应与之对齐。

#### 5.2.1 现状

| 层 | 机制 | 特点 |
|----|------|------|
| 控件 / 保留式回调 | `UiEventQueue`（`SegQueue` 类型擦除） | 可在非 Bevy system 上下文 lock-free 入队；`drain_actions::<T>()` 非破坏/按类型抽 |
| 应用 | 手写 PreUpdate drain | 与 Bevy 调度、多监听者、`MessageReader` 生态脱节 |
| 引擎输入 | Bevy `Message` | 标准 system 参数 |

#### 5.2.2 结论（推荐架构）

**要结合 Bevy `Message`，但不要用 Message 直接替换控件入队通道。**

1. **保留 `UiEventQueue` 作为控件发射端**  
   - 投影/按钮/`emit_ui_action` 仍可推队列  
   - 适配 retained 回调、多线程边界、类型擦除多 payload  

2. **增加桥接：队列 → Bevy `Message`**  
   - 在 `PreUpdate`（widget action 路由之后）将本帧入队的应用消息 **写入 Bevy message 通道**  
   - 应用用 `MessageReader<…>` 在 `Update`（或明确 schedule）里处理  
   - 同一消息类型可被多个 system 读取（符合 Bevy 习惯）；若需「只处理一次」用 Bevy 既有 reader 语义  

3. **应用可见类型（建议）**  
   - 方案 A（推荐）：`UiMessage<T> { entity: Entity, action: T }` 且 `T: Message` 或 `UiMessage<T>: Message`  
   - 方案 B：应用直接 `#[derive(Message)] enum AppMsg`，桥接时 `MessageWriter<AppMsg>` 只写 payload，source entity 另用 `UiMessageMeta` 或并行 message  
   - **倾向 A**：保留 entity 来源，marker 查询与列表项删除仍方便  

4. **注册**  
   - `app.add_message::<UiMessage<AppMsg>>()`（或宏/`add_picus_messages::<AppMsg>()` 一次完成 add_message + 桥接系统）  
   - 未注册的类型可继续只存在于 `UiEventQueue`（兼容旧 drain），或 debug 下告警  

5. **`UiEmit<T>`**  
   - 组件数据：点击时 `emit` → 入 `UiEventQueue` → 桥 → `MessageWriter`  
   - 与 `BuiltinUiAction::Clicked`：无 `UiEmit` 时仍发 Builtin（若 Builtin 也走 Message，同样桥接）  

6. **不必强行的**  
   - 控件内部每一跳都 `MessageWriter`（回调里常无 `World`）  
   - 立刻删除 `drain_actions`（迁移期与测试保留；文档标为底层/兼容）  

#### 5.2.3 工作项

- [ ] 设计并实现 `UiMessage<T>`（或最终命名）及 `Message` 实现边界  
- [ ] `add_picus_messages::<T>()` / builder 等价：注册 message 类型 + 安装 queue→message 桥  
- [ ] 桥接系统 schedule 与顺序文档 + 测试（同帧：点击 → 桥 → reader 可见）  
- [ ] `UiEmit<T>` 与 `UiButton`（及其他点击控件按需）对接  
- [ ] 保留 `UiEventQueue::drain_actions` 与 `emit_ui_action`；文档区分「底层队列」与「应用 Message」  
- [ ] 指针类内部事件（`UiPointerHitEvent` 等）是否上 Message：**默认可仍走队列**；若统一上 Message 需单独评估流量与 schedule，不阻塞应用消息路径  
- [ ] 迁移指南：手写 drain → `MessageReader<UiMessage<T>>`  

#### 5.2.4 验收

- 应用 system 仅依赖 Bevy `MessageReader` 即可处理按钮业务，无需 `ResMut<UiEventQueue>`  
- 旧 drain 测试与路径仍可通过  

---

### 5.3 控件绑定与内置动作

- [ ] `UiEmit<T>`（BSN 可挂）  
- [ ] 语义：有 `UiEmit<T>` → 业务 `T`；否则 `BuiltinUiAction::Clicked`（若采用 Message，二者都经桥）  
- [ ] 低层 `button(entity, T, label)` 继续可用，发出同一 `T`  
- [ ] disabled 按钮不发射（与现行为一致）  

---

### 5.4 宏改造（**必做**，详细设计）

宏是压低样板的主路径，不是可选项。建议新建 workspace crate：

```text
crates/picus_macros/   # proc-macro crate
picus / picus_core     # 重导出宏与配套 trait/注册表
```

#### 5.4.1 目标能力一览

| 宏 | 性质 | 解决的问题 |
|----|------|------------|
| `#[derive(UiComponent)]` | proc-macro derive | 自动接入 `UiComponentTemplate` 注册与投影依赖 |
| `#[derive(ProjectionResource)]` 或属性 | proc-macro | Resource 变更驱动投影，免手写 `register_projection_resource` |
| `#[derive(PicusMessage)]` 或封装 | proc-macro | `Message` + 可选 `UiMessage` 胶水、注册桥 |
| `register_ui_components!(A, B, C)` | macro_rules 或 proc | 批量注册（无 inventory 时的显式列表） |
| `classes!("a", "b")` | macro_rules | `StyleClass` 构造 |
| **（可选增强）** `picus_app!` / builder 辅助 | 视需要 | 减少 main 样板 |

手写 `register_ui_component` / `impl` 注册路径**永久保留**（宏生成代码的可读 fallback）。

#### 5.4.2 `#[derive(UiComponent)]`

**输入约束**

- 目标类型已是 `Component`（可要求用户同时 `#[derive(Component)]`，或宏再展开 Component——**推荐用户显式 `Component`**，宏只做 Picus 侧）  
- 已实现 `UiComponentTemplate`（derive **不**生成 `project` 函数体；投影逻辑仍手写）  
- 满足 BSN 时：`Default + Clone` 由用户负责；宏可在文档/可选 lint 中提醒  

**生成内容（建议）**

1. **注册入口**（二选一或组合，实现时定一种主策略）：  
   - **策略 Inventory（推荐优先评估）**  
     - 用 `inventory` 或 `linkme` 提交 `fn register(app: &mut App)`  
     - `PicusPlugin` / `app.finalize_picus()` / builder `build` 时 `inventory::iter` 全部注册  
     - 优点：零主函数列表；缺点：依赖 link 行为、部分平台/工具需验证（Windows MSVC 已常见可用）  
   - **策略 显式批量（必做保底）**  
     - 宏生成 `CountLabel::register_picus(app)` 或 `picus_register_ui_component::<CountLabel>(app)`  
     - `register_ui_components!(CountLabel, Foo, Bar)` 展开为多次调用  
     - 不依赖 linkme，CI 最稳  

2. **投影依赖**  
   - 属性可选：`#[ui_component(resources(Count, Draft))]`  
   - 生成 `UiComponentTemplate::register_projection_dependencies` 的默认 impl 扩展，或生成在 `register` 时调用 `register_projection_resource::<Count>()`  
   - 若已手写 `register_projection_dependencies`，宏需 `#[ui_component(manual_deps)]` 跳过冲突  

3. **类型名样式别名（若 StyleTypeRegistry 需要）**  
   - 可选 `#[ui_component(style_name = "todo.item")]` 注册选择器用名  

**示例展开（示意）**

```rust
#[derive(Component, Clone, Default, UiComponent)]
#[ui_component(resources(Count))]
struct CountLabel;

// 宏生成大致：
impl CountLabel {
    pub fn register_picus(app: &mut App) {
        app.register_ui_component::<Self>();
        app.register_projection_resource::<Count>();
    }
}
// inventory 策略下另生成 submit 节点
```

**测试**

- [ ] trybuild：缺少 `UiComponentTemplate` 的清晰报错  
- [ ] 集成：仅 `register_picus` / inventory 收集后投影可用  
- [ ] resources 属性确实触发 Resource 变更后的合成 dirty  

#### 5.4.3 `#[derive(ProjectionResource)]`（或 `#[projection_resource]`）

- [ ] 标记在 `#[derive(Resource)]` 类型上  
- [ ] 生成注册 helper 与/或 inventory 提交  
- [ ] 与 `UiComponent` 的 `resources(...)` 去重（重复 register 应幂等）  

#### 5.4.4 消息相关宏

- [ ] 应用枚举：`#[derive(Clone, Debug, Message)]` 用 Bevy 官方 derive；Picus 侧提供：  
  - `#[derive(PicusMessage)]` **或** `add_picus_messages::<T>()` 不强制额外 derive  
- [ ] 若采用 `UiMessage<T>`：文档说明 `T` 的 bound（`Clone + Send + Sync + 'static`，及 Message 包装）  
- [ ] **（可选）** `#[picus_message]` 在一个属性里完成：Bevy Message 断言 + 注册桥接的 inventory 项  

最小必做：**`add_picus_messages::<T>()` API + 文档**；derive 糖在同一宏 crate 内做完更佳。

#### 5.4.5 `classes!` 与轻量 macro_rules

- [ ] `classes!("todo.item", "selected")` → `StyleClass(vec![…])`  
- [ ] 可放在 `picus` 的 `macro_rules`，不必进 proc-macro crate  
- [ ] 与 BSN 字段 patch 兼容  

#### 5.4.6 批量注册宏

- [ ] `register_ui_components!(A, B, C)`  
- [ ] `register_projection_resources!(X, Y)`  
- [ ] 与 derive 生成的 `register_picus` 一致  

#### 5.4.7 工程与导出

- [ ] 根 `Cargo.toml` members + workspace 依赖  
- [ ] `picus::UiComponent` / `picus::classes` 重导出，应用只依赖 `picus`  
- [ ] `picus_core` 不强制依赖 proc-macro（避免核心编译变慢）：宏在 `picus` facade 导出；生成代码调用 `picus_core` / `AppPicusExt` 公开 API  
- [ ] 文档：`docs` 一节「宏参考」+ 每个宏的失败模式  
- [ ] `Agents.md`：新组件用 `#[derive(UiComponent)]`，禁止教手写长注册链（除非底层）  

#### 5.4.8 宏验收

- [ ] 至少一个完整 example（如 calculator 或 timer）**去掉**手写 `register_ui_component` 长链，改用 derive + 批量/自动注册  
- [ ] 无 inventory 平台问题时的 fallback 路径有文档  
- [ ] CI 含 macros crate 测试 + trybuild  

#### 5.4.9 明确不在宏范围（初期）

- 不生成 `project` 函数体（UI 结构仍手写或 BSN）  
- 不生成完整 Bevy `Component` 反射全套（除非已有基础设施）  
- 不做 JSX 式 `view!` 宏（可列为更远期，不进本必做清单）  

---

### 5.5 样式 DX

- [ ] `classes!`（见宏节）  
- [ ] `styled(view, &resolved)` / `ctx.styled(view)`  
- [ ] InlineStyle builder 补齐常用缺口  
- [ ] 文档：无主题 = 无可见默认；样式层 0–4  

---

### 5.6 减少双层写作

- [ ] 投影 helper（flex 列/行 + 子节点）  
- [ ] 文档：何时不要拆 Component  
- [ ] 细粒度 vs 容器内 map 对照（todo 模式）  
- [ ] 组合控件（按 gallery 缺口）：`UiFormRow`、内容壳等  
- [ ] 不做闭包 Component  
- [ ] **（远期）** 函数组件宏 `#[ui_view]` — 非 §5.4 必做核心  

---

### 5.7 App builder（入口统一）

- [ ] `picus::App` 或加强的 `AppPicusExt` 链：  
  - 窗口标题 / size / `run_app*` 选项  
  - **主题加载与 variant**（§5.1）  
  - `add_picus_messages`  
  - 宏注册 finalize（inventory flush）  
  - `run`  
- [ ] 无主题时行为：可运行、UI 无框架默认可见样式（契约测试）  
- [ ] 不隐藏底层 Bevy `App` 逃逸舱（`into_inner()` / 先 `App::new()` 再 ext）  

---

### 5.8 示例迁移（无单独 minimal）

**原则**

- **不新增** `examples/minimal` 作为特殊圣地  
- **所有** `examples/*` 应用例：能迁移的全部迁移  
- **复杂例**：简单内容（标题栏按钮、主题切换入口、设置里的开关/按钮、通用 Message 路径）先迁；深度自定义投影可保留，但注册/消息应尽量新 API  

**清单（逐项勾选）**

| Example | 迁移期望 |
|---------|----------|
| [ ] `timer` | Message + 宏注册 + 显式主题；去掉手写 drain |
| [ ] `calculator` | 同上 |
| [ ] `todo_list` | 同上；动态实体可保留 template |
| [ ] `overlay_hit_routing` | Builtin/业务 Message 路径统一；宏若适用则用 |
| [ ] `async_downloader` | 按钮/对话框简单路径迁 Message；异步逻辑保留 |
| [ ] `game_2048` | 输入与按钮路径迁 Message；棋盘投影可保留 |
| [ ] `chess_game` | 同上 |
| [ ] `gallery` | **部分迁移**：壳、导航外围、demo 页里简单按钮/控件；复杂 page 可渐进 |
| [ ] `picuscode` | **部分迁移**：设置保存/标题栏/简单按钮；bridge 与流式 Markdown 保留 |
| [ ] `shared_utils` | 仅当有 UI 样板 helper 时跟着改 |

**每例验收**

- 显式主题（RON 或等价），无「靠框架默认主题」  
- 业务交互尽量 `MessageReader`，无新的手写 drain（除非测底层队列）  
- 自定义 `UiComponent` 用宏注册  

**文档**

- [ ] README / examples 说明：以 **timer 或 calculator** 等真实例为入门，不设 parallel minimal  
- [ ] 复杂例 README 注明「哪些子系统已迁、哪些仍是底层投影」  

---

### 5.9 文档体系改造（DX 必做）

#### 5.9.1 问题

现行 `AGENTS.md`（约 500+ 行）同时承担：

- agent/人的**强制流程与契约**（合适）  
- 仓库布局、CodeWhale 同步、运行时阶段、样式细节、控件清单、Markdown 实现等**业务与架构说明**（对 AGENTS 过重）  

结果：

| 读者 | 体验 |
|------|------|
| Agent | 上下文噪声大；真正「必须遵守」的规则被淹没 |
| 人类学框架 | 没有系统的 `docs/` 教程/架构文；只能啃 AGENTS 或翻代码 |
| README | Quick start 仍是旧式 drain + 手写 template，与目标 DX 脱节 |

文档改造与 API DX **同等重要**：没有正确文档，宏/Message 不会成为默认路径。

#### 5.9.2 分层原则

| 载体 | 应包含 | 不应包含 |
|------|--------|----------|
| **`AGENTS.md`** | 可执行规则、禁止项、公共契约摘要、**指向 `docs/` 的链接**、工作区成员注意点（极短） | 长篇运行时 walkthrough、控件百科、picuscode 产品说明、样式 token 清单 |
| **`docs/**`** | 架构、应用指南、子系统深潜、示例索引、迁移笔记、宏参考、主题契约 | 与代码冲突且不维护的过时细节；应链到 rustdoc 的 API 签名堆砌 |
| **`README.md`** | 一句话定位、安装、**与目标 DX 一致的** Quick start、文档地图、examples 入口 | 完整架构书 |
| **rustdoc (`picus`)** | 类型/函数契约、简短示例 | 跨 crate 系统设计长文 |
| **`docs/plans/*`** | 进行中的计划（本文）；完成后可归档或链到已落地 docs | 当作用户手册 |

**单一事实来源**：某一主题只在一处写全；其它处链接。避免 AGENTS 与 docs 双份长文漂移。

#### 5.9.3 目标目录结构（建议）

```text
docs/
  README.md                 # 文档地图（索引）
  architecture/
    overview.md             # Bevy + Masonry + 投影总览
    crates.md               # crate 边界与依赖方向
    runtime.md              # 调度阶段、窗口 runtime、绘制
    input-ime-hit.md        # 输入 / IME / hit testing
    projection.md           # 合成、失效、UiProjectorRegistry
  guide/
    app.md                  # 应用怎么写：builder、主题、Message、BSN
    components-bsn.md       # BSN、Default+Clone 契约、内置控件
    styling-themes.md       # 无主题不显示、RON、variant、token、backdrop
    events-messages.md      # UiEventQueue vs Bevy Message、UiEmit
    macros.md               # derive / classes! / 注册
    overlays-scroll.md
    markdown-streaming.md
    i18n-fonts-icons.md
    multi-window.md
    testing.md
    migration-dx.md         # 从 drain/手写 register 迁到新路径
  examples/
    index.md                # 各 example 教什么、迁移状态
  reference/
    public-modules.md       # picus facade 模块地图
    style-tokens.md         # 需要时从 AGENTS/主题拆出的参考表
  contributing/
    codewhale-submodule.md  # 从 AGENTS §1.1–1.2 迁出的完整流程
  plans/
    app-dx.md               # 本计划
```

实现时可合并邻近文件，但**主题归属**应覆盖上表；名称可微调。

#### 5.9.4 从现行 `AGENTS.md` 迁出的内容映射

| 现行 AGENTS 区块（约） | 迁往 | 根 `AGENTS.md` 仅保留 |
|------------------------|------|------------------------|
| §1 Workspace 长描述 / crate 列表 | `docs/architecture/crates.md` | 一句：依赖 `picus` facade；禁止 reintroduce 上游 masonry/xilem app crate；链到 crates.md |
| §1.1 picuscode / CodeWhale 产品与 bridge | `examples/picuscode/README.md` 与/或 `docs/examples/` | 一句：picuscode 为集成例；测试勿碰用户 `~/.codewhale/` |
| §1.2 同步 CodeWhale submodule | `docs/contributing/codewhale-submodule.md` | 链 + 「改 submodule 先读该文」 |
| §2 Runtime Architecture | `docs/architecture/runtime.md` | 可选 3–5 条**不变量**子弹（如 retained 不依赖 Bevy render graph） |
| §3 Input / IME / Hit | `docs/architecture/input-ime-hit.md` | 仅关键禁止项（若有） |
| §4 ECS UI Model / 控件清单 | `docs/guide/components-bsn.md` + rustdoc | BSN：`Default+Clone` 公共组件契约一条 |
| §5 BSN 迁移规则 | `docs/guide/components-bsn.md` | 同上契约 + 链 |
| §6 Synthesis / Events | `docs/architecture/projection.md` + `docs/guide/events-messages.md` | 投影依赖必须 register；应用优先 Message（落地后） |
| §7 Styling | `docs/guide/styling-themes.md` | **无主题不显示**；生产色来自 RON 非 widget 默认；链 |
| §8 Scroll / Overlay | `docs/guide/overlays-scroll.md` | 无则只链 |
| §8.1 Markdown | `docs/guide/markdown-streaming.md` | 无则只链 |
| §9 Assets / i18n / icons | `docs/guide/i18n-fonts-icons.md` | 无则只链 |
| §10 Surface | `docs/architecture/runtime.md` 或 `surface.md` | 无则只链 |
| §11 Plugin helpers | `docs/guide/app.md` | 链 |

#### 5.9.5 瘦身后的 `AGENTS.md` 建议结构

目标量级：**约 80–150 行**，而非 500+。

```markdown
# AGENTS.md
## 角色与范围          # 本文件是强制规则，不是教程
## 依赖与边界          # picus facade；禁止项；链 architecture/crates
## 应用默认写法        # Message + 宏 + 显式主题；禁止默认 dark；链 guide/app
## 必须遵守的契约      # BSN Default+Clone；投影依赖注册；无主题无可见默认；样式 token 来源
## 禁止项              # 列表短而硬
## 文档地图            # 链 docs/README.md
## 子模块 / 特殊流程   # 仅链 codewhale-submodule、picuscode 测试路径注意
## 改本文的规则        # 新契约先写 docs，AGENTS 只加摘要+链接
```

- [ ] 按上表拆分并落地链接  
- [ ] 删除 AGENTS 中已迁走的长文，避免双份  
- [ ] 系统/工具若注入 AGENTS：保证**契约摘要仍足够**；细节标明「实现前必读 docs/X」

#### 5.9.6 README 改造

- [ ] Quick start **与目标 DX 一致**（显式主题、Message、宏/BSN 内置控件；不用旧 drain 作为主路径）  
- [ ] 文档地图：Architecture / App guide / Styling / Examples  
- [ ] examples 表：各例用途 + 链 `docs/examples/index.md`  
- [ ] 去掉或降级「唯一正确是手写 UiComponentTemplate + drain」的暗示  

#### 5.9.7 与 DX 其它工作项的文档交付绑定

每项 API/行为落地时，**同步**更新对应 docs（禁止只改代码）：

| 能力 | 文档落点 |
|------|----------|
| 主题契约 / builder 主题 API | `guide/styling-themes.md` + `guide/app.md` |
| Message 桥 / UiEmit | `guide/events-messages.md` |
| 宏 | `guide/macros.md` |
| App builder | `guide/app.md` |
| 双层写作 / helper | `guide/components-bsn.md` 或 `guide/app.md` |
| example 迁移状态 | `examples/index.md` + 各 example 短 README（可选） |

- [ ] 约定：合并 DX 相关改动时 checklist 含「docs 已更新」  

#### 5.9.8 rustdoc 与 `docs/` 分工

- [ ] `picus` facade 模块级 rustdoc：一句话 + 指向 guide 章节名  
- [ ] 长教程不进 rustdoc  
- [ ] 公共契约类型（`UiComponentTemplate`、`UiEventQueue`）与 guide 表述一致  

#### 5.9.9 文档验收

- [ ] `AGENTS.md` 显著缩短；无大段 runtime/控件百科  
- [ ] `docs/README.md` 可当目录覆盖主路径  
- [ ] 新人只读 README + `guide/app.md` + 一个已迁移 example 能写简单应用  
- [ ] Agent 硬规则可从瘦身 AGENTS 执行；深潜有「读 docs/X」指针  
- [ ] 无主题 / Message / 宏 等契约在 docs 与 AGENTS 摘要中**一致**  
- [ ] 旧 README Quick start 不再教过时主路径  

#### 5.9.10 文档工作项勾选汇总

- [ ] 建立 `docs/` 地图与目录骨架（§5.9.3）  
- [ ] 从 `AGENTS.md` 按映射表迁出内容并改写为文档体裁（§5.9.4）  
- [ ] 瘦身 `AGENTS.md`（§5.9.5）  
- [ ] 重写 README 入口与 Quick start（§5.9.6）  
- [ ] `guide/app.md` / `events-messages.md` / `styling-themes.md` / `macros.md` 等与代码同步  
- [ ] `docs/examples/index.md` + 全 example 迁移状态  
- [ ] `contributing/codewhale-submodule.md`  
- [ ] rustdoc 交叉链接策略（§5.9.8）  
- [ ] DX 改动的 docs checklist 约定（§5.9.7）  
- [ ] i18n / a11y / 多窗口 / 性能指引落在对应 guide（非堆回 AGENTS）  

---

### 5.10 测试与调试

- [ ] 主题契约测试：无 sheet/无 variant → 无「框架塞入的可见默认色」  
- [ ] Message 桥：入队 → 同帧或下一 system 阶段 reader 收到  
- [ ] `UiEmit` 点击路径  
- [ ] macros trybuild + 注册幂等  
- [ ] 扩展 headless helpers：click → message → resource  
- [ ] **（可选）** dirty 原因 debug 输出  
- [ ] 全 examples：至少保证 `cargo check -p example_*`；关键例保留主题 parse 测试  

---

### 5.11 脚手架与工具

- [ ] **（可选）** `picus new` / cargo-generate：生成**带显式主题 RON** 的应用骨架（不是「无主题 Hello」）  
- [ ] 或文档：复制已迁移的 timer/calculator 骨架  

---

## 6. 风险与缓解

| 风险 | 缓解 |
|------|------|
| Message 与 UiEventQueue 双轨混乱 | 文档：应用只读 Message；队列为发射/兼容层 |
| 同帧顺序 | 桥接系统顺序测试；写明 schedule |
| inventory/linkme 平台 | 宏必做「显式 register_picus」保底；inventory 为增强 |
| 全 examples 迁移量大 | 复杂例允许部分迁移，表中逐项勾选 |
| 误加「默认 dark」 | §5.1 契约测试锁死 |
| 宏生成代码难调试 | 生成 `register_picus` 可读函数；expand 文档 |
| AGENTS 与 docs 双份漂移 | 单一事实来源；AGENTS 只摘要+链接；改契约先改 docs |
| Agent 上下文变短后丢约束 | 瘦身 AGENTS 保留硬规则列表；关键实现前「必读 docs/X」 |

---

## 7. 变更记录

| 日期 | 摘要 |
|------|------|
| 2026-07-14 | 初稿 |
| 2026-07-14 | 去轮次，扁平清单 |
| 2026-07-14 | 按反馈修订：(1) 取消独立 minimal，全量/部分迁移全部 examples；(2) 取消默认主题，保持无主题不显示，主题入口对齐 builder；(3) 业务消息与 Bevy `Message` 结合的架构结论与工作项；(4) 宏改为必做并写详细设计 |
| 2026-07-14 | 文档纳入 DX 必做：AGENTS 过重/docs 过少的职责拆分、目标目录、内容映射、瘦身结构、README/rustdoc 分工与验收 |
