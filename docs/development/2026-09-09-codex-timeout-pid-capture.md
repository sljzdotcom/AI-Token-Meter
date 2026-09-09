# Codex 超时进程取证稳定性

关联需求：`REQ-20260909-009`。日期：2026-09-09。发布事务：0.6.3/build18。

## 失败证据

合并提交 `dc91bac3ef2d2276daaed757a9c796e991f2cd7d` 已通过 main 的 macOS CI 34354932107 和 Windows CI 34354932151。随后首次运行正式跨平台打包脚本时，完整 Swift 门禁中的 `CLICollectorTests.codexTimeoutIsBounded` 约 1.933 秒后以函数级 `transportFailure` 失败。脚本在创建 `v0.6.3` 标签、草稿 Release 或公开资产前停止；公开稳定版和三个更新源仍为 0.6.2。

该测试已经观察到预期 `timedOut`。后续失败来自测试等待 shell 子进程写 PID 临时文件：在完整负载下，0.5 秒截止可以先终止尚未获调度写文件的子进程。v0.5.0 发布和紧凑浮岛发布记录包含相同形态，因此本次不以原样重跑放行。

## 修复

`CodexAppServerClient` 增加默认为空的内部启动观察闭包。`Process.run()` 成功后，代码先把进程及管道登记到既有清理容器，再把系统提供的 `processIdentifier` 交给观察者。生产调用不提供观察者，因此命令、协议、环境、期限、解析和返回行为不变。

超时测试改用 `NSLock` 保护的内存捕获器取得 PID，不再依赖被测子进程写文件。它仍要求 0.5 秒请求得到 `timedOut`、完整调用少于 2 秒，并确认真实 PID 在 2 秒内退出。覆盖“截止先于进程登记”的另一项测试保持原样。

## 失败先行与定向验证

- RED：测试先调用新的启动观察参数，编译准确失败为 `extra argument 'processDidLaunch' in call`。
- GREEN：实现最小观察入口后，`codexTimeoutIsBounded` 1/1 在约 1.058 秒通过。
- 回归：全部 `CLICollectorTests` 16/16 在约 17.726 秒通过，包括登记前超时、忽略普通终止信号、握手和清理路径。

## 完整验证与审查

- 正式脚本使用的完整 Swift 编排通过：464 项主测试、3 项刷新调度、18 项 PTY runner 和 6 项 Gemini PTY，共 491 项；原失败用例在主测试负载中约 1.034 秒通过。
- 6 份跨平台合同、Windows 资产归一化、release feed 回归、228 份 Markdown、公开发布安全与差异检查通过。
- 无 Widget 的 0.6.3/build18 Apple Silicon Release App 重新构建；便携资源、Sparkle 2.9.4 framework/helper、`@rpath`、arm64 文件类型、嵌套组件与应用严格签名验证通过。

隔离复核结果为 Critical 0、Important 0、Minor 0。启动观察发生在 `processBox.set` 之后，因此观察者取得 PID 时清理容器已经持有进程；即使超时先于登记，既有 `set` 仍会立即重放停止请求，再记录系统 PID。捕获器的读写均经过同一把锁；进程启动失败、PID 缺失或回收失败仍会使测试失败。默认闭包为空，全部生产初始化调用保持原行为。

修复与本地证据提交为 `60fe1eb`。PR/main 双平台 CI 与恢复发布证据将在同一记录中继续补充；在这些条件完成前，本需求保持进行中。
