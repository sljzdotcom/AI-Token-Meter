# 2026-09-08：Windows 登录成功后额度无法显示的调查

关联 REQ-20260908-001；基线 main `3dfd4ae` / 0.5.0。最初仅做静态调查；收到用户路径证据后进入隔离分支 `codex/windows-cli-recovery` 实施，尚未发布修复、未在用户 Windows 上执行命令。

## 用户证据

四张截图分别显示：Codex 终端登录成功；Codex 详情“未安装”；Claude 详情“需要设置”；Settings 已读到 Claude 原生 CLI 2.1.263 和已登录账号，但 Codex 账户状态暂不可用。公开记录不复制邮箱、授权 URL、Key 尾号或用户截图。

登录、CLI 发现与额度采集是不同链路。不能把这些截图解释为两个账号均未登录，也不能根据 Settings 的身份成功认定额度采集应当成功。

## 已确认的代码事实

1. `windows/src-tauri/src/collectors/application.rs` 的 `locate_with_cancellation` 返回 `Option`，进程启动失败、版本探测超时、WSL 失败等都丢失为 None；`build_requests` 随后统一生成 NotInstalled。而 `accounts/windows_service.rs` 使用 `accounts/cli_discovery.rs` 的 Found/Missing/Unavailable 三态。这解释了同一次 CLI 检测故障可在 Settings 显示“状态暂不可用”、详情显示“未安装”，但尚不能证明具体机器的失败阶段。
2. `platform/windows/process.rs` 的两条启动路径都清理环境，并把 PATH 设为实际启动可执行文件的父目录。对 `codex.cmd`，实际可执行文件是 cmd.exe，因此 PATH 只剩 Windows System32。若 npm 包装脚本依赖其他目录的 node.exe，会发生“交互终端可用，App 探测失败”。是否就是用户当前安装方式，仍需 Windows 路径证据；不能把假设写成已复现的机器根因。
3. Windows Claude 采集使用 USERPROFILE 为工作目录，先执行 auth status，再用 ConPTY 启动 `--ax-screen-reader --safe-mode` 并发送 `/usage`。命中 `Permission Required: Accessing workspace` 会返回 SetupRequired；刷新退避也可能保留旧设置状态，所以截图不足以证明最新一次确切输出。
4. Windows 没有 macOS `ClaudeUsageWorkspace` / `ClaudeSetupScriptBuilder` 对应的隔离工作区与用户初始化入口。现有 Claude Windows 真实 ConPTY 测试使用自制 Node fixture；它不经历真实 CLI 工作区权限流程。现有命令构造测试也不能代替 npm shim 在独立 Node 目录下的 Windows 进程执行验证。

## 下一步证据与安全边界

维护环境是 macOS，没有用户那台 Windows 的远程执行接口。此前请求以下只读输出确认 Native/npm/WSL 与 Node 所在目录，用户已经提供，不必再次索取：

```powershell
Get-Command codex,claude,node -All | Select-Object Name,CommandType,Source
where.exe codex
where.exe node
codex --version
```

不要求再次登录、不读取 auth.json/凭据存储、不打印完整环境或授权 URL、不让用户信任整个主目录。路径中的用户名可自行遮去，保留目录层级。随后针对实际入口补失败测试，统一检测分类与启动依赖，并单独验证 Claude 的工作区初始化路径；运行时环境修改不能顺便放开任意 provider override、权限或自动信任。

## 2026-09-08 补充证据与实施

用户确认 `codex-cli 0.153.4`，Codex 的 `.ps1` / `.cmd` / 扩展名为空的包装器位于用户 npm 目录；Node 位于独立的 `Program Files/nodejs` 目录；Claude 为用户本地 bin 下的原生 exe。这与代码中的受限 PATH 缺少 Node 条件相符，不是 CLI 未安装。用户的终端成功输出并不等于已运行新版应用验证。

推荐方案已记录为[规格](../design/specifications/2026-09-08-windows-cli-recovery-design.md)与[实施计划](../design/implementation-plans/2026-09-08-windows-cli-recovery.md)，设计提交 `ef9cc35`。三部分依次为显式 npm/Node 启动、统一发现失败分类、Claude 隔离工作区初始化。基线的 executable_locator、cli_discovery、claude_collector 定向测试通过；它们尚未覆盖此次新增回归。

## 状态

### 分段证据

