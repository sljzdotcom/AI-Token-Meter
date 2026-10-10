# Antigravity 单次恢复启动与共享额度诊断

**状态：** 开发完成，独立代码复审与 v0.10.9/build37 双平台发布完成；现场额度状态未验证。

## 问题与范围

用户在 Services 点击单次额度检查后，Terminal 打开 `Open Antigravity Login.command`，但窗口没有 CLI 提示，Services 持续等待。历史上下文中有一份来自 0.10.6/build34 的诊断摘要；它没有记录暂停原因和阶段，不能说明登录或额度查询的具体失败原因。这是背景资料，不是本轮新需求或现场复测结果。自动刷新暂停仍按安全设计保留，单次检查成功也不会解除暂停。

本轮修复 Terminal 空白等待路径、回执测试隔离和共享额度缺失时详情页无提示的问题。真实 `agy 1.3.1` 输出与用户设备上的额度恢复仍未验证；不得把合成测试结果解释为现场成功。

## 实现

- `CLIAuthenticationLauncher` 显式注入脚本完成回执 opener。测试策略默认生成 `/usr/bin/false` 回执命令；合成流程只使用私有临时目录、fake CLI、fake opener、精简子进程环境和允许列表路径，不触碰真实 URL handler、CLI、认证目录或现场缓存。
- Antigravity 登录脚本在启动、等待 FIFO handoff、启动 CLI、登录成功/失败/超时、回执无法发送时打印阶段或可理解错误。脚本 handoff 及 CLI 最长等待 300 秒；AppModel 在 310 秒后结束等待并清除该次 token/FIFO。
- 新增 production launcher → FIFO token writer → 生成脚本 → fake CLI/opener → `GeminiLoginReceipt` → AppModel 的合成流程测试；验证单次额度可更新，同时自动刷新仍暂停。
- AppModel 测试可把应用支持目录注入专属临时目录，避免恢复状态测试读取生产 backoff/cache 路径。Shell 测试子进程的 `HOME`、`ZDOTDIR`、`TMPDIR`、工作目录和 `PATH` 全部限定在测试 fixture。
- App 取消时先取消 FIFO token writer，再通过 POSIX 记录锁把状态改为 `cancelled`。helper 在启动 CLI 的同一锁上检查状态并持锁到 `exec`，取消与 CLI 启动有明确先后；helper 确认后或最多等待 60 秒后清理标记。
- Antigravity 详情页在当前快照存在 Gemini 额度、但没有 Claude/GPT 共享额度时显示本地化说明，不再静默隐藏；有效共享窗口仍照常显示。

## macOS CI 超时调查

- 原生 macOS CI 的 20 分钟门禁取消于 `scripts/test.sh`。完整日志显示，最后启动且没有完成事件的是“Antigravity watchdog kills a CLI process group that ignores TERM”合成测试；该测试把 CLI 时限设为 1 秒。
- 根因在 watchdog 生命周期：原实现从 watchdog 进程启动时开始计量，短测试时限可能在 readiness 握手及 CLI 进程组建立前耗尽。测试随后等待脚本进程退出，没有本地等待上限，意外挂起会占满整项 CI 作业。
- 修复后由 CLI 启动门控写入受限权限的进程组标记，watchdog 完成 readiness 后等待该标记，再计量 CLI 时限。期限最终判定与应用取消共用 launch gate；watchdog 最多用 20 次、每次 50 毫秒的非阻塞锁尝试，不能被卡在等待启动器释放的锁上。脚本退出时清理标记；合成测试等待有上限并只清理本次创建的子进程。
- 回归结果：认证脚本 16 项通过，完整 Swift 主套件 633 项、刷新调度 3 项、PTY 18 项通过；合同、发布工具、362 份 Markdown 与公开发布安全检查通过。另以真实 Perl watchdog 和持锁进程覆盖启动锁跨越完整 10 秒期限、最终回收 bootstrapper 的情形。独立复审未发现发布阻塞；双平台原生 CI 尚待重跑。

## 验证与审查

- fail-closed 静态审查通过后，合成 launcher/AppModel 流程的六种结果均通过。终端捕获到 handoff 和 CLI 进度，回执解析与 AppModel 状态断言通过；取消门控另由独立锁进程和 watchdog 退出时序覆盖。
- `CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-module-cache scripts/test.sh` 的完整本机门禁通过：633 项 Swift 主套件（12 Widget、295 Core、326 App），另有3项刷新调度与18项 PTY 回归；共享合同、Windows release 归一化、更新 feed、362 份 Markdown 与公开发布安全门禁通过。认证脚本定向套件 16 项通过，其中包括真实 watchdog 在 launch gate 持锁跨过完整10秒启动期限仍会回收 bootstrapper 的回归。随后 `scripts/check-docs.sh` 独立通过。
- watchdog 在取消/期限边界最多做1秒非阻塞取锁；若 gate 一直不可用，则按超时结束以保证清理有界，因此极窄边界下取消可能被归类为超时，但不会让启动器逃逸或让等待无限延长。
- Windows：135 项 frontend tests、25 项 density 生命周期测试、macOS Chrome/Edge 实际样式验证、TypeScript/Vite production build、46 项 Rust 单元测试及集成测试、`cargo fmt --check` 和严格 Clippy 通过。
- Windows frontend、密度 lifecycle、Chrome/Edge 样式验证、production build、Rust、格式与严格 Clippy 检查在候选前序门禁中通过；本次 Swift 登录脚本门控未改变 Windows 运行时代码。npm 曾报告开发依赖 `source-map-js` 1.2.1 高危告警，已在兼容范围升级至 1.2.2，`npm audit` 结果为零漏洞。
- 无真实 `agy`、Google 登录、系统 URL 分发、用户安装应用、生产认证缓存或真实账号测试。`GeminiUsageParser` 的输出合同仍以仓库 1.1.28 fixture 为依据；本记录不确认 1.3.1 格式兼容。

## 发布

本次由用户明确授权推送修复分支、创建并合并 PR、发布双平台 v0.10.9/build37、签名 macOS/Windows 安装包并同步三个更新源。PR #61、双平台原生 CI、发布 workflow `38063497306`、签名/篡改校验和三个匿名更新源验收均已完成；逐项资产哈希及公开下载证据见[发布记录](2026-10-10-v0.10.9-release.md)。自动刷新仍保持暂停；发布不代表真实设备额度已恢复。
