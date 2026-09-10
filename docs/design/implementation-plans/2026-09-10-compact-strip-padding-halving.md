# Compact 圆环两侧留白减半实现计划

> **面向 AI 代理的工作者：** 在当前开发工作树按测试驱动逐步实现；每项完成后更新复选框和开发记录。

**目标：** 把 Compact 圆环左右留白从8.5各减至4.25，同时保持48圆环和全部纵向行为。  
**架构：** 以56.5逻辑宽作为跨平台单一合同，分别更新SwiftUI轮廓和React SVG轮廓；原生窗口从同一逻辑尺寸换算物理像素。  
**技术栈：** SwiftUI/CoreGraphics、React/SVG/CSS、Tauri/Rust、真实浏览器

---

### 任务1：尺寸合同失败先行

**文件：** `Tests/AIMeterAppTests/FloatingStripPreferencesTests.swift`、`Tests/AIMeterAppTests/FloatingStripDragShapeTests.swift`、`windows/src/components/FloatingStrip.test.tsx`、`windows/src-tauri/tests/strip_preferences.rs`、`contracts/fixtures/auxiliary/strip-behavior.json`

- [ ] 把预期宽度改为56.5、圆环范围改为4.25…52.25，并加入每侧留白4.25断言。
- [ ] 加入Windows 1.0×/1.25×/1.5×/2.0×最近物理像素断言。
- [ ] 运行Swift、Vitest、Rust及跨平台合同定向测试，确认旧65实现准确失败。

### 任务2：双平台最小实现

**文件：** `Sources/AIMeterCore/Preferences/FloatingStripPreferences.swift`、`Sources/AIMeterApp/Views/FloatingStripShape.swift`、`windows/src/components/FloatingStrip.tsx`、`windows/src/styles.css`、`windows/src-tauri/src/platform/windows/strip_preferences.rs`

- [ ] 将所有Compact逻辑宽统一为56.5，保持圆环48、高度和间距。
- [ ] 按规格更新macOS与SVG轮廓控制点，左右侧仍由同一轮廓镜像。
- [ ] 重跑任务1全部测试并确认通过。

### 任务3：渲染、点击和多DPI验证

**文件：** `Tests/AIMeterAppTests/CompactStripRenderingTests.swift`、`Tests/AIMeterAppTests/FloatingStripRenderingTests.swift`、`Tests/AIMeterAppTests/FloatingStripLayoutTests.swift`、`windows/src/test/density-browser-entry.tsx`、`windows/scripts/test-density-browser.mjs`

- [ ] 更新macOS 2×渲染和布局边界，确认四Provider、左右镜像、透明角与不透明主体。
- [ ] 更新Windows真实浏览器1–4 Provider场景，断言56.5宽、48圆环、点击顺序、一次玻璃拖动和无裁切。
- [ ] 生成65与56.5同倍率对比图并记录实际像素尺寸。
- [ ] 验证Comfortable、折叠、拖动吸附和纵向高度没有变化。

### 任务4：记录、完整验证与发布准备

**文件：** `docs/development/2026-09-10-compact-strip-padding-halving.md`、`docs/development/README.md`、`docs/requirements-backlog.md`

- [ ] 记录红绿证据、截图、尺寸、完整门禁与现场边界。
- [ ] 运行完整双平台本机门禁、文档、合同、公开安全、Release App和差异检查；完成独立审查。
- [ ] 两项需求通过后按 `REQ-20260910-008` 制作下一稳定修复版并执行发布事务。
