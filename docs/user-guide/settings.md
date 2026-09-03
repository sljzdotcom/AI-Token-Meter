# 设置参考

使用 macOS 菜单栏或 Windows 系统托盘的齿轮打开设置；macOS 也可按 `⌘,`。设置窗口固定分为 **Appearance、Monitoring、Services、About** 四个顶部 Tab；Settings 自身始终使用平台系统字体，不受显示字体偏好影响。

## Appearance

### Desktop Widget

Widget 尺寸与摆放由 macOS 桌面“编辑小组件”管理，因此 Settings 不重复提供尺寸或刷新频率选项。Widget 始终使用系统字体和深海背景，不继承浮动条的 Antonio/DIN 选择；主应用刷新或 DeepSeek 余额基准变化后会发布脱敏快照并请求系统更新时间线。

Windows Preview 不包含桌面 Widget，因此 Windows Settings 不显示 Widget 配置，也不会伪装存在该能力。

### Show floating meter

- 默认：开启。
- 作用：显示或隐藏贴边浮岛。
- 关闭后：菜单栏入口仍然可用。

### Screen edge

- **Automatic（默认）**：可从三个 Logo 以外的任意玻璃空白处上下移动，也可横向拖到另一侧；松手后吸附最近边缘。
- **Left**：固定在屏幕左侧，仍可上下拖动。
- **Right**：固定在屏幕右侧，仍可上下拖动。
- 设置变更立即生效，不需要重启应用。
- AI Token Meter 会保存当前显示器、最后侧边和相对垂直位置；已保存显示器断开时回到主屏幕、Automatic、右侧和垂直中点，可见区域改变时自动夹紧。

macOS 浮岛使用桌面层，普通应用和全屏应用可覆盖它；Windows 使用无任务栏 Win32 工具窗口，并在同显示器出现全屏前台应用时隐藏。点击 Provider 后的临时详情都会在普通应用窗口上方显示，关闭或自动隐藏后立即撤销临时置前。两平台都会保存显示器、Left/Right 侧边与归一化垂直位置；显示器断开时回退主屏。

三个服务圆环只负责打开详情。移动浮岛可从顶部、圆环之间或底部的玻璃空白处开始；Logo 命中区和透明肩部不会触发拖动，因此不会与服务点击互相冲突。

键盘或 VoiceOver 用户可聚焦浮岛玻璃表面：上/下方向键按 10% 步进移动，左/右方向键会明确把侧边偏好设为 Left/Right；VoiceOver 也提供同名自定义动作并朗读当前侧边与垂直位置。

### Display font

- **System Default（默认）**：使用当前平台系统 UI 字体；macOS 为 San Francisco。
- **Antonio**：使用本机已经安装的 Antonio 字体家族。
- **DIN Condensed**：使用本机已经安装的 DIN Condensed 字体家族。
- **Alimama FangYuanTi VF**：使用本机已安装的阿里妈妈方圆体可变字体。
- **Fira Code**：优先使用 `Fira Code`，兼容 `Fira Code VF` 家族名。
- **Leigo**：使用 Ricardo Medina 的 Leigo Regular，兼容 `Leigo` 与 `Leigo Regular` 家族名。
- **Menlo**：使用本机提供的 Menlo 等宽字体；Windows 未安装时按缺失字体处理。
- **Alimama DaoLiTi**：使用本机已安装的阿里妈妈刀隶体。
- Settings 始终使用平台系统字体和原有字号：切换显示字体或内容字号不会改变 Settings、八个选项名称、说明或按钮的字形和大小。
- 切换会立即应用到菜单点击面板、浮动条及 Provider 详情，不需要退出或重新打开窗口；三个选项只显示名称，不提供对应字体的字形预览。
- `Restore Default Font` 会把选择写回 System Default；已经处于默认字体时按钮禁用。
- 任意自定义字体未安装时，对应选项会显示 `Not installed` 且不能选择。已保存的字体临时不可用时，AI Token Meter 会安全回退到系统字体，但保留偏好；重新安装后可自动恢复。
- AI Token Meter 不下载、安装或分发字体文件。请先通过 macOS 安装并注册相应字体，再重新打开 Settings 或重启应用。

