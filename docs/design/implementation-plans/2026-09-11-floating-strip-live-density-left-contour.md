# 悬浮条密度即时生效与方案 B 轮廓实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法跟踪进度。

**目标：** 让展开悬浮条在 Settings 切换密度后立即同步改变原生窗口与内容尺寸，并在双平台应用已确认的方案 B 圆角和加强反向肩弧。

**架构：** SwiftUI 与 React 使用同一组 65pt 标准 Bézier 坐标，Compact 只缩放横向坐标，Comfortable 同时映射 88pt 肩深；左右和上下均由计算镜像产生。macOS 保留现有同步外观链并补原生 frame 回归；Windows 为异步显示器协调增加一次性完成回执，使设置事件只在原生窗口应用最新密度后发布。

**技术栈：** Swift 6、SwiftUI/AppKit、Swift Testing、TypeScript 7、React 19、Vitest、Rust、Tauri 2、Cargo tests、真实 Chromium 密度门禁。

---

## 文件与职责

- 修改 `Sources/AIMeterApp/Views/FloatingStripShape.swift`：从方案 B 标准坐标生成三密度、上下镜像和左右镜像路径。
- 修改 `Tests/AIMeterAppTests/VisualSystemTests.swift`：验证方案 B 关键填充点、竖直切线效果及上下镜像。
- 修改 `Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`：继续覆盖三密度、1–4 服务和左右贴边圆环包含。
- 修改 `windows/src/components/FloatingStrip.tsx`：导出并使用与 Swift 相同的方案 B SVG 路径生成器。
- 修改 `windows/src/components/FloatingStrip.test.tsx`：验证三档路径、严格镜像和旧尖角回归点。
- 修改 `windows/src-tauri/src/platform/windows/window_controller.rs`：让原生轮廓采样辅助函数使用方案 B 坐标。
- 修改 `windows/src-tauri/src/platform/windows/display_coordinator.rs`：为合并式 reconcile 请求增加按轮次完成回执。
- 修改 `windows/src-tauri/src/lib.rs`：密度设置等待对应原生协调完成，只发布仍为最新的设置状态。
- 修改 `windows/src-tauri/tests/multidisplay.rs`：验证回执不会在原生工作完成前触发，且连续请求按最后状态完成。
- 修改 `windows/src-tauri/tests/window_policy.rs`：验证方案 B 原生采样、圆角和水平镜像。
- 修改 `Tests/AIMeterAppTests/AppModelDisplaySettingsTests.swift`：验证 macOS 密度变更触发外观更新；屏幕测试直接验证既有 NSPanel frame 即时变化。
- 修改 `contracts/fixtures/auxiliary/strip-behavior.json`：登记方案 B 标准坐标和肩深，作为双平台测试合同。
- 修改 `Tests/AIMeterCoreTests/RefreshBackoffTests.swift` 与 `windows/src/components/FloatingStrip.test.tsx`：验证两端读取同一合同。
- 创建 `docs/development/2026-09-11-floating-strip-live-density-left-contour.md`：记录根因、实现和验证证据。
- 修改 `docs/development/README.md`、`docs/requirements-backlog.md`、`docs/project-status.md`、`CHANGELOG.md`：登记完成状态与候选边界。

### 任务 1：方案 B 轮廓合同和双平台路径

- [ ] **步骤 1：编写失败的 Swift 几何测试**

在 `VisualSystemTests.swift` 增加方案 B 断言：Mini 右贴边路径的边界包围盒为 `(0, 4, 65, H-8)`；左贴边在 `(63, 60)` 处有背景、旧路径没有；上肩与下肩采样严格镜像。让测试直接调用生产 `FloatingStripShape`。

- [ ] **步骤 2：运行 Swift 红灯**

运行：

```bash
swift test --filter VisualSystemTests
```

预期：新圆角/肩弧断言失败，旧轮廓仍以 `8/22` 起始且缺少第三段竖直接入曲线。

- [ ] **步骤 3：编写失败的 TypeScript 与 Rust 几何测试**

