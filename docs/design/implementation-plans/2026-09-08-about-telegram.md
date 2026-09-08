# 关于页 Telegram 实现计划

> 面向 AI 工作者：使用 subagent-driven-development 完成一个跨平台品牌链接任务，随后进行任务规格/质量审查及最终整分支审查。用户已授权直接实施。

**目标：** Windows 精简作者文字，两端新增固定 Telegram 图标链接。
**架构：** 扩展现有 AuthorLinks、AppBrand 和 BrandLinksView；Rust 枚举映射固定 URL；保留点击边界。
**技术栈：** React/TypeScript、Rust/Tauri、SwiftUI/AppKit。

## 全局约束

Windows 中文/英文作者整行和可见作者链接标题删除，无障碍分组名称保留；macOS 原作者行不改。新增可辨识 Telegram @sljzdotcom，目标固定 https://t.me/sljzdotcom；Twitter/GitHub 目标及交互不变。只激活时打开默认浏览器。保留系统字体字号、焦点、等高、窄宽度换行、深红粗体更新提示。不发布，不修改版本、更新源、签名。只在当前隔离工作区修改，不覆盖其他执行者改动。

### Task 1: 双平台品牌链接与行为验证

**所有权：** Windows settings/AuthorLinks.tsx、SettingsWindow.tsx、styles.css、相关测试、src-tauri/src/brand_links.rs 与 tests/brand_links.rs，以及必要的浏览器测试；macOS AppBrand.swift、BrandLinksView.swift 及对应 Core/App 测试。控制者负责文档和唯一台账。

- [x] 先运行现有聚焦测试建立基线；补失败的真实渲染/路由测试，记录 RED。Windows 两种语言查询无作者文字与可见组标题、三链接 href、点击 target、无自动打开与错误恢复；macOS 实际三个 NSButton 点击目标和辅助功能，窄宽度渲染；Rust 将字符串 telegram 反序列化并验证固定目标，同时拒绝未知输入。
  ```tsx
  expect(screen.getByRole("link", {name: /Telegram.*sljzdotcom/})).toHaveAttribute("href", "https://t.me/sljzdotcom")
  expect(screen.queryByText("作者链接")).not.toBeInTheDocument()
  ```
  ```rust
  let target: BrandLink = serde_json::from_str(r#""telegram""#).unwrap();
  assert_eq!(target.url(), "https://t.me/sljzdotcom");
  ```
- [x] 最小实现：删除 Windows 作者 p 与组标题 small，保留 aria-label；新增 Telegram 数据项/装饰纸飞机 SVG；必要局部 CSS 保证不收缩图标、等高和换行。Rust 新增 Telegram 枚举值和固定 URL。macOS 新增 Link 数据项和纸飞机图标分支，复用已有按钮与 ViewThatFits。
- [x] 运行 GREEN 和全量前端/Rust/Swift 验证及构建；先 scripts/check-docs.sh。使用任务专属临时缓存/日志避免并发共享构建干扰。Windows production build、密度/更新样式回归和实际浏览器 Tab/Enter、双语言宽窄布局检查；不在测试中实际打开远程链接。
- [x] 自审，只提交自己负责的代码与测试；报告 RED/GREEN、命令/结果、边界、提交。交独立任务规格与质量审查。

## 控制者收尾

- [ ] 任务审查通过，补开发日志、用户指南/当前状态、Unreleased、设计/开发索引和 REQ-009 记录。
- [ ] 最终整分支审查，文档与差异检查；回传提交范围、验证证据、真机边界和未发布状态给协调入口。保留分支，不直接合并。
