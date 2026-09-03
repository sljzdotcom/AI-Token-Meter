# AI Token Meter Windows 平台设计

- **需求：** `REQ-20260903-004`
- **日期：** 2026-09-03
- **状态：** 方案 A 已确认，待书面规格复核

## 1. 目标

为 AI Token Meter 增加可公开发布的 Windows 11 x64 版本，并把后续产品开发调整为 macOS 与 Windows 同版本、同功能语义、同 GitHub Release 的双平台流程。

Windows 首版必须包含：

- Windows 系统托盘入口；
- 默认右侧、可切换左侧的贴边浮动条；
- Claude Code、OpenAI Codex、DeepSeek 三个 Provider；
- Provider 详情、Settings、账户恢复与手动应用更新；
- 与 macOS 一致的状态语义、百分比口径、Provider 正式名称和隐私边界；
- Windows 11 x64 安装包、校验文件、更新签名和公开 Release 文档。

Windows Widget 不进入首版。macOS Widget 继续由 `REQ-20260901-003` 管理，不因 Windows 工作被误标为完成。

## 2. 已确认方案

采用**方案 A：双原生壳层 + 共享产品合同**。

- macOS 保持现有 Swift 6、SwiftUI、AppKit、WebKit、WidgetKit 与 Sparkle 实现；
- Windows 新增 Tauri 2 桌面应用，Rust 负责系统集成、进程、网络、缓存和凭据，React + TypeScript 负责视图；
- 两个平台不强行共享 UI 源码，以免重写已稳定的 macOS 行为；
- 两个平台共享版本号、领域数据合同、脱敏外部响应 fixture、状态与文案清单、功能对等矩阵和发布门禁；
- 同一正式版本只有在两个平台各自通过自动化与真实验收后才发布。

### 2.1 未采用方案

1. **Swift 直接编译 Windows：** 当前 App 大量依赖 SwiftUI、AppKit、WebKit、WidgetKit、Security 和 Sparkle；Windows Swift 不提供这些框架，仍需重写完整平台层，生态与维护收益不足。
2. **WinUI 3/.NET 独立复刻：** 原生体验良好，但 Provider 核心、更新、窗口与 UI 都会形成另一套高重复实现，长期漂移风险最高。
3. **macOS 和 Windows 全部改写为 Tauri：** 能共享更多 UI，但会重做成熟的桌面层窗口、菜单栏、Keychain、Widget 与 Sparkle，回归成本与现阶段收益不匹配。

## 3. 代码库结构

首阶段不移动现有 macOS 目录，避免无价值的大规模路径重构。新增结构如下：

```text
AI-Meter/
├── Sources/                         # 现有 macOS 源码
├── Tests/                           # 现有 macOS 测试
├── Package.swift                    # macOS Swift Package
├── windows/
│   ├── package.json
│   ├── src/                         # React + TypeScript 视图
│   │   ├── components/
│   │   ├── details/
│   │   ├── settings/
│   │   ├── state/
│   │   └── theme/
│   ├── src-tauri/
│   │   ├── Cargo.toml
│   │   ├── tauri.conf.json
│   │   ├── capabilities/
│   │   └── src/
│   │       ├── accounts/
│   │       ├── collectors/
│   │       ├── coordination/
│   │       ├── domain/
│   │       ├── persistence/
│   │       ├── platform/windows/
│   │       ├── security/
│   │       └── updater/
│   └── tests/
├── contracts/
│   ├── schemas/                     # 版本化共享 JSON Schema
│   ├── fixtures/                    # 脱敏共享解析样本
│   ├── presentation/                # 名称、状态、颜色与显示合同
│   └── parity/                      # 双平台功能对等清单
└── VERSION                          # 两个平台唯一产品版本来源
```

`contracts/` 是产品语义来源，不保存凭据或真实账户响应。Swift 和 Rust 可以保留各自适合平台的类型，但必须通过同一批 fixture 和 Schema 验证。

## 4. 共享产品合同

### 4.1 `UsageSnapshot` 合同

共享合同至少覆盖：

- `schemaVersion`、Provider ID、正式显示名称；
- 主/次指标、单位、已使用比例、重置时间；
- `fresh`、`cached`、`refreshing`、`notInstalled`、`authenticationRequired`、`setupRequired`、`unavailable`、`unrecognizedOutput`；
- 数据采集时间、过期时间和来源版本；
- OpenAI Codex 重置券数量与到期时间；
- Claude Code / OpenAI Codex 本机 30 日聚合；
- DeepSeek 余额、余额基准和每日历史。

