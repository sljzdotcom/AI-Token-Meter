# DeepSeek 凭据读取优先级反转修复

## 背景与目标

Windows 平台合并后的文档证据提交 `efdebb9` 没有修改产品源码，但其 macOS CI `33744143766` 在高并发完整测试中同时出现两条 DeepSeek 凭据失败。目标是在保留 2 秒安全超时的前提下，消除即时 Keychain/SecretStore 读取被低优先级任务饿死而产生的假超时。

关联需求：`REQ-20260903-008`。

## 影响范围

- DeepSeek 用量采集前的 Keychain 读取；
- Settings 中 DeepSeek 账户状态与换 Key 前的旧凭据读取；
- 对应调度优先级与超时回归测试。

## 失败证据与根因

[失败 macOS CI 33744143766](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33744143766) 的 360 项主测试中只有两项失败：

- `A verified candidate replaces the Key and returns its masked status` 在约 2.014 秒后得到 `keychainFailure`；
- `Collector requires a stored API key before making a request` 在约 2.003 秒后预期 `authenticationRequired`、实际得到 `timedOut`。

两项使用的都是立即返回的内存 SecretStore，且同一源码在前序本机和 main CI 中通过。两个独立读取器都把同步 Keychain 工作固定放入 `.utility` detached task，而倒计时竞争任务没有相同的降级约束；高负载时，真正读取可能被饿死到 2 秒边界。

失败先行测试直接记录 SecretStore 执行时的 `Task.currentPriority`。修复前两条路径均为 raw value `21`，低于用户触发工作的 `.userInitiated` raw value `25`，稳定失败，不依赖墙钟或 CI 负载碰运气。

## 实现与关键决定

1. 两个同步 SecretStore 读取 task 明确使用 `.userInitiated`，与用户点击刷新、打开 Settings 和换 Key 的交互语义一致。
2. 保留既有 2 秒超时、单次 continuation、串行读取门和错误映射；真正卡住的 Keychain 仍会及时降级，不把超时简单放宽。
3. Keychain 写入和失败恢复仍使用 `.utility`。本次 CI 证据只涉及带时限的读取竞争，不扩大改动面。

## 自动化验证

- 修复前：两条新优先级测试 2/2 稳定失败，实测优先级 `21 < 25`；
- 修复后聚焦测试：优先级、缺 Key、成功替换以及两个真实阻塞超时场景共 6/6 通过；
- 完整 macOS 回归：362 项普通测试、12 项独立 PTY 测试，共 374 项、72 个测试组通过；
- 4 份跨平台合同、Release feed 探测、129 份 Markdown 与公开发布安全门禁通过；
- 精确提交 `2c6a194` 的 [macOS main CI 33745691851](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33745691851) 在 1 分 55 秒内全绿；
- 同一提交的 [Windows main CI 33745691724](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33745691724) 在 7 分 42 秒内通过完整 runtime 与 NSIS 安装器构建。

## 本机与界面验收

本修复没有界面变化。DeepSeek 的账户状态、换 Key 与余额展示协议保持不变。

## 安全与隐私

测试只使用合成占位 Key，不读取或记录真实 Keychain 内容。产品代码没有新增日志、网络请求或凭据暴露面。

## Git 节点

- `eea044b`：DeepSeek 两条读取路径提升为 `.userInitiated`，增加失败先行优先级回归并同步测试/需求文档；
- `2c6a194`：补齐 PTY fallback wait 的同类 QoS 修复和最终 374 项基线。

## 已知限制与后续工作

系统 Keychain 自身真实阻塞超过 2 秒时仍会按既有设计显示不可用或采集超时；后台读取结束前，同一读取器不会启动第二次并发读取。
