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

- [ ] 先增加合成 CLI 端到端测试：生成脚本，fake `agy` 返回 0，注入 fake opener 收到精确 token 且 `result=success`；再测非零返回 `result=failure`、看门狗到期不返回成功。测试不使用系统 `/usr/bin/open`。
- [ ] 单独运行 `swift test --filter CLIAuthenticationScriptBuilderTests`，确认新增行为因脚本仍无退出码门控/期限而失败。
- [ ] 实现脚本退出码回执、可注入的 callback opener 和 300 秒只针对本次 child PID 的 watchdog；收到 HUP/INT/TERM 时只清理本次 child 并回送失败或让 AppModel deadline 到期。保留 UUID 参数校验与 shell quoting。
- [ ] 再运行同一过滤测试，确认成功、失败、超时与信号路径均符合断言；用 fake launcher opener 检查脚本路径和每次唯一 token。

### 任务 2：quota-only collector 与 coordinator 缓存提交

**文件：**
- 修改：`Sources/AIMeterCore/Collectors/GeminiCollector.swift`
- 新建或修改：`Sources/AIMeterCore/Collectors/OneTimeQuotaCollecting.swift`
- 修改：`Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`
- 修改：`Sources/AIMeterCore/Persistence/SnapshotCache.swift`
- 测试：`Tests/AIMeterCoreTests/GeminiCollectorTests.swift`
- 测试：`Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`
- 测试：`Tests/AIMeterCoreTests/GeminiCacheTests.swift`

- [ ] 先写测试证明 one-time collector 只执行 `--version` 与 `-p /usage`，不执行 `/model`、`models`；失败输出与 timeout 不产出快照。
- [ ] 先写 coordinator 测试：unknown/authentication suspension 可开始一次专用采集；成功更新 Gemini cache 且保留其他 cache；coordinator 重建后仍 suspended；普通 `refresh(manual:true/false)` 不再调用 Gemini。并发 one-time 调用最多进入 collector 一次。
- [ ] 运行三个定向过滤测试，确认因新 API 不存在或断言不符而失败。
- [ ] 实现单独的 quota-only collector 协议/方法；coordinator 持有专用单次 in-flight gate，成功 parser 回执后原子合并缓存，失败不写缓存且不碰 backoff。移除仅为旧回调清暂停使用的生产注入依赖。
- [ ] 运行定向测试，检查 CLI 参数序列、缓存文件重载、暂停 JSON 重载和并发调用次数；原正常 Gemini collector 的 model/catalog 读取回归不变。

### 任务 3：AppModel 一次性有限状态机

**文件：**
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/AppDelegate.swift`
- 测试：`Tests/AIMeterAppTests/AppModelStartupTests.swift`
- 新建或修改：`Tests/AIMeterAppTests/AppModelGeminiRecoveryTests.swift`

- [ ] 先写 fake operation 测试：unknown/auth pause 可启动一次登录；打开失败、非零、token 不匹配、重复/过期 token 与 deadline 均不调用 quota check；成功 token 调用 quota check 一次，不调用全局 refresh/清暂停操作。
- [ ] 先写竞态与重启测试：重复点击/回执、登录等待期间 timer refresh、quota check 并发、`stop()` 后回执、模拟 relaunch 后旧 token 都不能再调用 Gemini collector。
- [ ] 运行 `swift test --filter AppModelGeminiRecoveryTests` 确认缺少状态机/guard 时失败。
- [ ] 实现 `idle/awaitingLogin/checkingQuota/succeeded/failed` 内存状态、330 秒可注入 deadline、回调结果参数验证和 App stop 作废；成功只调用 coordinator 专用 one-time API，不调用全局 `refresh()`。App stop 丢弃 token；startup 不恢复状态。
- [ ] 回跑 focused 测试，确认每个状态只允许一次转移、deadline 与 completion 竞态只由先到的一方认领、常规 refresh 保持不受影响。

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

- [ ] 先增加暂停状态 action 选择测试，覆盖 `.unknown`、`.authenticationRequired`、其他 pause 都显示 one-time recovery，正常状态仍显示原动作。
- [ ] 先运行 focused 测试确认旧条件只对认证暂停显示登录入口，因此 unknown 用例失败。
- [ ] 实现共用的一次性入口与明确说明：会运行官方 Terminal 登录、可能出现浏览器授权页、一次采集上限、成功后仍暂停；显示 busy、失败/过期与成功“单次检查成功，自动刷新仍暂停”。三语言翻译均准确，不声称 OS 沙箱隔离。
- [ ] 再运行 UI/动作及 localization tests；检查 Settings 和 detail 两处都能启动同一 AppModel action，非暂停用户流程不变。

### 任务 5：诊断、文档、独立审查与本地交付

**文件：**
- 修改：`Sources/AIMeterCore/Diagnostics/GeminiDiagnostic.swift`
- 修改：`Sources/AIMeterCore/Collectors/GeminiCollector.swift`
- 修改：`docs/design/README.md`
- 修改：`docs/development/README.md`
- 新建：`docs/development/2026-10-07-antigravity-one-time-recovery.md`
- 修改：`docs/requirements-backlog.md`

- [ ] 先写诊断测试覆盖登录成功/失败/超时与 quota-only 成功/失败阶段和严格字段白名单，证明摘要无原始 CLI 文本、凭据、用户路径和 OAuth URL。
- [ ] 运行 focused diagnostic tests，确认新阶段/结果行为失败。
- [ ] 实现最少诊断枚举与本地结果文案；补设计索引、开发记录和项目文档索引；更新 backlog 需求证据，保留所有历史记录。
- [ ] 依序运行 `scripts/test.sh`、`scripts/check-docs.sh`、`git diff --check` 与构建/合同检查；任何受环境限制的检查如实记录。
- [ ] 请求独立代码审查，按 Critical/Important 修正并对修正重跑对应回归；保存本地实现提交及最终测试证据，不推送、不发布、不安装、不启动应用。

## 計畫自检

- 规格第1–6项分别由任务1–5覆盖：UI/三语由任务4，回执/期限由任务1、3，次数/暂停/缓存由任务2、3，诊断由任务5，Windows与正常行为由任务2、4、完整验证覆盖。
- 运行时采用 token 单次认领、collector actor gate 与持久 suspended 三层保护；登录失败不进入 quota collector；成功结果只写 Gemini 缓存并不调用全局刷新。
- 路径都遵循 Swift 源码和现有测试布局；Windows Rust 不参与构建路径变更，跨平台额度 JSON 合同由既有合同测试守护。
