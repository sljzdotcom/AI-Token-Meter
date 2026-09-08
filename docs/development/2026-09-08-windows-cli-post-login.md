# 2026-09-08：Windows 登录成功后额度无法显示的调查

关联 REQ-20260908-001；基线 main `3dfd4ae` / 0.5.0。当前只完成静态链路调查，未修改生产代码、未发布修复、未在用户 Windows 上执行命令。

## 用户证据

四张截图分别显示：Codex 终端登录成功；Codex 详情“未安装”；Claude 详情“需要设置”；Settings 已读到 Claude 原生 CLI 2.1.263 和已登录账号，但 Codex 账户状态暂不可用。公开记录不复制邮箱、授权 URL、Key 尾号或用户截图。

登录、CLI 发现与额度采集是不同链路。不能把这些截图解释为两个账号均未登录，也不能根据 Settings 的身份成功认定额度采集应当成功。

## 已确认的代码事实

1. `windows/src-tauri/src/collectors/application.rs` 的 `locate_with_cancellation` 返回 `Option`，进程启动失败、版本探测超时、WSL 失败等都丢失为 None；`build_requests` 随后统一生成 NotInstalled。而 `accounts/windows_service.rs` 使用 `accounts/cli_discovery.rs` 的 Found/Missing/Unavailable 三态。这解释了同一次 CLI 检测故障可在 Settings 显示“状态暂不可用”、详情显示“未安装”，但尚不能证明具体机器的失败阶段。
2. `platform/windows/process.rs` 的两条启动路径都清理环境，并把 PATH 设为实际启动可执行文件的父目录。对 `codex.cmd`，实际可执行文件是 cmd.exe，因此 PATH 只剩 Windows System32。若 npm 包装脚本依赖其他目录的 node.exe，会发生“交互终端可用，App 探测失败”。是否就是用户当前安装方式，仍需 Windows 路径证据；不能把假设写成已复现的机器根因。
3. Windows Claude 采集使用 USERPROFILE 为工作目录，先执行 auth status，再用 ConPTY 启动 `--ax-screen-reader --safe-mode` 并发送 `/usage`。命中 `Permission Required: Accessing workspace` 会返回 SetupRequired；刷新退避也可能保留旧设置状态，所以截图不足以证明最新一次确切输出。
4. Windows 没有 macOS `ClaudeUsageWorkspace` / `ClaudeSetupScriptBuilder` 对应的隔离工作区与用户初始化入口。现有 Claude Windows 真实 ConPTY 测试使用自制 Node fixture；它不经历真实 CLI 工作区权限流程。现有命令构造测试也不能代替 npm shim 在独立 Node 目录下的 Windows 进程执行验证。

## 下一步证据与安全边界

维护环境是 macOS，没有用户那台 Windows 的远程执行接口。请仅收集发生问题机器中以下只读输出，先确认 Native/npm/WSL 与 Node 所在目录：

```powershell
Get-Command codex,claude,node -All | Select-Object Name,CommandType,Source
where.exe codex
where.exe node
codex --version
```

不要求再次登录、不读取 auth.json/凭据存储、不打印完整环境或授权 URL、不让用户信任整个主目录。路径中的用户名可自行遮去，保留目录层级。随后针对实际入口补失败测试，统一检测分类与启动依赖，并单独验证 Claude 的工作区初始化路径；运行时环境修改不能顺便放开任意 provider override、权限或自动信任。

## 状态

受环境限制：等待 Windows 路径/版本证据后继续验证根因。该记录不是修复完成或真机通过声明。既有自动化和 0.5.0 发布通过记录仍保留，同时明确其未覆盖本次真实安装方式/初始化路径。
