# Antigravity login startup recovery implementation plan

> **面向 AI 代理的工作者：** 按本计划逐项执行；脚本测试必须先通过独立 fail-closed 静态审查，后续实现遵循测试驱动。

**目标：** 让 Antigravity 一次性额度检查在启动、等待认证和失败时都提供明确进度与可恢复结果，同时确保合成验证永远不触碰真实 opener、CLI、认证目录或用户状态。

**架构：** `CLIAuthenticationLauncher` 为认证脚本和认证可执行文件提供显式注入点；合成测试使用唯一临时根目录、受限环境、fake `agy`、fake callback opener 和受控脚本运行器。生成脚本应将启动、FIFO握手、CLI运行及终态写到终端，并对握手前的失败给出可读错误；AppModel 继续使用有界等待和仅针对自身登录 token 的取消。普通刷新仍跳过暂停服务，成功的一次性检查仍不解除自动刷新暂停。

**技术栈：** Swift Testing、Swift `Process`、macOS zsh/FIFO、现有 `AppModel` 一次性恢复状态机。

---

## 安全前置门

- [x] 独立静态审查 `NSWorkspace.open`、脚本回执 opener、CLI discovery、环境继承、测试目录、FIFO、Perl/watchdog、信号与清理；审查者放行后才执行新的合成测试。
- [x] 测试默认策略把脚本回执 opener 设为 `/usr/bin/false`；完整夹具显式注入临时 fake CLI/opener，使用 0700 根目录和 allowlist 子进程环境。
- [x] 未重放历史临时测试；将其改写成正式、无系统 URL 分发的 fixture，并经静态复审。

## 任务 1：覆盖生产 launcher 的隔离合成端到端路径

**文件：**
- 修改：`Sources/AIMeterApp/System/CLIAuthenticationLauncher.swift`
- 修改：`Tests/AIMeterAppTests/CLIAuthenticationLauncherTests.swift`
- 修改：`Tests/AIMeterCoreTests/CLIAuthenticationScriptBuilderTests.swift`

- [x] launcher 支持 callback opener 注入；生产默认 `/usr/bin/open`，测试策略默认 `/usr/bin/false`，合成 fixture 验证脚本/CLI/opener 路径均位于临时根。
- [x] `GeminiLoginRecoveryEndToEndTests` 调用生产 launcher、FIFO token writer、生成的 zsh、fake CLI/opener，再将解析后的回执送给 AppModel。
- [x] 安全门通过后运行完整流程；脚本启动、handoff、fake CLI 成功、回执到 AppModel 及临时脚本/FIFO清理均有断言。
- [x] 运行 launcher/script 与 AppModel 定向覆盖，并运行完整 Swift 测试。

## 任务 2：令等待阶段和失败出口可见、可恢复

**文件：**
- 修改：`Sources/AIMeterCore/Accounts/CLIAuthenticationScriptBuilder.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 修改：`Sources/AIMeterApp/Views/GeminiDetailView.swift`
- 修改：`Sources/AIMeterApp/Views/ServicesSettingsView.swift`
- 修改：对应 localization 与 `Tests/AIMeterAppTests/AppModelGeminiRecoveryTests.swift`
- 修改：对应 script、receipt 与 detail/settings tests

- [x] 生成脚本在缺失 FIFO、打开失败、handoff 超时、非法 token、CLI失败/超时及回执 opener 失败路径输出可理解错误；脚本构造断言覆盖消息和超时值，执行式超时/CLI路径使用 fake fixture。
- [x] 终端显示 helper启动、等待 handoff、开始 CLI、结束/超时/失败及回执错误；握手前无 token 的失败由终端说明并在 310 秒 AppModel 有界超时后结束等待（脚本等待 300 秒）。
- [x] 现有取消路径仍仅清理该 token 对应的 FIFO和该脚本进程组；端到端断言成功后暂停仍在。
- [x] AppModel 端到端合成断言覆盖暂停状态、成功回执、单次额度更新与自动暂停保留；原有状态机测试覆盖失败/取消/过期。

## 任务 3：共享额度验收边界、文档、审查与本地收尾

**文件：**
- 修改：`docs/requirements-backlog.md`（只整合 REQ-20261010-001/002/003 对应行，保留协调入口期间其他变更）
- 修改：`docs/development/README.md` 或对应开发日志索引
- 新增：`docs/development/2026-10-10-antigravity-login-startup-recovery.md`
- 记录：本计划及 `docs/design/README.md`

- [x] 保留四行共享额度解析/缓存/展示的合成合同覆盖；详情页在当前快照只有 Gemini quota 时明确显示共享额度不可用。该合同固定于已有 1.1.28 fixture，不能证明真实 `agy 1.3.1` 输出兼容或现场额度恢复。
- [x] `scripts/check-docs.sh` 通过（359 份 Markdown）；其后完整 Swift 测试通过（328 项，56 suites）。
- [ ] 完成独立代码审查并处理所有 Critical/Important 发现；安全隔离审查已通过。
- [ ] 更新开发记录与台账，完成 Git 状态/凭据隐私检查及提交；REQ-20261010-004 已明确授权本轮随后推送、PR、合并、双平台签名 Release 和更新源同步。
