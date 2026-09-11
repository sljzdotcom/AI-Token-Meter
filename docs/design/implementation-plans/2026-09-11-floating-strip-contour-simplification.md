# 浮动条镜像轮廓与底部入口移除实现计划

> **面向 AI 代理的工作者：** 在当前隔离worktree内按TDD逐任务实现；每个行为先运行失败测试，再写最少生产代码并复验。

**目标：** 双平台交付14pt内凹收起把手、严格镜像的展开上下肩，并完全移除浮动条底部Settings入口及额外高度。

**架构：** 共享密度合同把展开高度收敛为内容高度，把收起命中窗口改为20×96。macOS SwiftUI与Windows React分别删除Settings子树；两端轮廓使用同一组上肩坐标垂直镜像生成下肩，Windows原生窗口尺寸与区域同步更新。

**技术栈：** Swift 6、SwiftUI/AppKit、Swift Testing、React 19、TypeScript、Vitest、Tauri 2/Rust、Chromium。

---

### 任务1：共享尺寸与收起窗口合同

**文件：** `contracts/fixtures/auxiliary/strip-behavior.json`、`Sources/AIMeterCore/Preferences/FloatingStripPreferences.swift`、`Tests/AIMeterCoreTests/FloatingStripPreferencesTests.swift`、`Sources/AIMeterApp/System/FloatingStripLayout.swift`、`Tests/AIMeterAppTests/FloatingStripLayoutTests.swift`、`windows/src-tauri/src/platform/windows/strip_preferences.rs`

- [x] 写失败测试，断言三档展开高度等于内容高度，四服务分别为344/344/428，收起窗口为20×96。
- [x] 运行Swift Core/Layout与Rust偏好测试，确认旧42/48pt附加高度和16pt命中宽度导致失败。
- [x] 删除密度Settings附加高度，将`baseHeight`和`height(providerCount:)`直接返回内容高度；两端收起窗口宽度改20。
- [x] 重跑定向测试并提交尺寸合同检查点。

### 任务2：macOS镜像轮廓与无底部入口视图

**文件：** `Tests/AIMeterAppTests/VisualSystemTests.swift`、`Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`、`Tests/AIMeterAppTests/FloatingStripRenderingTests.swift`、`Sources/AIMeterApp/Views/FloatingStripShape.swift`、`Sources/AIMeterApp/Views/FloatingStripDragShape.swift`、`Sources/AIMeterApp/Views/FloatingStripView.swift`

- [x] 写失败测试，逐点验证三档上下肩垂直镜像、四环周界包含、视图结构不存在Settings按钮与辅助功能标签，以及14×88收起可见轮廓处于20×96命中窗口。
- [x] 运行macOS定向测试，确认旧下肩Settings凸瓣、7pt胶囊和Settings按钮准确失败。
- [x] 将展开路径下半部改为上肩控制点的垂直镜像；删除Settings状态、弧形、按钮和命中Shape；用14×88自定义Shape替换收起胶囊。
- [x] 从拖动Shape删除Settings按钮扣除区域，重跑布局、视觉、拖动和2×渲染测试并提交。

### 任务3：Windows镜像轮廓与无底部入口视图

**文件：** `windows/src/components/FloatingStrip.test.tsx`、`windows/src/components/GeminiStrip.test.tsx`、`windows/src/App.test.tsx`、`windows/src/components/FloatingStrip.tsx`、`windows/src/styles.css`、`windows/src/test/density-browser-entry.tsx`、`windows/src-tauri/src/platform/windows/window_controller.rs`

- [x] 写失败Vitest/Rust测试，断言展开尺寸不含Settings区、DOM与辅助功能树无Settings按钮、折叠把手可见宽14且窗口宽20、原生区域上下镜像。
- [x] 运行定向Vitest和Rust测试，确认旧DOM、CSS、尺寸与区域地标准确失败。
- [x] 删除`onSettingsOpen`、悬停状态和Settings DOM/CSS；以14pt内凹伪元素绘制收起把手；重写SVG下肩和原生区域地标为上肩镜像。
- [x] 更新真实浏览器夹具为新高度和无Settings交互，重跑组件、App、Rust与Chromium专项并提交。

### 任务4：完整验证、文档与候选

**文件：** `docs/design/README.md`、`docs/development/README.md`、新开发日志、`docs/requirements-backlog.md`及受影响用户文档。

- [x] 运行完整Swift、Vitest/build、Rust test/fmt/严格Clippy、真实Chromium、合同、文档和公开安全检查。
- [x] 更新设计索引、开发日志、用户可见说明和需求证据；运行`scripts/check-docs.sh`。
- [x] 独立审查差异并修复Critical/Important问题；复验受影响门禁。
- [x] 提交可评审候选并回传测试与提交证据；不创建新tag或Release。
