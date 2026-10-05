# 测试进程外部操作隔离

**需求：** REQ-20261001-003
**规格：** [测试进程外部操作隔离设计](../design/specifications/2026-10-05-test-process-external-action-safety-design.md)
**计划：** [实施计划](../design/implementation-plans/2026-10-05-test-process-external-action-safety.md)

## 问题

0.10.5 中，两个 `GeminiAvailabilityTests` 用例直接触发 `beginSignIn(.gemini)`，没有注入专用操作闭包；AppModel 因此采用真实默认登录启动器。此前测试进程可能将登录脚本交给系统打开。静态调用链与两次新登录终端会话吻合，但无法归因较早的全部窗口，也没有证据说明账号侧发生了何种变化。

## 实现

- 新增 `SystemActionPolicy`，用 XCTest 环境标记、测试 bundle 和 SwiftPM/XCTest runner 进程名识别测试上下文。
- AppModel 的默认登录、安装、指南和 Claude workspace 操作在 XCTest 中 fail-closed；用户在正式 App 中显式操作仍走原来的启动流程。
- 认证与安装启动器在测试进程拒绝默认系统打开，并把默认脚本目录放到唯一临时路径，避免访问真实 Application Support。Codex 指南、Claude workspace 设置和品牌链接共用同一策略。
- 为两个 Gemini 可用性用例补齐专用模拟操作，并增加策略、低层启动器和 AppModel 回归。

## 验证与边界

针对性 Swift 测试共56项通过，另有1项平台条件测试跳过；修正安装专属错误类型后，安装器/策略/诊断重点测试再次通过。`swift build`通过。完整`swift test`中288项Core和12项Widget测试通过；317项App测试中仅大小写敏感本地化镜像测试失败，因为当前环境的`hdiutil`返回`Device not configured`。未改动或跳过该测试。`scripts/check-docs.sh`检查338份Markdown通过，`git diff --check`通过。使用`gpt-6-luna/high`独立审查完整改动，最终未发现Critical、Important或Minor问题。

未启动真实 Terminal、浏览器、`agy`、登录/OAuth或真实 CLI 安装；没有改动真实账号。进程枚举因系统 `sysmond` 服务不可用而无法读取，故不能确认机器当前是否仍有相关会话。此修复不能证明 M4 Max 额度状态已恢复。最新稳定版公开发布另由 REQ-20261005-001 跟踪。
