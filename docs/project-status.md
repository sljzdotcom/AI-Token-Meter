# 当前项目状态

- **事实快照：** 2026-09-02
- **产品：** AI Token Meter
- **应用版本：** `0.1.2`（build `3`）
- **维护分支：** `main`

本页只描述当前有效事实。功能演进过程查[开发日志](development/README.md)，需求状态查[需求台账](requirements-backlog.md)，历史取舍查[设计记录](design/README.md)。

## 一句话定位

AI Token Meter 是面向 Apple Silicon、macOS 14+ 的原生菜单栏与桌面浮岛应用，在本机汇总 Claude Code、OpenAI Codex 和 DeepSeek 的额度、余额、重置信息及受限的本机/官网历史聚合。

## 当前能力矩阵

| 能力 | Claude Code | OpenAI Codex | DeepSeek |
| --- | --- | --- | --- |
| 主要来源 | CLI `/usage` | 自动发现的 CLI/桌面 App 内置 `app-server` JSON-RPC | 官方余额 API |
| 身份状态 | `claude auth status --json` | `app-server` account/read | Keychain 中 API Key 后四位 |
| 主指标 | 当前会话与周额度已用比例 | 通用速率限制已用比例 | 相对余额基准的已消耗比例 |
| 补充详情 | 本机近 30 天会话、活跃日、Token、趋势 | 重置券；本机近 30 天 Token、连续日、最长会话 | 隔离官网会话中的近 30 天成本、请求、Token、趋势 |
| 登录/换号 | Services 打开官方 CLI 登录 | Services 打开官方 CLI 登录 | 两阶段验证后替换 Key |
| 失败降级 | 最近成功快照或明确错误 | 最近成功快照或明确错误 | 余额与历史各自独立缓存/错误 |

“本机近 30 天”不是跨设备官方账户报表；“DeepSeek 余额基准”也不是预算或账单上限。完整口径见[服务与指标说明](user-guide/providers.md)。

## 当前界面

- 菜单栏：18×18pt Quantum Dial 模板图像，显示三项服务中最高有效已用比例和精确百分比；
- 桌面浮岛：默认右侧贴边，可 Automatic/Left/Right，记录屏幕和纵向位置，只在桌面层显示；
- 详情：用户点击后临时位于普通应用窗口上方，空白点击或 3/5/8/15/30 秒无交互后关闭；
- Settings：Appearance、Monitoring、Services、About 四个 Tab，始终使用系统字体；
- 显示字体：System、Antonio、DIN Condensed、Alimama FangYuanTi VF、Fira Code、Leigo、Menlo、Alimama DaoLiTi；未安装项禁用并安全回退；
- Widget 源码：Small、Medium、Large 三种布局已实现，但只有带有效 Apple Development 身份和 App Group 的构建才能安装到桌面。

## 数据与持久化

| 数据 | 位置/所有者 | 是否敏感 |
| --- | --- | --- |
| Claude Code/OpenAI Codex 凭证 | 官方 CLI 自行管理 | 是；本应用不读取凭证文件 |
| DeepSeek API Key | Keychain 服务 `com.millerpan.AIMeter.deepseek` | 是；`AfterFirstUnlockThisDeviceOnly` |
| 统一快照 | `~/Library/Application Support/AI Meter/usage-snapshots.json` | 脱敏聚合 |
| DeepSeek 历史 | 同目录 `deepseek-usage-history.json` | 标准化逐日聚合 |
| Claude Code 工作区 | 同目录 `ClaudeUsageWorkspace/` | 空隔离工作区与批准状态 |
| 外观、通知、位置等 | `UserDefaults` | 非敏感偏好 |
| DeepSeek 官网会话 | App 隔离 WebKit 数据存储 | 敏感，由 WebKit 管理 |
| Widget 快照 | 签名 App Group 容器 | 最小脱敏展示数据 |

产品已改名，但 Bundle ID `com.millerpan.AIMeter`、可执行文件 `AIMeterApp`、Keychain 服务和 `Application Support/AI Meter` 保持不变，这是兼容策略，不是遗漏。

## 构建与验证基线

- Swift 6 / SwiftPM；无第三方 Package 依赖；
- Debug/测试和 Release 均面向 `arm64-apple-macosx14.0`；
- 完整自动化基线：**318 项测试、64 个测试组**，另有环境门控的 Keychain 与真实 CLI 冒烟检查；
- `scripts/test.sh` 同时运行 Swift 测试与文档一致性检查；
- `scripts/build-app.sh` 默认在没有开发证书时输出无 Widget、ad-hoc 签名的主应用，并验证主应用资源位于可跨机器解析的标准目录；
- 当前没有 Git remote、Git tag、公开 Release、CI、Developer ID 公证或许可证声明。

## 明确未完成或受限

| 事项 | 状态 | 恢复条件 |
| --- | --- | --- |
| Widget Gallery 与三尺寸真实桌面验收 | 已延期 | 用户恢复事项，并准备 Apple Development 证书/Team ID |
| Mission Control、第二普通 Space、真实指针拖动、多显示器补验 | 受环境限制 | 有对应系统 Space、显示器和可操作真实指针环境 |
| 稳定签名下旧 DeepSeek Key 再录入 | 维护提示 | 有稳定签名发行包时重新保存一次可信 Key |
| M4 Max nvm Codex 真实界面复验 | 待用户确认 | 在 M4 Max 完整替换为 0.1.2，重新打开后检查 OpenAI Codex 账户与详情 |
| 公开分发 | 未进入发布 | 确定许可证、远程仓库、版本号、Developer ID、公证、校验和与支持策略 |

以上状态不得在证据不足时改写为“已完成”。逐项依据见[需求台账](requirements-backlog.md)。

## 快速入口

- 使用：[安装与首次使用](user-guide/getting-started.md)、[设置参考](user-guide/settings.md)、[故障排查](user-guide/troubleshooting.md)
- 技术：[架构概览](architecture/overview.md)、[架构决策](architecture/decisions.md)、[隐私与安全](security-and-privacy.md)
- 维护：[维护手册](development/maintenance-playbook.md)、[测试指南](development/testing.md)、[发布流程](development/release-process.md)
- 历史：[开发日志](development/README.md)、[提交历史](development/commit-history.md)、[CHANGELOG](../CHANGELOG.md)
