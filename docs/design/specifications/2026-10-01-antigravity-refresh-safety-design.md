# Antigravity 静默刷新与异常暂停

## 需求

`REQ-20261001-001`：macOS 上 AI Token Meter 的 Antigravity CLI 额度刷新曾导致反复打开 Google 授权页；M4 Max 一夜出现数百个页面为用户报告，未独立测量。macOS 与 Windows 保持相同产品版本；修复不操作用户 Google 账号、凭据、Keychain 或 shell 配置。

## 已证实行为

- 应用通过 `agy -p /usage` headless 查询额度，但环境额外设置的 `NO_BROWSER=true` 并无官方文档佐证。
- `agy` 官方文档说明：headless 查询使用缓存凭据；无认证且无终端时返回 `authentication required`。本地交互启动则可能因缺少有效会话而打开浏览器。
- `RefreshCoordinator` 已把退避状态持久化，但手动刷新会绕过 `.cli` 退避；认证退避只有 15 分钟，并且 Gemini 的 Retry/检查入口会先清除它。
- macOS wake 与 app URL 入口调用默认手动刷新；命令超时/取消只终止直接子进程。

## 决策

1. Gemini 用量采集继续仅用官方 `-p /usage` headless 命令；不设置未经证实的 browser 环境变量，不读取 Google 认证文件来推测登录状态。
2. Gemini 任意采集失败后写入持久终态暂停。暂停不随时间、启动、唤醒、手动刷新或 Retry 自动解除；只有用户显式点击 Antigravity Sign in、完成交互终端流程后，才解除并执行一次受限检查。再次失败立即重新暂停。
3. Gemini 首次使用仍在自动刷新时可采集。非 Gemini 服务保留现有刷新与退避行为。
4. headless 命令进程放入独立进程组；超时、取消、输出超限或 runner 收尾时终止该组，避免仅杀父进程后遗留本次启动的子进程。不得通过进程名杀死其他 agy 或浏览器实例。
5. 自动刷新、wake、deep-link、detail Retry 及 Services 中的普通 Retry 均不得解除暂停。deep-link 仅接受本应用登录脚本携带的 completion 路由。
6. Windows 共享版本号和 release 流程，本次 Windows 运行行为不变。

## 验收

- 错误、超时、取消后的 Gemini 暂停能跨 coordinator 重建保持；所有隐式和手动刷新入口均不会重新启动 agy。
- 只有显式登录流程的完成回调可以清除暂停；该流程至多触发一次 headless 检查。
- timeout/cancel 清理同一进程组内的 fake 子进程，同时不影响组外控制进程。
- 原有成功额度解析、其他供应商收集与 Windows 产品行为通过回归。
- fake CLI 与测试凭据全部本地构造；不运行真实 agy，不接触系统 Keychain/浏览器。
- 本地/CI 通过不能替代 M4 Max 现场验证。
