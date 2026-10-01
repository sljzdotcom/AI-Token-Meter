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
- PR [#56](https://github.com/sljzdotcom/AI-Token-Meter/pull/56) 合入 `main`：`b4a4b45`；候选 `71e62d2`，双平台 PR CI `36881216889` / `36881216958` 通过；
- `v0.10.5` 于 2026-10-01 公开：[GitHub Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.10.5)，发布 workflow [36883701586](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/36883701586)，Windows 签名资产构建与校验通过；stable appcast 提交 `257a4c8`；
- macOS ZIP SHA-256：`6df2a9270801f401bb1ff13ec37ff9ce07c6060a68caa7c476d23c524909cfd6`；Windows installer SHA-256：`2059567f68f268b6c0df22d18f37973be7072aac51edaca5f8c4ca664bac7415`；两份 sidecar 与匿名下载所得摘要一致；
- Release 含 macOS ZIP/SHA-256/appcast 与 Windows NSIS/SHA-256/minisign/latest.json 共七项资产；匿名重下核对一致。stable macOS appcast、Windows stable `latest.json` 和固定 Preview `latest-preview.json` 均指向 0.10.5；Release 中 appcast 与根稳定 appcast相同；
- 本机最终打包过程中出现两个运行精确 Antigravity 登录脚本的 Terminal 会话；仅停止其已核实的独立进程组并关闭对应标签。会话来源及任何账户侧影响均未确认；该事件单独跟踪，不能视作此版本已修复或与发布测试无关的证据；
- M4 Max 实机更新后的额度状态与真实超时根因仍待现场核验；未运行真实 `agy` / Google OAuth。