在 `FloatingStrip.test.tsx` 断言 `meterContourPath("mini", 4)` 包含：

```text
M 65 4 C 63 18 54 29 37 30
C 18 31 5 42 1 58
C 0 62 0 66 0 70
```

并断言 Compact 横向等比、Comfortable 肩深为 88、左侧 clip-path 只用镜像变换。更新 `window_policy.rs` 的关键点为方案 B 端点和竖直接入点。

- [ ] **步骤 4：运行前端与 Rust 红灯**

运行：

```bash
cd windows && npm test -- --run src/components/FloatingStrip.test.tsx
cargo test --locked --manifest-path windows/src-tauri/Cargo.toml --test window_policy meter_shape
```

预期：路径字符串和原生采样关键点均因仍使用旧轮廓而失败。

- [ ] **步骤 5：实现最小方案 B 路径**

Swift 和 TypeScript 都以 Mini 参考坐标定义上肩三段曲线：

```text
M (65,4)
C (63,18) (54,29) (37,30)
C (18,31) (5,42) (1,58)
C (0,62) (0,66) (0,70)
```

下肩按 `H-y` 反射，左贴边按 `W-x` 反射。Compact 使用 `W=78,S=70`，Comfortable 使用 `W=108,S=88`。Rust 采样函数应用相同标准坐标并保留 64 段采样。

- [ ] **步骤 6：更新共享合同并验证两端读取**

在 `strip-behavior.json` 增加 `expandedContour`：`referenceWidth=65`、`compactShoulderDepth=70`、`comfortableShoulderDepth=88` 和上肩四个端点/六个控制点。Swift 与前端合同测试断言生产常量等于该合同。

- [ ] **步骤 7：运行几何绿灯与包含回归**

运行：

```bash
swift test --filter 'VisualSystemTests|FloatingStripDragShapeTests|RefreshBackoffTests'
cd windows && npm test -- --run src/components/FloatingStrip.test.tsx src/components/GeminiStrip.test.tsx
cargo test --locked --manifest-path windows/src-tauri/Cargo.toml --test window_policy
```

预期：全部通过，三密度 × 1–4 服务 × 左右贴边圆环周界仍完整位于背景内。

- [ ] **步骤 8：提交轮廓任务**

```bash
git add contracts/fixtures/auxiliary/strip-behavior.json Sources/AIMeterApp/Views/FloatingStripShape.swift Tests/AIMeterAppTests/VisualSystemTests.swift Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift Tests/AIMeterCoreTests/RefreshBackoffTests.swift windows/src/components/FloatingStrip.tsx windows/src/components/FloatingStrip.test.tsx windows/src-tauri/src/platform/windows/window_controller.rs windows/src-tauri/tests/window_policy.rs
git commit -m "fix: round floating strip contour"
```

### 任务 2：Windows 原生尺寸完成顺序与 macOS 即时 frame 回归

- [ ] **步骤 1：编写失败的 reconcile 完成回执测试**

在 `multidisplay.rs` 建立一个阻塞的第一轮原生工作，申请带回执的 reconcile；断言工作释放前 `recv_timeout` 超时，释放后收到成功。再在第一轮执行中申请第二个回执，断言它只在第二轮完成后返回。

- [ ] **步骤 2：运行 Rust 红灯**

运行：

```bash
cargo test --locked --manifest-path windows/src-tauri/Cargo.toml --test multidisplay reconcile_receipt
```

预期：因 `ReconcileQueue` 尚无带回执请求接口而编译失败。

- [ ] **步骤 3：实现合并式完成回执**

把 `ReconcileQueue` 的布尔二元组替换为明确状态：`active`、`rerun`、`pending_receipts`。每轮开始时取走当时等待的发送端；`reconcile_once` 完成后只通知该轮回执。执行中到达的请求设置 `rerun` 并留给下一轮，不能提前完成。

- [ ] **步骤 4：让设置命令等待原生应用再发布事件**

