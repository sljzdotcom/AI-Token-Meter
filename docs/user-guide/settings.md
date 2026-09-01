# 设置参考

使用菜单栏齿轮或 `⌘,` 打开设置。设置窗口固定分为 **Appearance、Monitoring、Services、About** 四个顶部 Tab；Settings 自身始终使用 macOS 系统字体。

## Appearance

### Desktop Widget

Widget 尺寸与摆放由 macOS 桌面“编辑小组件”管理，因此 Settings 不重复提供尺寸或刷新频率选项。Widget 始终使用系统字体和深海背景，不继承浮动条的 Antonio/DIN 选择；主应用刷新或 DeepSeek 余额基准变化后会发布脱敏快照并请求系统更新时间线。

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

浮岛和详情固定使用 macOS 桌面层，这不是可切换偏好。普通应用窗口和全屏应用会覆盖它们；返回 Finder 桌面或普通桌面 Space 后，浮岛会按已保存的显示器、Left/Right 侧边和垂直位置重新出现。切换 Space 会关闭当前详情，但不会改写这些位置偏好。

三个服务圆环只负责打开详情。移动浮岛可从顶部、圆环之间或底部的玻璃空白处开始；Logo 命中区和透明肩部不会触发拖动，因此不会与服务点击互相冲突。

键盘或 VoiceOver 用户可聚焦浮岛玻璃表面：上/下方向键按 10% 步进移动，左/右方向键会明确把侧边偏好设为 Left/Right；VoiceOver 也提供同名自定义动作并朗读当前侧边与垂直位置。

### Display font

- **System Default（默认）**：使用 macOS 系统字体 San Francisco，并保留界面原有的 Rounded 设计请求。
- **Antonio**：使用本机已经安装的 Antonio 字体家族。
- **DIN Condensed**：使用本机已经安装的 DIN Condensed 字体家族。
- Settings 始终使用 macOS 系统字体和原有字号：切换显示字体或内容字号不会改变 Settings、三个选项名称、说明或按钮的字形和大小。
- 切换会立即应用到菜单点击面板、浮动条及 Provider 详情，不需要退出或重新打开窗口；三个选项只显示名称，不提供对应字体的字形预览。
- `Restore Default Font` 会把选择写回 System Default；已经处于默认字体时按钮禁用。
- Antonio 或 DIN Condensed 未安装时，对应选项仍会显示 `Not installed`，但不能选择。已保存的自定义字体临时不可用时，AI Token Meter 会安全回退到系统字体，不会覆盖已保存选择。
- AI Token Meter 不下载、安装或分发字体文件。请先通过 macOS 安装并注册相应字体，再重新打开 Settings 或重启应用。

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
- 开启时 macOS 会请求通知权限。
- 只对有明确上限的额度比例生效，并抑制同一周期的重复通知。

### Open AI Token Meter at login

- 默认：关闭。
- 使用 macOS 登录项服务注册当前应用。
- 移动应用位置后应关闭再开启一次，以刷新路径。

## Services

Services 集中放置外部服务的当前账户、重新登录、配置与一次性操作。打开 Settings 时会并行检查三项服务；`Checking`、`Connected`、`Sign-in required`、`CLI not installed` 与 `Account status unavailable` 相互区分。

### Claude 与 Codex 账户

- 已连接时显示 CLI 报告的账户标识；Claude 优先显示邮箱与认证方式，Codex 的 ChatGPT 账户显示邮箱和套餐，API Key 登录只显示 `API Key account`。
- 未连接时显示 **Sign in**，已连接时仍显示 **Sign in again**，方便换账号或修复过期登录。
- 按钮只会打开固定的官方命令 `claude auth login` 或 `codex login`；密码、浏览器授权和 MFA 仍由官方流程处理。
- 打开登录后，应用每 3 秒检查一次，最长 2 分钟；也可随时点击 **Check Status**。重复点击同一服务会取消旧回查任务。
- 账户信息读取失败时不会把“无法检查”误写成“已退出登录”，也不会影响已缓存的额度显示。

### Claude workspace setup

- 仅在 Claude 的隔离用量工作区需要首次批准时使用。
- **Authorize Usage Workspace** 会打开终端，由用户本人确认工作区；应用不会自动接受信任或权限提示。它与账号登录是两件独立的事。
- 既有批准继续使用兼容目录 `Application Support/AI Meter/ClaudeUsageWorkspace`。

### Balance baseline

- 默认：¥100。
- 最小有效值：¥1。
- 作用：决定 DeepSeek 圆环的参考起点，不会影响账户、充值或消费。

### DeepSeek API Key

- 当前 Key 只显示 `API Key ••••ABCD` 形式的最后四位；输入框始终为空，不回填完整 Key。
- **Save API Key / Replace API Key**：先用候选 Key 调用 DeepSeek 官方余额接口；只有验证成功才更新 macOS Keychain 并刷新。
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

显示名称已经改为 AI Token Meter，但 Bundle Identifier、可执行文件名、Keychain 身份和 `Application Support/AI Meter` 目录暂时保留。这是有意的兼容设计，用于沿用升级前的偏好、密钥访问、缓存与 Claude 工作区批准。

## About

- 显示 App Icon、**AI Token Meter**、副标题 **Private AI usage monitor**、版本号和 build 号。
- 显示简短隐私说明；不包含外部账户入口、账户操作或诊断数据上传。
