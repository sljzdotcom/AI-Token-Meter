# 菜单面板应用 Logo 实现计划

> **面向 AI 代理的工作者：** 在当前隔离 worktree 内按测试驱动步骤执行；每项先获得预期红灯，再做最小实现并复验。用户已批准应用现有 Logo 与普通尺寸/留白的推荐方案，不需要再次请求一般视觉选择。

**目标：** 在 macOS 菜单栏弹出面板和 Windows 原生托盘菜单顶部展示现有 AI Token Meter 应用 Logo，同时保留现有布局、操作和本地化行为。

**架构：** macOS 让 `MenuBarPanel` 接受默认应用图标并在现有标题块前渲染；Windows 用一个可独立构建和测试的 Tauri `IconMenuItem` 作为菜单首行，并复用同一默认窗口图标。两端都不增加资源或自定义背景。

**技术栈：** SwiftUI/AppKit、Swift Testing/ImageRenderer、Rust/Tauri 2 MockRuntime、Cargo、GitHub Actions。

---

### 任务 1：macOS 菜单面板品牌头部

**文件：**
- 创建：`Tests/AIMeterAppTests/MenuBarBrandingTests.swift`
- 修改：`Sources/AIMeterApp/Views/MenuBarPanel.swift`

- [ ] **步骤 1：编写真实渲染失败测试**

  构造纯品红 `NSImage`，把它注入真实 `MenuBarPanel` 后用 `ImageRenderer` 以 2× 渲染。扫描输出中独立推导的品红像素范围，要求存在约 64×64 像素的顶部前导图像块；没有品牌图标或图标未接入真实面板时测试必须失败。测试使用独立 `UserDefaults`、demo `AppModel` 和空白合成数据。

- [ ] **步骤 2：运行测试确认 RED**

  运行：
  ```sh
  swift test --filter MenuBarBrandingTests
  ```
  预期：编译或断言失败，原因是 `MenuBarPanel` 尚不支持注入并显示品牌图标。

- [ ] **步骤 3：实现最少 SwiftUI 变更**

  为 `MenuBarPanel` 增加默认读取应用图标的初始化参数，在现有 `header` 中把图标放到标题块前：
  ```swift
  Image(nsImage: brandIcon)
      .resizable()
      .interpolation(.high)
      .frame(width: 32, height: 32)
      .accessibilityHidden(true)
  ```
  图标与标题块使用 10pt 间距；原标题、副标题、`Spacer` 与刷新按钮保持原行为。

- [ ] **步骤 4：运行定向 GREEN 与相关回归**

  运行品牌渲染测试，以及 `TypographyTests`、`AppBrandTests`、菜单栏摘要与图标测试。确认合成图像不写入仓库，只保存到 `/private/tmp/req002-menu-brand/` 供本轮查看。

### 任务 2：Windows 托盘菜单品牌行

**文件：**
- 修改：`windows/src-tauri/Cargo.toml`
- 修改：`windows/src-tauri/src/platform/windows/tray.rs`
- 修改：`windows/src-tauri/tests/tray_summary.rs`

- [ ] **步骤 1：编写真实 Tauri 菜单项失败测试**

  仅在 dev/test 依赖启用 Tauri `test` 特性。测试通过 `tauri::test::mock_app()` 调用待新增的 `build_brand_header`：合法 1×1 RGBA 图像应得到 ID `brand-header`、文本 `AI Token Meter`、`enabled == false` 的 `IconMenuItem`；非法 RGBA 字节数必须构建失败，以证明图标参数被实际消费。

- [ ] **步骤 2：运行测试确认 RED**

  运行：
  ```sh
  cargo test --offline --locked --manifest-path windows/src-tauri/Cargo.toml --test tray_summary
  ```
  预期：编译失败，原因是 `build_brand_header` 尚不存在。

- [ ] **步骤 3：实现最少原生菜单变更**

  用 `IconMenuItemBuilder` 实现通用运行时构建函数：
  ```rust
  IconMenuItemBuilder::with_id("brand-header", "AI Token Meter")
      .enabled(false)
      .icon(icon)
      .build(app)
  ```
  `install` 先取得默认窗口图标，构建品牌行并将其置于菜单第一项；品牌行后加分隔线。托盘本身继续复用同一图标，其他菜单项与事件路由不变。

- [ ] **步骤 4：运行定向 GREEN 与格式/静态检查**

  重跑 `tray_summary`，然后运行 `cargo fmt --check` 和严格 Clippy。若 MockRuntime 暴露平台限制，保留原始失败证据并改用可执行的真实菜单构建边界，不退化为源码字符串断言。

### 任务 3：完整验证、记录、审查与本地整合

**文件：**
- 创建：`docs/development/2026-09-09-menu-panel-app-logo.md`
- 修改：`CHANGELOG.md`
- 修改：`docs/development/README.md`
- 修改：`docs/project-status.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：运行完整门禁**

  顺序运行完整 Swift 测试、Windows 前端测试/生产构建、宿主 Rust 测试、Rust 格式与严格 Clippy、跨平台合同、文档及公开安全检查。记录测试数、退出码、跳过项和 macOS/Windows 现场边界，不用宿主 MockRuntime 冒充 Windows 真机观感。

- [ ] **步骤 2：补开发与用户可见记录**

  在 Unreleased 记录菜单品牌增强；开发日志记录 RED/GREEN、渲染证据、两端实现、验证与限制；同步索引、当前状态、提交历史和需求台账。需求只有在实现、审查和本地主分支整合完成后标记 `已完成`。

- [ ] **步骤 3：审查并修复发现**

  对照规格逐项审查：macOS 真实面板接入、32pt/10pt、装饰性可访问性、刷新位置；Windows 首行图标、禁用状态、分隔与事件保持；无新增资产、敏感数据、版本或更新源变化。任何发现先修复并重跑受影响验证。

- [ ] **步骤 4：提交并本地整合**

  提交规格/计划检查点和测试驱动实现检查点；最终验证后以 fast-forward 或审查后的合并提交整合本地 `main`，保留协调入口未提交的 REQ-002 状态以及既有 stash。向协调入口回传需求 ID、提交、验证、审查、合并 SHA、未发布状态与真机边界。