旧缓存必须向后兼容；遇到未知新字段应忽略，未知主版本应安全降级为空状态，不得显示错误数值。

### 4.2 展示合同

共享并测试以下规则：

- 正式名称固定为 **Claude Code**、**OpenAI Codex**、**DeepSeek**；
- DeepSeek 圆环表示相对余额基准的“已消耗比例”，余额越少圆环越长；
- Provider 品牌色只用于正常状态，warning、critical、cached、unavailable 等语义状态优先；
- Settings 始终使用系统字体；浮动条、菜单和详情使用用户选择的展示字体并有安全回退；
- Logo、卡片顺序、详情主要模块和无障碍名称保持一致；
- 平台差异不得改变数据含义，只能改变符合平台习惯的控件与窗口表现。

### 4.3 功能对等矩阵

每个新需求必须在 `contracts/parity/features.yml` 记录：

- 功能 ID 和关联 `REQ`；
- macOS 状态与测试证据；
- Windows 状态与测试证据；
- 是否为明确的平台专属能力；
- 文档、截图、安装与更新验收状态。

普通跨平台功能若一端缺失，状态不能标为“已完成”，正式 Release 也不能继续。Widget、菜单栏/托盘名称等平台专属项目必须明确声明，不用伪造一模一样的系统能力。

## 5. Windows 运行架构

```text
Claude Code native / WSL ─┐
OpenAI Codex native / WSL ├─> Rust Collectors ─> RefreshCoordinator
DeepSeek API ──────────────┘             │
                                         ├─> versioned cache
DeepSeek WebView2 ─> Normalizer ─────────┘
                                         │
                                    UsageSnapshot
                                         │
                    ┌────────────────────┼───────────────────┐
                    │                    │                   │
                 Tray menu        Floating strip       Detail/Settings
```

Rust 后端是 Windows 界面访问本机能力的唯一入口。React 层不能直接运行任意命令、读取任意文件或取得明文 DeepSeek Key。所有 Tauri command 都使用最小 capability 白名单和结构化输入，不开放通用 shell API。

刷新保持三个 Provider 并发、单项故障隔离、缓存可辨识降级。应用默认每 300 秒刷新；应用更新检查与 Provider 刷新完全分离。

## 6. CLI 发现与执行

### 6.1 发现原则

Windows GUI 应用同样不能假设获得用户交互式终端的 PATH。发现器采用以下顺序：

1. 用户在 Settings 中保存的已验证自定义路径；
2. 当前进程 PATH 与 `where.exe` 可见路径；
3. Windows 用户级/系统级 PATH 注册值；
4. 官方原生安装、WinGet、npm、常见 Node 管理器与 `%LOCALAPPDATA%` / `%APPDATA%` 约定；
5. 官方桌面应用提供且经过验证的稳定可执行文件；
6. WSL2 已安装发行版中的固定命令探测。

不得写死用户名称、某个 Node 版本或某一台机器的绝对路径。候选路径必须是普通文件、可执行、名称匹配且能通过 `--version` 健康检查。

WSL 发行版名称从 `wsl.exe` 的结构化/逐行输出取得，只作为独立参数传递，不拼入 shell 字符串。首选默认发行版，也允许用户在 Settings 选择已经验证的发行版。

### 6.2 Claude Code

- 支持官方原生 Windows 安装和 WSL2；
- 原生交互式 `/usage` 通过 ConPTY 运行；
- WSL2 通过 `wsl.exe --distribution <name> --exec ...` 适配，并在发行版内使用 PTY；
- 使用应用专属空工作区，首次信任仍由用户本人完成；
- 登录按钮只启动固定的 `claude auth login`，不得附加用户可控 shell 片段；
- 本机活动只聚合白名单计量字段，不读取会话正文。

Claude Code 官方支持 Windows 原生安装和 WSL，原生模式可使用 PowerShell 或 Git Bash。设计依据：<https://code.claude.com/docs/en/getting-started>。

### 6.3 OpenAI Codex

