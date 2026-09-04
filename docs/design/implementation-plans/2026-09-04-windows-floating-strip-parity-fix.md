# Windows 浮动条视觉与贴边稳定性修复实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让 Windows 浮动条复用 macOS 平滑肩部轮廓，消除系统白边与 GDI 锯齿，并让拖动释放后的吸附和位置保存稳定可靠。

**架构：** WebView2 的归一化 SVG clip path 是唯一可见裁剪源，Win32 只配置无激活工具窗口、关闭阴影和 DWM 边框。拖动生命周期由原生原子状态与左键释放监视器管理，显示器拓扑恢复在拖动期间让路。

**技术栈：** React 19、TypeScript、SVG/CSS、Tauri 2、Rust、Win32/DWM、Vitest、Cargo test、GitHub Actions。

---

## 文件结构

- 修改 `windows/src/components/FloatingStrip.tsx`：输出可缩放且左右镜像的 macOS 同源 SVG clip path。
- 修改 `windows/src/styles.css`：让完整背景使用 SVG clip，删除粗多边形和肩部伪元素依赖。
- 修改 `windows/src/App.test.tsx`：锁定真实 DOM 的左右路径与背景容器行为。
- 修改 `windows/src-tauri/tauri.conf.json`：对 meter 明确关闭窗口阴影。
- 修改 `windows/src-tauri/Cargo.toml`：加入 DWM API 所需 Windows feature。
- 修改 `windows/src-tauri/src/platform/windows/window_controller.rs`：移除 GDI 可见裁剪，复用 macOS 几何做纯缩放测试，并关闭 DWM 外框。
- 创建 `windows/src-tauri/src/platform/windows/meter_drag.rs`：管理拖动会话、鼠标释放、吸附和保存。
- 修改 `windows/src-tauri/src/platform/windows/display_topology.rs`：建立启动基线，拖动中不执行恢复。
- 修改 `windows/src-tauri/src/lib.rs`、`windows/src/Shell.tsx`：接通 `begin_meter_drag` 并删除不可靠的 Promise 完成回调。
- 修改 Windows 入门/排障/测试文档与 `docs/requirements-backlog.md`，创建本任务开发日志。
- 同步根、macOS、npm、Cargo、Tauri 版本为 `0.3.0-preview.1`，更新 Release notes 后运行现有双平台发布流程。

### 任务 1：以 macOS 路径替换 Windows 双重粗裁剪

- [x] 在 `windows/src/App.test.tsx` 添加失败测试：真实 `FloatingStrip` DOM 必须包含 `meter-clip-right/left` 两条归一化 Bezier，且 nav 暴露右/左 clip 选择类；旧 polygon 实现无法满足断言。
- [x] 运行 `npm test -- --run windows/src/App.test.tsx`（在 `windows/` 中实际运行 `npm test -- src/App.test.tsx`），确认因 SVG path 缺失而失败。
- [x] 在 `FloatingStrip.tsx` 输出 SVG defs，路径坐标独立按 108×356 手工归一化；在 CSS 用 `url(#meter-clip-right)`/`url(#meter-clip-left)`，删除 `polygon()` 与 strip-only 伪元素隐藏规则。
- [x] 在 `window_controller.rs` 添加失败测试，断言 108×356 右侧关键点为 `(108,16)/(66,28)/(0,88)/(0,268)/(66,328)/(108,340)`，左侧为 `x=108-x`；旧点集必须失败。
- [x] 运行 `cargo test --manifest-path windows/src-tauri/Cargo.toml window_controller` 确认预期失败。
- [x] 把纯 `meter_shape_points` 改为 64 段 macOS同源四段 Bezier，只保留几何/合同用途；Windows `apply_windows_meter_style` 不再调用 `SetWindowRgn`，而是关闭 shadow 并尝试 `DwmSetWindowAttribute(DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE)`。
- [x] 在 `tauri.conf.json` 设置 meter `shadow:false`，运行前端与 Rust 定向测试确认绿色。
- [x] 提交检查点：`fix: match Windows meter to macOS silhouette`（`2614186`）。

### 任务 2：让原生拖动释放控制唯一一次吸附

- [x] 在 `meter_drag.rs` 添加失败测试：`Idle -> Active` 只允许一次；Active 时拓扑恢复返回 false；`Released/TimedOut -> Idle`；错误完成也必须解锁。
- [x] 运行 `cargo test --manifest-path windows/src-tauri/Cargo.toml meter_drag`，确认模块/行为尚不存在而失败。
- [x] 实现 `MeterDragState` 的原子会话；`begin_meter_drag` 先占用会话，再在 Windows 线程轮询 `GetAsyncKeyState(VK_LBUTTON)`，释放后调用既有吸附、稳定显示器 ID 和原子设置保存逻辑。
- [x] 修改 `Shell.tsx`：先 `invoke("begin_meter_drag")`，成功后调用 `startDragging()`；删除 `.then(() => invoke("meter_drag_ended"))`。
- [x] 修改 `display_topology.rs`：启动首次轮询只建基线；会话 Active 时不恢复窗口；拓扑真正变化且非拖动状态才恢复。
- [x] 运行 Rust 定向测试、前端测试和构建确认绿色。
- [x] 提交检查点：`fix: settle Windows meter drag after pointer release`（`e78ad48`）。

### 任务 3：文档、完整回归和 Preview 发布

- [x] 更新 Windows 入门、排障和测试文档，创建 `docs/development/2026-09-04-windows-floating-strip-parity-fix.md`，记录根因、红绿证据、平台限制和真机验收项。
- [x] 将所有平台版本同步到 `0.3.0-preview.1`，增加同版本 Release notes；运行版本合同测试确认 macOS/Windows 一致。
- [x] 运行 `scripts/check-docs.sh`、公开安全/秘密检查、前端完整测试与构建、Rust 完整测试/rustfmt/Clippy、macOS 完整测试和跨平台合同。
- [x] 对照规格逐项复核差异；只有验证证据完整后更新 backlog 状态和开发日志。
- [ ] 提交发布候选，推送分支并等待 macOS/Windows CI；CI 失败则回到对应红绿循环。
- [ ] CI 全绿后合并 `main`，创建并验证 `v0.3.0-preview.1` GitHub prerelease；重新下载并验证 macOS Sparkle 签名、Windows minisign、SHA-256 和 preview feed，稳定 appcast 必须保持稳定版。
- [ ] Windows 真实视觉若尚未由用户复验，需求状态保持 `待用户确认`，不得用 CI/构建代替 DPI 和拖动验收。