将 `set_strip_preferences` 改为异步命令：完成持久化和详情关闭后取得 reconcile 回执，通过 `tauri::async_runtime::spawn_blocking` 有界等待。成功后重新读取当前设置；若当前密度已不同，旧命令不发布过期事件，最后一次命令在其回执完成后发布 `app-settings-changed`。失败返回既有“Window resize failed”。

- [ ] **步骤 5：运行 Rust 绿灯和前端设置集成测试**

运行：

```bash
cargo test --locked --manifest-path windows/src-tauri/Cargo.toml --test multidisplay
cd windows && npm test -- --run src/App.test.tsx src/settings/SettingsGemini.test.tsx
```

预期：回执时序、快速连续请求和 Settings 调用全部通过。

- [ ] **步骤 6：补 macOS 现有同步链的回归证据**

先在 `AppModelDisplaySettingsTests.swift` 增加普通测试，确认 `setStripPreferences` 保存 Mini 后同步调用 `floatingAppearanceHandler` 且处理器观察到 65pt。再增加受 `AI_METER_SCREEN_TESTS=1` 控制的原生窗口测试：创建 controller、记录 Compact frame、调用 `setStripPreferences` 与 `applyAppearance` 链，断言 frame 立即为 Mini 宽度并保持贴边和中心。

- [ ] **步骤 7：运行 macOS 回归**

运行：

```bash
swift test --filter AppModelDisplaySettingsTests
AI_METER_SCREEN_TESTS=1 swift test --filter AppModelDisplaySettingsTests.densitySettingImmediatelyResizesExpandedPanel
```

预期：普通同步通知和本机原生 frame 测试通过；若红灯定位到现有 handler/controller 链，再只修复该断点并重新运行。

- [ ] **步骤 8：提交即时生效任务**

```bash
git add windows/src-tauri/src/platform/windows/display_coordinator.rs windows/src-tauri/src/lib.rs windows/src-tauri/tests/multidisplay.rs Tests/AIMeterAppTests/AppModelDisplaySettingsTests.swift
git commit -m "fix: apply strip density before settings update"
```

### 任务 3：真实渲染、完整门禁、文档与独立复核

- [ ] **步骤 1：运行双平台专项渲染**

```bash
swift test --filter 'FloatingStripRenderingTests|CompactStripRenderingTests|FloatingStripDragShapeTests|VisualSystemTests'
cd windows && npm run test:density
```

确认 24 组展开场景、收起态、真实浏览器圆环周界和方案 B 左右镜像全部通过。

- [ ] **步骤 2：运行完整门禁**

```bash
scripts/test.sh
cd windows && npm test
cargo test --locked --manifest-path windows/src-tauri/Cargo.toml
cargo fmt --manifest-path windows/src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path windows/src-tauri/Cargo.toml --all-targets -- -D warnings
scripts/check-docs.sh
git diff --check
```

所有命令必须通过；不可删除、跳过或放宽既有断言。

- [ ] **步骤 3：更新长期文档**

开发记录写明：Windows 根因为设置事件早于异步原生 reconcile 完成；macOS 同步链经原生 frame 回归确认；方案 B 的标准路径和镜像采样证据。把 REQ-013/014/015 标记为已完成并记录提交、测试与审查证据；`CHANGELOG.md` 只写未发布修复，不增加版本号或发布声明。

- [ ] **步骤 4：运行独立审查并修复发现**

审查范围为本计划相对基线 `8580464` 的全部代码和文档。审查者分别报告 Critical、Important、Minor；任何发现先补失败测试再修复并重跑相关门禁。

- [ ] **步骤 5：提交文档与审查收尾**

```bash
git add docs/development/2026-09-11-floating-strip-live-density-left-contour.md docs/development/README.md docs/requirements-backlog.md docs/project-status.md CHANGELOG.md docs/design/README.md
git commit -m "docs: record floating strip density and contour fix"
```

- [ ] **步骤 6：回传候选证据**

向需求协调任务回传最终提交范围、Swift/前端/Rust/真实浏览器/文档门禁数量、独立审查结果与未发布边界。不得创建 tag、GitHub Release 或修改更新源。
