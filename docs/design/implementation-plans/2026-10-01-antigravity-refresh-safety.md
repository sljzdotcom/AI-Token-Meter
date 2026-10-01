# Antigravity 静默刷新与异常暂停实施计划

> **面向 AI 代理的工作者：** 按本计划逐项实现；步骤用复选框跟踪，先写并运行失败测试，再实现最小修复。

**目标：** 防止未认证、超时或取消后的 Antigravity 查询被后台事件不断重启，同时允许用户显式启动一次交互登录。

**架构：** 在持久化刷新协调器中加入 Gemini 专属的终态暂停，所有非登录入口遵守暂停。macOS 登录按钮通过官方 `agy` 交互会话解除暂停，回调只触发单次查询。命令 runner 将被监管命令置于独立进程组并对整组清理。

**技术栈：** Swift 6、Foundation/Darwin、Swift Testing、SwiftUI。

---

### 任务 1：Gemini 持久暂停状态

**文件：** `Sources/AIMeterCore/Coordination/RefreshBackoff.swift`、`Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`、`Tests/AIMeterCoreTests/RefreshBackoffTests.swift`、`Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`

- [x] 加测试：Gemini 任意失败后，无论 automatic/manual 及重建 coordinator，下一轮都不调用 collector。
- [x] 加测试：清除终态暂停后下一轮恢复查询，其他 provider 退避行为不变。
- [x] 运行指定测试确认失败，再实现持久 `.suspended` 状态与只供显式登录调用的清除 API。

### 任务 2：刷新入口与显式登录

**文件：** `Sources/AIMeterApp/AppModel.swift`、`Sources/AIMeterApp/AppDelegate.swift`、`Sources/AIMeterApp/System/CLIAuthenticationLauncher.swift`、`Sources/AIMeterCore/Accounts/CLIAuthenticationScriptBuilder.swift`、`Sources/AIMeterApp/Views/ServicesSettingsView.swift`、对应英文/简繁本地化资源；新增 `Tests/AIMeterAppTests/AntigravityRefreshSafetyTests.swift`。

- [x] 先加 AppModel/launcher 测试，证明无效及重复回执不清暂停，明确登录脚本完成信号至多触发一次查询。
- [x] 运行测试确认失败，再调整路由为 explicit sign-in completion；普通 Gemini Retry 不清持久暂停。
- [x] 运行 Gemini/Services 用例，确认暂停时提供专属交互登录按钮，并禁用并发登录启动。

### 任务 3：命令进程组清理

**文件：** `Sources/AIMeterCore/Collectors/BoundedCommandRunner.swift`、`Tests/AIMeterCoreTests/BoundedCommandRunnerTests.swift`。

- [x] 加取消 fake descendant 测试：同组子进程收到清理信号并退出，组外 sentinel 不受影响。
- [x] 运行测试确认旧实现失败；采用 `posix_spawn` 独立进程组和有界 TERM→KILL 清理，不按名称全局杀进程。
- [x] 运行 Gemini collector 和命令执行专项，核对常规输出/退出码/期限不变。

### 任务 4：版本发布、完整门禁和交付

**文件：** `VERSION`、`CHANGELOG.md`、`README.md`、`docs/releases/`、`docs/development/`、设计与需求索引、双平台锁文件如发布脚本要求。

- [ ] 递增稳定补丁版本与 macOS build，记录 Windows 同版本但不改变其运行逻辑。
- [ ] 运行 Swift/Windows/文档/安全/正式打包门禁；保存准确输出和本机候选路径。
- [ ] 请求 `gpt-6-luna/high` 独立代码审查；修复所有 Critical/Important 项并复核。
- [ ] 推送特定 release 分支、创建 PR、等待精确双平台 CI、合并 main、创建 tag/Release、发布签名资产与更新源。
- [ ] 匿名核对公开资产哈希/签名和三个更新入口，更新 REQ-20261001-001 完成证据并回传协调任务。
