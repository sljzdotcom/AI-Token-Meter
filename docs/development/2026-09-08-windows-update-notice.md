# Windows 关于页新版提示强调

关联需求：REQ-20260908-007。登记日期：2026-09-08。

## 范围和决定

用户要求发现新版时提示更醒目，指定深红色加粗，且此次不发布。沿用 `UpdateState.phase`，只在 available 状态增加局部样式；不改检查或安装流程、系统字体和字号，不修改 macOS。中英文提示共用同一状态样式。

- [规格](../design/specifications/2026-09-08-windows-update-notice-design.md)
- [实施计划](../design/implementation-plans/2026-09-08-windows-update-notice.md)

## 验证记录

- 隔离工作区基线：85 项 Windows 前端测试通过。
- RED：新增三项真实 Settings 组件测试先因缺少 available 强调类失败；最小样式实现后同三项 GREEN。
- 完整 Windows 前端测试：88 项通过；TypeScript 检查与 Vite 生产构建通过，主代理独立复跑确认。
- 真实 Chrome 样式门禁：21 项生命周期测试、632 项既有字号角色及 14 个更新状态样本（中英文 × 七状态）通过；available 实测 `rgb(153, 27, 27)` / `700`，其他状态不强调，各状态字体字号相同。
- 浏览器测试初次受沙盒本地监听限制，获许可后执行；测试挂载更新时需先点击真实 About 页签，修正测试挂载时序后通过，不改生产页签逻辑。
- 186 份 Markdown 文档检查、四份共享合同、公开源码/历史敏感信息检查和差异空白检查通过。

## 开发检查点

- 需求登记：`f7cc2c5`；规格与计划：`844c9ae`。
- 实现：`15d6a35`（组件样式与回归）；独立任务规格/质量审查通过，无发现。
- 最终整分支审查通过，无 Critical/Important/Minor；完成日期 2026-09-08。按既有授权合入本地 main，发布按用户要求留待以后；不触发发布工作流。

## 发布边界

本项仅进入 Unreleased，不修改版本、签名、更新清单或 Release，不执行安装/账户操作。Windows 真实 WebView2/DPI 字形效果未在本机实测，浏览器计算样式结果不冒充 Windows 现场验收。
