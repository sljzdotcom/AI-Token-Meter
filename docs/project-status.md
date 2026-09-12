# 当前项目状态

- **事实快照：** 2026-09-13
- **产品：** AI Token Meter
- **当前公开稳定版：** 双平台 [`0.9.0`](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.9.0)（macOS build `26`）
- **当前源码版本：** 双平台 `0.9.0`（macOS build `26`）
- **维护分支：** `main`

本页只描述当前有效事实。功能演进过程查[开发日志](development/README.md)，需求状态查[需求台账](requirements-backlog.md)，历史取舍查[设计记录](design/README.md)。

**macOS Services四产品Logo已随0.9.0公开：** Claude Code、OpenAI Codex、DeepSeek和Google Antigravity分组标题复用同一18pt Logo组件与6pt名称间距，使用系统主前景色适配浅深外观，装饰Logo不进入辅助功能树。实现通过147项串行真实窗口专项、完整Swift门禁及Release App四Logo、三语言资源和签名验证，最终独立审查`0/0/0`。Core、Windows与数据逻辑无变化。[开发记录](development/2026-09-12-macos-services-provider-logos.md)。

**macOS三语言即时切换已随0.9.0公开：** Settings → Appearance固定提供English、简体中文和繁體中文，缺失或未知偏好默认English；菜单栏、Settings、悬浮条、四Provider详情、通知、日期数字和辅助功能文案使用共享状态即时切换并持久保存。实现通过语言资源、界面专项、完整Swift门禁与正式App签名验证，完整分支复审Critical/Important/Minor为`0/0/0`。[开发记录](development/2026-09-12-macos-app-language-selection.md)。

**自动隐藏展开轮廓修复已随0.9.0公开：** macOS自动隐藏不再拉伸20×96pt收起窗口来形成展开态，而是直接提交与关闭自动隐藏时相同的三档精确尺寸和方案B圆润轮廓，只保留内容显示时序。真实`NSPanel`回归覆盖Comfortable、Compact、Mini的收起→展开终态、Settings即时切换、转换快速反向与拖拽期间更新；Windows合同与原生门禁确认现有路径一致。[开发记录](development/2026-09-12-floating-strip-expanded-state-unification.md)。

**0.8.2 悬浮条即时密度与方案 B 轮廓已公开：** Settings 切换 Comfortable、Compact、Mini 时，已展开的原生窗口会先完成尺寸调整再显示最新设置；左贴边暴露侧上下角恢复圆润，贴屏侧上下反向肩弧加强到30pt/px，并与右贴边严格镜像。PR、main和正式发布门禁全绿；七项公开资产、两端更新签名、篡改拒绝和三个更新入口均完成公网验收，[发布记录](development/2026-09-11-v0.8.2-release.md)。

**0.8.1 浮动条轮廓与设置分类更新已公开：** 收起态采用14pt/px内凹把手，展开上下肩严格镜像，底部Settings入口完全移除；Settings新增Floating Strip/悬浮条页签并精简Appearance。PR、main和正式发布门禁全绿；七项公开资产、两端更新签名、篡改拒绝和三个更新入口均完成公网验收，[发布记录](development/2026-09-11-v0.8.1-release.md)。

**0.8.0 Mini浮动条更新已公开：** Appearance固定提供Comfortable 108、Compact 78、Mini 65pt/px；旧Compact不会迁移为Mini。自动收起可独立关闭，Settings齿轮仅由底部深色弧区悬停或键盘焦点显示。PR #34、main与正式发布门禁全绿；七项公开资产、两端更新签名、篡改拒绝和三个更新入口均完成公网验收，[发布记录](development/2026-09-11-v0.8.0-release.md)。

**0.7.3浮动条与强调色更新已公开：** Compact采用65pt/px新贴边轮廓，新增悬停展开/收起、150/800ms双延迟和底部Settings入口；Google Antigravity详情恢复青绿色强调。候选、PR、main和正式发布双平台门禁均通过；七项公开资产、两端更新签名、篡改拒绝与三个更新入口已完成公网验收，[发布记录](development/2026-09-11-v0.7.3-release.md)。

