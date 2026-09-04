# 故障排查

建议先在 AI Token Meter 菜单栏手动刷新一次，再按下面的服务分类检查。

## Claude Code

### 显示 Unavailable 或 Request timed out

1. 打开 Settings > Services，先点击 Claude Code 的 **Check Status**。
2. 显示 `Sign-in required` 时点击 **Sign in**；需要换账号时点击 **Sign in again**，在官方 Terminal 流程中完成登录。
3. 显示 `CLI not installed` 时先安装 Claude Code；也可在终端运行 `claude auth status --json` 检查 CLI。
4. 回到 AI Token Meter 手动刷新。
5. 如果仍提示工作区设置，点击 **Authorize Usage Workspace**，在打开的终端中批准私有工作区。

公司代理、防火墙、CLI 升级或 Claude Code 服务端延迟都可能导致超时。AI Token Meter 会保留最近成功缓存，但不会把缓存伪装成实时数据。

### 官方客户端显示 0%，AI Token Meter 数字不同

- 对比两边的“更新时间”和额度窗口名称；
- AI Token Meter 只解析 `/usage` 中的实际用量，不采用促销说明中的百分比；
- 手动刷新后再比较；
- 如果仍不一致，记录 Claude Code CLI 版本与界面文字变化，但不要在问题报告中粘贴凭证或完整账户响应。

## OpenAI Codex

### 显示需要登录或不可用

1. 打开 Settings > Services，点击 OpenAI Codex 的 **Check Status**。
2. 显示 `Sign-in required` 时点击 **Sign in**；需要换账号时点击 **Sign in again**，在官方 Terminal 流程中完成登录。
3. 显示 `CLI not installed` 时先完全退出并升级 AI Token Meter 至 `0.1.2` 或更高版本；该版本会自动发现 nvm、常见 Node 管理器和 ChatGPT/Codex App 内置 CLI。确实未安装时点击 **Open Install Guide**。
4. 回到 AI Token Meter 手动刷新。

AI Token Meter 依赖 `app-server` 的结构化接口。接口不可用或格式变化时会显示明确状态，不会回退到脆弱的终端截图识别。

如果终端能运行 `codex`、Finder 启动的旧版应用却显示未安装，可用 `command -v codex` 核对。路径位于 `~/.nvm/versions/node/.../bin/codex` 表示是旧版定位器未覆盖 nvm，不需要重新安装 CLI；安装 `0.1.2` 后重新打开应用即可。新版还会把该目录置于子进程 PATH 首位，避免 `env: node: No such file or directory`。

Windows 中 PowerShell 能运行 CLI、应用却显示未安装时：

1. 在 Services 查看来源是否为 `Native Windows` 或 `WSL`，点击 **Check Status**；
2. 重新打开应用，让它重新读取注册表 PATH 和 WSL 发行版；
3. 原生 npm/Node 脚本必须能找到配套 `node.exe`，应用不会执行任意 PowerShell profile；
4. WSL CLI 必须能在所选发行版非交互启动。不要把 Linux 路径手工填入设置，应用只使用受控发现结果。

### 百分比和官方客户端不同

- AI Token Meter 展示顶层通用额度，不用模型专属额度覆盖它；
- 官方界面可能显示“剩余”，AI Token Meter 圆环显示“已使用”，两者相加应接近 100%；
- 检查刷新时间和重置时间；
- 若官方显示剩余 95%，AI Token Meter 应显示已使用约 5%。

### 本机活动显示 Unavailable

- 这不影响上方官方额度和重置额度；
- 确认当前 macOS 用户曾使用 OpenAI Codex CLI 或 OpenAI Codex App，并且 `~/.codex/state_5.sqlite` 可读；
- 本机 Token 是线程级聚合估算，不应与官网跨设备账户统计直接比较。

## DeepSeek

### 没有余额

- 打开 Settings > Services，确认显示遮罩状态 `API Key ••••XXXX`；
- 如果 Keychain 中确实有旧 Key，但临时构建显示 `Account status unavailable`，通常是 ad-hoc 签名的 CDHash 在重编译后变化，macOS 不再允许新二进制静默读取旧项。不要删除旧 Key；改用稳定代码签名，或在确认新构建可信后从 Services 重新录入一次；
- 401：输入新 Key 并点击 **Replace API Key**；候选值验证失败不会覆盖原 Key；
- 429：等待后重试；
- 连接失败：检查网络、代理和 DeepSeek 服务状态。

