# macOS About 作者行移除设计

关联需求：`REQ-20260909-006`。日期：2026-09-09。基线：公开稳定版0.6.2，开发接单提交`df5e6e7`。

## 目标与范围

macOS Settings → About 不再显示独立的 `Author: Miller` 文字行。标题、副标题、版本、Twitter、GitHub、Telegram、隐私说明和软件更新控件全部保留；Windows 已在既有需求中移除作者行，本次不再修改。README、MIT License及代码版权信息继续保留作者归属。用户随后另行授权生成新版本，发布工作由 `REQ-20260909-008` 独立追踪。

## 方案比较与决定

1. **删除独立 SwiftUI `Text` 行（采用）：** 只移除 `AboutSettingsView` 中显示 `AppBrand.authorLine` 的节点。原有纵向间距由剩余相邻节点自然计算，因此不会留下专用空白；`AppBrand` 的作者元数据和三条固定链接继续存在，避免把界面精简扩大成开源署名变更。
2. **用条件分支隐藏：** 会留下永远为假的展示路径，不能表达用户已经明确删除该行的决定。
3. **删除全部作者元数据或社交入口：** 会改变 README、许可证或现有联系入口，超出本需求。

## 交互与布局

About 顶部仍使用64pt应用图标和右侧纵向摘要。版本文字之后直接显示现有三条图标链接；`VStack` 的4pt间距和 `BrandLinksView` 自身的自适应横排/竖排规则不变。没有新增文案、控件、远程资源或点击行为。

## 验证

这是一个可逆的一行展示删除，不新增镜像实现的专用测试。保留并运行现有 `AppBrandTests`、`BrandLinksViewTests` 与 `SettingsStructureTests`，确认版本与三条固定链接、窄宽度换行、点击和失败反馈仍有效；随后运行完整Swift测试、Release构建、文档检查、公开安全检查及差异检查。最终差异必须只涉及macOS About展示和相应文档，不修改Windows、版本、标签或更新源。
