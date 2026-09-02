# GitHub CI PTY 退出与尾部读取竞态修复

## 背景与目标

`v0.2.0` 发布提交在 GitHub macOS runner 的完整 CI 中出现两项 PTY 时序失败。目标是消除高负载环境下的误报超时和输出尾部丢失，同时保持命令超时、取消及并发刷新语义，并以不改写既有公开标签的 `v0.2.1` 补丁发布。

关联需求：`REQ-20260903-001`。

## 影响范围

- `PTYCommandRunner` 的进程退出等待和非阻塞尾部排空；
- `CodexAppServerClient` 复用的进程退出等待器；
- GitHub Actions 的完整 Swift 回归。

## 失败证据

[首次失败 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33653128303) 中：

- `A parent exit cannot leave the runner waiting on a descendant PTY` 在 2 秒边界误报 `timedOut`；
- `Concurrent terminal commands do not lose their output` 有结果缺少预期输出尾部；
- Homebrew tap trust 文字只是 runner 警告，`gitleaks` 安装成功，真正退出码来自测试步骤。

## 实现与关键决定

1. 保留 Foundation `terminationHandler` 快路径，同时在 `Process.run()` 成功后从独立并发 Dispatch 队列调用 `waitUntilExit()` 作为退出确认；Swift cooperative executor 不再承担阻塞等待。
2. 两条完成路径通过同一把锁和单次完成门合并，continuation 与阻塞等待信号只完成一次。
3. PTY 收到停止请求后，短暂的 `EAGAIN` 不再因为已经读到任意字节就立即结束；继续排空到 EOF、字节上限或既有 750ms 截止时间，避免终端回显被误当成最终输出。
4. `openpty` 分配锁、命令运行并发、超时终止和敏感环境白名单保持不变。
5. 默认完整测试入口把 PTY suite 放入独立测试进程；聚焦测试仍按传入参数单次执行。这保留全部覆盖，同时避免 GitHub runner 同时进行截图、文件扫描、子进程和 PTY 测试造成的系统级抢占噪声。

## 自动化验证

- PTY 聚焦测试：11/11 通过；
- 同一聚焦测试连续 10 轮压力复验：10/10 通过；
- 完整回归：361 项测试、70 个测试组通过；默认入口把 350 项普通测试与 11 项 PTY 系统资源测试分成两个进程；
- 文档检查：120 份 Markdown 通过；
- 公开发布安全门禁通过；
- 公开 CI 结果在完成后回填。
- v0.2.1 正式 ZIP：`AI-Token-Meter-0.2.1-macOS-arm64.zip`，3,467,098 字节，SHA-256 `39334e1ea706e121c30d5cc85720142ff336b0cd696c8eb544336caa7c8724a0`；归档签名验证通过，篡改副本被拒绝。

## 本机与界面验收

本修复没有界面变化。真实服务仍通过既有 Claude Code、OpenAI Codex 采集与账户测试覆盖。

## 安全与隐私

不新增日志、网络请求、环境变量读取或凭据访问。备用等待只观察当前子进程退出，不扫描其他进程。

## Git 节点

- 实现提交和最终成功 CI 将在验证完成后回填。

## 已知限制与后续工作

真正的命令超时和系统资源耗尽仍按原错误类型上报；尾部排空继续受字节和时间上限约束，不会无限等待。
