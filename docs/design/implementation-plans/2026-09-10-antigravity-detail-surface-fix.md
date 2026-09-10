# Antigravity 详情底板修复实现计划

> **面向 AI 代理的工作者：** 在当前开发工作树按测试驱动逐步实现；每项完成后更新复选框和开发记录。

**目标：** 恢复 macOS Antigravity 详情的统一深色底板，并用双平台视觉证据证明所有状态不透明。  
**架构：** macOS 复用现有 `aiMeterDetailSurface` 主题修饰器；Windows 保持现有产品样式并扩展真实浏览器报告。  
**技术栈：** SwiftUI/ImageRenderer、React/CSS、Playwright/Edge 或 Chrome、Swift Testing、Vitest

---

### 任务1：失败先行确认 macOS 缺失底板

**文件：** `Tests/AIMeterAppTests/GeminiDetailPanelLayoutTests.swift`

- [ ] 创建 fresh、cached、unavailable 固定快照的 `ImageRenderer` 测试，断言详情中心/边缘内侧不透明、圆角外侧透明。
- [ ] 运行 Gemini 详情定向测试，确认旧实现因中心或边缘内侧 alpha 为零而失败。

### 任务2：应用统一详情表面

**文件：** `Sources/AIMeterApp/Views/GeminiDetailView.swift`、`Tests/AIMeterAppTests/GeminiDetailPanelLayoutTests.swift`

- [ ] 让根布局占满详情窗口并应用 `.aiMeterDetailSurface()`，不改变额度和账户动作。
- [ ] 重跑 Gemini 详情测试，确认三种状态底板不透明且圆角外侧透明。
- [ ] 运行其他 Provider 详情和完整 Swift 测试，确认无双重底板或内容回归。

### 任务3：补 Windows 对等性证据

**文件：** `windows/src/test/density-browser-entry.tsx`、`windows/scripts/test-density-browser.mjs`、`windows/src/details/GeminiDetail.test.tsx`

- [ ] 先让浏览器报告要求 Antigravity 和其他 Provider 的非透明背景、Antigravity 强调色与状态覆盖，并确认旧报告缺字段而失败。
- [ ] 扩展真实浏览器场景，采集四 Provider 与 Antigravity cached 状态的计算后背景和强调色。
- [ ] 运行 Vitest、真实浏览器和生产构建；确认连接与四额度展示保持。

### 任务4：记录与审查

**文件：** `docs/development/2026-09-10-antigravity-detail-surface-fix.md`、`docs/development/README.md`、`docs/requirements-backlog.md`

- [ ] 记录根因、红绿证据、双平台结果、现场边界和提交。
- [ ] 运行文档、合同、公开安全和差异检查；完成独立差异审查并关闭发现。