### 圆环比例看起来不对

圆环表示相对 **Balance baseline** 的已消耗比例，不是最近 30 天成本占比。检查设置中的基准值，并按以下公式核对：

```text
(余额基准 - 当前余额) / 余额基准
```

### 30 天图表要求登录或没有更新

1. 点击 DeepSeek 圆环打开详情。
2. Windows 点击 **Sync official history**；查看详情本身不会再自动创建官网窗口。按钮应先显示 Opening，官方页就绪后显示同步进行中并暂停详情自动隐藏。macOS 按当前版本的 WebKit 入口操作。
3. 在独立官方窗口完成登录，保持窗口开启直至用量与费用两组数据到齐；完成后 Windows 会关闭官网窗口、恢复详情并显示原生图表。
4. 若不想继续，可直接用标题栏关闭官网窗口；详情应恢复并允许再次点击同步。若显示固定失败提示，先检查网络后点 **Try again**。
5. 如果官网可用但图表仍空白，完全退出并重开最新版 AI Token Meter，再重新显式同步。

官网内部接口属于未公开实现，发生变化时 AI Token Meter 可能只能显示最后缓存和官方入口。此时余额 API 仍可独立工作；完全退出并重开最新版后再刷新可排除旧进程影响。

### 官方登录页能够显示，但输入框不能输入

- 完全退出并重新打开最新版 AI Token Meter，旧进程不会自动获得新的窗口焦点能力；
- Windows 在 DeepSeek 详情中点击 **Sync official history**，等待官方窗口出现并获得前台焦点，再点击手机号或验证码输入框；重复点击只应聚焦现有窗口，不应出现第二个窗口；
- Claude Code、OpenAI Codex 详情按设计不会激活 AI Token Meter，它们是只读面板；
- 如果出现 CAPTCHA、法律协议或额外安全确认，请在官方页面手动处理；
- AI Token Meter 不保存手机号、验证码或表单内容。不要把验证码、Cookie 或完整登录截图附在问题报告中。

### Windows Display font 下拉框文字不可见

- 完全退出并安装包含紧凑密度修复的 Preview；旧进程不会载入新的原生控件配色；
- 打开 Settings → Appearance，展开 **Display font**。选项应在未悬停时就是白底深色文字，鼠标悬停和键盘焦点也保持可辨识；
- 如果仍是白底白字，记录 Windows 版本、系统浅色/深色模式和缩放比例，但不要附带 Services 账号区域；
- 浏览器计算样式门禁不能替代 Windows 原生弹出列表。该项在需求台账中保持待用户确认，直到交互式 Windows 11 真机通过。

## 界面

### 检查更新失败或 Update Now 不可用

- `Update Now` 只有本轮手动检查发现高于当前版本的稳定版后才启用；已经是最新版时保持禁用。
- 出现离线提示时检查网络或代理，然后再次点击 **Check for Updates**。应用不会在后台自动重试。
- `0.1.2` 没有更新按钮；先从 GitHub Release 手动安装当前版本，之后版本才能应用内更新。
- 安装阶段确保 App 位于当前用户可替换的位置，通常为 `/Applications/AI Token Meter.app`，并完全处理 macOS 显示的权限提示。
- 如果 Sparkle 报告签名或归档验证失败，不要绕过。继续使用当前版本，并从项目官方 GitHub Release 重新下载或报告问题。
- Windows 若提示 updater signature、target 或 manifest 无效，同样不要绕过；当前版本会保留不变。确认 `latest.json`、NSIS `setup.exe` 与同名 `.exe.sig` 来自同一个 GitHub Release，而不是只供 CI 调试的安装器。
- Windows SmartScreen 的“未知发布者”来自当前尚无 Authenticode 证书，并不代表 minisign 更新验证失败。只从官方 Release 下载并核对 `.sha256`；来源不符时取消安装。
- 需要回退时，完全退出应用，从官方 Release 下载上一版本并手动替换。偏好和非敏感缓存通常保留；操作前仍建议备份 `Application Support/AI Meter`。

### Widget Gallery 找不到 AI Token Meter