Fira Code、Leigo 和 Menlo 的中文覆盖可能不完整，中英文混排时由 macOS 字体级联补齐中文字形。Widget、Settings、系统菜单和通知始终使用系统字体。

字体选择只影响 AI Token Meter 自己绘制的文字，不改变 Provider Logo、SF Symbols、圆环、品牌颜色、深海背景或 DeepSeek 官方网页内容。

### Detail auto-hide

- 可选：3、5、8、15、30 秒。
- 默认：8 秒。
- 作用：点击某个圆环后，详情面板在无交互时自动收起。
- 例外：鼠标悬停、详情内键盘焦点、VoiceOver 运行和 DeepSeek 登录交互期间暂停倒计时；点击悬浮条和详情以外的区域会立即关闭。

## Monitoring

### Refresh interval

- 固定：5 分钟。
- 启动时会先刷新一次，也可从菜单栏手动刷新。

### Usage alerts at 70% and 90%

- 默认：关闭。
- 开启时系统会请求通知权限；首个 Windows Preview 的通知对等状态以功能矩阵和 Release Notes 为准。
- 只对有明确上限的额度比例生效，并抑制同一周期的重复通知。

### Open AI Token Meter at login

- 默认：关闭。
- macOS 使用登录项服务；Windows 使用当前用户启动项。移动或重装应用后应重新切换一次。
- 移动应用位置后应关闭再开启一次，以刷新路径。

## Services

Services 集中放置外部服务的当前账户、重新登录、配置与一次性操作。打开 Settings 时会并行检查三项服务；`Checking`、`Connected`、`Sign-in required`、`CLI not installed` 与 `Account status unavailable` 相互区分。

### Claude Code 与 OpenAI Codex 账户

- 已连接时显示 CLI 报告的账户标识；Claude Code 优先显示邮箱与认证方式，OpenAI Codex 的 ChatGPT 账户显示邮箱和套餐，API Key 登录只显示 `API Key account`。
- 未连接时显示 **Sign in**，已连接时仍显示 **Sign in again**，方便换账号或修复过期登录。
- 按钮只会打开固定的官方命令 `claude auth login` 或 `codex login`；密码、浏览器授权和 MFA 仍由官方流程处理。
- 打开登录后可随时点击 **Check Status**。macOS 会进行最长 2 分钟的定时回查；Windows Preview 使用固定登录终端并由用户完成后手动检查，界面不会接收密码或 MFA。
- 账户信息读取失败时不会把“无法检查”误写成“已退出登录”，也不会影响已缓存的额度显示。
- OpenAI Codex 显示 `CLI not installed` 时，登录按钮会替换为 **Open Install Guide**，只打开 OpenAI 官方安装说明；安装后点击 **Check Status**。AI Token Meter 会自动发现 nvm 等 Node 管理器目录和 ChatGPT/Codex App 内置二进制。

Windows 在每张 Claude Code/OpenAI Codex 服务卡内提供运行方式：

- **Automatic**：先查找原生 Windows CLI，找不到后才尝试 WSL；
- **Native Windows**：只使用 Windows CLI，可填写可选的自定义 CLI 路径；
- **WSL**：只使用所选发行版；显式发行版不存在时不会静默切到其他发行版；
- 账户检查、Sign in 与后台额度采集始终使用同一份选择，WSL 官方额度不会拼接 Windows profile 的本机活动。

### Claude Code workspace setup

- 仅在 Claude Code 的隔离用量工作区需要首次批准时使用。
- **Authorize Usage Workspace** 会打开终端，由用户本人确认工作区；应用不会自动接受信任或权限提示。它与账号登录是两件独立的事。
- 既有批准继续使用兼容目录 `Application Support/AI Meter/ClaudeUsageWorkspace`。

