# Antigravity 单次恢复启动与共享额度诊断

**状态：** 开发完成，独立代码复审与 v0.10.9/build37 双平台发布进行中。

## 问题与范围

用户在 Services 点击单次额度检查后，Terminal 打开 `Open Antigravity Login.command`，但窗口没有 CLI 提示，Services 持续等待。此前生成的诊断来自 0.10.6/build34，暂停原因和阶段都没有记录；它不能说明登录或额度查询的具体失败原因。自动刷新暂停仍按安全设计保留，单次检查成功也不会解除暂停。

本轮修复 Terminal 空白等待路径、回执测试隔离和共享额度缺失时详情页无提示的问题。真实 `agy 1.3.1` 输出与用户设备上的额度恢复仍未验证；不得把合成测试结果解释为现场成功。

## 实现

- `CLIAuthenticationLauncher` 显式注入脚本完成回执 opener。测试策略默认生成 `/usr/bin/false` 回执命令；合成流程只使用私有临时目录、fake CLI、fake opener、精简子进程环境和允许列表路径，不触碰真实 URL handler、CLI、认证目录或现场缓存。
- Antigravity 登录脚本在启动、等待 FIFO handoff、启动 CLI、登录成功/失败/超时、回执无法发送时打印阶段或可理解错误。脚本 handoff 及 CLI 最长等待 300 秒；AppModel 在 310 秒后结束等待并清除该次 token/FIFO。
- 新增 production launcher → FIFO token writer → 生成脚本 → fake CLI/opener → `GeminiLoginReceipt` → AppModel 的合成流程测试；验证单次额度可更新，同时自动刷新仍暂停。
- AppModel 测试可把应用支持目录注入专属临时目录，避免恢复状态测试读取生产 backoff/cache 路径。Shell 测试子进程的 `HOME`、`ZDOTDIR`、`TMPDIR`、工作目录和 `PATH` 全部限定在测试 fixture。
- App 取消时先取消 FIFO token writer，再通过 POSIX 记录锁把状态改为 `cancelled`。helper 在启动 CLI 的同一锁上检查状态并持锁到 `exec`，取消与 CLI 启动有明确先后；helper 确认后或最多等待 60 秒后清理标记。
- Antigravity 详情页在当前快照存在 Gemini 额度、但没有 Claude/GPT 共享额度时显示本地化说明，不再静默隐藏；有效共享窗口仍照常显示。

## 验证与审查

- fail-closed 静态审查通过后，合成 launcher/AppModel 流程的六种结果均通过。终端捕获到 handoff 和 CLI 进度，回执解析与 AppModel 状态断言通过；取消门控另由独立锁进程和 watchdog 退出时序覆盖。
- `bash scripts/test.sh` 的 Swift 主套件通过：632 项（12 Widget、294 Core、326 App），另有3项刷新调度与18项 PTY 回归；共享合同、Windows release 归一化、更新 feed、363 份 Markdown 与公开发布隐私门禁通过。随后 `scripts/check-docs.sh` 独立通过。
- Windows：135 项 frontend tests、25 项 density 生命周期测试、macOS Chrome/Edge 实际样式验证、TypeScript/Vite production build、46 项 Rust 单元测试及集成测试、`cargo fmt --check` 和严格 Clippy 通过。
- Windows frontend、密度 lifecycle、Chrome/Edge 样式验证、production build、Rust、格式与严格 Clippy 检查在候选前序门禁中通过；本次 Swift 登录脚本门控未改变 Windows 运行时代码。npm 曾报告开发依赖 `source-map-js` 1.2.1 高危告警，已在兼容范围升级至 1.2.2，`npm audit` 结果为零漏洞。
- 无真实 `agy`、Google 登录、系统 URL 分发、用户安装应用、生产认证缓存或真实账号测试。`GeminiUsageParser` 的输出合同仍以仓库 1.1.28 fixture 为依据；本记录不确认 1.3.1 格式兼容。
- 剩余低概率边界：watchdog 通知父 shell 使用进程信号；正常结束路径会清理并等待 watchdog，但若父 shell 被不可捕获方式杀死且 PID 在检查与发信之间复用，理论上仍有极窄窗口。该情况不在正常应用取消路径中复现，本次审查未将其列为发布阻塞。

## 发布

本次由用户明确授权推送修复分支、创建并合并 PR、发布双平台 v0.10.9/build37、签名 macOS/Windows 安装包并同步三个更新源。待双平台原生 CI、合并、发布工作流与匿名更新源验收完成后补充证据。自动刷新仍保持暂停；发布不代表真实设备额度已恢复。
