# Antigravity 单次受控恢复实现计划

> **执行方式：** 当前开发入口按用户授权内联执行本计划；每个实现步骤按红—绿循环完成，最后独立审查。

**目标：** 为 macOS Antigravity 持久暂停提供显式登录与单次有界额度检查，并确保成功也不恢复自动刷新。

**架构：** AppModel 管理内存中的一次性状态与过期 token；登录脚本回送 token 和退出结果，coordinator 通过独立 quota-only collector 串行运行一次并只在成功解析后更新 Gemini 缓存。持久 suspended backoff 不清除，常规调度继续过滤 Gemini。

**技术栈：** Swift 6、SwiftUI/AppKit、Foundation `Process`、Swift Testing；本计划仅改 macOS Swift 源码和共享 Swift core，不改 Windows Rust 行为。

---

### 任务 1：一次性回执脚本与超时回调

**文件：**
- 修改：`Sources/AIMeterCore/Accounts/CLIAuthenticationScriptBuilder.swift`
- 测试：`Tests/AIMeterCoreTests/CLIAuthenticationScriptBuilderTests.swift`
- 修改：`Sources/AIMeterApp/System/CLIAuthenticationLauncher.swift`
- 测试：`Tests/AIMeterAppTests/CLIAuthenticationLauncherTests.swift`

- [x] 先增加合成 CLI 端到端测试：生成脚本，fake `agy` 返回 0，注入 fake opener 收到精确 token 且 `result=success`；再测非零返回 `result=failure`、看门狗到期不返回成功。测试不使用系统 `/usr/bin/open`。
- [x] 单独运行 `swift test --filter CLIAuthenticationScriptBuilderTests`，新增测试覆盖退出码门控、期限与进程回收。
- [x] 实现脚本退出码回执、可注入的 callback opener 和 300 秒进程组 watchdog；收到 HUP/INT/TERM 时只清理本次进程组并回送失败或让 AppModel deadline 到期。UUID 经私有 FIFO 传递，保留 shell quoting。
- [x] 再运行定向过滤测试，确认成功、失败、超时、终端关闭和遗留子进程路径符合断言；fake opener 检查脚本与回执。

### 任务 2：quota-only collector 与 coordinator 缓存提交

**文件：**
- 修改：`Sources/AIMeterCore/Collectors/GeminiCollector.swift`
- 新建或修改：`Sources/AIMeterCore/Collectors/OneTimeQuotaCollecting.swift`
- 修改：`Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`
- 修改：`Sources/AIMeterCore/Persistence/SnapshotCache.swift`
- 测试：`Tests/AIMeterCoreTests/GeminiCollectorTests.swift`
- 测试：`Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`
- 测试：`Tests/AIMeterCoreTests/GeminiCacheTests.swift`

- [x] 先写测试证明 one-time collector 只执行 `--version` 与 `-p /usage`，不执行 `/model`、`models`；失败输出与 timeout 不产出快照。
- [x] 先写 coordinator 测试：unknown/authentication suspension 可开始一次专用采集；成功更新 Gemini cache 且保留其他 cache；coordinator 重建后仍 suspended；普通 `refresh(manual:true/false)` 不再调用 Gemini。并发 one-time 调用最多进入 collector 一次。
- [x] 运行定向过滤测试，确认新 API 与缓存行为符合预期。
- [x] 实现独立 quota-only collector 路径与单次 in-flight gate；成功原子合并缓存，失败不写缓存且不碰 backoff，并移除旧回调清暂停依赖。
- [x] 定向验证 CLI 参数、缓存重载、暂停 JSON 重载、并发次数及普通 Gemini supplemental 行为；补并发刷新不能覆盖新缓存的回归。

### 任务 3：AppModel 一次性有限状态机

