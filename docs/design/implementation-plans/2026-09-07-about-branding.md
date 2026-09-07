# 关于品牌展示实现计划

> 面向 AI 工作者：使用 subagent-driven-development，完成任务审查和最终整分支审查。用户已授权直接按推荐实施。

**目标：** 双平台关于页添加作者社交入口、Windows 设置顶部显示现有软件图标。
**架构：** 小型品牌链接组件与固定枚举 URL 边界；Windows 复用现有系统浏览器打开方式，不新增依赖。
**技术栈：** SwiftUI、React/TypeScript、Rust/Tauri。

## 全局约束

Twitter：@MillerPanYue → https://twitter.com/MillerPanYue 。GitHub → https://github.com/sljzdotcom/AI-Token-Meter 。仅点击打开系统浏览器，不自动联网、不在应用内导航。Settings 始终系统字体。Windows 顶部现有软件 Logo 显示 40px，本地资源；社交图标 14–16px、装饰性，文本可访问，链接可换行。失败要有可恢复提示。不得改 CLI 安装行为、凭据、发布版本或更新源。

### Task 1: 双平台作者入口和 Windows 软件图标

**所有权：** Sources/AIMeterCore/Presentation/AppBrand.swift、Sources/AIMeterApp/Views/AboutSettingsView.swift 及小型新 BrandLinksView；对应 Core/App 测试；Windows 新 settings/AuthorLinks.tsx、对应测试、SettingsWindow.tsx、Shell.tsx、styles.css、localization.ts；后端新品牌链接模块、lib.rs 注册与测试。根控制者负责公开文档，不要编辑需求列表或 README。

- [x] 先写会失败的行为测试：真实 About/AuthorLinks 渲染后的两链接名称、目标和点击；渲染不得调用浏览器，点击失败后出现提示；中英文和 Logo 均覆盖。Swift 尽量测试注入 openURL 的真实视图/操作边界，不做源码 grep。Windows 后端反序列化拒绝任意目标字符串，生产参数仅固定 HTTPS 地址。
  ```tsx
  expect(screen.getByRole("link", {name: /MillerPanYue/})).toHaveAttribute("href", "https://twitter.com/MillerPanYue")
  ```
  ```rust
  assert!(serde_json::from_str::<BrandLink>(r#""file:///tmp/other""#).is_err());
  ```
- [x] 运行聚焦测试，记录 RED 缺少渲染/行为的实际失败，然后最小实现。SwiftUI 品牌链接可独立小组件；Windows href 保留语义，但 preventDefault 后调用注入回调，Shell 传入固定 Tauri 命令。后端复用现有 Windows rundll32 URL 打开模式或等价固定 URL API，不添加任意 URL 启动器；不要在单元测试中真实打开网页。
- [x] Windows 直接 import 现有 src-tauri/icons/128x128.png 为构建资源，img alt 空且 aria-hidden，标题仍为可访问文本。使用局部 CSS，flex-wrap 的社交链接和 focus-visible；中英文文案进入 localization，不改变详情字体。
- [x] 聚焦 GREEN 后一次完整 Swift、React、Rust 测试及格式/Clippy、production build、密度检查；Swift 构建不可与另一 Swift 测试并行。可复用当前工作树 windows/node_modules 链接与项目默认缓存，所有日志放 /private/tmp，报告写本任务专属内部目录。
- [x] 自审并提交仅代码/测试文件，返回提交号和 RED/GREEN/全量验证证据，交独立任务审查。

## 控制者收尾

- [x] 独立任务审查与最终分支审查通过，检查真实浏览器布局（不使用真实账户）。
- [x] 补 Settings 指南、README/CHANGELOG 未发布说明、开发日志、文档索引及需求列表。
- [x] scripts/check-docs.sh 与公开安全检查、双平台 CI/构建后合入 main；当前 0.4.0 安装包和更新源不变。
