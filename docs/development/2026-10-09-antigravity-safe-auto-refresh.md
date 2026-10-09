# Antigravity 安全自动刷新恢复

**需求：** REQ-20261009-001
**基线：** v0.10.7 / build35，`4158fea`
**状态：** 本地实现、独立复审与全量验证完成；未发布。

## 背景

用户提供旧版现场诊断：`appVersion=0.10.6 build=34`、`refreshPaused=true reason=unknown`、额度时间为 2026-10-01 05:18:21 UTC，且阶段/结果未记录。这个摘要不能证明额度用尽或账号退出。旧版没有交互恢复入口。后来 v0.10.7 加入显式登录与单次额度检查，但一次性检查成功也故意保留 `suspended`；这就是升级后仍暂停的直接原因。

当前请求要求在一次登录和一次额度读取均成功后恢复正常周期刷新，并确保无认证时绝不拉起交互登录。工作限定在 macOS 本地源码；没有运行 `agy`，没有访问凭证、Keychain、浏览器或 M4 Max 应用，不发布。

## 官方能力与安全边界

Google Antigravity CLI 官方 [Headless mode 文档](https://antigravity.google/docs/cli/headless/)说明 `agy -p` 以非交互方式单次运行，使用缓存凭证；没有终端且未认证时以 `authentication required` 退出。文档还明确指出 `/usage` 是 CLI 自行处理的只读命令，应该独立作为 `agy -p /usage` 运行，不通过 Agent stream 发送。官方 [Model quotas 文档](https://antigravity.google/docs/cli/commands/usage/)说明 `/usage` 会刷新后端额度信息。当前 runner 以独立进程组启动，stdin 指向 `/dev/null`，stdout/stderr 为管道，不创建 PTY；CLI 版本和响应均在解析前校验。

没有根据 `NO_BROWSER` 等未证实环境变量假设安全。自动路径始终只查询 `/usage`；非成功认证检查转为认证错误并保持暂停，不启动登录 opener。只有用户主动完成官方登录后，AppModel 才调用一次受控查询。

## 实现

- `collectGeminiQuotaOnce()` 只接受 AppModel 在本次登录回执匹配后取得的一次性授权；校验 Gemini provider 和非空额度指标后，先原子保存缓存，再持久化清除 Gemini `suspended`。若暂停状态写入失败，会恢复旧缓存并继续暂停；查询失败、无效快照、提交前取消都不会解除暂停。
- 在恢复入口等待协调器授权前，AppModel 先同步占用本地恢复状态，重复点击不能打开第二个登录流程。协调器提交成功后，即使取消信号随后到达，AppModel 仍完成同一任务的界面同步。
- AppModel 收到有效单次结果后同步清除暂停/需操作展示状态，更新缓存与 Widget，并说明自动刷新已恢复。
- 原有自动刷新调度、用户配置间隔、启动/唤醒路径和进程组停止机制沿用；后续 headless 刷新若认证失败则再次持久暂停。
- 新文案覆盖英文、简体与繁体中文；Windows 代码和行为未更改。
- 将既有包含短时 fake CLI 看门狗的测试 suite 设为串行，避免并行负载延迟启动假进程，导致测试先于 fixture 生成 PID 文件超时。

## 验证

- 定向回归：协调器 20 项、AppModel 恢复 6 项、默认外部动作策略 1 项通过；覆盖一次性授权、失败/无效响应、缓存回滚、取消、并发开始和成功后恢复。
- `scripts/test.sh`：Core 286 项/48 suites、App 322 项/54 suites、Widget 12 项/3 suites、刷新调度 3 项、PTY 18 项全部通过，共 641 项。跨平台合同 6 fixtures、portability/schema/unavailable 配额、Windows 更新包归一化、release-feed、350 个 Markdown 文件和 public-release safety 检查全部通过。
- Release 配置 `swift build --product AIMeterApp -c release` 通过；仅生成本地临时构建产物，不签名或安装。
- 复核期间未运行含真实 `agy` 的采集器；新增/修改 CLI 运行只使用 fake executable，且默认运行环境拒绝登录 opener。
- 独立复审：初审的授权调用顺序、恢复启动竞态、持久化错误处理及提交取消同步问题均已修复；最终复审未发现阻塞项。
- 未进行真实 CLI、账号、Keychain、系统 URL opener、Terminal、浏览器和设备现场测试；没有更新包或公开 Release。

## Git 节点

本地分支：`codex/antigravity-safe-auto-refresh`。实现提交：`6b014a7`（基于 `4158fea`）。仅保留在本地；未推送、合并、打标签或发布。