- 优先支持 Windows 原生 Codex CLI / ChatGPT Windows 环境；
- 使用 `codex app-server` 读取官方额度和重置券；
- 读取 `%USERPROFILE%\.codex` 的允许聚合字段；
- WSL2 作为显式后备，读取该发行版的 `~/.codex`；
- 登录按钮只启动固定的 `codex login`；
- 原生和 WSL 的账户、配置与本机活动分别标明来源，不能把两个环境的数据默默混合。

OpenAI 官方将 Windows 11 列为推荐基线，支持原生 Windows 和 WSL2；Windows App 的原生 Codex home 为 `%USERPROFILE%\.codex`。设计依据：<https://learn.chatgpt.com/docs/windows/windows-sandbox>、<https://learn.chatgpt.com/docs/windows/windows-app>、<https://learn.chatgpt.com/docs/windows/wsl>。

### 6.4 进程边界

- 原生交互进程使用 Windows ConPTY；
- 非交互进程使用显式 executable + argument 数组；
- 每个命令具有超时、取消、进程树终止和输出大小上限；
- 只把所选 executable 所需的父目录加入子进程 PATH；
- 不加载 PowerShell profile、CMD AutoRun 或未知项目启动脚本；
- 原始 CLI 输出只在内存解析，禁止写入日志或缓存。

## 7. DeepSeek

### 7.1 API Key

- Key 存入 Windows Credential Manager，使用当前登录用户的系统保护；
- Settings 只显示末四位遮罩；
- 替换时先用候选 Key 调官方余额接口，成功后才替换旧 Key；
- 删除、读取失败与验证失败均不得在日志中输出明文；
- 不把 Key 写入前端状态、Tauri Store、环境变量、JSON 配置或崩溃报告。

Microsoft 建议桌面应用优先使用 Windows Credential Manager 的 `CredWrite` / `CredRead` 保存凭据。设计依据：<https://learn.microsoft.com/en-us/windows/win32/secbp/handling-passwords>。

### 7.2 最近 30 天历史

- 使用独立 WebView2 用户数据目录，不读取 Edge、Chrome 或其他浏览器 Cookie；
- 导航和数据接收只允许 DeepSeek 官方 HTTPS 域名；
- 通过受控 WebView2 适配器接收需要的用量与费用响应，完整分片到齐后再标准化；
- 只保存日期、费用、请求数、Token 数和更新时间；
- 官网格式变化只让历史图降级，不影响余额 API；
- 登录表单、Cookie、Authorization Header 与原始响应不进入业务层。

## 8. Windows 交互与视觉

### 8.1 系统托盘

- 使用与 macOS Quantum Dial 同一设计语言的 Windows 托盘图标；
- 针对浅色、深色和高对比度任务栏分别输出清晰资源；
- 左键显示菜单，右键也提供标准上下文菜单；
- 菜单包含三项摘要、刷新、Settings、显示/隐藏浮动条、About 和退出；
- 托盘图标不依赖彩色细节表达唯一状态，避免小尺寸和高对比模式丢失信息。

Tauri 2 提供托盘图标、菜单和鼠标事件能力。设计依据：<https://v2.tauri.app/learn/system-tray/>。

### 8.2 浮动条

- 无标题、透明、无任务栏按钮、默认不激活的工具窗口；
- 默认吸附主显示器右侧，Settings 可选左侧或自动恢复最后一侧；
- 保留确认过的紧凑 S 曲线、深海背景、三个 Logo 和品牌环；
- 背景覆盖完整形状，包括上下肩部，不出现拼接边框或突兀阴影；
- 允许从非 Logo 区域垂直拖动，并按显示器工作区保存归一化位置；
- 处理 100%–300% DPI、任务栏位置变化、显示器连接/断开与分辨率变化；
- 不注册 AppBar、不占用工作区，不挤压普通应用窗口；
- 普通窗口自然覆盖浮动条；检测到当前显示器有全屏前台应用时隐藏，返回桌面后恢复；
- 不依赖未公开的 `WorkerW` 父子窗口注入作为唯一方案。

“桌面层但不覆盖全屏应用”必须在 Windows 11 真机验证；若标准窗口层级无法在某个系统版本稳定重现，则安全回退为前台非桌面时隐藏，而不是让浮动条常驻最上层。

### 8.3 详情窗口

