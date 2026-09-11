# Changelog

本文件记录 AI Token Meter 面向使用者的主要变化。格式参考 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，版本号遵循语义化版本思路。

## Unreleased

### Changed

- 浮动条展开高度只保留圆环内容区，上下肩恢复严格镜像；收起态改为14pt/px可见内凹把手，并使用20pt/px宽的透明命中窗口提升可发现性。

### Removed

- 移除悬浮条底部的Settings弧区、齿轮和悬停入口；设置仍可从应用现有菜单进入。

## 0.8.0 - 2026-09-11

稳定功能版；macOS build23。新增65pt/px Mini浮动条密度，恢复78pt/px Compact，并加入可关闭的自动收起设置。[发布记录](docs/development/2026-09-11-v0.8.0-release.md)。

### Added

- Appearance新增Mini浮动条密度，使用65pt/px宽度与48pt/px圆环；尺寸选择固定为Comfortable、Compact、Mini。
- 新增“自动收起浮动条”开关；关闭后浮动条立即保持展开，显示/收起延迟值保留并暂时禁用编辑。

### Changed

- Compact恢复为原有78pt/px宽度；升级时已有Compact设置继续保持Compact，不会自动迁移为Mini。

### Fixed

- 底部Settings提示弧改为深色，并把齿轮显隐限制在底部弧区悬停或键盘焦点，浮动条主体悬停不再显示齿轮。

## 0.7.3 - 2026-09-11

稳定界面更新；macOS build22。重新设计65pt贴边浮动条的悬停交互与Settings入口，并恢复Google Antigravity详情强调色。[发布记录](docs/development/2026-09-11-v0.7.3-release.md)。

### Added

- 浮动条闲置时收成7pt/px可见细条，指针进入后展开；Appearance可分别设置0–2000ms显示延迟和0–5000ms收起延迟，默认150/800ms。
- 浮动条底部弧区增加Settings齿轮，直接打开唯一Appearance设置窗口。

### Changed

- Compact展开宽度采用65pt/px，48pt/px圆环保持；新的分区高度和镜像Bezier轮廓完整包住1至4个Provider，过渡先处理内容可见性再调整窗口。

### Fixed

- Google Antigravity详情恢复青绿色标题、额度值与进度强调，深海不透明底板和额度语义保持。

## 0.7.2 - 2026-09-10

稳定修复版；macOS build21。恢复Google Antigravity详情深色底板，并把Compact圆环两侧留白精确减半。[发布记录](docs/development/2026-09-10-v0.7.2-release.md)。

### Changed

- Compact浮动条逻辑宽度由65收至56.5pt/px，每侧留白由8.5减至4.25；48pt/px圆环、Logo、间距、1至4项高度、Comfortable和折叠把手保持。

### Fixed

- macOS Google Antigravity详情恢复与其他Provider一致的深海黑蓝底板，连接、四个额度窗口、重试和安装入口保持；Windows同步增加真实浏览器表面对等性门禁。

## 0.7.1 - 2026-09-10

稳定功能版；macOS build20。迁移Google Antigravity额度采集，并交付更紧凑的浮动条与菜单面板品牌调整。[发布记录](docs/development/2026-09-10-v0.7.0-release.md)。

### Changed

- 第四项服务迁移为 Google Antigravity，使用官方 `agy -p /usage` 展示 Gemini 与 Claude/GPT 的五小时和每周四个额度窗口；卡片显示剩余百分比，进度环继续表示已用比例。
- macOS 与 Windows 的安装、登录、详情、缓存迁移和 Widget 可见名称同步更新；内部 `gemini` 兼容标识继续保留用户排序、隐藏和历史缓存。
- Compact浮动条宽度由78收至65pt/px；48pt/px圆环、间距、全部高度、Comfortable与折叠把手保持。
- macOS菜单面板标题区应用Logo由32pt放大至40pt；面板宽度、标题、副标题、间距和系统菜单栏图标保持。

### Removed

- 当前采集不再发现旧 `gemini` CLI，也不再使用交互式 `/model`、PTY/ConPTY 按键注入或旧 npm Gemini 启动适配。

### Fixed

- Windows Codex与ConPTY协议测试改用Cargo构建的原生夹具，保留真实协议、进程回收和既有超时门禁，同时消除外部Node冷启动波动。
- Windows前端测试文件改为串行调度，保留每项5秒防挂死上限，避免签名发布runner上的jsdom冷启动资源竞争阻断发行。

