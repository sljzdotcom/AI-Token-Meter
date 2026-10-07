# Antigravity 一次性受控恢复入口

**需求：** REQ-20261001-002  
**基线：** v0.10.6 / build34；本地产品修复从 0.10.6 源码基线开始
**状态：** macOS实现与自动化验证已完成，正在准备v0.10.7/build35双平台发布；未运行真实 `agy`、未登录或改动已安装应用及现场暂停状态。原 M4 Max 现场问题仍待用户确认。

## 背景与目标

用户再次提供的本机诊断仍为 `refreshPaused=true reason=unknown`、`lastQuotaAt=2026-10-01T05:18:21Z`、`stage/result=unknown`。这与 0.10.6 的持久暂停设计一致：旧版本迁移时原因缺失会保留为 `unknown`，该版本没有恢复入口，诊断环为空就不会编造阶段。它不证明账号已登出、额度耗尽或发生了新的失败。

本次已批准产品修复为未知/认证及其他持久暂停添加同一个用户主动入口。官方登录成功回执最多启动一轮只读额度查询；成功也保留持久暂停，避免回到无限周期重试。实现只在本地分支，不发布。

## 实现范围与决定

- 登录脚本将本次 `agy` 置于独立进程组，使用 300 秒看门狗；只对该组发出 TERM/KILL，并等待整个进程组结束，避免 CLI 主进程退出后遗留子进程。成功、非零退出、超时和终端关闭分别回送含 UUID token 和退出码的固定回执。UUID 只在内存和短暂的 0600 FIFO 中传递，不写入登录脚本。
- 回归审查发现 FIFO 的重定向会在Shell打开动作中先于 `read -t` 无限期等待 writer。现改为用读写模式非阻塞打开 FIFO，再由有界 `read -t` 等待；缺失写入者时返回超时并移除 FIFO，不启动 CLI。
- AppModel 使用内存 token、330 秒 deadline 和单次状态机。错误、取消、超时、过期或伪造回执均不会调用额度查询。`stop()` 会使 token 失效并取消任务。
- 独立 collector 路径只运行版本检查和 `/usage`；不运行 `/model`、`models`、全局刷新或其他 provider 命令。成功解析后只原子替换 Gemini 缓存并保留其他 provider 数据，不清除持久 `suspended` 记录。并发的普通 provider 刷新不会把较早缓存覆盖到磁盘或 UI/Widget。失败保留原缓存。
- Services 和 Antigravity 详情在所有持久暂停原因下显示“一次性检查额度”；英文、简体和繁体均说明 Terminal 登录、可能出现浏览器授权，以及成功后自动刷新仍暂停。界面不声称能阻止 CLI 打开浏览器。
- 登录结果和 collector 阶段继续写入脱敏的诊断白名单。无日志的旧暂停仍显示 unknown。
- Windows Rust 行为与跨平台 JSON 合同未修改。

## 验证证据

- 最终恢复相关定向回归：37 项通过，其中登录脚本/进程组/回执、协调器与 AppModel 刷新竞态均覆盖；包含 TERM-resistant 子进程、组长先退出、内存/磁盘缓存竞争。
- 本次最终完整 Swift suite：Core 303 项、App 321 项、Widget 12 项，共 636 项通过；其中3项调度与18项PTY runner测试按仓库流程独立串行执行。真实CLI/Keychain及UI宿主能力条件测试按设计跳过。
- `AIMeterAppTests.AppLocalizationTests/caseSensitiveResourceLookup` 在 `/private/tmp` 隔离目录内创建并清理临时 HFSX 映像，单项通过；同一最终源码上的完整 Swift suite 也包含此用例并通过。没有修改物理磁盘或系统保护设置。
- `scripts/check-docs.sh`：348 个 Markdown 文件通过；跨平台合同 6 fixtures、Windows 资产归一化、release feed 与公共发布安全检查通过；`bash -n` 和 `git diff --check` 通过。
- 独立代码复审发现并推动修复三类竞态/生命周期问题（超时后残留进程组、较早普通刷新覆盖新缓存或界面额度），以及启动失败遗留 FIFO 的清理问题；对应回归和最终定向复审均通过。
- 初始实现 Git 检查点：`a2de0f3`（`feat: add one-time Antigravity recovery`）；发布候选还包含无界FIFO等待修复和回归测试。当前恢复功能只在发布候选中，设备诊断仍明确显示0.10.6/build34；现场恢复与自动刷新恢复不作保证。
- 未运行真实登录、CLI、浏览器、Keychain、生产应用或 OS 隔离探针；没有现场验证或可更新安装包。

## 未完成边界

此本地提交不会改变现有安装版。用户提供的摘要明确显示运行版本仍是 0.10.6/build34；这次修复只在本地源码分支，尚未打包或安装，所以该版本仍显示旧的“已暂停”。而且新实现也刻意保留持久暂停：它不把 `refreshPaused` 改成 false；只有成功的一次性检查会写入 `stage=login/usage` 及更新 `lastQuotaAt`。当前摘要中的 `reason=unknown` 和无阶段记录表示旧诊断没有保存原因，且尚无这次新实现的阶段记录；它不证明账号已登出或额度耗尽。未验证真实 M4 Max 额度恢复。
