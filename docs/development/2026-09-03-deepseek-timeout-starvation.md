# DeepSeek 截止时间线程池饥饿修复

**需求：** `REQ-20260903-010`  
**日期：** 2026-09-03  
**平台：** macOS

## 背景与目标

浮动条位置分支的 macOS PR 门禁连续两次在 `blockedSecretReadTimesOut` 失败。用例要求被阻塞 200 ms 的 Keychain 抽象在 20 ms 后返回超时，且不得使用迟到的 API Key 发起 DeepSeek 请求；现场却等到读取完成后发起请求，并把测试服务器的响应报告为 `transportFailure`。

该问题与显示器位置功能无业务耦合，已独立登记，避免把 CI 韧性修复混入位置需求的验收口径。

## 失败证据与根因

- PR #4 macOS workflow：`33764239141`；
- 首次失败 job：`100677899394`；
- 失败重跑 job：`100679090398`；
- 两次均为同一断言：预期 `timedOut`，实际得到 `transportFailure`，且测试记录到了网络请求。

原实现把阻塞读取和 `Task.sleep` 截止时间都放在 Swift 协作式并发执行器上。高并发门禁中，阻塞的高优先级读取占住执行线程，20 ms 的睡眠任务虽然到期却无法及时恢复；读取在约 200 ms 后返回，先赢得一次性 continuation，迟到凭据因此进入网络层。

## 实现

- 新增 `NonStarvingDeadline`，由独立串行 GCD 队列负责单调时钟截止时间，不依赖 Swift 协作线程池获得执行机会，也不受系统时间调整影响；
- 保留一次性加锁 continuation，确保读取结果和超时只有一个可以恢复调用方；
- DeepSeek 用量采集与 Settings 凭据状态读取共用同一截止时间实现，删除两份容易再次分叉的本地竞速代码；
- 到期后底层读取可以安全完成清理，但其结果不会触发网络请求，也不会改变已经返回的状态。

## 验证

- `blockedSecretReadTimesOut`：200 ms 阻塞读取在约 20 ms 返回 `timedOut`，网络请求计数保持为零；
- `slowRead`：Settings 状态读取在约 20 ms 降级为 unavailable；
- macOS 完整验证：374 项主测试与 12 项独立 PTY 测试通过，共 386 项、72 个测试组；
- 4 份跨平台 fixture、Release feed probe、133 份 Markdown 和公开发布安全门禁通过。

## 安全与隐私

修复不读取或记录 API Key 内容，只改变截止时间的调度来源。迟到凭据不再进入网络层，反而收紧了秘密的使用边界；文档、测试和日志均未写入真实凭据。

## Git 与 CI

- `0ae1cbe`：独立 GCD 单调时钟截止时间与两条 DeepSeek 调用链收敛；
- `4f6c3af`：复审后修正单调时钟术语；
- 独立复审未发现 Critical/Important；
- [PR #4](https://github.com/sljzdotcom/AI-Token-Meter/pull/4) 的 [macOS CI 33766095915](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766095915) 与 [Windows CI 33766096437](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766096437) 通过；
- 合并提交 `c2d2e64` 的 [macOS main CI 33766955625](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766955625) 与 [Windows main CI 33766955622](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766955622) 再次通过。
