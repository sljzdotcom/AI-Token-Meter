# 历史设计记录

本目录是 AI Token Meter 设计规格与实施计划的唯一存放位置，用于回答“当时为什么这样决定”，不替代当前用户指南、架构文档或需求台账。

## 目录约定

- [`specifications/`](specifications/)：需求边界、方案比较、交互、架构和安全设计；
- [`implementation-plans/`](implementation-plans/)：经确认规格对应的测试驱动实施步骤；
- 新资料继续使用 `YYYY-MM-DD-topic-design.md` 与 `YYYY-MM-DD-topic.md` 命名；
- 旧内部目录 `docs/superpowers` 已在 2026-09-02 完整迁移，不得重新创建。

## 完整索引

| 日期 | 主题 | 规格 | 计划 |
| --- | --- | --- | --- |
| 2026-08-28 | 首版应用 | [规格](specifications/2026-08-28-ai-meter-design.md) | [计划](implementation-plans/2026-08-28-ai-meter-implementation.md) |
| 2026-08-29/30 | 详情自动关闭 | [规格](specifications/2026-08-29-detail-panel-dismissal-design.md) | [计划](implementation-plans/2026-08-30-detail-panel-dismissal.md) |
| 2026-08-30 | Claude Code 隔离用量工作区 | [规格](specifications/2026-08-30-claude-usage-workspace-design.md) | [计划](implementation-plans/2026-08-30-claude-usage-workspace.md) |
| 2026-08-30 | DeepSeek 网页输入焦点 | [规格](specifications/2026-08-30-deepseek-web-focus-design.md) | [计划](implementation-plans/2026-08-30-deepseek-web-focus.md) |
| 2026-08-30 | Provider 详情增强 | [规格](specifications/2026-08-30-provider-detail-enhancements-design.md) | [计划](implementation-plans/2026-08-30-provider-detail-enhancements.md) |
| 2026-08-30 | 用量准确性 | [规格](specifications/2026-08-30-usage-accuracy-design.md) | [计划](implementation-plans/2026-08-30-usage-accuracy.md) |
| 2026-08-31 | OpenAI Codex 与 DeepSeek 详情 | [规格](specifications/2026-08-31-codex-deepseek-details-design.md) | [计划](implementation-plans/2026-08-31-codex-deepseek-details.md) |
| 2026-08-31 | OpenAI Codex 重置券卡片 | [规格](specifications/2026-08-31-codex-reset-credit-card-design.md) | [计划](implementation-plans/2026-08-31-codex-reset-credit-card.md) |
| 2026-08-31 | 显示字体选择 | [规格](specifications/2026-08-31-display-font-selection-design.md) | [计划](implementation-plans/2026-08-31-display-font-selection.md) |
| 2026-08-31 | 浮岛紧凑肩部 | [规格](specifications/2026-08-31-floating-strip-compact-shoulder-design.md) | [计划](implementation-plans/2026-08-31-floating-strip-compact-shoulder.md) |
| 2026-08-31 | 浮岛圆滑接边与品牌色 | [规格](specifications/2026-08-31-floating-strip-corner-shadow-design.md) | [计划](implementation-plans/2026-08-31-floating-strip-visual-polish.md) |
| 2026-08-31 | 深海背景 | [规格](specifications/2026-08-31-floating-strip-deep-sea-background-design.md) | [计划](implementation-plans/2026-08-31-floating-strip-deep-sea-background.md) |
| 2026-08-31 | 浮岛回归修复 | [规格](specifications/2026-08-31-floating-strip-regression-fixes-design.md) | [计划](implementation-plans/2026-08-31-floating-strip-regression-fixes.md) |
| 2026-08-31 | 浮岛 S 曲线 | [规格](specifications/2026-08-31-floating-strip-s-curve-design.md) | [计划](implementation-plans/2026-08-31-floating-strip-s-curve.md) |
| 2026-08-31 | 视觉系统与贴边 | [规格](specifications/2026-08-31-visual-system-edge-docking-design.md) | [计划](implementation-plans/2026-08-31-visual-system-edge-docking.md) |
| 2026-09-01 | Claude Code 本机活动 | [规格](specifications/2026-09-01-claude-detail-local-activity-design.md) | [计划](implementation-plans/2026-09-01-claude-detail-local-activity.md) |
| 2026-09-01 | 详情窗口置前 | [规格](specifications/2026-09-01-detail-panel-frontmost-design.md) | [计划](implementation-plans/2026-09-01-detail-panel-frontmost.md) |
| 2026-09-01 | 字体目录扩展 | [规格](specifications/2026-09-01-display-font-catalog-expansion-design.md) | [计划](implementation-plans/2026-09-01-display-font-catalog-expansion.md) |
| 2026-09-01 | 桌面层与背景裁切 | [规格](specifications/2026-09-01-floating-strip-desktop-layer-and-background-crop-design.md) | [计划](implementation-plans/2026-09-01-floating-strip-desktop-layer-and-background-crop.md) |
| 2026-09-01 | 服务账户与重新登录 | [规格](specifications/2026-09-01-service-account-relogin-design.md) | [计划](implementation-plans/2026-09-01-service-account-relogin.md) |
| 2026-09-01 | Settings 字体隔离与字号 | [规格](specifications/2026-09-01-settings-font-isolation-and-content-size-step-design.md) | [计划](implementation-plans/2026-09-01-settings-font-isolation-and-content-size-step.md) |
| 2026-09-01 | Settings 分类与品牌迁移 | [规格](specifications/2026-09-01-settings-tabs-and-brand-migration-design.md) | [计划](implementation-plans/2026-09-01-settings-tabs-and-brand-migration.md) |
| 2026-09-01 | WidgetKit | [规格](specifications/2026-09-01-widgetkit-extension-design.md) | [计划](implementation-plans/2026-09-01-widgetkit-extension.md) |
| 2026-09-02 | Claude Code 详情卡片精简 | [规格](specifications/2026-09-02-claude-detail-card-removal-design.md) | [计划](implementation-plans/2026-09-02-claude-detail-card-removal.md) |
| 2026-09-02 | Claude Code 隐私说明移除 | [规格](specifications/2026-09-02-claude-detail-privacy-note-removal-design.md) | [计划](implementation-plans/2026-09-02-claude-detail-privacy-note-removal.md) |
| 2026-09-02 | 菜单栏 Quantum Dial | [规格](specifications/2026-09-02-menu-bar-quantum-dial-design.md) | [计划](implementation-plans/2026-09-02-menu-bar-quantum-dial.md) |
| 2026-09-02 | Provider 名称统一 | [规格](specifications/2026-09-02-provider-visible-name-standardization-design.md) | [计划](implementation-plans/2026-09-02-provider-visible-name-standardization.md) |
| 2026-09-02 | 全项目复盘与文档治理 | [规格](specifications/2026-09-02-project-retrospective-and-documentation-governance-design.md) | [计划](implementation-plans/2026-09-02-project-retrospective-and-documentation-governance.md) |
| 2026-09-02 | 公开 GitHub 发布 | [规格](specifications/2026-09-02-public-github-release-design.md) | [计划](implementation-plans/2026-09-02-public-github-release.md) |
| 2026-09-03 | Windows 平台与双平台同步发布 | [规格](specifications/2026-09-03-windows-platform-design.md) | [计划](implementation-plans/2026-09-03-windows-platform.md) |
| 2026-09-04 | Windows DeepSeek 历史窗口与界面密度 | [规格](specifications/2026-09-04-windows-deepseek-history-and-density-design.md) | [计划](implementation-plans/2026-09-04-windows-deepseek-history-and-density.md) |

## 阅读顺序与历史边界

1. 先看根目录 [README](../../README.md) 和 [当前项目状态](../project-status.md)；
2. 使用问题查 [用户指南](../user-guide/)；
3. 当前技术边界查 [架构概览](../architecture/overview.md) 与 [架构决策](../architecture/decisions.md)；
4. 只有在追溯取舍或复现开发过程时才阅读本目录。

早期文档中的功能可能已被后续实现替代。例如第一版 DeepSeek 使用“本地月预算”，当前实现是“可配置余额基准 + 官方 30 天用量聚合”。原始规格保留历史事实，当前行为以源码、用户指南、项目状态和 [CHANGELOG](../../CHANGELOG.md) 为准。
