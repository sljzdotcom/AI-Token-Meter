# Antigravity 详情底板修复规格

**需求：** `REQ-20260910-007`  
**状态：** 已实现并通过双平台专项验证
**范围：** macOS 修复，Windows 对等性验证；不改采集、账户或额度模型

## 问题与根因

Google Antigravity 已能连接并返回额度，但 macOS 点击浮动条后显示的 `GeminiDetailView` 只渲染滚动内容和内边距，没有使用其他三个 Provider 共同采用的 `.aiMeterDetailSurface()`。详情窗口本身允许透明，因此该分支会透出桌面。Windows `ProviderDetail` 已使用不透明深色渐变，并为 `provider-detail--gemini` 保留绿色强调色，静态实现没有同类缺口。

## 方案比较与决定

1. **在 `GeminiDetailView` 根视图应用统一详情底板（采用）。** 复用现有主题、圆角、文字色、减少透明度和高对比度行为，只修缺失分支。
2. 在 `FloatingDetailView` 外层统一加底板。会让已有 Claude、Codex、DeepSeek 出现双层内边距和双重背景。
3. 把详情 `NSPanel` 改为不透明。会破坏现有圆角透明窗口边缘，并把样式职责移到窗口层。

## 设计

- macOS 把 Antigravity 内容放入可占满详情窗口的根布局，随后应用 `.aiMeterDetailSurface()`；保留现有滚动、内容组织、Retry、安装指南、四个额度窗口及连接状态。
- 统一底板继续由 `AIMeterVisualTheme.detailGlass` 提供深海黑蓝渐变；文字使用主题主色，减少透明度时切换为实色底板。Antigravity Logo、额度进度和既有绿色语义不改。
- Windows 产品代码保持现有 `provider-detail` 深色渐变和 `--detail-accent: #3ed6b2`；新增真实浏览器样式证据，覆盖 Antigravity fresh/cached/unavailable，并确认其他 Provider 仍有非透明底板。
- 首次打开、刷新、Provider 切换与各状态只替换内容数据，不改变根表面，因此底板在全部状态持续存在。

## 验收

- macOS 新视觉测试在修复前准确得到透明背景，修复后 fresh、cached、unavailable 的正文底板 alpha 大于0.9；带阴影的圆角外侧采样 alpha 小于0.5，仍保持窗口外透明语义。
- Windows 真实浏览器报告四个 Provider、fresh/unavailable 和 Antigravity cached 的计算后背景图不为 `none`，背景底色不透明，Antigravity 强调色为 `rgb(62, 214, 178)`。
- Antigravity 四个额度、剩余百分比、重置时间、来源版本、Retry/安装入口和连接状态回归通过；Claude、Codex、DeepSeek 详情不变。

## 边界

- 用户未说明发生平台；源码差异支持 macOS 根因，Windows 以真实浏览器对等性验证，不制造无证据的产品改动。
- 真实账户连接不在自动测试中重做；使用固定快照验证详情渲染，不读取或记录用户凭据。
