# 贴边浮动条悬停交互实现计划

> **面向 AI 代理的工作者：** 在当前隔离worktree内按TDD执行；每项完成后更新本文件复选框并提交检查点。

**目标：** 在macOS和Windows实现用户确认的65pt贴边浮动条、底部Settings入口、可配置展开/收起延迟，并保证圆环在终态和过渡帧始终位于背景轮廓内。

**架构：** 共享偏好升级为schema 3，分别保存显示延迟与收起延迟。两端窗口控制器用同一状态机决定展开或收起；界面把Provider内容区与Settings弧区分开布局，背景路径按实际总高绘制。macOS以SwiftUI/AppKit实现，Windows以React/Tauri/Rust实现，并用共享合同锁定尺寸和语义。

**技术栈：** Swift 6、SwiftUI/AppKit、React 19、TypeScript、Tauri 2/Rust、Swift Testing、Vitest、Node真实浏览器测试。

---

### 任务1：偏好与状态机

**文件：**
- 修改：`Sources/AIMeterCore/Preferences/FloatingStripPreferences.swift`
- 修改：`Sources/AIMeterCore/UI/FloatingStripFoldState.swift`
- 修改：`Tests/AIMeterCoreTests/FloatingStripPreferencesTests.swift`
- 修改：`windows/src/state/stripPreferences.ts`
- 修改：`windows/src-tauri/src/platform/windows/strip_preferences.rs`
- 修改：`windows/src-tauri/src/platform/windows/strip_runtime.rs`

- [x] 写失败测试，锁定schema 3、150/800ms默认值、范围归一化、旧配置迁移和快速进出取消计时。
- [x] 运行Swift与Rust定向测试，确认旧实现因字段或状态机语义缺失而失败。
- [x] 实现最小偏好迁移与双延迟状态机。
- [x] 重跑定向测试，纳入统一功能候选。

### 任务2：65pt轮廓、圆环包含与Settings入口

**文件：**
- 修改：`Sources/AIMeterApp/Views/FloatingStripShape.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripDragShape.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/System/FloatingStripDisplayState.swift`
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`
- 修改：`Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`
- 修改：`Tests/AIMeterAppTests/FloatingStripRenderingTests.swift`
- 修改：`windows/src/components/FloatingStrip.tsx`
- 修改：`windows/src/components/FloatingStrip.test.tsx`
- 修改：`windows/src/styles.css`
- 修改：`windows/src/Shell.tsx`
- 修改：`windows/src-tauri/src/platform/windows/window_controller.rs`

- [x] 写失败测试，锁定65pt宽、48pt圆环、底部弧区、16pt命中区、Settings点击和左右路径包含。
- [x] 运行Swift、Vitest与Rust定向测试，确认旧几何和入口缺失会失败。
- [x] 实现新的分区布局、镜像Bezier轮廓、齿轮按钮和安全的内容显示顺序。
- [x] 重跑定向与渲染测试，检查1至4个Provider及左右边缘，纳入统一功能候选。

### 任务3：Settings双延迟编辑

**文件：**
- 修改：`Sources/AIMeterApp/Views/AppearanceSettingsView.swift`
- 修改：`Tests/AIMeterAppTests/SettingsViewTests.swift`
- 修改：`windows/src/settings/SettingsWindow.tsx`
- 修改：`windows/src/settings/SettingsGemini.test.tsx`
- 修改：`windows/src/localization.ts`

- [x] 写失败测试，锁定两个独立毫秒字段、允许范围、立即保存和中英文标签。
- [x] 实现macOS与Windows设置控件及本地化。
- [x] 运行设置专项测试，纳入统一功能候选。

### 任务4：真实浏览器、跨平台合同与文档

**文件：**
- 修改：`contracts/fixtures/auxiliary/strip-behavior.json`
- 修改：`windows/src/test/density-browser-entry.tsx`
- 修改：`windows/scripts/test-density-browser.mjs`
- 修改：`docs/design/README.md`
- 创建：`docs/development/2026-09-10-floating-strip-hover-redesign.md`
- 修改：`docs/development/README.md`
- 修改：`docs/requirements-backlog.md`

- [x] 更新共享尺寸合同并让旧实现准确失败。
- [x] 在真实浏览器采样左右、1至4个Provider、不同DPI等效缩放和圆环/背景路径包含关系。
- [x] 运行`scripts/check-docs.sh`、跨平台合同及完整项目门禁。
- [x] 记录定向测试、截图和限制；完整门禁与审查继续收口。

### 任务5：0.7.3稳定版发布

**文件：**
- 修改：全部版本元数据与版本合同
- 创建：`docs/releases/v0.7.3.md`
- 创建：`docs/design/implementation-plans/2026-09-10-v0.7.3-release.md`
- 创建：`docs/development/2026-09-10-v0.7.3-release.md`
- 修改：`docs/requirements-backlog.md`

- [x] 以失败先行版本合同同步0.7.3/build22，并完成完整本机门禁；差异审查已派发。
- [ ] 推送候选PR，等待macOS与Windows原生CI全绿后合入main。
- [ ] 从干净main创建签名标签与草稿Release，等待发布workflow公开双平台资产和更新源。
- [ ] 匿名重下七项资产，核对SHA、两端签名、篡改拒绝与三个更新入口；提交发布证据并完成发布后双平台CI。
