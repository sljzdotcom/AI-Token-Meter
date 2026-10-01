# 0.10.5/build33 Antigravity 状态与诊断发布记录

## 需求和范围

- 需求：`REQ-20261001-002`。
- 本次仅改 macOS Antigravity的状态与诊断表达；Windows随双平台共享版本发布，运行行为不变。
- 不启动真实 `agy` 或 Google OAuth，不读取或改动 M4 Pro 账户/环境。M4 Max实机验证仍待用户安装后确认。
- 保留任何采集失败后的持久暂停与防浏览器弹窗保护；仅明确认证错误提供登录恢复入口。

## 变更

- 把最近缓存额度标为 Last known quota，保留原采集时间；失败后立即显示分类后的暂停原因。
- 持久暂停原因采用固定枚举并迁移旧版未知错误；不可由启动、唤醒、定时、Retry或普通手动检查清除。
- 本地诊断限20条/16KiB，仅有阶段、固定结果类别、耗时和输出截断标志；不收集正文、参数、路径、环境、身份或令牌。Services和详情页提供复制入口。
- 非认证暂停显示独立暂停状态，不再暗示需要登录/重试；已发现CLI不再显示安装引导。
- 完整规格与计划：[设计](../design/specifications/2026-10-01-antigravity-status-and-diagnostics-design.md) · [计划](../design/implementation-plans/2026-10-01-antigravity-status-and-diagnostics.md)。

## 验证与发布证据

- macOS Swift：269 Core、305 App、12 Widget、3刷新调度、18 PTY测试通过（合计607项）；
- Windows frontend：133 Vitest、25 density lifecycle/browser support、Chrome真实浏览器密度场景通过；生产前端构建通过；
- Windows Rust：`cargo test --locked`及本地回环集成测试通过；
- `scripts/check-docs.sh`：334 Markdown文件通过；
- 当前候选分支、PR、main/tag、签名资产、SHA-256、更新源与发布workflow：待正式发布流程完成后填写。