## 0.7.0 - 2026-09-10

未公开候选；macOS build19。标签工作流的macOS验证通过，Windows签名任务在前端测试文件并行资源竞争处停止；Release保持草稿，更新源没有切换。恢复版使用0.7.1/build20，不移动或重写该标签。[失败与恢复记录](docs/development/2026-09-10-v0.7.0-release.md)。

## 0.6.3 - 2026-09-09

稳定界面补丁版；macOS build18。精简macOS关于页的作者展示，同时保持联系入口与开源归属。[发布记录](docs/development/2026-09-09-v0.6.3-release.md)。

### Changed

- macOS Settings → About 移除独立的 `Author: Miller` 可见行及其占用的间距；版本、Twitter、GitHub、Telegram、更新功能和开源署名保持。

## 0.6.2 - 2026-09-09

稳定品牌补丁版；macOS build17。在双平台菜单面板顶部加入现有应用 Logo，保持原有布局与操作。[发布记录](docs/development/2026-09-09-v0.6.2-release.md)。

### Added

- macOS 菜单栏弹出面板在 AI Token Meter 标题旁显示现有应用 Logo；Windows 原生托盘菜单以同一 Logo 和软件名作为顶部品牌行。两端保持现有背景、刷新入口、用量摘要与菜单操作不变。

## 0.6.1 - 2026-09-09

稳定修复版；macOS build16。统一四服务商首次接入与恢复路径，修正Gemini安装指南。真实账号和交互式Windows GUI/DPI继续保留现场验收边界。[发布记录](docs/development/2026-09-09-v0.6.1-release.md)。

### Fixed

- macOS与Windows的Claude Code、OpenAI Codex和DeepSeek详情在需要安装、登录、凭据或设置处理时可直接打开Services；带认证原因的旧额度缓存继续显示数据并提供恢复动作，普通网络超时缓存保持只读。
- Claude Code/OpenAI Codex账户状态无法检查时不再在同一服务卡显示两个“Check Status”按钮。
- DeepSeek首次配置显示“Save API Key”，已有Key才显示“Replace API Key”；首次失败不再错误声称保留旧Key。API Key缺失时先引导Services，官网历史登录不再冒充余额凭据恢复；Windows验证期间保持完整busy生命周期和单事务保护。
- Gemini详情与Services的按钮改为明确的0.58.0安装指南并直达官方安装页；缺失时显示Node.js 20+、固定版本安装命令、Google登录及回到应用检查状态的完整步骤，已安装、待登录或已连接状态使用对应提示，不误导重装。

## 0.6.0 - 2026-09-09

稳定功能版已公开发布；macOS build15。新增 Gemini 与四服务布局，并交付此前积累的设置和界面改进。真实 Gemini 账号、Windows 交互式 GUI/DPI 仍保留现场验收边界。[发布记录](docs/development/2026-09-08-v0.6.0-release.md)。

### Fixed

- macOS 设置→监测的用量刷新间隔可编辑并保存，支持 30 秒至 24 小时，默认五分钟；新间隔重排等待、重启保留，已有采集和限流退避保持有效。

### Added

- Gemini 青绿第四按钮、四服务显示与排序。新配置显示四项，旧配置追加隐藏 Gemini 并保留既有顺序。
- 读取固定 Gemini CLI 0.58.0 普通 OAuth 模式的官方模型档位额度，保留全部可见档位、重置说明、来源版本及缓存时间。Windows 首期仅原生 CLI；认证、配置、版本或输出不受支持时明确显示状态。

- macOS 与 Windows 关于页增加带纸飞机图标的 Telegram @sljzdotcom 链接，仅点击时打开默认浏览器。

### Changed

- macOS 与 Windows 浮动条展开态去掉顶部横线，背景拖动、Provider 点击和折叠态竖线保持。

- Windows 关于页移除中英文作者整行及可见社交标题，保留 Twitter/GitHub 并与 Telegram 一起显示；无障碍名称和键盘操作保留。当时的 macOS 作者行仍保留，后续变化见 Unreleased。

- Windows 设置 → 关于：发现新版本时，提示文字使用深红色加粗；中英文同步，字号和其他更新状态不变。

## 0.5.1 - 2026-09-08

稳定补丁版已公开；macOS build14，仅同步版本，产品行为不变。[发布记录](docs/development/2026-09-08-v0.5.1-release.md)。

### Fixed