- 确认安装包存在 `Contents/PlugIns/AITokenMeterWidget.appex`；默认构建输出 `Widget skipped` 表示当前包不含 Widget；
- 在 Xcode > Settings > Accounts 登录 Apple Account 并创建 Apple Development 证书，再以 `AI_METER_INCLUDE_WIDGET=1 bash scripts/build-app.sh` 重建；
- 把完整 `AI Token Meter.app` 放入 `/Applications`，启动一次后重新打开桌面 Widget Gallery；
- ad-hoc 签名无法提供可用的 App Group Widget，构建脚本不会伪装支持。

### Widget 显示 Unavailable 或数据没有立即更新

- 打开主应用并手动刷新一次；Widget 不直接调用 CLI、API 或 Keychain；
- 核对主应用与扩展是否来自同一次构建，双方 App Group 不一致时无法共享数据；
- WidgetKit 使用系统刷新预算，主应用请求更新不等于逐秒刷新；
- 快照超时会标记为陈旧而不是继续伪装实时；共享文件损坏或版本未知会安全回到三项 Unavailable；
- 可运行 `scripts/verify-widget-bundle.sh "dist/AI Token Meter.app"` 检查嵌套签名、App Group 与扩展沙箱。

### 悬浮条不见了

- 如果当前应用处于全屏，或普通应用窗口覆盖了屏幕边缘，这是桌面层浮岛的预期行为；退出全屏、显示桌面或切回普通桌面 Space 后再确认。
- 打开设置并开启 **Show floating meter**；
- 多显示器切换后等待窗口重新定位；目标屏在线时浮岛应留在原物理屏幕，目标屏断开时应临时出现在当前主屏并保留侧边和相对高度；
- 仍未出现时完全退出并重开 AI Token Meter。

AI Token Meter 不提供“始终置顶”开关。浮岛按设计不会覆盖普通应用或全屏应用；只有用户点击 Provider 后的临时详情会显示在普通应用窗口上方，并在关闭、自动隐藏或切换 Space 后退出。返回桌面后浮岛仍会使用此前保存的侧边与垂直位置。Mission Control 中不悬在 Space 缩略图上方也是设计目标，但当前候选尚未完成该场景的直接实机验收；如果发现异常悬浮，请记录 macOS 版本和复现步骤。

Windows 会在浮岛所在显示器出现全屏前台应用时主动隐藏，离开全屏后恢复；若普通窗口模式也一直不见，请从系统托盘启用 **Show Meter**，再检查已保存显示器是否已断开。两平台都不会用临时主屏回退覆盖保存目标；重新接入目标屏后会自动恢复。问题报告请包含系统版本、缩放比例、显示器布局和“目标屏是否曾断开”，但不要包含账户截图或显示器序列信息。

Windows 浮岛若仍出现白色矩形外框、锯齿肩部或拖动后跳边，先在 Settings → About 确认版本至少为 `0.3.0-preview.1`；`preview.0` 使用旧的 GDI/多边形双重裁剪。升级后完全退出并重新启动，使新的窗口阴影、DWM 边框和拖动监视策略生效。若只在特定缩放比例复现，请同时记录 Windows 缩放比例和浮岛所在显示器，不要用截图裁掉问题边缘。

若启动 Windows 版时浮动条正常出现，但系统同时打开标题为 “AI Token Meter” 的空白 Windows Terminal，请升级到 `0.3.0-preview.2` 或更新版本；`preview.1` 的主程序仍使用 Console subsystem。新版本会在发布流程中解析真实 PE Header 并拒绝任何 Console subsystem 构建。

### 详情一直不消失

- 点击屏幕空白处；
- 确认鼠标没有停留在详情面板上；
- DeepSeek 登录交互期间自动隐藏会暂停；
- 在设置中重新选择自动隐藏时间。

## 日志与诊断

- macOS 崩溃报告：`~/Library/Logs/DiagnosticReports/AIMeterApp-*.ips`
- 实时系统日志：打开“控制台”App，以进程名 `AIMeterApp` 筛选。
- Windows 非敏感诊断位于当前用户 LocalAppData 下的 AI Token Meter 目录，也可结合“事件查看器”检查应用崩溃；提交前仍需人工移除姓名、邮箱、手机号与账户标识。
- 开发与验收记录：[开发日志索引](../development/README.md)

报告问题前请删除截图或日志中的姓名、邮箱、API Key、Bearer Token、Cookie 和账户标识。安全问题请按 [SECURITY.md](../../SECURITY.md) 私下报告。