**文件：**
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/AppDelegate.swift`
- 测试：`Tests/AIMeterAppTests/AppModelStartupTests.swift`
- 新建或修改：`Tests/AIMeterAppTests/AppModelGeminiRecoveryTests.swift`

- [x] 先写 fake operation 测试：unknown/auth pause 可启动一次登录；打开失败、非零、token 不匹配、重复/过期 token 与 deadline 均不调用 quota check；成功 token 调用 quota check 一次，不调用全局 refresh/清暂停操作。
- [x] 覆盖重复点击/回执、登录等待期间普通刷新、quota check 并发、`stop()` 后回执及新启动不恢复旧 token。
- [x] 运行 `swift test --filter AppModelGeminiRecoveryTests` 验证状态机与 guard。
- [x] 实现 `idle/awaitingLogin/checkingQuota/succeeded/failed` 内存状态、330 秒可注入 deadline、回调参数验证和 App stop 作废；成功只调用 coordinator 专用 one-time API，不调用全局 `refresh()`。startup 不恢复状态。
- [x] 回跑 focused 测试；额外以延迟普通刷新验证较旧的 UI/Widget 结果不会覆盖单次新额度。

### 任务 4：暂停入口、辅助信息与本地化

**文件：**
- 修改：`Sources/AIMeterApp/Views/ServicesSettingsView.swift`
- 修改：`Sources/AIMeterApp/Views/GeminiDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/FloatingStripView.swift`
- 修改：`Sources/AIMeterApp/System/SettingsNotice.swift`
- 修改：`Sources/AIMeterApp/Resources/en.lproj/Localizable.strings`
- 修改：`Sources/AIMeterApp/Resources/zh-Hans.lproj/Localizable.strings`
- 修改：`Sources/AIMeterApp/Resources/zh-Hant.lproj/Localizable.strings`
- 测试：`Tests/AIMeterAppTests/GeminiAvailabilityTests.swift`

- [x] 增加暂停状态 action 覆盖与所有暂停原因的一次性入口。
- [x] 在 Services 和详情复用同一入口，并明确 Terminal/可能的浏览器授权、有限检查和成功后仍暂停；补齐 busy、失败/过期/成功提示及三语言文本，没有声称 OS 沙箱隔离。
- [x] UI/动作及 localization 定向测试通过；非暂停流程保持既有动作。

### 任务 5：诊断、文档、独立审查与本地交付

**文件：**
- 修改：`Sources/AIMeterCore/Diagnostics/GeminiDiagnostic.swift`
- 修改：`Sources/AIMeterCore/Collectors/GeminiCollector.swift`
- 修改：`docs/design/README.md`
- 修改：`docs/development/README.md`
- 新建：`docs/development/2026-10-07-antigravity-one-time-recovery.md`
- 修改：`docs/requirements-backlog.md`

- [x] 增加诊断测试覆盖登录结果、quota-only 阶段/结果与字段白名单；摘要不保存原始 CLI 文本、凭据、用户路径或 OAuth URL。
- [x] 运行诊断定向测试；补齐阶段、文案、设计/开发索引与需求记录，保留历史。
- [x] 完成验证：完整 323 项 Swift 测试通过（排除一个需受限 HFSX 磁盘映像的测试）；`scripts/check-docs.sh`、`git diff --check`、shell syntax、跨平台合同、Release feed、Windows 资产归一化及公开发布安全检查通过。无发布。
- [x] 独立最终审查覆盖超时进程组、遗留子进程、缓存并发、AppModel UI 快照竞态、FIFO token 与失败清理；按反馈修复并重跑相关测试。提交本地实现与收尾证据；不推送、不发布、不安装、不启动应用。

## 計畫自检

- 规格第1–6项分别由任务1–5覆盖：UI/三语由任务4，回执/期限由任务1、3，次数/暂停/缓存由任务2、3，诊断由任务5，Windows与正常行为由任务2、4、完整验证覆盖。
- 运行时采用 token 单次认领、collector actor gate 与持久 suspended 三层保护；登录失败不进入 quota collector；成功结果只写 Gemini 缓存并不调用全局刷新。
- 路径都遵循 Swift 源码和现有测试布局；Windows Rust 不参与构建路径变更，跨平台额度 JSON 合同由既有合同测试守护。