- Windows 标准官方 npm Codex 安装使用显式 Node + JS 入口，覆盖 npm 与 Node 分离目录及空格路径；不再依赖包装器通过 PATH 寻找 Node。保留受限进程环境及已有登录信息。
- Windows 账户、额度、登录与自定义路径验证共用有界 CLI 发现；只有确认缺失才显示未安装，启动失败保持可重试状态，取消不再被误判为需要安装。

- Windows 自定义 Codex 包装器路径与实际 Node/JS 启动目标分开保存，修复保存成功后再次发现失败。

修复已通过双平台 CI；真实账号额度与工作区初始化仍需新版现场验收。详细边界见[调查与开发记录](docs/development/2026-09-08-windows-cli-post-login.md)。

### Added

- Windows Claude Code 服务卡增加“初始化额度读取”，认证探测、额度与用户初始化共用原生/WSL 专用空工作区；不自动信任主目录、不重复登录。
- 显式“检查状态”重试所选服务额度，自动检查不循环重试；服务端限流等待继续保留。

## 0.5.0 - 2026-09-07

已公开发布；macOS build13、Windows x64 签名安装包及三个更新入口均已验证。见[发布记录](docs/development/2026-09-07-v0.5.0-release.md)。

### Added

- 双平台 About 在作者 Miller 后增加 Twitter/X 与项目 GitHub 图标链接；Windows Settings 顶部复用本地软件 Logo，外部链接仅在点击时打开默认浏览器。
- 双平台 Settings 根据 CLI 状态提供橙色安装/登录按钮；已连接显示账号与重新登录，检测中防重复操作。用户点击后才打开固定官方安装器，安装完成自动重新查找，不自动登录、升级已有 CLI 或清除凭据。
- Windows Settings 四个页签增加 16px 线性图标，保留中英文名称与系统字体，并支持左右方向键切换。

### Fixed

- Windows 覆盖官方独立安装目录；区分 CLI 确认缺失与检测失败，避免已有 CLI 暂时不可用时误重装。显式 WSL/自定义路径不被静默改成 Native。
- 安装/登录轮询有界、可恢复，检查迟到与重复点击不会覆盖当前操作；macOS 检测暂时不可用不再误判重新登录成功。

CLI 按需在线获取；更新签名、公钥、配置与凭据兼容身份均保持不变。验证和集成状态见[CLI 开发记录](docs/development/2026-09-07-cli-onboarding.md)与[About 开发记录](docs/development/2026-09-07-about-branding.md)。

## 0.4.0 - 2026-09-07

### Added

- macOS 与 Windows 增加系统主屏、指定显示器、所有显示器三种展示模式；每屏独立记住边缘和高度，共用采集与唯一详情/官网登录流程。
- Windows 增加 English / 简体中文即时切换，以及 Microsoft YaHei（新安装和恢复默认）、SimHei、KaiTi 字体；保留旧字体偏好，Settings 始终使用系统字体。不分发字体文件。

### Fixed

- macOS 固定左/右贴边不再锁住横向拖动；以系统主屏而非焦点窗口所在屏做回退。指定屏断开时不覆盖原目标，重连恢复。
- Windows 三个 Provider 详情所有文字在 0.3.0 基础上减小 1 CSS px；不改变浮动条、Settings 或 macOS 字号。

实现、复审和现场验收边界见[开发日志](docs/development/2026-09-07-multidisplay-and-windows-localization.md)；发布进度见[发布记录](docs/development/2026-09-07-v0.4.0-release.md)。

## 0.3.0 - 2026-09-06

### Changed

- 双平台同版本更新：macOS稳定appcast、Windows稳定与旧Preview更新源均指向本版；原配置和签名验证公钥不变。
- 发布通道不代表全部Windows真机项目已验收；仍无Authenticode，macOS仍为ad-hoc signed、not notarized，Widget证书继续延期。

### Added

- 双平台 Compact/Comfortable 浮动条密度、可选闲置折叠、服务显示排序与右键快捷菜单；保留原深海背景与品牌Logo。
- 独立刷新/需操作内环、缓存新鲜度提示和跨重启的限流/认证退避。

### Fixed

- 密度和折叠不重新解释历史位置比例；Windows兼容旧450逻辑像素窗口的已保存位置。
- 新浮动条配置损坏不会重置显示器或CLI配置；取消刷新不留下限流惩罚，时钟回拨不绕过Retry-After。

## 0.3.0-preview.3 - 2026-09-04

### Fixed

