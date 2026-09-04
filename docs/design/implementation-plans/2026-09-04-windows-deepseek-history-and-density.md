# Windows DeepSeek 历史窗口与界面密度修复实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 test-driven-development 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法跟踪进度。

**目标：** 消除 Windows DeepSeek 自动空白窗口和无响应同步，建立可关闭、可恢复的官网同步生命周期，并把 Windows 详情、Settings 和字体选择框恢复为紧凑清晰的视觉尺度。

**架构：** Rust/Tauri 后端拥有官网 WebView2 的唯一生命周期和安全会话，React 详情页只通过命令与脱敏状态事件展示同步反馈。Windows CSS 使用集中密度变量控制详情和 Settings，macOS 源码保持不变。

**技术栈：** Rust、Tauri 2、WebView2、React 19、TypeScript、Vitest、CSS、Windows 11。

---

## 文件结构

- 修改 `windows/src-tauri/src/lib.rs`：移除查看 DeepSeek 详情时的隐式官网窗口启动，接入同步状态事件。
- 修改 `windows/src-tauri/src/platform/windows/deepseek_webview.rs`：实现隐藏加载、唯一窗口、置前、关闭清理、成功和失败恢复。
- 创建或修改 `windows/src-tauri/src/platform/windows/deepseek_history_window.rs`：用纯状态机描述官网窗口生命周期，供跨平台 Rust 单测验证。
- 修改 `windows/src-tauri/tests/deepseek_history.rs`：覆盖生命周期状态、重复打开和会话清理。
- 修改 `windows/src/Shell.tsx`：展示同步状态和命令错误，不再吞掉错误；同步期间暂停详情自动隐藏。
- 修改 `windows/src/details/ProviderDetail.tsx`、`windows/src/details/DeepSeekDetail.tsx`：接收同步状态并渲染可恢复反馈。
- 修改 `windows/src/App.test.tsx`：覆盖按钮状态、错误反馈和自动隐藏暂停。
- 修改 `windows/src/styles.css`：集中 Windows 详情/Settings 密度，并修复 select/option 配色。
- 更新 `docs/README.md`、`docs/design/README.md`、用户指南、开发日志、变更日志和需求台账。

### 任务 1：阻止查看详情隐式打开官网窗口

- [x] 在 Rust 中提取“查看详情只展示详情窗口”的可测试行为边界，并添加失败测试：DeepSeek 历史为空时也不得产生官网同步动作。
- [x] 运行 `cargo test --manifest-path windows/src-tauri/Cargo.toml --test deepseek_history`，确认测试因现有自动打开分支失败。
- [x] 从 `show_provider_detail` 删除隐式 `open_history_window` 调用，保持三项 Provider 的详情事件一致。
- [x] 重跑定向 Rust 测试并提交第一个 Git 检查点。

### 任务 2：建立官网同步窗口状态机与关闭恢复

- [x] 为 `idle/opening/active/completed/cancelled/failed` 编写纯 Rust 状态转移测试；每项测试分别能捕获重复建窗、关闭不清理、失败不恢复和成功不关闭的生产回归。
- [x] 实现最小状态机和动作：`CreateHidden`、`ShowFocused`、`FocusExisting`、`RestoreDetail`、`DestroyHistory`、`EmitStatus`。
- [x] 把 WebView2 改为隐藏构建；用 nonce 绑定的官方 ready 回调触发显示和聚焦；用 `on_window_event(CloseRequested/Destroyed)` 清理会话并恢复详情。
- [x] 已有窗口时直接显示和聚焦，禁止 destroy/recreate；完整数据时销毁窗口并恢复更新后的详情。
- [x] 对构建/加载错误清理窗口和会话，并发出不含敏感信息的失败事件。
- [x] 运行 Rust 定向测试、完整 Rust 测试、rustfmt 和严格 Clippy，提交第二个 Git 检查点。

### 任务 3：让前端同步反馈可见并暂停自动隐藏

- [x] 添加失败先行的 Vitest 用例：点击同步立即显示 opening/禁用按钮；命令拒绝显示错误；同步 active 时不会自动隐藏；cancelled/completed 后恢复正常倒计时。
- [x] 在 `DetailSurface` 订阅同步状态事件，调用命令时捕获错误并映射为 `failed`。
- [x] 扩展 `DeepSeekHistory`：按状态显示 `Opening official page…`、`Sync in progress`、错误和可重试按钮；正常历史图表不受影响。
- [x] 运行 `npm --prefix windows test` 和 `npm --prefix windows run build`，提交第三个 Git 检查点。

### 任务 4：缩小 Windows 详情与 Settings，修复字体下拉框

- [x] 增加组件级失败测试：详情和 Settings 应带明确的 Windows 紧凑密度类；Display font 控件保留可访问名称和系统字体隔离。
- [x] 在 `styles.css` 定义集中字号/间距变量，把身份标题、主数值、区块标题、卡片数字、Settings 标题、导航和控件迁移到紧凑尺度。
- [x] 为 Settings 设置 `color-scheme: light`；为 `select` 与 `option` 明确指定 `color: #151821` 和 `background-color: #fff`，保留焦点可见性。
- [x] 使用浏览器视觉入口比较三项详情和四个 Settings tab，并增加真实 production CSS 计算样式门禁，确认无溢出、截断或层级丢失。
- [x] 运行前端测试、密度浏览器门禁和 production build，提交第四个 Git 检查点。

### 任务 5：文档、完整门禁和发布准备

- [x] 创建 `docs/development/2026-09-04-windows-deepseek-history-and-density.md`，记录根因、红绿测试、窗口生命周期、安全边界和真机待验收项。
- [x] 更新需求台账、文档索引、Windows 用户指南、故障排查和 `CHANGELOG.md`。
- [x] 运行 Windows 前端/Rust/rustfmt/严格 Clippy/Tauri 目标构建尝试、macOS 完整测试、跨平台合同、`scripts/check-docs.sh` 和公开安全扫描；Windows MSVC/Tauri 因本机缺 SDK 受限，明确交给 `windows-latest`。
- [x] 复查 Git diff、秘密扫描和发布资产范围；无证据时不得把真机登录同步标为已完成。
- [x] 提交最终文档与验证检查点；需求进入 `待用户确认`，列出 Windows 11 真机验收步骤。

> 自动化计划步骤已完成；`REQ-20260904-006` 不等于产品验收完成。Windows 11 真实 DeepSeek 登录、标题栏关闭、重复同步复用聚焦、成功聚合恢复与 Display font 原生下拉弹层仍须按开发日志手工确认，需求保持 `待用户确认`。
