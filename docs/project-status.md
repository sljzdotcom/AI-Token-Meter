# 当前项目状态

- **事实快照：** 2026-09-04
- **产品：** AI Token Meter
- **应用版本：** `0.3.0-preview.1`（build `8`）；最新稳定版 `0.2.2`（build `6`）
- **维护分支：** `main`

本页只描述当前有效事实。功能演进过程查[开发日志](development/README.md)，需求状态查[需求台账](requirements-backlog.md)，历史取舍查[设计记录](design/README.md)。

## 一句话定位

AI Token Meter 是面向 Apple Silicon macOS 14+ 与 Windows 11 x64 的本地桌面浮岛应用，在本机汇总 Claude Code、OpenAI Codex 和 DeepSeek 的额度、余额、重置信息及受限的本机/官网历史聚合。稳定版目前仍为 macOS；`0.3.0-preview.1` 修复 Windows 浮动条视觉与贴边拖动，并保持 macOS 同版本发布。交互式 Windows DPI 与升级终验仍需真机补齐。

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
- 桌面浮岛：默认右侧贴边，可 Automatic/Left/Right，按稳定物理显示器身份记录目标屏、侧边和纵向位置；目标屏断开时仅临时回当前主屏，重新接入后自动恢复；只在桌面层显示；
- 详情：用户点击后临时位于普通应用窗口上方，空白点击或 3/5/8/15/30 秒无交互后关闭；
- Settings：Appearance、Monitoring、Services、About 四个 Tab，始终使用系统字体；
- About：显示当前版本与手动更新状态；仅在用户点击检查时访问 GitHub，发现新版后可明确启动签名更新；
- 显示字体：System、Antonio、DIN Condensed、Alimama FangYuanTi VF、Fira Code、Leigo、Menlo、Alimama DaoLiTi；未安装项禁用并安全回退；
- Widget 源码：Small、Medium、Large 三种布局已实现，但只有带有效 Apple Development 身份和 App Group 的构建才能安装到桌面。

Windows 版保持同一视觉与交互口径：系统托盘取代 macOS 菜单栏入口；Win32 窗口默认右侧贴边，可切左侧并记忆显示器/纵向位置；同显示器全屏应用出现时隐藏；详情临时置前并按相同秒数或外部点击关闭。Windows Widget 暂不在 Preview 范围。

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

Windows 对应位置为 `%APPDATA%\AI Token Meter\settings.json`、`%LOCALAPPDATA%\AI Token Meter\cache\` 和独立 WebView2 数据目录；DeepSeek Key 使用 Windows Credential Manager。两平台都不保存 Claude Code/OpenAI Codex 凭证。

产品已改名，但 Bundle ID `com.millerpan.AIMeter`、可执行文件 `AIMeterApp`、Keychain 服务和 `Application Support/AI Meter` 保持不变，这是兼容策略，不是遗漏。

## 构建与验证基线

- Swift 6 / SwiftPM；更新层固定使用 Sparkle `2.9.4` 二进制依赖；
- Debug/测试和 Release 均面向 `arm64-apple-macosx14.0`；
- macOS 完整自动化基线：**374 项测试、72 个测试组**，其中 12 项 PTY 系统资源测试由独立测试进程执行；另有环境门控的 Keychain、真实 CLI 和真实 GUI 更新验收；
- `scripts/test.sh` 同时运行 Swift 测试与文档一致性检查；
- `scripts/build-app.sh` 默认在没有开发证书时输出无 Widget、ad-hoc 签名的主应用，并验证便携资源、Sparkle framework、helper、`@rpath` 和嵌套签名；
- 公开源码仓库为 [sljzdotcom/AI-Token-Meter](https://github.com/sljzdotcom/AI-Token-Meter)。稳定版 [v0.2.2](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.2.2) 提供 Apple Silicon ZIP 和 SHA-256；`v0.3.0-preview.0` 的首个双平台签名发布证据见[发布记录](development/2026-09-04-v0.3.0-preview.0-release.md)。当前双平台候选为 `v0.3.0-preview.1`，发布结果与公网资产复验记录见本次[修复日志](development/2026-09-04-windows-floating-strip-parity-fix.md)。
- 精确合并头 Windows CI [33742313609](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33742313609) 已通过 14 项前端测试与 production build、完整 Rust/Windows-only 运行测试、严格 rustfmt/Clippy、Release 模式 Tauri 壳和 current-user NSIS 构建，并上传可下载的 x64 CI 安装器。它是合并门禁证据，不是经过双平台签名流程的正式 Release。
- 浮动条稳定显示器位置已合入 `main` 提交 `c2d2e64`；[macOS CI 33766955625](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766955625) 与 [Windows CI 33766955622](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766955622) 对精确合并头完成复验。

## 明确未完成或受限

| 事项 | 状态 | 恢复条件 |
| --- | --- | --- |
| Widget Gallery 与三尺寸真实桌面验收 | 已延期 | 用户恢复事项，并准备 Apple Development 证书/Team ID |
| Mission Control、第二普通 Space、真实指针拖动、多显示器补验 | 受环境限制 | 有对应系统 Space、显示器和可操作真实指针环境 |
| 稳定签名下旧 DeepSeek Key 再录入 | 维护提示 | 有稳定签名发行包时重新保存一次可信 Key |
| M4 Max nvm Codex 真实界面复验 | 待用户确认 | 在 M4 Max 安装 0.1.2 或更高版本，重新打开后检查 OpenAI Codex 账户与详情 |
| Developer ID 与 Apple 公证 | 当前限制 | 公开包使用 ad-hoc 签名；首次打开可能需要 Finder 右键“打开” |
| Windows 11 真机视觉、DPI、全屏、拖动与 DeepSeek 网页登录 | 受环境限制 | 在交互式 Windows 11 x64 用户会话安装 Preview 后逐项验收 |
| Windows `preview.0 → preview.1` 签名更新演练 | 待用户确认 | `preview.1` 发布后在交互式 Windows 会话检查原位升级、设置/凭据保留，并另用错误签名 feed 证明旧版不被替换 |
| Windows Authenticode 发布者身份 | 当前限制 | 取得代码签名证书；此前 README/Release 必须保留 SmartScreen 说明 |

以上状态不得在证据不足时改写为“已完成”。逐项依据见[需求台账](requirements-backlog.md)。

## 快速入口

- 使用：[安装与首次使用](user-guide/getting-started.md)、[设置参考](user-guide/settings.md)、[故障排查](user-guide/troubleshooting.md)
- 技术：[架构概览](architecture/overview.md)、[架构决策](architecture/decisions.md)、[隐私与安全](security-and-privacy.md)
- 维护：[维护手册](development/maintenance-playbook.md)、[测试指南](development/testing.md)、[发布流程](development/release-process.md)
- 历史：[开发日志](development/README.md)、[提交历史](development/commit-history.md)、[CHANGELOG](../CHANGELOG.md)