- Windows 打开 DeepSeek 详情不再隐式创建空白官网窗口；只有用户点击 **Sync official history** 才启动独立官方会话，重复请求复用并聚焦现有窗口，关闭、加载超时、失败与成功都会清理会话并恢复详情。
- Windows DeepSeek 详情新增 opening/active/failed 可见反馈、重试入口和同步期间自动隐藏暂停；迟到命令错误与旧窗口回调不能覆盖较新的同步会话。
- Windows Provider 详情与 Settings 使用专属紧凑字号和间距；Display font 原生下拉明确为白底深色文字，Settings 继续使用系统字体。macOS 样式不变。

### Changed

- macOS 与 Windows 同步提升到 `0.3.0-preview.3`（build `10`）；本版将 `REQ-20260904-006` 的全部修复正式交付到公开 Preview 与 Windows 更新源。

### Security

- Windows DeepSeek 官网 ready、分片、关闭和超时回调绑定短期 nonce 与会话代次，只接受精确官方 HTTPS origin；前端只接收六个固定脱敏状态词，网页正文、Cookie、Authorization、API Key 和个人账号信息不会进入事件或业务日志。

## 0.3.0-preview.2 - 2026-09-04

### Fixed

- Windows 主程序改为 GUI subsystem，启动时不再额外分配标题为 “AI Token Meter” 的空白 Windows Terminal。
- Windows CI 与正式发布流程新增真实 PE Header 门禁，要求主 `.exe` 的 subsystem 为 `2`，防止控制台窗口在后续构建中复发。

### Changed

- macOS 与 Windows 同步提升到 `0.3.0-preview.2`（build `9`）；macOS 浮动条、窗口行为与 Provider 数据链路不变。

## 0.3.0-preview.1 - 2026-09-04

### Fixed

- Windows 浮动条精确复用 macOS 的四段 Bezier 轮廓，并由 WebView2 单一抗锯齿裁剪；消除 CSS 粗多边形与 Win32 GDI Region 双重裁剪造成的凹凸、锯齿和反向肩部缺失。
- Windows meter 明确关闭系统阴影和 DWM 边框色，避免透明无边框窗口出现 1px 白框及上下白条；深海背景覆盖完整上下肩部。
- Windows 拖动改为原生鼠标释放后只吸附一次；拖动期间显示器拓扑恢复暂停，首次拓扑轮询只建立基线，避免提前吸附、位置跳动和重启后漂移。

### Changed

- macOS 与 Windows 同步提升到 `0.3.0-preview.1`（build `8`）；macOS 浮动条视觉与窗口行为不变。

## 0.3.0-preview.0 - 2026-09-04

### Added

- 新增 Windows 11 x64 应用：Tauri 2 + Rust + React 界面、Win32 左右贴边浮动条、系统托盘、全屏应用隐藏、详情置前、设置页与三服务状态。
- Windows 可从原生安装和 WSL 发现 Claude Code/OpenAI Codex CLI，显示当前账号、运行来源与版本，并提供固定官方重新登录入口。
- Windows DeepSeek 使用 Credential Manager 保存 API Key，先经官方余额接口验证再替换；隔离 WebView2 会话可聚合官网最近 30 天成本、请求与 Token。
- 新增 Windows current-user NSIS、手动检查/立即更新、Tauri minisign 更新验证，以及 macOS/Windows 同版本草稿 Release 门禁。
- 新增根 `VERSION`、跨平台用量 Schema、展示合同、共享 fixture、功能对等矩阵和 Windows CI。

### Security

- Windows CLI 进程使用受限参数、Job Object 与 ConPTY，登录只允许固定官方命令；日志、事件、缓存和网页桥均拒绝凭据与个人身份字段。
- Windows 更新私钥不进入源码或普通 CI；未配置仓库 Secret 时正式发布工作流明确失败并保留草稿 Release。首个 Preview 在取得 Authenticode 证书前会如实说明 SmartScreen 边界。

### Fixed

- 浮动条改用稳定物理显示器身份恢复位置；重启、休眠、主屏角色变化或屏幕枚举顺序变化不再把已保存的左/右侧、相对高度或目标屏擅自改回默认值。
- 目标显示器断开时，macOS 与 Windows 只临时回到当前主屏并保留侧边和高度，不覆盖原目标；目标屏重新接入后自动恢复。macOS 会无损迁移旧版数字屏幕编号。
- DeepSeek Keychain 读取截止时间改由独立 GCD 单调时钟队列驱动；高并发下即使阻塞读取占满 Swift 协作线程，超时仍会准时生效，迟到的 API Key 不会继续触发网络请求。
- macOS DeepSeek Keychain 读取不再以低于交互操作的任务优先级运行，避免高并发环境中即时凭据读取被饿死到 2 秒边界并误报超时；真正阻塞的读取仍保留原有截止时间。
- macOS PTY 父进程退出的备用确认不再运行于低 QoS 队列，避免 runner 高负载时已退出命令等待到截止时间；命令取消、超时和后代进程清理语义不变。