**0.7.2详情与Compact修复已公开：** macOS Google Antigravity详情补回统一深海黑蓝底板；Compact逻辑宽度由65收至56.5pt/px，每侧留白由8.5精确减至4.25，48pt/px圆环和全部纵向尺寸保持。PR #30、合并后main及发布workflow `34486558748`的双平台门禁全部通过；七项公网资产、两端签名与篡改拒绝、三个更新入口均已验收，[发布记录](development/2026-09-10-v0.7.2-release.md)。

**菜单面板顶部Logo放大已随0.7.1交付：** macOS菜单栏弹出面板的应用Logo由32pt放大至40pt，面板宽度、文字、间距、系统菜单栏图标及Windows界面保持。实现`7cadd72`经[PR #25](https://github.com/sljzdotcom/AI-Token-Meter/pull/25)合并为`0a086a2`，PR及main双平台门禁全部通过。[开发记录](development/2026-09-10-menu-panel-logo-enlargement.md)。

**Google Antigravity 迁移已随0.7.1交付：** 第四项保留内部 `gemini` 兼容标识，界面改名 Google Antigravity；采集切换为官方 `agy -p /usage`，展示 Gemini 与 Claude/GPT 的五小时和每周四个额度窗口。旧 Gemini CLI 0.58.0 交互式采集已从当前实现移除。[PR #22](https://github.com/sljzdotcom/AI-Token-Meter/pull/22)合并为`4892b09`，其后[PR #23](https://github.com/sljzdotcom/AI-Token-Meter/pull/23)以`a5f7509`消除Windows测试夹具冷启动波动；最终main双平台workflow `34433979440`/`34433979383`全绿。

**Compact 65pt 已随0.7.1交付：** Compact浮动条宽度从78收至65pt/px，48pt/px圆环与全部高度保持。[PR #26](https://github.com/sljzdotcom/AI-Token-Meter/pull/26)最终候选`c199c6c`及合并提交`1c0a1ed`的双平台原生CI全部通过；[开发记录](development/2026-09-10-compact-strip-width.md)。

**0.6.2/build17 已公开发布：** 菜单面板Logo已随双平台稳定版交付。PR #19最终提交`39c3460`的macOS/Windows CI、合并提交`ac0ec30`的main双平台CI以及发布workflow `34336495454`全部通过；发布后稳定appcast提交为`416ed08`。七项公开资产与三条更新入口已通过无登录下载、SHA-256、Sparkle/Tauri签名和篡改拒绝验证。[发布记录](development/2026-09-09-v0.6.2-release.md)。

PR首轮Windows CI暴露的Tauri MockRuntime入口失败已由`REQ-20260909-005`按上游已知缺陷修正；完整261项runtime、严格Clippy、NSIS、GUI subsystem和安装器上传均在后续PR、main及正式发布门禁中通过，没有跳过或降低测试。

**0.6.3/build18 已公开发布：** macOS About作者行精简已随双平台稳定版交付。PR #20和测试可靠性修复PR #21的最终候选、合并提交`dff54d8`的main双平台CI以及发布workflow `34361767956`全部通过；稳定appcast提交为`fd737f0`。七项公开资产与三条更新入口已通过无登录下载、SHA-256、Sparkle/Tauri签名和篡改拒绝验证。Google Antigravity与Gemini CLI的产品关系另由REQ-007调研，不在缺少官方结论时混入本补丁版。[发布记录](development/2026-09-09-v0.6.3-release.md)。

**菜单面板品牌增强已随0.6.2交付：** macOS菜单栏弹出面板在标题左侧复用32pt应用图标，Windows原生托盘菜单以同一图标和软件名作为不可点击的首行；背景、刷新位置、用量摘要和事件路由保持。合成截图只使用纯色测试图标，功能基线491项Swift、124项前端、261项Rust、真实浏览器密度和无Widget Release App门禁均通过。[开发记录](development/2026-09-09-menu-panel-app-logo.md)。

**Windows CLI 恢复已合入 main（`aca64fc`）：** 显式 npm/Node 启动、统一发现失败分类、Claude 原生/WSL 隔离工作区与初始化、保留限流的手动恢复及自定义包装器路径持久化均已完成。独立复审无阻塞，`d3c57f2` 双平台 CI 全绿，Windows 229 项 Rust 和 NSIS 构建通过。[PR #12](https://github.com/sljzdotcom/AI-Token-Meter/pull/12) 与[调查日志](development/2026-09-08-windows-cli-post-login.md)记录集成证据。修复已随 0.5.1 公开发布，真实账号额度与初始化保留现场验收边界。

**0.6.0 Gemini 已公开发布：** 第四个青绿按钮、四项显示排序与旧配置迁移，以及固定 Gemini CLI 0.58.0 普通 OAuth 的额度采集已交付。Windows 首期仅原生 CLI。最终本地回归为 485 项 Swift、109 项前端、256 项宿主 Rust；PR、main 与正式发布流水线的原生 Windows SDK/ConPTY 门禁全部通过。真实 Gemini 账号不在本轮自动化范围，仍作为现场边界。见[采集日志](development/2026-09-08-gemini-collector.md)、[发布记录](development/2026-09-08-v0.6.0-release.md)和[唯一台账](requirements-backlog.md)。

**0.3.0 功能：** 紧凑/舒适密度、闲置折叠、服务显示排序、右键菜单、状态内环和持久化退避已通过双平台 CI 并合入 main（`c67112e`）。真实 Windows 桌面验收仍受环境限制，见[本阶段日志](development/2026-09-06-compact-progressive-strip.md)和[发布记录](development/2026-09-06-v0.3.0-release.md)。

**macOS 刷新间隔修复（列入 0.6.0）：** 已完成原生编辑/明确保存与运行时重排等待，保留限流退避；440项Swift、Release及两阶段审查通过，由开发入口以 `0a00aae` 完成主分支整合。见[开发记录](development/2026-09-08-macos-refresh-interval.md)。

**顶部横线移除（列入 0.6.0）：** 已完成双平台展开态装饰删除，背景拖动及折叠态竖线保留；442项Swift、96项前端、221项宿主Rust、构建和两阶段审查通过，由开发入口以 `0a00aae` 完成主分支整合；[开发记录](development/2026-09-08-remove-strip-drag-hint.md)。

## 一句话定位

**About 精简已随0.6.3交付：** macOS 关于页已移除独立作者行，版本及 Twitter、GitHub、Telegram 图标链接保持；README、许可证和代码版权归属不变。[开发记录](development/2026-09-09-macos-about-author-line-removal.md)。0.6.0 已完成两端 Telegram 链接与 Windows 作者行移除，[历史记录](development/2026-09-08-about-telegram.md)。

**0.6.0 更新提示：** Windows 关于页发现新版的提示改为深红色加粗，见[开发记录](development/2026-09-08-windows-update-notice.md)。

**0.5.0 品牌改进：** 双平台 About 作者社交图标链接及 Windows 设置顶部 Logo 已合入 main（`12ad5e2`）并公开发布；[完成证据](development/2026-09-07-about-branding.md)。CLI 安装器按点击在线获取，CLI 本体不内嵌安装包。

**0.5.0 安装与账户引导：** CLI 安装/登录引导和 Windows Settings 页签图标已通过最终复审、双平台 CI 和构建验证，PR #10 已合入 main（`266b1f3`）并公开发布；见[需求与验证](development/2026-09-07-cli-onboarding.md)及[本版发布记录](development/2026-09-07-v0.5.0-release.md)。

**0.4.0 历史功能：** 已实现多显示器选择/跨屏拖动、Windows 中英文和中文字体、详情统一减小 1px。[PR #9](https://github.com/sljzdotcom/AI-Token-Meter/pull/9) 保留集成记录。macOS 本地 405+13 项及 Release 构建通过；Windows 本地 57 项前端、197 项 Rust、21 项密度生命周期及 632 项计算样式通过，实际 Windows CI 完成原生运行、NSIS 和 GUI subsystem 验证。独立复审全部阻断已关闭。这些功能继续包含在 0.5.1；物理双屏/DPI 与历史间歇性终端超时的确切原因仍有环境边界，详见[本轮记录](development/2026-09-07-multidisplay-and-windows-localization.md)。

AI Token Meter 是面向 Apple Silicon macOS 14+ 与 Windows 11 x64 的本地桌面浮岛应用，在本机汇总 Claude Code、OpenAI Codex、DeepSeek 和 Google Antigravity 的额度、余额、重置信息及受限的本机/官网历史聚合。两平台使用同版本稳定更新通道；Windows 真实登录、窗口聚焦和原生字体下拉仍需真机确认。

## 当前能力矩阵

| 能力 | Claude Code | OpenAI Codex | DeepSeek | Google Antigravity |
| --- | --- | --- | --- | --- |
| 主要来源 | CLI `/usage` | 自动发现的 CLI/桌面 App 内置 `app-server` JSON-RPC | 官方余额 API | 官方 Antigravity CLI 1.1.28+ 的 `/usage` |
| 身份状态 | `claude auth status --json` | `app-server` account/read | Keychain 中 API Key 后四位 | `agy` 的 Google 登录状态 |
| 主指标 | 当前会话与周额度已用比例 | 通用速率限制已用比例 | 相对余额基准的已消耗比例 | 四个官方窗口的剩余额度换算已用比例 |
| 补充详情 | 本机近 30 天会话、活跃日、Token、趋势 | 重置券；本机近 30 天 Token、连续日、最长会话 | 隔离官网会话中的近 30 天成本、请求、Token、趋势 | Gemini 与 Claude/GPT 的五小时、每周窗口，重置时间、CLI 版本与采集时间 |
| 登录/换号 | Services 打开官方 CLI 登录 | Services 打开官方 CLI 登录 | 两阶段验证后替换 Key | 先在官方 CLI 登录；Services 提供官方指南与重新检查 |
| 失败降级 | 最近成功快照或明确错误 | 最近成功快照或明确错误 | 余额与历史各自独立缓存/错误 | 最近成功快照或明确的安装、认证、版本、配置、解析状态 |

“本机近 30 天”不是跨设备官方账户报表；“DeepSeek 余额基准”也不是预算或账单上限。Google Antigravity 只读取 `agy /usage` 的四个额度窗口，首期不展示 AI Credits。完整口径见[服务与指标说明](user-guide/providers.md)。

## 当前界面

- 菜单栏：18×18pt Quantum Dial 模板图像显示四项服务中最高有效已用比例和精确百分比；点击后的面板标题左侧显示现有应用 Logo；
- 桌面浮岛：默认右侧贴边，可 Automatic/Left/Right，按稳定物理显示器身份记录目标屏、侧边和纵向位置；目标屏断开时仅临时回当前主屏，重新接入后自动恢复；只在桌面层显示；
- 详情：用户点击后临时位于普通应用窗口上方，空白点击或 3/5/8/15/30 秒无交互后关闭；
- Settings：Appearance、Floating Strip、Monitoring、Services、About 五个Tab；macOS Services候选四产品分组标题显示对应Logo，提供默认English、简体中文和繁體中文，Windows提供English/简体中文，两端即时切换应用自有文案，Windows简中把Floating Strip显示为“悬浮条”且Settings始终使用系统字体；
- About：显示当前版本与手动更新状态；仅在用户点击检查时访问 GitHub，发现新版后可明确启动签名更新；
- 显示字体：System、Antonio、DIN Condensed、Alimama FangYuanTi VF、Fira Code、Leigo、Menlo、Alimama DaoLiTi；未安装项禁用并安全回退；
- Widget 源码：Small、Medium、Large 三种布局已实现，但只有带有效 Apple Development 身份和 App Group 的构建才能安装到桌面。

Windows 版保持同一视觉与交互口径：系统托盘取代 macOS 菜单栏入口；Win32 窗口默认右侧贴边，可切左侧并记忆显示器/纵向位置；同显示器全屏应用出现时隐藏；详情临时置前并按相同秒数或外部点击关闭。Windows Widget 暂不在 Preview 范围。

`0.3.0-preview.3` 把 Windows 的“查看详情”和“同步官网历史”分离：点击圆环只显示详情，用户点击 **Sync official history** 后才创建唯一隐藏加载的官方 WebView2；ready 后显示聚焦，关闭/失败/超时恢复详情，完成后销毁官网窗口并展示聚合。opening/active 期间暂停详情自动隐藏。Windows Provider 详情与 Settings 使用专属紧凑密度，Settings 保持系统字体并为字体下拉固定可读浅色配色；macOS 源码和样式未改变。

## 数据与持久化

| 数据 | 位置/所有者 | 是否敏感 |
| --- | --- | --- |
| Claude Code/OpenAI Codex/Google Antigravity 凭证 | 官方 CLI 自行管理 | 是；本应用不读取凭证文件 |
| DeepSeek API Key | Keychain 服务 `com.millerpan.AIMeter.deepseek` | 是；`AfterFirstUnlockThisDeviceOnly` |
| 统一快照 | `~/Library/Application Support/AI Meter/usage-snapshots.json` | 脱敏聚合 |
| DeepSeek 历史 | 同目录 `deepseek-usage-history.json` | 标准化逐日聚合 |
| Claude Code 工作区 | 同目录 `ClaudeUsageWorkspace/` | 空隔离工作区与批准状态 |
| 外观、通知、位置等 | `UserDefaults` | 非敏感偏好 |
| DeepSeek 官网会话 | App 隔离 WebKit 数据存储 | 敏感，由 WebKit 管理 |
| Widget 快照 | 签名 App Group 容器 | 最小脱敏展示数据 |

Windows 对应位置为 `%APPDATA%\AI Token Meter\settings.json`、`%LOCALAPPDATA%\AI Token Meter\cache\` 和独立 WebView2 数据目录；DeepSeek Key 使用 Windows Credential Manager。两平台都不保存 Claude Code、OpenAI Codex 或 Google Antigravity 凭证。

产品已改名，但 Bundle ID `com.millerpan.AIMeter`、可执行文件 `AIMeterApp`、Keychain 服务和 `Application Support/AI Meter` 保持不变，这是兼容策略，不是遗漏。

## 构建与验证基线

- Swift 6 / SwiftPM；更新层固定使用 Sparkle `2.9.4` 二进制依赖；
- Debug/测试和 Release 均面向 `arm64-apple-macosx14.0`；
- 0.6.1最终本地基线：**463项普通测试 + 3项独立刷新调度 + 18项PTY runner + 6项Gemini PTY，总计490项Swift**；Windows为124项前端、5项终端协议、25项浏览器生命周期、8个Gemini详情场景、16组布局、608个文字角色和259项宿主Rust，格式、严格Clippy与无Widget Release App验证通过。
- 0.6.2菜单面板品牌基线：**464项普通测试 + 3项独立刷新调度 + 18项PTY runner + 6项Gemini PTY，总计491项Swift**；Windows为124项前端、5项终端协议、25项浏览器生命周期、8个Gemini详情场景、16组布局、608个文字角色和261项宿主Rust，production build、格式、严格Clippy、6份跨平台合同、219份Markdown、公开安全与无Widget Release App验证通过。
- 0.6.3 macOS About 精简稳定版：**464项普通测试 + 3项独立刷新调度 + 18项PTY runner + 6项Gemini PTY，总计491项Swift**；无Widget Release App、6份跨平台合同、228份Markdown与公开安全检查通过，审查为Critical/Important/Minor `0/0/0`，发布workflow的macOS与Windows签名门禁均通过。
- Antigravity迁移合入main后的基线：**461项普通测试 + 3项独立刷新调度 + 18项PTY runner，总计482项Swift**；Windows为124项前端、241项宿主Rust、25项浏览器生命周期、8个Antigravity详情状态、16组布局和608个文字角色。PR #22实现门禁、PR #23原生测试夹具门禁与main最终workflow `34433979440`/`34433979383`均通过；Windows包含完整runtime、真实ConPTY、真实Edge、严格Clippy、NSIS、GUI subsystem和安装器上传。
- 0.7.1/build20稳定版通过**463项普通测试 + 3项独立刷新调度 + 18项PTY runner，总计484项Swift**；Windows为124项前端、241项宿主Rust、25项浏览器生命周期、8个Antigravity详情状态、16组布局和608个文字角色。前端文件串行调度并保留单项5秒上限；PR #28、合并后main与发布workflow `34460178399`的原生双平台门禁全部通过。
- 0.7.2/build21通过**464项普通测试 + 3项独立刷新调度 + 18项PTY runner，总计485项Swift**；Windows为124项前端、242项宿主Rust、25项浏览器生命周期、8个Antigravity详情状态、9种详情表面、16组布局和608个文字角色。production build、格式、严格Clippy、6份合同、254份Markdown、公开安全与无Widget Release App验证通过；PR #30、合并后main及发布workflow `34486558748`再次完成原生门禁。
- 四服务功能候选`8ad127e`及合并`99e018a`的双平台CI全绿；最终发布候选`7437f63`的PR CI `34314828597`/`34314828526`与合并提交`6fc4e1e`的main CI `34315740630`/`34315740626`再次通过，Windows包含严格Clippy、完整runtime、真实ConPTY、GUI subsystem、NSIS与上传。
- 0.6.0 最终本地基线：**458 项普通测试 + 3 项独立刷新调度 + 18 项 PTY runner + 6 项 Gemini PTY，总计 485 项 Swift**，109 项前端、16 组浮动条密度布局、632 个浏览器文字角色、256 项宿主 Rust 与严格 Clippy；官方 Gemini CLI 隔离合成回归覆盖 2 项测试/4 种场景。
- 0.6.0 最终候选 `26d207b` 的 PR macOS/Windows CI `34301410029`/`34301410049` 全绿；合并提交 `94bf320` 的 main CI `34302154141`/`34302154157` 再次通过，Windows 包含严格 Clippy、完整 runtime、真实 ConPTY、GUI subsystem、NSIS 与上传。
- 已发布 0.5.1 基线继续保留：433 项 Swift、85 项前端、21 项密度生命周期、632 个浏览器文字角色、229 项原生 Windows Rust 及双平台标签门禁全部通过。
- `scripts/test.sh` 同时运行 Swift 测试与文档一致性检查；
- `scripts/build-app.sh` 默认在没有开发证书时输出无 Widget、ad-hoc 签名的主应用，并验证便携资源、Sparkle framework、helper、`@rpath` 和嵌套签名；
- 0.8.0/build23候选基线为493项Swift、133项Windows前端、246项宿主Rust、24组真实浏览器布局；完整门禁与正式发布证据见[0.8.0发布记录](development/2026-09-11-v0.8.0-release.md)。
- 0.8.1/build24候选基线为496项Swift、136项Windows前端、246项宿主Rust、24组展开和2组收起真实浏览器布局；完整门禁与正式发布证据见[0.8.1发布记录](development/2026-09-11-v0.8.1-release.md)。
- 0.8.2/build25稳定版基线为499项Swift、137项Windows前端、254项宿主Rust、24组展开和2组收起真实浏览器布局，并覆盖真实`NSPanel`密度即时调整；完整发布证据见[0.8.2发布记录](development/2026-09-11-v0.8.2-release.md)。
- macOS三语言最终实现`7417628`及本地main合并`207ad63`通过**531项主测试 + 3项独立刷新调度 + 18项PTY runner，共552项Swift**；另有51项资源与真实界面专项。无Widget Release App签名与便携资源验证通过，内含三份各321项的语言表；6份跨平台合同、290份Markdown、公开安全检查及最终独立审查`0/0/0`通过。
- macOS Services产品Logo实现`eeaf215`及本地main合并`28ec9f6`通过**538项主测试 + 3项独立刷新调度 + 18项PTY runner，共559项Swift**；另有147项、18个suite的串行真实窗口专项。无Widget Release App包含四份Provider Logo与三份语言资源，主App及Sparkle嵌套签名有效；6份跨平台合同、293份Markdown、公开安全检查和最终独立审查`0/0/0`通过。
- 0.9.0/build26通过**539项主测试 + 3项独立刷新调度 + 18项PTY runner，共560项Swift**，以及147项真实WindowServer专项；Windows通过137项前端、254项宿主Rust、25项浏览器进程生命周期、24组展开、2组收起和608个文字角色。production build、格式、严格Clippy、6份合同、296份Markdown、公开安全和无Widget Release App签名门禁通过。
- 公开源码仓库为 [sljzdotcom/AI-Token-Meter](https://github.com/sljzdotcom/AI-Token-Meter)。[v0.9.0](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.9.0) 提供两端安装包、SHA-256 与签名更新清单；[公开验收证据](development/2026-09-12-v0.9.0-release.md)。
- `v0.6.3` 标签指向 `dff54d8`，appcast `fd737f0` 首项为0.6.3/build18，Windows stable/旧 Preview同步为0.6.3；发布 workflow `34361767956`三项job成功，七项公开资产的匿名重下、签名/哈希/篡改拒绝与更新兼容已验证。
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
| Windows 11 真机视觉、DPI、全屏与拖动 | 受环境限制 | 在交互式 Windows 11 x64 用户会话安装 Preview 后逐项验收 |
| Windows DeepSeek 显式同步、关闭、复用聚焦、真实登录/聚合与字体下拉 | 待用户确认 | `0.3.0-preview.3` 已列入 `REQ-20260904-006` 修复；在交互式 Windows 11/WebView2 会话按开发日志逐项确认 |
| Windows `preview.0 → preview.1` 签名更新演练 | 待用户确认 | `preview.1` 发布后在交互式 Windows 会话检查原位升级、设置/凭据保留，并另用错误签名 feed 证明旧版不被替换 |
| Windows Authenticode 发布者身份 | 当前限制 | 取得代码签名证书；此前 README/Release 必须保留 SmartScreen 说明 |
| Google Antigravity 真实账号与实际额度 | 待用户确认 | 用户安装包含 REQ-007 的后续版本后，在已登录 `agy` 1.1.28+ 的设备核对四窗口、重置时间和刷新；本轮只做了脱敏只读 CLI 探测，不展示个人额度 |

以上状态不得在证据不足时改写为“已完成”。逐项依据见[需求台账](requirements-backlog.md)。

## 快速入口

- 使用：[安装与首次使用](user-guide/getting-started.md)、[设置参考](user-guide/settings.md)、[故障排查](user-guide/troubleshooting.md)
- 技术：[架构概览](architecture/overview.md)、[架构决策](architecture/decisions.md)、[隐私与安全](security-and-privacy.md)
- 维护：[维护手册](development/maintenance-playbook.md)、[测试指南](development/testing.md)、[发布流程](development/release-process.md)
- 历史：[开发日志](development/README.md)、[提交历史](development/commit-history.md)、[CHANGELOG](../CHANGELOG.md)
