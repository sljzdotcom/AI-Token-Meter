# macOS 用量刷新间隔编辑

关联需求：REQ-20260908-010。日期：2026-09-08。

## 根因与范围

用户看到的五分钟位于 Monitoring 的 Refresh interval。旧 `MonitoringSettingsView` 使用不可编辑的 `LabeledContent` 显示固定文字，`AppModel.start()` 又固定等待 300 秒，没有编辑绑定、偏好键或可更新的计时链路。不是软件版本更新检查故障。

采用项目既有 30 至 86400 整数秒范围，默认 300 秒；macOS 增加系统原生输入和 Apply/回车提交，错误值保留旧设置。保存更新偏好并重排等待，正在进行的采集不取消、不并发；周期刷新继续通过原协调器的限流和退避。

- [规格](../design/specifications/2026-09-08-macos-refresh-interval-design.md)
- [计划](../design/implementation-plans/2026-09-08-macos-refresh-interval.md)

## 开发检查点

REQ-009 完成回传后开始。本项需求基线 `be758f1` 在独立分支 `codex/macos-refresh-interval` 以 `99e0dd8` 引入；只解决相邻台账行冲突，保留 REQ-009 完成记录。诊断/规格/计划提交 `7d58936`。实现提交 `57f8ea3`。文档检查点 `ee85e32` 后，以 `c6a57a2` 合入协调入口 `c9aa58c`，保留其 REQ-009 完成行、项目状态与整合证据；只处理相邻台账行冲突，生产代码未变。

## 测试证据

全部在本需求隔离缓存 `/private/tmp/req010-swift` 执行，测试使用独立偏好域和注入的账号/采集/时钟边界。

| 检查 | 结果与证据 |
| --- | --- |
| 旧代码原生编辑 RED | 实际 NSHostingView 找不到可编辑 NSTextField，按预期失败；`/private/tmp/req010-ui-red.log` |
| 校验 RED → GREEN | 空值、非数字、小数及越界拒绝，边界值接受；实际 Apply/字段回车 action 保存，重建模型与原生视图恢复；`/private/tmp/req010-validation-red.log`、`/private/tmp/req010-ui-green.log` |
| 调度 RED → GREEN | 固定等待在3项测试中暴露6个失败；替换等待后，21项聚焦测试通过。覆盖旧等待迟到、停止、相同值、正在采集不取消/重叠、身份检查不重启和原协调器；`/private/tmp/req010-scheduling-red.log`、`/private/tmp/req010-focused-green.log` |
| 控制者复测 | 初版实现合入协调文档后，独立复跑21项聚焦测试及后续脚本门禁退出0；`/private/tmp/req010-root-verification.log`，该轮尚未覆盖后续审查发现的失焦边界 |
| 全量 Swift | `AI_METER_TEST_BUILD_DIR=/private/tmp/req010-swift scripts/test.sh` 退出0：426项主测试/83套件及13项独立PTY，共439项；`/private/tmp/req010-full.log` |
| 脚本门禁 | 全量测试前 `scripts/check-docs.sh` 通过；4份跨平台合同夹具、可移植性、Windows资产标准化、feed探针、193份文档和公开安全检查全通过 |
| Release | 隔离缓存 Release 构建（命令见下）退出0，App和Widget链接成功；`/private/tmp/req010-release.log`。仅编译，无签名打包/发布 |

Release 复现命令：

```sh
CLANG_MODULE_CACHE_PATH=/private/tmp/req010-swift/clang-module-cache swift build --disable-sandbox --cache-path /private/tmp/req010-swift/swiftpm-state/cache --config-path /private/tmp/req010-swift/swiftpm-state/config --security-path /private/tmp/req010-swift/swiftpm-state/security --scratch-path /private/tmp/req010-swift/build -c release
```

首次基线因 Sparkle 下载域名解析受限而退出1；复制已有依赖缓存到本任务独立目录后，8项启动基线及后续测试正常通过，未修改缓存来源或跳过测试。上述原生交互通过控件 action 验证，未声称在用户已安装应用执行物理键鼠验收。独立审查以实际原生焦点探针发现：默认 NSTextFieldCell 在结束编辑时也发送 action，导致失焦提前保存，违背仅 Apply/回车提交约定。`01da4da` 关闭结束编辑时发送 action；真实 NSTextView 编辑后失焦保持旧值的 RED 在两种提交方式下记录4个准确失败，修复后22项聚焦测试与门禁通过。回归使用真实 Return NSEvent keyDown 及原生 Apply 动作确认仍可提交。证据：`/private/tmp/req010-focus-red.log`、`/private/tmp/req010-focus-green.log`。独立定向复审确认 I1 关闭，无剩余 Critical/Important；最终分支独立审查 `c9aa58c..f206c5b` 亦通过，0 Critical / 0 Important，保留同一可选 Minor 测试边界。

初审还记录非阻塞测试边界：注入的采集操作不携带 manual 参数，新增调度测试无法单独捕获未来误改为 manual:true。当前生产调用已审查为 manual:false，既有 RefreshCoordinator 回归通过；不声称新增测试已证明完整退避集成链路。

## 修复后最终回归

`01da4da` 后再次先运行文档检查，再用相同独立缓存执行完整 `scripts/test.sh`，退出0：**427项主测试/83套件 + 13项独立PTY = 440项**，所有合同/可移植性/资产标准化/feed/193份文档/公开安全门禁通过。随后使用上述完整命令重建 Release，退出0（6.66秒）。控制者直接核对原始日志，未以先前439项结果代替修复后的最终结果。

- `/private/tmp/req010-focus-full.log`
- `/private/tmp/req010-focus-release.log`

## 完成与回传

2026-09-08 完成开发交付。分支 `codex/macos-refresh-interval`，规格 `7d58936`、实现 `57f8ea3`、失焦修复 `01da4da`、过程证据 `f206c5b`。协调入口最新 `c9aa58c` 已在 `c6a57a2` 同步；相对该基线仅包含 REQ-010 的 macOS 代码、测试和文档，REQ-009 整合记录完整保留。控制者收尾再次检查193份文档及差异格式，台账仅关闭本行，无其他可执行待办；结果回传协调入口核验整合。本开发对话不直接合并 main。

## 交付边界

本次暂不发布、不安装替换用户应用，不改版本、签名、更新源、Release、Windows 设置或真实账户。用户安装包的实际现场编辑仍与测试宿主的原生控件验证区分。REQ-007/009 仍留在未发布内容中，REQ-001 真实 Windows/WSL 验收仍受环境限制。