## 0.2.2 - 2026-09-03

### Fixed

- 从 Settings 启动更新时，应用会在展示 Sparkle 标准安装流程前隐藏 Settings 并激活自身，避免下载完成后的 **Install and Relaunch** 窗口被设置窗口遮挡、界面看似长期停在 Preparing。
- 只让 SwiftUI Settings 窗口临时让位，不关闭普通窗口，不改变手动检查、用户确认、EdDSA 签名验证、失败回退或重新启动行为。
- 发布回归中的 32 路并发 PTY fixture 改为只使用 Shell 内建读取，避免测试自身额外派生 64 个进程并在 GitHub runner 资源紧张时产生假性输出缺失；同时显式验证每个子命令退出码。

## 0.2.1 - 2026-09-03

### Fixed

- 修复 GitHub macOS runner 或本机高负载下，Foundation 进程退出回调延迟导致 PTY 命令误报超时，以及非阻塞读取短暂无数据时过早停止、遗漏输出尾部的问题。
- 完整验证将 PTY 系统资源测试放入独立测试进程，在不减少覆盖的前提下避免截图、文件扫描和大量子进程造成的 runner 抢占噪声。

## 0.2.0 - 2026-09-02

### Added

- Settings → About 新增 `Check for Updates` 与 `Update Now`。只有用户手动检查才访问 GitHub；发现更高稳定版本后，由用户明确启动下载和安装。
- 集成 Sparkle 2.9.4，通过 GitHub `appcast.xml` 发现版本，并用标准更新窗口完成下载、替换与重新启动。
- 新增可复现的更新发布入口，统一生成 Apple Silicon ZIP、SHA-256、appcast enclosure 与签名验证证据。

### Changed

- 应用版本升级为 `0.2.0`（build `4`）。Release 构建现在完整嵌入 Sparkle framework、Updater、Autoupdate 与两个 XPC helper，并验证 `@rpath` 和嵌套签名。
- `0.1.2` 到 `0.2.0` 仍需手动替换一次；从 `0.2.0` 开始，后续稳定版本可在应用内完成手动检查与更新。

### Fixed

- 修复高并发 CLI 刷新或发布回归中，macOS `openpty` 偶发竞争失败并误报 `transportFailure` 的问题；仅串行化极短的 PTY 分配临界区，命令执行与三服务采集仍可并行。
- ad-hoc 分发不再错误启用会拒绝无 TeamIdentifier framework 的 library validation；真实开发者证书构建仍保留 hardened runtime。
- 更新归档验证会核对 ZIP 长度、版本、build 与 EdDSA 签名，并明确拒绝被追加或篡改的归档。

### Security

- 更新包使用 Sparkle EdDSA 签名；App 仅内置公开验证键，生产私钥只保存在维护者 macOS Keychain 中，不导出到仓库、日志或 Release。
- 公开发布门禁新增 `.key` 与 Sparkle 私钥导出标记检查；签名或下载验证失败时不会替换当前 App。

## 0.1.2 - 2026-09-02

### Added

- OpenAI Codex 确实未安装时，Services 显示可操作的 OpenAI 官方 CLI 安装指南入口。
- 增加公开项目所需的 MIT License、行为准则、支持说明、Issue/PR 模板、macOS CI 与脱敏产品截图。
- README 增加英文摘要、GitHub Release 安装说明和作者 Miller；About 页面同步显示作者信息。

### Fixed

- 修复完整并发测试或高负载下，PTY 子进程退出等待同步占用 Swift 并发线程池，进而导致 Claude/Codex 输出丢失或误报超时的问题；退出通知现在采用真正异步的 continuation，并覆盖 32 路并发回归。
- 修复 Finder 启动的 AI Token Meter 无法发现通过 nvm 安装在 `~/.nvm/versions/node/*/bin` 中的 OpenAI Codex CLI，因而错误显示 `CLI not installed` 的问题。
- 启动 Codex `app-server` 和登录脚本时把所选 CLI 的同目录放到 PATH 首位，使 `#!/usr/bin/env node` 能找到匹配的 Node 运行时。
- 增加已安装 ChatGPT/Codex 桌面应用内置原生 `codex` 的安全后备路径，并保持用户显式 CLI 优先。