### Balance baseline

- 默认：¥100。
- 最小有效值：¥1。
- 作用：决定 DeepSeek 圆环的参考起点，不会影响账户、充值或消费。

Windows 的 Monitoring 还允许把定时刷新设置为 30 秒至 24 小时；默认仍为 5 分钟。手动刷新会取消同一 Provider 的旧任务，定时刷新不会与在途任务重叠。

### Usage alerts 与 Launch at login

- **Usage alerts at 70% and 90%**：每项额度在 70% 和 90% 各提醒一次；降到 10% 以下或进入新的重置周期后重新布防。Windows 使用系统通知，macOS 使用通知中心；
- **Open AI Token Meter at login**：只为当前系统用户启用。Windows 写入当前用户启动项，不要求管理员权限；macOS 使用系统登录项服务。

### DeepSeek API Key

- 当前 Key 只显示 `API Key ••••ABCD` 形式的最后四位；输入框始终为空，不回填完整 Key。
- **Save API Key / Replace API Key**：先用候选 Key 调用 DeepSeek 官方余额接口；只有验证成功才更新 macOS Keychain 或 Windows Credential Manager 并刷新。Windows 候选 Key 由原生 Credential UI 直接交给 Rust 后端，不进入 React/WebView 状态或字符串 IPC。
- 401 会提示 Key 无效；网络、超时、响应异常或 Keychain 写入失败都会保留旧 Key，并保留输入内容供修改重试。
- **Remove**：从 Keychain 删除密钥并刷新状态。
- 验证期间按钮和输入框会暂时禁用，并显示 `Verifying…`。

## 本地持久化

| 内容 | 保存位置/机制 | 敏感性 |
| --- | --- | --- |
| DeepSeek API Key | macOS Keychain | 敏感，不进入普通偏好或缓存 |
| 外观（含显示字体）、通知、基准、自动隐藏时间与浮岛位置 | `UserDefaults` | 非敏感 |
| 最近一次统一用量快照 | `Application Support/AI Meter` | 非敏感，写入前清理敏感文本 |
| DeepSeek 标准化每日用量 | `Application Support/AI Meter` | 非敏感聚合数据 |
| DeepSeek 官网登录会话 | App 隔离 WebKit 数据存储 | 敏感会话，由 WebKit 管理，不写入业务缓存 |
| Widget 展示快照 | Apple Development 签名双方专用 App Group | 非敏感最小展示数据；主应用刷新覆盖 |

Windows 的非敏感 JSON 位于 `%APPDATA%\AI Token Meter` 与 `%LOCALAPPDATA%\AI Token Meter`，DeepSeek Key 位于 Windows Credential Manager，官网历史位于独立 WebView2 用户数据目录。Windows 不创建 Widget App Group。

显示名称已经改为 AI Token Meter，但 Bundle Identifier、可执行文件名、Keychain 身份和 `Application Support/AI Meter` 目录暂时保留。这是有意的兼容设计，用于沿用升级前的偏好、密钥访问、缓存与 Claude Code 工作区批准。

## About

- 显示 App Icon、**AI Token Meter**、副标题 **Private AI usage monitor**、版本号和 build 号。
- 显示简短隐私说明；不包含外部账户入口、账户操作或诊断数据上传。
- **Check for Updates**：只有点击时才读取项目的 GitHub 更新清单；macOS 使用 appcast，Windows 使用 `latest.json`。应用启动、定时刷新和后台驻留都不会检查更新。
- 检查结果会显示正在检查、已是最新版、发现版本、离线或安全失败；`Last checked` 只记录本次用户操作的时间。
- **Update Now**：仅在本轮已发现更高版本时启用。macOS 使用 Sparkle EdDSA，Windows 使用 Tauri minisign + NSIS；验证通过后才替换并重新启动，应用不会静默安装。
- `0.1.2` 没有这两个按钮，因此需要从 GitHub Release 手动安装一次当前版本。之后的稳定版本可以从本页更新。
