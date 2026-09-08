# macOS 刷新间隔实现计划

> 面向 AI 工作者：使用 subagent-driven-development，执行一个设置/调度任务，任务规格质量审查后再做整分支审查。用户已授权普通技术决定直接处理。

**目标：** 修复 macOS 设置用量刷新间隔固定五分钟不可修改。
**架构：** 原生编辑器提交已校验整数秒，AppModel 偏好持久化与可重新计时的周期等待；保留 RefreshCoordinator 退避边界。
**技术栈：** SwiftUI、AppKit 原生控件（必要时）、Swift Concurrency、UserDefaults、Swift Testing。

## 全局约束

默认300秒，合法30至86400整数秒；Apply/回车提交，错误不破坏旧值。保存时重新开始等待，正在采集不取消/重叠，停止或旧等待迟到不再触发。周期调用仍为manual:false，不清退避/限流，不重启身份检查/登录任务。仅macOS，不改Windows/版本更新/发布/账户。当前隔离分支codex/macos-refresh-interval，不写主仓库，不覆盖其他工作者。

### Task 1: 原生设置编辑、持久化与运行时调度

**所有权：** Sources/AIMeterApp/AppModel.swift、Views/MonitoringSettingsView.swift、必要小型新刷新偏好/调度或编辑组件及对应 Tests/AIMeterAppTests/CoreTests 文件；不得修改docs/backlog（控制者负责）、Windows或无关功能。

- [x] 先读规格并核对静态标签/300秒睡眠的诊断；运行现有相关基线测试。先补实际原生渲染编辑测试，在旧代码中因没有输入控件而失败。测试使用独立UserDefaults、假SecretStore、注入身份/用量操作，避免真实账号与网络。
  ```swift
  let input = try #require(descendants.compactMap { $0 as? NSTextField }.first { $0.isEditable })
  input.stringValue = "60"
  // 通过真实字段编辑事件及 Apply 按钮动作提交；断言模型/偏好/重新构造模型与渲染值。
  ```
- [x] 增加验证失败用例：空/小数/非数字/29/86401保持旧值，30/300/86400接受；损坏持久化回退300。运行RED后最小实现范围校验/持久化和系统原生编辑器，不依靠源码文本断言。
- [x] 调度测试先RED：用可控等待和可门控采集，启动一次刷新后等待300；保存60取消旧等待并等待60；只有新等待完成后调用自动刷新；更改中途不取消采集/不并发，停机/迟到旧等待无刷新，重启读取60。用真实AppModel路径，假对象只在外部操作/时间边界。
- [x] 最小调整AppModel循环：区分可取消等待与正在采集，保存只重排周期，不重复启动身份检查或清除登录任务；调度不绕过RefreshCoordinator退避。按测试实现，不增加后台软件更新。
- [x] GREEN后 scripts/check-docs.sh、完整 scripts/test.sh（使用任务独立AI_METER_TEST_BUILD_DIR）与Release构建；保留原有环境限制，不能以跳过失败冒充通过。日志放/private/tmp/req010-*，报告完整命令/退出码/计数。
- [x] 自审并仅提交负责代码/测试；报告有效RED/GREEN与证据路径、根因、边界、Git范围，交独立审查。

## 控制者收尾

- [x] 补开发日志、指南、Unreleased、设计/开发索引、当前项目状态及REQ-010完成记录。
- [x] 独立任务审查与最终分支审查通过，检查无Windows/版本源变更，回传协调入口整合，不直接合并/发布。