### Security

- 增加公开发布门禁，对当前文件、完整 Git 历史和 Release ZIP 执行高置信度凭据检查，并在可用时使用 Gitleaks 复核且不回显秘密。

## 0.1.1 - 2026-09-02

### Added

- 新增当前项目状态、架构决策、维护手册与全项目复盘文档，并把断链、版本、测试基线和目录治理纳入自动检查。
- 新增原生 macOS WidgetKit 桌面组件，支持 Small、Medium、Large；最小尺寸仅显示 Claude、Codex、DeepSeek 三个 Logo 状态环，中型显示三张额度卡，大型追加最近重置与 Codex 重置券摘要。
- 新增隐私安全的 App Group Widget 快照、30 分钟系统时间线建议、过期状态降级和点击唤醒主应用深链；Widget 本身不联网、不调用 CLI、不访问 Keychain。
- 构建脚本新增 `AI_METER_INCLUDE_WIDGET=auto|0|1`、Apple Development 身份检测、嵌套扩展签名及 App Group 一致性验证；无开发签名时普通主应用仍可构建。
- Settings 采用 Appearance、Monitoring、Services、About 四个顶部 Tab，并按职责安置现有选项。
- Services 新增 Claude、Codex、DeepSeek 常驻账户状态；Claude/Codex 支持通过官方 CLI 一键登录或重新登录、有限自动回查和手动检查状态。
- DeepSeek 设置新增遮罩 Key 身份和两阶段替换：候选 Key 先通过官方余额接口验证，验证成功后才更新 Keychain。
- Appearance 新增全局显示字体选择：System Default、Antonio、DIN Condensed，以及 `Restore Default Font`；可用字体会即时应用到 App 自绘文字并持久化，缺失字体禁用且安全回退到系统字体。第三方字体须由用户预先安装，AI Token Meter 不下载或分发字体文件。
- 显示字体目录新增 Alimama FangYuanTi VF、Fira Code、Leigo、Menlo、Alimama DaoLiTi，支持已安装检测、别名解析与系统字体安全回退。
- 贴边浮岛内部新增静态黑蓝「深海波纹」背景；左右贴边时仅纹理随轮廓镜像，Logo、品牌进度色、点击和拖动行为保持不变，资源缺失时自动回退到原玻璃底色。
- 可配置的详情自动隐藏时间：3、5、8、15 或 30 秒，默认 8 秒。
- 点击悬浮条和详情以外区域立即关闭详情；悬停和 DeepSeek 登录交互暂停倒计时。
- Claude 专用空工作区和一次性批准入口，减少用户项目、MCP 与工作区信任对 `/usage` 的干扰。
- Claude 新增额度优先专用详情页，并补充明确标注为 `This Mac` 的 Claude Code 最近 30 天会话、活跃日、Token 总量与每日趋势。
- Codex 可用重置额度、名称和到期日的只读展示。
- Codex 额度优先详情页，以及近 30 天本机 Token、当前连续使用天数和最长会话三项聚合。
- DeepSeek 可配置余额基准，默认 ¥100。
- DeepSeek 最近 30 天成本、API 请求数、Token 数和每日成本图表。
- DeepSeek App 内隔离官网登录会话、标准化历史缓存与缓存降级。
- 三个悬浮圆环统一为大尺寸品牌 Logo，并保留完整无障碍描述。
- 支持 Automatic、Left、Right 三种贴边模式；自动模式可在拖动结束时吸附最近侧边。
- 记住浮岛所在显示器、最终侧边和垂直位置，显示器布局变化时重新夹紧到可见区域。
- 新增无文字仪表指针 App Icon，并在本地构建时生成全部 macOS 图标尺寸。
- 完整 GitHub 风格文档体系：用户指南、架构、隐私、安全、开发、测试、发布和提交历史。
- 可移植测试脚本，把 SwiftPM 与 Clang 缓存隔离到临时目录，兼容 Dropbox 和受限执行环境。

### Changed

