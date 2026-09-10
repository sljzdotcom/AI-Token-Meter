# Windows Codex app-server 测试夹具稳定性

关联 `REQ-20260910-002`。本项只修复自动化测试基础设施，不改变 OpenAI Codex 采集、账户展示或发布版本。

## 背景与失败证据

Antigravity 迁移合入 `main` 后，Windows workflow `34422624107` 已通过前端、真实 Edge、production build、格式和严格 Clippy，但完整 Rust runtime 在 `performs_the_bounded_app_server_handshake_without_a_shell` 失败。测试专用 Node.js 夹具未能在产品规定的10秒内完成冷启动，调用于10.071秒按预期返回 `TimedOut`。同一合并提交的 PR Windows workflow `34421774210` 曾全绿，失败发生在此前一个 PowerShell 安装测试运行119.26秒之后，符合高负载 runner 的冷启动调度波动。

这里不能延长产品期限或弱化断言。测试仍必须证明进程不经过Shell启动，并完整交换 `initialize`、`account/read`、`account/rateLimits/read` 三阶段 JSON-RPC。

## 实现与关键决定

删除外部 `codex-app-server-fixture.js`，改由 Cargo 构建轻量原生 Rust fixture binary。额度和账户两组集成测试通过同一个辅助入口启动该夹具；夹具逐行读取JSON请求、写回与旧脚本等价的合成响应，并在每次响应后立即刷新stdout。

第一次失败先行尝试复用当前 libtest 可执行文件中的忽略测试作为夹具，该方案仍在10.006秒超时。libtest进程的测试调度和stdin生命周期不适合作为长连接协议服务，因此改为职责单一的独立fixture binary。该目标只供测试引用；Tauri应用仍显式使用 `ai-token-meter-windows` 主程序目标，生产代码、10秒截止时间和进程清理路径均未修改。

## 自动化验证

- `codex_collector` 与 `service_accounts` 共8项定向测试通过；
- Windows宿主完整241项 Rust 测试通过，原失败握手约0.29秒完成；
- Cargo格式检查和全部目标严格 Clippy 通过；
- PR #23修复候选`fad1039`的macOS workflow `34423801532`用时2分55秒全绿；Windows workflow `34423801483`用时10分21秒，通过完整runtime、真实Edge、严格Clippy、NSIS、GUI subsystem与安装器上传；
- 最终证据候选`da18207`的macOS workflow `34433244350`与Windows workflow `34433244356`全绿；PR #23合并为`a5f7509`；
- 合并后main macOS workflow `34433979440`用时2分56秒全绿；Windows workflow `34433979383`用时11分20秒，通过完整runtime、真实ConPTY、真实Edge、严格Clippy、NSIS、GUI subsystem与安装器上传。

测试响应仅含虚构邮箱、百分比和时间戳，不读取真实凭证、账户或额度。版本保持0.6.3，本项不创建标签、Release或更新源。

最终证据候选的Windows runtime随后暴露同类的外部Node冷启动问题，但发生在独立ConPTY综合测试；该问题单独登记为`REQ-20260910-003`。通用原生夹具增加固定终端输入模式后，两类测试可共用同一Cargo产物，同时各自保留原协议和期限。

## Git与后续

实现提交为`fad1039`，通用夹具补丁为`3fa035e`，证据提交为`da18207`。[PR #23](https://github.com/sljzdotcom/AI-Token-Meter/pull/23)已合并为`a5f7509`，最终候选与main双平台workflow全部通过。物理Windows桌面和真实账号不属于本项；它们继续按现有现场验收边界管理。