- Task 1：`65d2b19`，标准官方 npm Codex 包验证后使用显式 Node + JS 入口；保留受限环境。16 项定位、5 项发现、9 项进程、3 项安装策略测试通过，严格 Clippy/格式检查通过；独立规格与质量审查无阻塞。新增 Windows-only 真实 Node/进程用例待原生 CI 运行；macOS 交叉编译缺少 MSVC `assert.h`，不算 Windows 验证通过。
- Task 2：`c9af3c3`，账户、额度、登录与自定义路径验证复用统一有界发现执行器。Missing/Unavailable/Cancelled 分开，采集仅把 Missing 映射成未安装。11 项定向回归及本机 Rust 全套通过，严格 Clippy/格式检查通过；独立审查无阻塞。
- Task 3：`b68a020`，原生与 WSL 的认证/用量/初始化共用隔离工作区；显式按钮和中英文提示接入，自动检查不重试额度，手动重试保留 rateLimited 等待。本机 Rust 全套、严格 Clippy、85 项前端与生产构建通过；独立审查无阻塞。真实用户初始化/账号额度仍待现场验收。
- [草稿 PR #12](https://github.com/sljzdotcom/AI-Token-Meter/pull/12) 已启动原生 Windows CI；仍为实施中的草稿，不是发布或用户真机通过声明。
- 初轮原生 Windows CI `34183253409`：前端、密度、构建、严格 Rust 编译检查通过；npm 定位 16 项通过。真实 Node 入口已成功执行，但 process_runner 的 PATH 字符串比较把同一目录的 Windows 长路径/8.3 短路径当成不同路径（10 通过/1 失败）。登记 REQ-20260908-003，修正路径断言后必须重新跑原生 CI，不能称该轮全套通过。
- 路径回归修正 `3955bb2`：完整 PATH 与期望目录规范化后严格相等，保留单目录约束及入口/参数/退出码断言。本机 process_runner 9 项通过，原生 CI 仍需复跑。
- Task 4：`06b5fe4`，appcast 测试验证每条发布元数据及官方版本对应下载地址，不再固定 0.2.2；5 类损坏数据用例在朴素字段存在性检查变异下均失败，正确契约下通过。本机完整 Swift 420 + PTY 13、全部脚本尾部验证通过。签名仅在该单元测试检查 Base64/64 字节格式，真正资产签名验证仍由既有发布门禁负责。
- 根任务复跑 Windows 侧本机测试：Rust 220、严格 Clippy/格式、前端 85、生产构建、21 项浏览器生命周期与 632 个文字角色计算样式全部通过。这是 macOS 宿主上的 Windows 代码验证；原生 Windows 复跑与用户账号现场验收分别追踪。
- 整分支审查发现 REQ-20260908-004：自定义 npm 路径验证成功后，把实际 JS 启动目标存入设置；后续发现因只接受包装器/原生入口而拒绝。必须保留用户选择的规范化包装器路径，与实际启动目标分离，补保存→重载→再次发现回归。合并前处理，不以分段测试通过掩盖集成问题。
- `ceec724` 的 macOS 原生 CI `34184562223` 已通过；Windows 原生 CI `34184562214` 仍在运行，自定义路径修正后仍需验证新提交。
- 该轮 Windows CI 随后在严格 Clippy 因初始化命令 Windows 分支的多余 return 失败，尚未进入运行测试；归 REQ-20260908-005。与自定义路径问题放在同一最终修复轮次，保留 lint 门禁和返回错误语义，不使用忽略规则。
- 完整 Swift 基线发现既有 `stableAppcastContract` 固定断言 0.2.2，更新源滚动为 0.5.0 后三条断言失败；归入 REQ-20260908-002，不能称全套通过。不是本次 Windows 代码引入，也不会通过跳过测试处理。

### 最终修复与复验

`d3c57f2` 保留 CLI 的两种不同身份：设置持久化用户选择的规范化包装器路径，受限进程实际执行经过官方包验证的 JS 入口及独立 Node。没有扩大为任意 JS 文件执行。新增 `saved_official_codex_wrapper_is_rediscovered_after_settings_reload` 覆盖设置序列化、重载和再次发现。

该测试先因缺少独立配置路径接口编译失败，随后额外做行为变异验证：临时恢复旧逻辑、保存 JS 执行目标时，测试在 `rediscovered npm candidate` 处失败；恢复保存包装器后同一测试通过。错误实现未提交。

同一提交将 Windows 初始化命令的多余 `return` 改为等价尾表达式，没有忽略 Clippy；Swift appcast 负例补入合法 Base64 但解码仅 5 字节的签名，继续要求 64 字节格式。该格式断言不替代真正的发布签名验证。

根任务在修复后重跑本机完整 Rust：221 项通过，严格 Clippy、格式检查通过；完整 macOS 420 项主测试、13 项独立 PTY、跨平台合同、发布资产/更新源脚本、180 份文档和公开安全检查通过。原生 Windows 与 macOS 的精确提交复验分别见 [Windows CI 34185344676](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34185344676) 与 [macOS CI 34185344720](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34185344720)，当前仍待完成。

最终定向复审已逐项确认自定义路径、Windows 条件分支与签名负例修复，无新增 Critical/Important/Minor。根任务再次验证前端 85 项、生产构建、21 项浏览器生命周期和 632 个文字角色计算样式均通过。临时 SDD 报告归档到本机临时目录，不作为第二文档体系；持久证据以本日志、设计/计划、需求台账和 Git 提交为准。

### 原生 CI 终验（2026-09-08）

精确实现提交 `d3c57f2` 的上述两轮 CI 均成功。Windows 原生严格 Clippy、229 项 Rust 测试、85 项前端、21 项浏览器生命周期、632 个文字角色计算样式、NSIS debug 安装包构建、GUI subsystem 检查及产物上传全部通过。两个关键用例 `separated_official_npm_entry_runs_with_node_only_path` 与 `saved_official_codex_wrapper_is_rediscovered_after_settings_reload` 在真实 Windows runner 中通过。

macOS CI 完整验证成功，与本机 420 + 13 项及合同/文档/公开安全门禁结果一致。临时 CI 安装包不等于公开签名 Release，也未接入应用更新源。

实现与自动化验收完成，[PR #12](https://github.com/sljzdotcom/AI-Token-Meter/pull/12) 已按既有授权合入 main，合并提交 `aca64fc`。REQ-20260908-002 至 005 已完成；001 保留受环境限制：没有用户 Windows 远程会话，未操作真实账号或工作区信任，需新版交付后完成现场额度验收。本轮未修改版本号、签名或更新源，公开版仍是 0.5.0。
