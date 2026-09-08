# macOS 用量刷新间隔编辑

关联需求：REQ-20260908-010。日期：2026-09-08。

## 根因与范围

用户看到的五分钟位于 Monitoring 的 Refresh interval。旧 `MonitoringSettingsView` 使用不可编辑的 `LabeledContent` 显示固定文字，`AppModel.start()` 又固定等待 300 秒，没有编辑绑定、偏好键或可更新的计时链路。不是软件版本更新检查故障。

采用项目既有 30 至 86400 整数秒范围，默认 300 秒；macOS 增加系统原生输入和 Apply/回车提交，错误值保留旧设置。保存更新偏好并重排等待，正在进行的采集不取消、不并发；周期刷新继续通过原协调器的限流和退避。

- [规格](../design/specifications/2026-09-08-macos-refresh-interval-design.md)
- [计划](../design/implementation-plans/2026-09-08-macos-refresh-interval.md)

## 开发检查点

REQ-009 完成回传后开始。本项需求基线 `be758f1` 在独立分支 `codex/macos-refresh-interval` 以 `99e0dd8` 引入；只解决相邻台账行冲突，保留 REQ-009 完成记录。诊断/规格/计划提交 `7d58936`。实现、红绿测试与独立审查结果在完成后记录。

## 交付边界

本次暂不发布、不安装替换用户应用，不改版本、签名、更新源、Release、Windows 设置或真实账户。用户安装包的实际现场编辑仍与测试宿主的原生控件验证区分。REQ-007/009 仍留在未发布内容中，REQ-001 真实 Windows/WSL 验收仍受环境限制。