- 需求状态统一由 `docs/requirements-backlog.md` 管理，全部设计规格和实施计划统一归档到 `docs/design`；删除已被正式记录覆盖的旧需求副本与临时代理报告。
- 当前界面、辅助功能、通知、Widget 与现行文档中的服务名称统一为 **Claude Code**、**OpenAI Codex** 和 **DeepSeek**；CLI 命令、路径、缓存标识与历史记录保持兼容。
- 菜单栏顶部的通用仪表 SF Symbol 改为自绘 18×18pt Quantum Dial；断环进度与指针跟随三项服务中的最高有效已用比例，保留精确百分比文字，无数据时使用中性状态，并自动适配 macOS 菜单栏前景色。
- 产品显示名称改为 **AI Token Meter**，副标题改为 **Private AI usage monitor**，构建产物改为 `dist/AI Token Meter.app`；Bundle Identifier、可执行文件名、Keychain 身份和旧数据目录保持兼容。
- Claude 详情页移除 Token composition、Top models 以及底部的内联隐私说明，保留官方额度、本机三项统计和每日趋势；底层隐私保护、采集与旧缓存兼容性不变。
- Settings 现在固定使用 macOS 系统字体，字体选项只显示名称；浮动条、三个详情页和菜单点击面板的产品文字统一增大 1pt。
- Claude、Codex、DeepSeek 现在分别使用黄橙、玫红紫、薄荷紫品牌渐变，并同步到圆环、菜单卡片、详情进度条、标题和关键数据；Claude 与 Codex 的异常语义色仍优先，DeepSeek 始终保留用于表达余额消耗的原薄荷紫渐变。
- Codex 重置券改为分层卡片：突出可用数量、完整到期时间和自然日剩余状态，并按券数量自适应详情高度。
- 悬浮条由带边距的圆角矩形改为左右可镜像的无缝贴边浮岛；详情始终向桌面内部展开。
- Claude、Codex、DeepSeek 详情页统一为深色玻璃表面、青绿至蓝紫重点色和一致的卡片层级。
- Claude、Codex、DeepSeek Logo 使用同一套光学校正规则，使三个图形在 60 点圆环中的视觉重量接近。
- Claude 交互命令以终端 Enter 对应的 CR 提交，并在读取到额度结果后尽快结束。
- Claude 解析器忽略促销说明中的百分比，只接受明确的已用/剩余额度行。
- Codex 优先展示顶层通用速率限制，不再被模型专属窗口覆盖。
- 圆环和中央数值共享同一展示指标；0% 或无百分比指标时不再绘制虚假最小弧。
- DeepSeek 圆环由“余额文本”改为相对余额基准的已消耗比例。
- DeepSeek API Key 输入框不再回显旧值；替换失败时保留输入与原 Key，成功后才清空并刷新额度。
- 历史设计资料从内部命名的 `docs/superpowers` 迁移到 `docs/design`。

### Fixed

