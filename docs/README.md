# AI Token Meter 文档

## 需求与进度

- [当前项目状态](project-status.md)：版本、能力、数据、验证基线、发布事实和仍未完成事项的权威快照。
- [待完成需求与需求历史](requirements-backlog.md)：所有新需求的第一登记点，包含进行中、待确认、延期、受环境限制和已完成事项。

这里是 AI Token Meter 的长期维护文档入口。文档分为用户指南、架构、安全、开发与历史设计记录五类；README 只保留最常用的信息，本目录负责完整说明。

## 用户指南

| 文档 | 内容 |
| --- | --- |
| [安装与首次使用](user-guide/getting-started.md) | 系统要求、构建、移动到应用程序、三项服务首次配置 |
| [服务与指标说明](user-guide/providers.md) | Claude Code、OpenAI Codex、DeepSeek 的数据来源、口径和限制 |
| [设置参考](user-guide/settings.md) | 每一项设置的行为、默认值和注意事项 |
| [故障排查](user-guide/troubleshooting.md) | 登录、超时、数据不一致、缓存、通知和悬浮条问题 |

## 设计与实现

| 文档 | 内容 |
| --- | --- |
| [架构概览](architecture/overview.md) | 数据流、模块边界、刷新和降级机制 |
| [架构决策记录](architecture/decisions.md) | 长期技术决定、原因、代价和重新评估条件 |
| [代码库结构](architecture/repository-structure.md) | 每个顶层目录与核心源码目录的职责 |
| [隐私与安全](security-and-privacy.md) | 凭证、WebKit 会话、缓存、日志和网络边界 |
| [菜单栏 Quantum Dial 图标设计](design/specifications/2026-09-02-menu-bar-quantum-dial-design.md) | 动态菜单栏图标的视觉构成、数据映射、系统适配和验收边界 |
| [Provider 用户可见名称统一设计](design/specifications/2026-09-02-provider-visible-name-standardization-design.md) | Claude Code、OpenAI Codex 当前名称及兼容边界 |
| [GitHub 应用内更新设计](design/specifications/2026-09-02-github-app-update-design.md) | 手动检查、一键安全更新、Sparkle、签名与首次升级边界 |

## 开发与维护

| 文档 | 内容 |
| --- | --- |
| [待完成需求与需求历史](requirements-backlog.md) | 唯一需求队列、状态、阻塞和交付证据 |
| [开发环境](development/setup.md) | 工具链、运行、Demo 模式和编码约定 |
| [测试指南](development/testing.md) | 普通测试、真实 CLI 冒烟测试、打包验证 |
| [发布流程](development/release-process.md) | 版本号、变更日志、构建、签名和发布检查清单 |
| [维护手册](development/maintenance-playbook.md) | 变更影响矩阵、Provider 诊断、安装、回滚和安全处置 |
| [提交历史](development/commit-history.md) | 从项目创建至今的 Git 节点与阶段说明 |
| [开发日志索引](development/README.md) | 按日期查阅详细开发与验收记录 |
| [全项目复盘](development/2026-09-02-project-retrospective.md) | 全仓库盘点、文档差距、清理证据和最终验证 |
| [公开 GitHub 发布](development/2026-09-02-public-github-release.md) | MIT、作者、社区文件、脱敏截图、历史扫描、CI 与 Release 证据 |
| [WidgetKit 开发日志](development/2026-09-01-widgetkit-extension.md) | 三尺寸 Widget、共享快照、签名保护与当前实机验收边界 |
| [Claude 详情与本机活动](development/2026-09-01-claude-detail-local-activity.md) | 官方额度、本机 30 天活动、隐私边界和真实数据验收 |
| [Claude 详情卡片精简](development/2026-09-02-claude-detail-card-removal.md) | 移除 Token composition 与 Top models、兼容边界、测试和安装验收 |
| [Claude 详情隐私说明移除](development/2026-09-02-claude-detail-privacy-note-removal.md) | 移除内联隐私说明、保留底层边界并记录测试与安装验收 |
| [菜单栏 Quantum Dial](development/2026-09-02-menu-bar-quantum-dial.md) | 自绘菜单栏动态图标、比例映射、像素渲染、安装和真实数据链路验收 |
| [Provider 用户可见名称统一](development/2026-09-02-provider-visible-name-standardization.md) | Claude Code、OpenAI Codex、DeepSeek 的集中名称源、兼容边界与验收证据 |
| [显示字体目录扩充](development/2026-09-01-display-font-catalog-expansion.md) | 八项字体、安装检测、别名、回退与真实 Settings 验收 |
| [贡献指南](../CONTRIBUTING.md) | 分支、提交、测试、文档和评审要求 |
| [安全报告](../SECURITY.md) | 私下报告漏洞的要求与范围 |
| [版本变更](../CHANGELOG.md) | 已发布版本和未发布改动 |

## 历史设计记录

- [设计规格目录](design/specifications/)
- [实施计划目录](design/implementation-plans/)

这些文件记录当时的决策上下文与实施过程，可能包含后来已被替代的设想。判断当前产品行为时，以源码、用户指南和 `CHANGELOG.md` 为准；历史文档不应当被当作当前配置说明。

## 文档维护规则

1. 功能或行为变化必须同时更新 README、相关用户指南和 `CHANGELOG.md`。
2. 涉及数据来源、凭证或持久化的改动必须更新 `security-and-privacy.md`。
3. 新增或移动源码目录必须更新 `architecture/repository-structure.md`。
4. 每个可验收开发阶段应写入按日期命名的开发日志，并在 `development/README.md` 建立入口。
5. 发布前必须检查所有相对链接、示例命令、版本号和系统要求。
6. 每次完整测试都会运行 `scripts/check-docs.sh`；不得绕过失败的断链、版本、测试基线或目录治理检查。
