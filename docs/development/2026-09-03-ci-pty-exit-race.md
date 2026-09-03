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
- 文档检查：121 份 Markdown 通过；
- 公开发布安全门禁通过；
- [公开 CI 33654906546](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33654906546) 验证 PTY 测试隔离提交通过；[v0.2.1 最终 CI 33655946917](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33655946917) 再次完整通过。
- v0.2.1 正式 ZIP：`AI-Token-Meter-0.2.1-macOS-arm64.zip`，3,467,098 字节，SHA-256 `39334e1ea706e121c30d5cc85720142ff336b0cd696c8eb544336caa7c8724a0`；归档签名验证通过，篡改副本被拒绝。

## 本机与界面验收

本修复没有界面变化。真实服务仍通过既有 Claude Code、OpenAI Codex 采集与账户测试覆盖。

## 安全与隐私

不新增日志、网络请求、环境变量读取或凭据访问。备用等待只观察当前子进程退出，不扫描其他进程。

## Git 节点

- `9f50b78`：加固退出确认与 PTY 尾部排空；
- `a627f0b`：隔离 PTY 系统资源测试，恢复公开 CI；
- `7177530`、`cc30859`：准备并固化 v0.2.1（build 5）发布元数据；
- `v0.2.1`：公开补丁标签与双资产 GitHub Release。

## 已知限制与后续工作

真正的命令超时和系统资源耗尽仍按原错误类型上报；尾部排空继续受字节和时间上限约束，不会无限等待。

## v0.2.2 发布前复发与补充修复

`v0.2.2` 的同一提交先通过 PR CI，但随后 `main` CI `33701394513` 在独立 PTY 测试进程中再次报告 32 路并发输出缺失。重新审计发现，产品 `PTYCommandRunner` 并未出现新的退出或读取错误；问题来自测试 fixture 在每个交互 Shell 中用 `printf | tr` 清理一行输入。32 路测试因此会额外短时派生 64 个子进程，GitHub runner 资源紧张时其中一条 fixture 可异常退出，而旧断言只检查正文、没有检查退出码，表现得像 PTY 丢失输出。

补充修复保持产品代码不变：fixture 直接使用终端规范模式下 `read` 得到的行，不再启动清理管道；并发测试同时断言 32 个 `CommandResult` 都以退出码 0 完成，再检查固定身份输出。修复后聚焦 PTY suite 11/11 通过，并发用例在本机连续 200 轮通过；[PR CI 33702291460](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33702291460) 与 [最终 main CI 33702415007](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33702415007) 均通过。修复提交 `c11da0a` 是 `v0.2.2` 标签目标。

## Windows 合并后的第二次复发

Windows 平台并入 `main` 后，`eea044b` 的 [macOS CI 33744944628](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33744944628) 中 362 项普通测试全部通过，但独立 PTY suite 的 `A parent exit cannot leave the runner waiting on a descendant PTY` 在 2.429 秒后误报 `timedOut`。其 fixture 的 Python 父进程会立即 `os._exit(0)`，只有脱离会话的后代继续持有 PTY；因此等待的对象应是已经退出的父 PID，而不是后代或 PTY EOF。

重新审计发现，负责直接调用 `Process.waitUntilExit()` 的并发 fallback queue 仍固定为 `.utility`。它正是 Foundation termination handler 延迟时的退出事实来源，却可能在 runner 高负载下被 2 秒 timeout task 抢先。新增失败先行的 QoS 合同测试后，把该专用队列提升为 `.userInitiated`，不改变 timeout 数值、信号处理、PTY 尾部排空或后代清理。

修复后：

- 失败先行 QoS 合同测试通过；
- PTY suite 12/12 通过，父进程退出用例约 0.85 秒完成；
- 父进程退出用例连续 20 轮通过；
- 本机完整回归为 362 项普通测试 + 12 项独立 PTY，共 374 项、72 个测试组；共享合同、Release feed、129 份 Markdown 与公开安全门禁通过；
- 新提交的远端 main CI 待补录，未通过前本需求重新保持“进行中”。