- 修复将 Release ZIP 复制到另一台 Mac 后，SwiftPM 图片资源仍回退到构建机绝对临时路径并导致应用启动崩溃的问题；主应用资源现直接嵌入标准 `Contents/Resources`，发布构建会强制验证完整资源布局。
- 修复自绘 Quantum Dial 在部分深色或动态菜单栏背景下与背景融合、看起来不可见的问题；图形现以 macOS 模板图像输出，由系统按当前菜单栏前景色可靠着色。
- Claude 本机活动改为受时限约束的可选数据源：官方额度失败会立即返回，官方额度成功也最多等待 2 秒；本机扫描采用流式读取、30 日文件时间过滤、总量/文件数/持续时间上限，避免大型历史目录拖慢统一刷新。
- 旧缓存与新采集中的 Claude 模型标识会执行长度、字符集和敏感文本校验；详情补充显式空状态和官方/本机分区无障碍标签，即使历史缓存仍含模型聚合也不会展示模型明细。
- Claude 详情页的标题与官方额度固定可见，只有本机活动区域按可用高度滚动。
- 修复桌面已有普通应用窗口时，点击 Provider 后详情页仍落在窗口栈底部的问题；浮岛继续保持桌面层，临时详情改用标准 floating 层级，关闭后立即移除。
- 将浮岛与详情从系统浮动层降到桌面层，移除全屏辅助行为，使普通应用窗口和全屏空间可自然覆盖 AI Token Meter。
- 修复切换 Space 后详情继续残留的问题；Space 变化现在关闭详情、撤销交互焦点并保持用户已保存的屏幕、侧边和垂直位置。
- 修复深海背景在浮岛上下肩部出现黑色断层的问题；背景改为 `1.22×` 等比裁切，左右仅水平镜像，原始 PNG 保持不变。
- 将浮岛上下跨度过大的长 S 肩部收窄为“短平台 + 紧凑圆弧”，减少贴边处过度外鼓，同时保持左右镜像、主体拖动、Logo 点击和详情布局不变。
- 修复 DeepSeek 在缓存、超时或余额告警状态下被橙色等语义色覆盖、看不到原薄荷紫配色的问题；采集状态继续通过详情文字、状态符号和无障碍描述表达。
- 浮岛肩部增加长切线收直缓冲并移除突兀的黑色外投影，使上下接边在浅色壁纸上更圆滑。
- 将贴边浮岛上下的分段平台肩部替换为从屏幕边缘尖点展开的连续 S 曲线，左右严格镜像并保留完整玻璃拖动区域。
- 修复贴边浮岛的反向半圆肩部退化成方形、顶部多余短横，以及贴边边缘的视觉接缝。
- 修复浮岛只能从很小的顶部手柄拖动的问题；现在 Logo 以外的玻璃空白区域均可拖动，Logo 点击仍只打开详情。
- 修复固定 Left/Right 模式下拖动命中区与显示布局不一致的问题，并保留位置重启后的恢复行为。
- 修复菜单栏中的 Settings 入口不能稳定打开或置前既有设置窗口的问题。
- 修复应用启动时同步读取钥匙串导致浮窗迟迟不出现的问题；DeepSeek 密钥读取超过 2 秒会独立降级，不再阻塞 Claude 与 Codex 刷新。
- 修复 DeepSeek 内嵌官网登录页的手机号、验证码等输入框无法获得键盘焦点的问题；Claude、Codex 详情仍保持不抢占当前应用。
- 修复 DeepSeek 官网 2026-08 拆分的按 API Key 用量/费用响应无法自动合并，导致原生 30 天图表不更新的问题。
- 修复 Claude 登录有效但因工作区信任或命令未提交而持续超时的问题。
- 修复 Codex 进程超时时丢失根因的问题。
- 修复外部点击监听只覆盖部分 App 或事件坐标的问题。
- 修复旧自动隐藏任务关闭新详情、退出后任务未取消的问题。
- 修复 Claude 促销百分比被误识别成真实周用量的问题。
- 修复 Codex 模型专属 0% 覆盖通用周额度的问题。
- 修复 0% 仍显示进度弧以及文字和圆环使用不同额度窗口的问题。
- 修复浮岛窗口边缘残留透明空白、损坏位置产生无效坐标，以及已保存显示器断开后设置无法重新定位的问题。
- 修复键盘或 VoiceOver 阅读详情时自动隐藏打断操作，以及旧详情交互状态影响新详情计时的问题。

### Security

- DeepSeek 官网响应仅接受官方 HTTPS 来源、受限负载大小和可识别结构。
- Codex 重置额度不保存兑换 ID，也不提供自动兑换或“立即使用”。
- DeepSeek 业务缓存只保存标准化逐日聚合，不保存 Cookie、授权头、登录字段或网页原始响应。
- Claude/Codex 账户邮箱与套餐、DeepSeek Key 后四位仅保留在 Settings 的内存状态，不进入快照、Widget、通知、日志或登录脚本；登录脚本权限固定为 `0700` 且只含批准的官方命令。
- Claude 本机活动读取采用 JSON 白名单字段、流式读取、文件时间过滤、总量/文件数/持续时间边界和安全模型标识规范化，不保存或展示提示词、回复、项目路径、标题和分支；本机读取失败或超时不会改变官方额度结果。

## 0.1.0 - 2026-08-28

### Added

- 原生 macOS 菜单栏应用和右侧悬浮用量条。
- Claude Code CLI `/usage` 采集与当前/次级额度解析。
- Codex CLI `app-server` 结构化速率限制采集。
- DeepSeek 官方余额 API 支持。
- DeepSeek API Key 的 macOS Keychain 存储。
- 三服务统一快照、5 分钟刷新、手动刷新和本地缓存降级。
- 70% / 90% 用量通知。
- 登录时启动、显示/隐藏悬浮条和设置窗口。
- SwiftPM 测试、release 构建、App Bundle 组装和 ad-hoc 签名脚本。

### Security

- CLI 凭证仍由官方工具管理，AI Meter 不读取凭证文件。
- API Key 不进入偏好、缓存或日志。
- 错误、缓存与通知在展示或持久化前进行敏感文本清理。

> 当前仓库尚未创建 `v0.1.0` tag。远程地址和正式发布确定后，再为版本标题添加实际的比较与 Release 链接。
