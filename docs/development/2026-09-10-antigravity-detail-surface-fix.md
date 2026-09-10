# Antigravity 详情底板修复

关联`REQ-20260910-007`，规格见[设计说明](../design/specifications/2026-09-10-antigravity-detail-surface-fix-design.md)，步骤见[实施计划](../design/implementation-plans/2026-09-10-antigravity-detail-surface-fix.md)。

## 根因与修复

macOS详情窗口本身允许透明，三个既有Provider的详情根视图都会应用统一的`aiMeterDetailSurface`，只有Antigravity的`GeminiDetailView`缺少这一层。因此连接和额度正常，但桌面会从详情内容之间透出。修复在Antigravity根滚动视图上应用同一深海黑蓝底板，并让其占满390×520详情区域；额度模型、四个窗口、连接、重试和安装入口均未改变。

Windows现有`provider-detail`已经使用深色不透明渐变，Antigravity也保留`#3ed6b2`强调色，产品代码无需重复修改。浏览器门禁增加九种详情表面场景，以计算后样式确认四个Provider均有深色渐变，Antigravity的fresh、cached、unavailable全部保持同一表面和强调色。

## 失败先行与专项证据

新增macOS位图测试先在旧实现上运行，fresh、cached、unavailable三种固定快照的正文和侧边采样均得到透明alpha，准确复现故障。应用统一底板后，三态正文采样alpha大于0.9，带阴影的外圆角采样小于0.5。Gemini、Claude和Codex详情布局与视觉专项共6项通过。

Windows浏览器断言在旧报告上先以缺少详情表面场景失败。补充报告后，Vitest的Antigravity详情2项、production build、25项浏览器生命周期、8种Antigravity状态、9种详情表面、16组浮动条及608个文字角色全部通过真实Chrome验证。

实现提交为`d8ae465`。截图和自动测试使用固定演示快照，不读取或保存真实账号、凭据或额度。真实账户连接已经由用户现场确认；物理Windows视觉继续属于现场观察边界。
