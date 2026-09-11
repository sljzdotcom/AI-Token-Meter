# Mini 浮动条与后续交互修正实现计划

> **面向 AI 代理的工作者：** 在当前隔离 worktree 内按 TDD 逐任务实现；每个任务先留下准确失败证据，完成后更新复选框并提交检查点。

**目标：** 双平台新增 65pt Mini 密度、恢复 78pt Compact，加入可关闭的自动收起偏好，并让底部深色弧区独立控制 Settings 齿轮。

**架构：** 偏好 schema 升级后新增 `mini` 密度与 `automaticallyCollapses` 布尔值，旧 `compact` 字符串原样保留。三档尺寸由共享语义函数集中决定；折叠状态机先判断自动收起开关，再处理指针、焦点和延迟。macOS SwiftUI 与 Windows React/Tauri 分别实现同一档位、轮廓和触发区合同，并以共享 fixture 和真实浏览器取样核对。

**技术栈：** Swift 6、SwiftUI/AppKit、React 19、TypeScript、Tauri 2/Rust、Swift Testing、Vitest、Chromium 浏览器门禁。

---

### 任务 1：三档密度与持久化迁移

**文件：** `Sources/AIMeterCore/Preferences/FloatingStripPreferences.swift`、`Tests/AIMeterCoreTests/FloatingStripPreferencesTests.swift`、`windows/src/state/stripPreferences.ts`、`windows/src-tauri/src/platform/windows/strip_preferences.rs`、`window_controller.rs` 及相应测试。

- [x] 写失败测试，锁定 `mini=65/48/10`、`compact=78/48/10`、`comfortable=108/60/12`；默认与旧 JSON 的 `compact` 仍解码为 Compact。
- [x] 运行 Swift、Vitest 与 Rust 偏好测试，确认旧实现因缺少 Mini、Compact 仍为 65 以及 schema 字段缺失而失败。
- [x] 将三端密度扩为三档；`automaticallyCollapses` 默认 `true`，旧 schema 补 `true` 并保留延迟值。
- [x] 重跑定向测试，覆盖 Mini 往返、旧 Compact 不迁移、窗口逻辑宽度和 1×/1.25×/1.5×/2×换算。
- [x] 提交偏好和尺寸检查点。

### 任务 2：自动收起状态机与窗口生命周期

**文件：** `Sources/AIMeterCore/UI/FloatingStripFoldState.swift`、对应 Swift 测试、`Sources/AIMeterApp/System/FloatingPanelController.swift`、`windows/src-tauri/src/platform/windows/strip_runtime.rs`、`display_coordinator.rs` 及对应 Rust 测试。

- [x] 写失败测试：关闭自动收起会取消待执行计时并展开；关闭期间离开不折叠；重新开启使用保留的 150/800ms；桌面、全屏、Settings、详情和拖动门禁优先级不变。
- [x] 运行 Swift/Rust 状态机测试并记录旧逻辑仍排程收起的失败。
- [x] 两端控制器在偏好变更时取消计时并立即展开；仅在开关为真时接受折叠排程。
- [x] 重跑状态机、窗口位置、多显示器与桌面可见性专项。
- [x] 提交状态机检查点。

### 任务 3：macOS Mini、Settings 与独立底弧命中区

**文件：** `AppearanceSettingsView.swift`、`FloatingStripShape.swift`、`FloatingStripDragShape.swift`、`FloatingStripView.swift` 及 FloatingStrip/Settings 测试。

- [x] 写失败测试：Settings 固定且只按 Comfortable/Compact/Mini 排列；开关关闭时 Stepper 禁用且值未改；Mini 四服务 65×386 且四环周界在路径内；Compact 宽 78；主体悬停不显示齿轮，底弧悬停或键盘焦点显示。
- [x] 运行 macOS 视图、渲染和 Settings 专项，确认准确失败。
- [x] 以密度属性决定路径和内容尺寸；将齿轮状态拆为 Settings 区 hover 与焦点，弧线改为深色，按钮与弧区共享连续命中容器。
- [x] Settings 增加 Mini 和自动收起 Toggle，并以 `.disabled(!automaticallyCollapses)` 控制两个 Stepper。
- [x] 重跑 1–4 Provider、镜像、2×渲染、拖动排除、键盘和系统字体测试后提交。

### 任务 4：Windows Mini、Settings 与独立底弧命中区

**文件：** `windows/src/components/FloatingStrip.tsx`、相关测试、`windows/src/styles.css`、`windows/src/settings/SettingsWindow.tsx`、相关 Settings 测试和 `windows/src/localization.ts`。

- [x] 写失败 Vitest：三档 option 固定且只按 Comfortable/Compact/Mini 排列；开关禁用延迟输入但保留值；Mini/Compact 宽 65/78；主体 pointer enter 不显示齿轮，Settings zone 才显示；focus-visible 可达。
- [x] 运行 Vitest，确认现有两档和整条 hover 选择器不满足合同。
- [x] 实现三档映射、独立 Settings zone、深色弧线，并删除 `.floating-strip:hover` 对齿轮可见性的控制。
- [x] 更新 Settings 三档、自动收起复选框、禁用延迟输入和中英文文本。
- [x] 重跑组件、Settings、本地化和 production build 后提交。

### 任务 5：跨平台合同、真实浏览器和文档收尾

**文件：** `contracts/fixtures/auxiliary/strip-behavior.json`、浏览器密度入口/脚本、本规格、设计/开发索引、新开发日志和需求台账。

- [x] 更新共享 fixture 为 Mini 65、Compact 78、Comfortable 108，并确认旧生产代码准确失败。
- [x] Chromium 覆盖三档、左右边、1–4 Provider；逐环采样周界；验证过渡帧、主体/底弧 hover、键盘焦点和关闭自动收起后的稳定展开。
- [x] 运行完整 Swift、Vitest/build、Rust test/fmt/严格 Clippy、文档、公开安全与无 Widget Release App 签名门禁。
- [x] 独立差异审查并修复 Critical/Important；记录并关闭测试命名 Minor、开发日志、台账和证据。
- [ ] 提交最终候选并完成 PR/main 双平台 CI 与合并；按REQ-20260911-005的明确授权继续创建tag、Release与更新源。
