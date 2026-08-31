# AI Meter 文档

这里是 AI Meter 的长期维护文档入口。文档分为用户指南、架构、安全、开发与历史设计记录五类；README 只保留最常用的信息，本目录负责完整说明。

## 用户指南

| 文档 | 内容 |
| --- | --- |
| [安装与首次使用](user-guide/getting-started.md) | 系统要求、构建、移动到应用程序、三项服务首次配置 |
| [服务与指标说明](user-guide/providers.md) | Claude、Codex、DeepSeek 的数据来源、口径和限制 |
| [设置参考](user-guide/settings.md) | 每一项设置的行为、默认值和注意事项 |
| [故障排查](user-guide/troubleshooting.md) | 登录、超时、数据不一致、缓存、通知和悬浮条问题 |

## 设计与实现

| 文档 | 内容 |
| --- | --- |
| [架构概览](architecture/overview.md) | 数据流、模块边界、刷新和降级机制 |
| [代码库结构](architecture/repository-structure.md) | 每个顶层目录与核心源码目录的职责 |
| [隐私与安全](security-and-privacy.md) | 凭证、WebKit 会话、缓存、日志和网络边界 |

## 开发与维护

| 文档 | 内容 |
| --- | --- |
| [下一阶段需求](next-phase-requirements.md) | 已登记但尚未进入设计与实现的新功能、产品调整与待确认事项 |
| [开发环境](development/setup.md) | 工具链、运行、Demo 模式和编码约定 |
| [测试指南](development/testing.md) | 普通测试、真实 CLI 冒烟测试、打包验证 |
| [发布流程](development/release-process.md) | 版本号、变更日志、构建、签名和发布检查清单 |
| [提交历史](development/commit-history.md) | 从项目创建至今的 Git 节点与阶段说明 |
| [开发日志索引](development/README.md) | 按日期查阅详细开发与验收记录 |
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
