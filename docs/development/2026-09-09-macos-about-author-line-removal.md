# macOS About 独立作者行移除

关联需求：`REQ-20260909-006`。日期：2026-09-09。公开基线：0.6.2/build17。

## 背景与目标

用户要求从 macOS Settings → About 删除完整的 `Author: Miller` 可见行，并一并消除该行占用的空隙。版本、软件更新、Twitter、GitHub、Telegram 和其他品牌内容必须保留；README、MIT License 与代码版权归属不属于本次界面精简。

## 影响范围与实现

`AboutSettingsView` 原先在版本文字与 `BrandLinksView` 之间单独渲染作者文字。实现直接删除这个文字节点及它的两个样式修饰符。页面继续使用原有纵向布局，删除节点后相邻控件自然靠拢，没有保留条件分支或专用占位。

`AppBrand.author`、`AppBrand.authorLine` 与三条社交链接元数据仍保留，README、许可证和版权文件未改。Windows 已在 0.6.0 移除作者行，本次没有修改 Windows 源码、版本号或更新源。

## 自动化验证

定向验证通过：`AppBrandTests`、`BrandLinksViewTests` 与 `SettingsStructureTests` 共 15 项，继续覆盖作者元数据、Twitter/GitHub/Telegram 固定目标、窄宽度换行、点击失败反馈和 Settings 结构。第一次直接运行 Swift 测试时，系统用户缓存目录被沙箱拒绝；改用项目提供的隔离缓存测试入口后通过，产品代码没有因此调整。

完整门禁通过：

- 464 项主测试、3 项独立刷新调度、18 项 PTY runner、6 项 Gemini PTY，共 491 项 Swift；
- 无 Widget 的 Apple Silicon Release App 完成编译、便携资源、Sparkle framework/helper、嵌套签名和更新 bundle 校验；
- 6 份跨平台 fixture 及可移植性、Schema 显示名、不可用额度语义检查通过；
- 222 份 Markdown 文档、公开发布安全检查和 `git diff --check` 通过。

## 审查与边界

逐项审查结果为 Critical 0、Important 0、Minor 0：About 顶部版本后直接进入三条图标链接；链接、更新与隐私控件保持；Windows 生产代码保持；README、LICENSE、版本与三个更新源无差异。删除独立节点后没有残留专用留白。真实已安装应用的最终视觉随获授权的新版本发布验证。

## Git 证据

- 接单登记：`df5e6e7`
- 规格与计划：`0b5ca3f`
- 实现与完整门禁候选：`657068f`
- 本地 `main` 快进整合：`657068f`；协调入口原有未提交需求记录保存在独立 stash，既有 stash 未改写
- 已纳入0.6.3/build18候选`bf915c7`；公开发布证据由 `REQ-20260909-008` 完成后补充

## 当前状态

2026-09-09 实现、完整门禁、审查及本地 `main` 整合完成。用户已另行明确授权生成可更新的新版本，本成果已纳入0.6.3/build18候选，发布过程由 `REQ-20260909-008` 追踪；本记录不会在公开资产验证前提前宣称已发布。
