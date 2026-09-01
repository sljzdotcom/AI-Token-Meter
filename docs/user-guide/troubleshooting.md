# 故障排查

建议先在 AI Token Meter 菜单栏手动刷新一次，再按下面的服务分类检查。

## Claude

### 显示 Unavailable 或 Request timed out

1. 打开 Settings > Services，先点击 Claude 的 **Check Status**。
2. 显示 `Sign-in required` 时点击 **Sign in**；需要换账号时点击 **Sign in again**，在官方 Terminal 流程中完成登录。
3. 显示 `CLI not installed` 时先安装 Claude Code；也可在终端运行 `claude auth status --json` 检查 CLI。
4. 回到 AI Token Meter 手动刷新。
5. 如果仍提示工作区设置，点击 **Authorize Usage Workspace**，在打开的终端中批准私有工作区。

公司代理、防火墙、CLI 升级或 Claude 服务端延迟都可能导致超时。AI Token Meter 会保留最近成功缓存，但不会把缓存伪装成实时数据。

### 官方客户端显示 0%，AI Token Meter 数字不同

- 对比两边的“更新时间”和额度窗口名称；
- AI Token Meter 只解析 `/usage` 中的实际用量，不采用促销说明中的百分比；
- 手动刷新后再比较；
- 如果仍不一致，记录 Claude CLI 版本与界面文字变化，但不要在问题报告中粘贴凭证或完整账户响应。

## Codex

### 显示需要登录或不可用

1. 打开 Settings > Services，点击 Codex 的 **Check Status**。
2. 显示 `Sign-in required` 时点击 **Sign in**；需要换账号时点击 **Sign in again**，在官方 Terminal 流程中完成登录。
3. 显示 `CLI not installed` 或 app-server 不可用时，安装或升级 Codex CLI。
4. 回到 AI Token Meter 手动刷新。

AI Token Meter 依赖 `app-server` 的结构化接口。接口不可用或格式变化时会显示明确状态，不会回退到脆弱的终端截图识别。

### 百分比和官方客户端不同

- AI Token Meter 展示顶层通用额度，不用模型专属额度覆盖它；
- 官方界面可能显示“剩余”，AI Token Meter 圆环显示“已使用”，两者相加应接近 100%；
- 检查刷新时间和重置时间；
- 若官方显示剩余 95%，AI Token Meter 应显示已使用约 5%。

### 本机活动显示 Unavailable

- 这不影响上方官方额度和重置额度；
- 确认当前 macOS 用户曾使用 Codex CLI 或 Codex App，并且 `~/.codex/state_5.sqlite` 可读；
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
2. AI Token Meter 会短暂成为当前活动 App；点击详情中的官方页面输入框并完成登录。
3. 保持详情开启直至官方用量页加载完成；用量与费用两组数据到齐后会自动切换成原生图表。
4. 如果官网可用但图表仍空白，尝试退出并重开 AI Token Meter。

官网内部接口属于未公开实现，发生变化时 AI Token Meter 可能只能显示最后缓存和官方入口。此时余额 API 仍可独立工作；完全退出并重开最新版后再刷新可排除旧进程影响。

### 官方登录页能够显示，但输入框不能输入

- 完全退出并重新打开最新版 AI Token Meter，旧进程不会自动获得新的窗口焦点能力；
- 点击 DeepSeek 圆环后，先确认 AI Token Meter 成为当前活动 App，再点击手机号或验证码输入框；
- Claude、Codex 详情按设计不会激活 AI Token Meter，它们是只读面板；
- 如果出现 CAPTCHA、法律协议或额外安全确认，请在官方页面手动处理；
- AI Token Meter 不保存手机号、验证码或表单内容。不要把验证码、Cookie 或完整登录截图附在问题报告中。

## 界面

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
- 多显示器切换后等待窗口重新定位；
- 仍未出现时完全退出并重开 AI Token Meter。

AI Token Meter 不提供“始终置顶”开关。浮岛按设计不会覆盖普通应用或全屏应用；只有用户点击 Provider 后的临时详情会显示在普通应用窗口上方，并在关闭、自动隐藏或切换 Space 后退出。返回桌面后浮岛仍会使用此前保存的侧边与垂直位置。Mission Control 中不悬在 Space 缩略图上方也是设计目标，但当前候选尚未完成该场景的直接实机验收；如果发现异常悬浮，请记录 macOS 版本和复现步骤。

### 详情一直不消失

- 点击屏幕空白处；
- 确认鼠标没有停留在详情面板上；
- DeepSeek 登录交互期间自动隐藏会暂停；
- 在设置中重新选择自动隐藏时间。

## 日志与诊断

- macOS 崩溃报告：`~/Library/Logs/DiagnosticReports/AIMeterApp-*.ips`
- 实时系统日志：打开“控制台”App，以进程名 `AIMeterApp` 筛选。
- 开发与验收记录：[开发日志索引](../development/README.md)

报告问题前请删除截图或日志中的姓名、邮箱、API Key、Bearer Token、Cookie 和账户标识。安全问题请按 [SECURITY.md](../../SECURITY.md) 私下报告。
