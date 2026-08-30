# 故障排查

建议先在 AI Meter 菜单栏手动刷新一次，再按下面的服务分类检查。

## Claude

### 显示 Unavailable 或 Request timed out

1. 在终端运行 `claude auth status`，确认登录状态正常。
2. 直接运行 `claude`，确认 CLI 本身可以启动。
3. 回到 AI Meter 手动刷新。
4. 如果仍提示工作区设置，点击 **Open one-time setup**，在打开的终端中批准私有工作区。

公司代理、防火墙、CLI 升级或 Claude 服务端延迟都可能导致超时。AI Meter 会保留最近成功缓存，但不会把缓存伪装成实时数据。

### 官方客户端显示 0%，AI Meter 数字不同

- 对比两边的“更新时间”和额度窗口名称；
- AI Meter 只解析 `/usage` 中的实际用量，不采用促销说明中的百分比；
- 手动刷新后再比较；
- 如果仍不一致，记录 Claude CLI 版本与界面文字变化，但不要在问题报告中粘贴凭证或完整账户响应。

## Codex

### 显示需要登录或不可用

1. 在终端启动 Codex CLI 并完成官方登录。
2. 升级过旧的 Codex CLI。
3. 回到 AI Meter 手动刷新。

AI Meter 依赖 `app-server` 的结构化接口。接口不可用或格式变化时会显示明确状态，不会回退到脆弱的终端截图识别。

### 百分比和官方客户端不同

- AI Meter 展示顶层通用额度，不用模型专属额度覆盖它；
- 官方界面可能显示“剩余”，AI Meter 圆环显示“已使用”，两者相加应接近 100%；
- 检查刷新时间和重置时间；
- 若官方显示剩余 95%，AI Meter 应显示已使用约 5%。

## DeepSeek

### 没有余额

- 打开设置，确认显示 **Stored securely in Keychain**；
- 401：删除并重新保存 API Key；
- 429：等待后重试；
- 连接失败：检查网络、代理和 DeepSeek 服务状态。

### 圆环比例看起来不对

圆环表示相对 **Balance baseline** 的已消耗比例，不是最近 30 天成本占比。检查设置中的基准值，并按以下公式核对：

```text
(余额基准 - 当前余额) / 余额基准
```

### 30 天图表要求登录或没有更新

1. 点击 DeepSeek 圆环打开详情。
2. AI Meter 会短暂成为当前活动 App；点击详情中的官方页面输入框并完成登录。
3. 保持详情开启直至官方用量页加载完成。
4. 如果官网可用但图表仍空白，尝试退出并重开 AI Meter。

官网页面结构变化时，AI Meter 可能只能显示最后缓存和官方入口。此时余额 API 仍可独立工作。

### 官方登录页能够显示，但输入框不能输入

- 完全退出并重新打开最新版 AI Meter，旧进程不会自动获得新的窗口焦点能力；
- 点击 DeepSeek 圆环后，先确认 AI Meter 成为当前活动 App，再点击手机号或验证码输入框；
- Claude、Codex 详情按设计不会激活 AI Meter，它们是只读面板；
- 如果出现 CAPTCHA、法律协议或额外安全确认，请在官方页面手动处理；
- AI Meter 不保存手机号、验证码或表单内容。不要把验证码、Cookie 或完整登录截图附在问题报告中。

## 界面

### 悬浮条不见了

- 打开设置并开启 **Show right-side floating meter**；
- 多显示器切换后等待窗口重新定位；
- 仍未出现时完全退出并重开 AI Meter。

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