- 点击 Logo 后显示在普通应用窗口之上并获得必要焦点；
- 同一时间只存在一个 Provider 详情；
- 点击窗口外、再次点击当前 Logo或达到设置的自动隐藏时间后关闭；
- DeepSeek 网页交互期间暂停自动隐藏，结束输入后恢复；
- 关闭后立即退出 topmost 状态，不残留透明置顶窗口；
- 内容结构、名称、品牌色和主要信息顺序与 macOS 保持一致，窗口尺寸可按内容与屏幕自适应。

### 8.4 Settings

Windows Settings 延续 Appearance、Monitoring、Services、About 分类：

- Settings 永远使用 Windows 系统字体；
- Appearance：侧边、位置、背景、展示字体与字号；
- Monitoring：刷新、启动项、阈值、详情自动隐藏；
- Services：当前账户、CLI 来源、健康状态、重新登录、DeepSeek Key 替换和 WSL 选择；
- About：产品名、作者 Miller、版本、隐私、GitHub、检查更新和立即更新。

## 9. 本地数据与隐私

| 数据 | Windows 位置/机制 | 说明 |
| --- | --- | --- |
| DeepSeek API Key | Windows Credential Manager | 不进入文件或前端状态 |
| UI 偏好 | `%APPDATA%\AI Token Meter\settings.json` | 无秘密，版本化、原子写入 |
| 快照与历史缓存 | `%LOCALAPPDATA%\AI Token Meter\cache\` | 脱敏、版本化、原子替换 |
| DeepSeek Web 会话 | `%LOCALAPPDATA%\AI Token Meter\WebView2\` | 独立于用户浏览器 |
| 日志 | `%LOCALAPPDATA%\AI Token Meter\logs\` | 轮转、大小限制、默认脱敏 |

Windows 与 macOS 均不增加自建服务器、遥测、广告或行为分析。日志只记录阶段、版本、耗时、错误类别和脱敏路径，不记录账户原始输出、会话正文、API Key、Cookie、手机号或邮箱。

## 10. 更新、安装与发布

### 10.1 用户交互

沿用已确认的手动策略：

- 不在启动时自动检查；
- 用户点击 `Check for Updates` 才联网；
- 发现新版后才启用 `Update Now`；
- 用户点击后下载、验证、启动安装并按 Windows 要求退出旧进程；
- 签名、下载或安装失败时保留当前版本并恢复可重试状态。

### 10.2 Windows 更新

- 使用 Tauri Updater；
- `latest.json` 提供 `windows-x86_64` URL、版本、发布日期、说明和必需的 updater signature；
- 更新私钥只位于维护者安全存储或 GitHub Actions Secret，仓库和 Release 只包含公钥/签名；
- 更新签名验证是硬门禁，SHA-256 只用于人工核验，不能替代发布者签名；
- Windows 安装器退出应用前保存非敏感状态并关闭子进程，不复制 CLI 凭据。

Tauri Updater 要求更新 URL、版本和签名，Windows 安装前会退出应用。设计依据：<https://v2.tauri.app/plugin/updater/>。

### 10.3 Windows 安装包

首版提供 NSIS `setup.exe`：

- 默认按当前用户安装到 `%LOCALAPPDATA%`，无需管理员权限；
- 安装开始菜单入口、卸载项和可选开机启动；
- 不捆绑 Claude Code、OpenAI Codex、Node、WSL 或浏览器；
- 缺少 CLI 时提供官方安装说明和重新检测按钮；
- 构建必须在 GitHub `windows-latest` 或真实 Windows 构建机完成，不把 macOS 交叉编译作为正式发布路径。

Tauri 支持 NSIS `setup.exe` 和 MSI；当前用户 NSIS 安装默认不要求管理员权限。设计依据：<https://v2.tauri.app/distribute/windows-installer/>。

### 10.4 Authenticode 边界

Windows updater signature 与 Authenticode 是两种不同信任：

- updater signature 用于 AI Token Meter 判断下载包是否由项目发布者授权，首个 Windows 版本必须具备；
- Authenticode 用于 Windows/SmartScreen 识别安装器发布者。没有证书仍可运行，但浏览器下载后可能提示未知发布者；
- 首个 `preview` 可以诚实发布未 Authenticode 签名的安装器，并在 README/Release Notes 明确提示；
- 正式稳定 Windows Release 应在取得有效证书或可信云签名能力后启用 Authenticode，证书和密码只能保存在 GitHub Secret/外部签名服务中。

Tauri 明确说明 Windows 代码签名不是执行应用的技术必需条件，但缺少签名可能触发 SmartScreen。设计依据：<https://v2.tauri.app/distribute/sign/windows/>。

### 10.5 统一 Release

根 `VERSION` 是唯一版本来源，构建时同步生成 macOS Bundle 版本与 Windows Tauri 版本。正式 Git tag 使用 `vX.Y.Z`，预览使用 `vX.Y.Z-preview.N`。

同一个 GitHub Release 至少包含：

```text
AI-Token-Meter-X.Y.Z-macOS-arm64.zip
AI-Token-Meter-X.Y.Z-macOS-arm64.zip.sha256
AI-Token-Meter-X.Y.Z-windows-x64-setup.exe
AI-Token-Meter-X.Y.Z-windows-x64-setup.exe.sha256
AI-Token-Meter-X.Y.Z-windows-x64-setup.nsis.zip.sig
appcast.xml
latest.json
```

建议首个公开 Windows 候选版本为 `v0.3.0-preview.1`。Preview Release 同样包含两个平台资产，但不进入 macOS 稳定 appcast；双平台真实验收完成后发布 `v0.3.0`。

## 11. CI 与发布门禁

### 11.1 Pull Request CI

每个 PR 至少包含：

1. macOS：Swift 测试、文档门禁、资源与 Release 构建检查；
2. Windows：Rust format/clippy/test、TypeScript lint/typecheck/test、Tauri release build；
3. Shared contract：Schema、fixture、Provider 名称、状态与百分比口径对等；
4. Security：源码、历史、fixture、日志样本和构建产物秘密扫描；
5. Version：Swift、Tauri、更新 feed 和 Release 文件名必须来自同一 `VERSION`；
6. Documentation：功能对等矩阵、CHANGELOG、用户指南和设计/开发记录链接完整。

### 11.2 Release 门禁

- macOS 与 Windows CI 均通过；
- 两个平台资产均完成构建、签名/校验和验证；
- 不允许先发布一个正式平台、稍后补另一个正式平台；
- Preview 可保留明确记录的已知问题，但不能缺失已承诺的首版 Provider；
- 从匿名网络重新下载所有资产，复核 SHA-256、更新签名、版本和安装；
- Release 后分别验证 Sparkle 和 Tauri 的真实升级闭环。

紧急平台专属修复仍使用同一个补丁版本，并重新运行另一平台回归；未改动平台也要重新生成或明确复用由同一 tag 构建且验证的资产。

## 12. 测试与验收

### 12.1 自动化测试

- 共享 Schema 与 macOS/Rust 编解码兼容；
- 三 Provider 成功、未安装、未登录、超时、格式变化与缓存降级；
- Windows 原生 PATH、用户 PATH、WinGet/npm/Node 管理器、自定义路径和 WSL2 发现；
- CLI 路径含空格、非 ASCII 用户名、Node shebang/launcher 和版本变化；
- ConPTY 正常退出、超时、取消、尾部输出、并发与进程树回收；
- Credential Manager 保存、替换、删除与错误不泄密；
- DeepSeek WebView2 域名限制、分片合并、缓存与登录失效；
- DPI、多显示器、左右侧、垂直位置、全屏隐藏和详情置顶的纯策略；
- 点击外部、自动隐藏、网页输入暂停和切换 Provider；
- Tauri capability 不暴露通用 shell/文件系统权限；
- Windows 更新高版本、同版、低版、篡改签名、离线与安装取消；
- 双平台版本、命名、颜色、状态、文档和 Release 资产门禁。

### 12.2 Windows 11 真机验收

至少在一台 Windows 11 x64 真机或可信虚拟机执行：

1. 普通用户安装、首次启动、托盘与卸载；
2. 原生 Claude Code 和原生 OpenAI Codex 登录、额度、账户与本机活动；
3. WSL2 Claude Code / Codex 的发现、选择、登录失效和恢复；
4. DeepSeek Key 新增、更换、移除、余额与独立网页登录；
5. 左右贴边、拖动、100%/150%/200% DPI、多显示器、任务栏位置变化；
6. 普通窗口覆盖、全屏 Edge/游戏时隐藏、返回桌面恢复；
7. 详情置前、外部点击、自动隐藏、Settings 与开机启动；
8. `preview.1 → preview.2` 或等价测试版本的真实检查、下载、签名验证、退出、替换和重启；
9. 重启 Windows 后偏好、缓存和 Credential Manager 数据保持正确；
10. 日志、缓存、崩溃文件和 Release 资产不含秘密或账户正文。

无法在 macOS 主机上观察到的 Windows 窗口层级、托盘、DPI、Credential Manager、ConPTY 和安装更新行为，不得仅凭编译通过宣称完成。

## 13. 分阶段实施与 Git 检查点

1. **跨平台基础：** `VERSION`、共享合同、fixture、功能矩阵、Windows 工程骨架与 CI；
2. **Windows 核心：** 领域模型、缓存、安全、刷新协调器和 DeepSeek 余额；
3. **CLI Provider：** Claude Code/OpenAI Codex 原生发现、ConPTY、账户和额度；
4. **WSL2：** 发行版发现、命令执行、来源选择与本机活动；
5. **Windows UI：** 托盘、浮动条、详情、Settings、字体、无障碍和视觉对等；
6. **DeepSeek 历史：** 隔离 WebView2、30 日历史与错误降级；
7. **分发更新：** NSIS、updater signature、`latest.json`、双平台 Release 工作流；
8. **真实验收：** Windows 11 完整矩阵、Preview Release、问题修复和 `v0.3.0` 稳定发布。

每阶段执行测试先行、开发日志、需求台账更新、代码审查、验证和独立 Git 提交。阶段性提交可以进入 `main`，但未通过正式 Release 门禁时不得创建稳定 tag。

## 14. 错误处理与安全回退

- CLI 未找到：显示安装来源诊断、官方链接、自定义路径和重新检测，不伪装成未登录；
- CLI 找到但运行时缺失：显示实际 launcher/runtime 错误并保留候选路径；
- WSL 未安装或发行版停止：原生 Provider 不受影响；
- Provider 登录失效：显示账户状态和重新登录，缓存明确标记陈旧；
- 单项超时：终止该进程树，其他 Provider 继续刷新；
- Credential Manager 拒绝访问：不降级为明文文件；
- DeepSeek 网页变化：历史图不可用，余额仍正常；
- 窗口层级不稳定：优先隐藏浮动条，不升级为常驻置顶；
- 更新签名失败：删除临时包并保持当前版本；
- Windows 构建或真机验收失败：需求保持进行中/受环境限制，不冒充双平台完成。

## 15. 文档同步

实施时同步维护：

- 根 README 的双平台下载、截图、依赖和安装；
- Windows Getting Started、Provider、Settings、排障与卸载指南；
- 架构概览、代码库结构、架构决策和安全隐私；
- 测试、维护、Release 流程与贡献指南；
- CHANGELOG、项目状态、需求台账、开发日志和提交历史；
- GitHub Issue/PR 模板中的平台与对等检查项；
- 每个 Release 的 macOS/Windows 已知限制与校验说明。

## 16. 非目标

- Windows 10、Windows ARM64、Linux；
- Windows Widget 或 Microsoft Store 发布；
- 重写现有 macOS UI、Widget 或更新系统；
- 捆绑、修改或代替安装 Claude Code/OpenAI Codex；
- 复制 CLI OAuth Token、读取对话正文或共享浏览器 Cookie；
- 后台自动更新、静默安装或任意更新源；
- 用未公开 Windows Shell 注入技巧作为唯一桌面层实现；
- 在没有真实 Windows 证据时宣称 Windows 版完成。

## 17. 自检结论

- **范围完整：** 首版托盘、贴边浮动条、三 Provider、详情、Settings、更新和同 Release 均有明确设计；
- **现有资产保护：** macOS 原生实现不重写，Widget 延期状态保持独立；
- **长期同步：** 共享合同、功能矩阵、单一版本和 CI/Release 门禁防止两端漂移；
- **平台现实：** ConPTY、Credential Manager、WebView2、NSIS、DPI、全屏和 WSL2 均有明确适配边界；
- **安全：** 前端不接触密钥或任意 shell，更新包强制签名，真实响应和凭据不进入日志；
- **发布诚实：** updater signature 与 Authenticode 分开管理，未签名 Preview 的 SmartScreen 限制会明确披露；
- **失败安全：** Provider、网页、窗口和更新均有不破坏其他功能的降级路径；
- **可验证：** 自动测试、Windows 真机矩阵、匿名资产复验和双平台真实升级闭环均有停止条件。
