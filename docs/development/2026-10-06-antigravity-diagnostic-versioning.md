# Antigravity现场诊断发行标识补充

**需求：** REQ-20261001-002
**规格与计划：** [状态/诊断规格](../design/specifications/2026-10-01-antigravity-status-and-diagnostics-design.md) · [实施计划](../design/implementation-plans/2026-10-01-antigravity-status-and-diagnostics.md)

## 触发与结论

2026-10-05现场摘要为 `refreshPaused=true reason=unknown`、`stage/result=unknown`，最后成功额度时间仍是 `2026-10-01T05:18:21Z`。这与旧持久暂停迁移为 `unknown` 且没有阶段事件记录一致。旧失败的阶段无法从现有状态重建；这些字段不能证明登录失效、额度耗尽或新版又发生超时。此次未清除暂停、未手动重试，也未触发 `agy`、真实登录或OAuth。

## 改动

复制的Antigravity诊断摘要现在加入 `appVersion` 和 `build`，值取自应用包元数据，并经过严格字符白名单和长度限制；缺失或不合规的值显示 `unknown`。该字段用于识别后续现场反馈来自哪个发行版。旧暂停仍保持原状态，stage/result缺失时仍诚实输出unknown。

## 验证与边界

诊断摘要和外部打开隔离相关测试通过；`swift build` 与 `scripts/check-docs.sh` 通过。完整 `swift test` 的测试用例除一项外通过；大小写敏感本地化测试无法创建/挂载测试磁盘映像，`hdiutil` 返回 `Device not configured`。此为当前环境限制，未改动该测试。合并改动经`gpt-6-luna/high`独立审查，未发现Critical、Important或Minor问题。

M4 Max现场仍停在未知原因的持久暂停；公开版本、版本号与build号尚待后续最终发布流程确认。进程列表查询也因本机 `sysmond` 服务不可用而无法完成，不能据此确认是否有遗留会话。
