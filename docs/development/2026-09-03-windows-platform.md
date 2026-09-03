# Windows 平台开发与验收日志

- **需求：** `REQ-20260903-004`
- **分支：** `codex/windows-platform`
- **规格：** [Windows 平台设计](../design/specifications/2026-09-03-windows-platform-design.md)
- **计划：** [Windows 平台实现计划](../design/implementation-plans/2026-09-03-windows-platform.md)
- **当前状态：** 进行中

## 背景与目标

在不重写已稳定 macOS SwiftUI/AppKit 应用的前提下，新增 Windows 11 x64 Tauri 2 + Rust + React 应用，并让两个平台共享版本、数据语义、展示身份、脱敏解析样本、功能对等矩阵和正式 Release 门禁。

Windows 首版包括系统托盘、左右贴边浮动条、Claude Code/OpenAI Codex/DeepSeek、详情、Settings、账户恢复和手动签名更新。Windows Widget 不进入首版。

## 2026-09-03：规格、计划和隔离环境

- 用户确认方案 A：保留 macOS 原生应用，在同仓库新增 Windows Tauri 应用。
- 正式规格覆盖原生/WSL CLI、ConPTY、Credential Manager、WebView2、窗口层级、Updater、NSIS、CI 和同版本 Release。
- 在 `codex/windows-platform` 分支及 `.worktrees/windows-platform` 隔离工作区开发。
- 开发前基线通过：351 项普通测试、11 项独立 PTY 测试、126 份 Markdown 文档检查和公开发布安全门禁。
- 实施计划检查点：`20b53bf`。

## Phase 1：共享产品合同

### 失败证据

先新增 `CrossPlatformContractTests`；首次运行时三个测试分别因 `VERSION`、`contracts/presentation/providers.json` 和 `contracts/fixtures/` 不存在而失败。失败原因与预期一致，未以语法错误代替行为红灯。

### 实现

- 根 `VERSION` 固定为当前稳定版 `0.2.2`，与 macOS Info.plist 同步；
- JSON Schema 定义 Provider、状态、指标、重置券、本机活动和 DeepSeek 每日历史；
- 展示合同固定 Claude Code、OpenAI Codex、DeepSeek 的顺序、名称、Logo 键、品牌色和进度语义；
- DeepSeek 圆环语义明确为 `consumedFromBalanceBaseline`；
- 四份脱敏 fixture 覆盖正常、重置券、余额和登录失效；
- 对等矩阵记录 macOS 已有证据、Windows 计划状态和 macOS-only Widget；
- Ruby 门禁检查版本、合同字段、比例、时间、证据路径和常见凭据/手机号泄露模式。

### 验证

- 新增 3 项 Swift 合同测试通过；
- 共享合同节点完整 macOS 回归：354 项普通测试 + 11 项 PTY 测试，共 365 项、72 个测试组通过；
- 4 份 fixture 合同门禁、127 份 Markdown、公开发布安全检查通过；
- Git 检查点：`a2b087d`。

## Phase 1：Windows Tauri 骨架

### 工具链与依赖

- Node `26.7.0`、npm `11.19.0`；
- Rust stable `1.98.0`，使用 minimal profile，并补充 rustfmt、clippy；
- npm 官方 registry 查询并精确锁定 Tauri API/CLI、React、TypeScript、Vite、Vitest 与 Testing Library；
- npm 安装审计为 0 个已知漏洞；Cargo 依赖写入 `Cargo.lock`。

### 失败证据

- 前端首次测试因 `src/App.tsx` 不存在而失败；
- 实现最小 App 后，第二项测试暴露 Vitest 未自动 cleanup，修复测试基础设施后通过；
- 首次 TypeScript 构建暴露 TypeScript 7 的 CSS side-effect import 声明和 Vite/Vitest 配置类型问题，按工具链要求修正；
- Rust 首次测试因 `src/lib.rs` 不存在而失败；
- 最小 library 完成后首次编译暴露 Tauri 编译期必需应用图标，随后使用项目既有 Quantum Dial 生成器输出 Windows ICO/PNG。

### 当前实现与验证

- React 壳层从根级展示合同读取三个正式 Provider 名称；
- 采集器尚未连接时只显示 `Unavailable`，不伪造 `0%`；
- Tauri capability 目前仅为 `core:default`，未启用通用 shell；
- Rust `app_metadata()` 从根级版本和展示合同生成元数据；
- 前端 2 项测试、TypeScript 生产构建、Rust 测试、rustfmt、零警告 clippy 均通过；
- 完整 `tauri build` 已在 macOS 生成 release 二进制，证明前后端配置连通；这不是 Windows 真机构建证据。

### 构建目录引出的门禁回归

安装 npm/Rust 依赖后，既有门禁暴露了两个范围缺陷：

1. 文档检查器遍历 `windows/node_modules` 的第三方 Markdown；
2. Gitleaks 直接扫描链接工作树目录，因 Rust `target` 二进制中的上游键盘文案误报 generic API key，单次无效扫描约 710 MB。

根因不是源码含有凭据：`git check-ignore` 和 `git ls-files` 均证明 `node_modules`/`target` 不属于公开工作树；直接 Gitleaks 证据将三个命中定位到 `target/**/libmuda*.rmeta`。修复分别为：

- 文档检查器按路径组件排除 `node_modules`；
- Gitleaks 工作树扫描改为先从 `git ls-files -co --exclude-standard` 构造临时、无符号链接的公开候选快照，再扫描该快照；Git 全历史仍由 `gitleaks git` 独立扫描。

两个修复均先加入失败测试，红灯分别准确显示第三方坏链和直接仓库扫描，然后转绿；未降低已追踪/未忽略源码与 Git 历史的安全检查强度。

加入两项门禁回归后的完整结果为 356 项普通测试 + 11 项 PTY 测试，共 367 项、72 个测试组通过；128 份 Markdown、共享合同和公开发布安全门禁通过。

## Phase 2：Windows 领域、缓存、设置与脱敏

### 失败证据

- 先加入共享 fixture、未知 Schema、比例越界、设置/缓存原子写入和敏感日志测试；首次编译按预期因 `domain`、`persistence`、`security` 模块不存在而失败；
- 第一轮最小实现暴露未知 Schema 的版本号没有传入安全降级辅助函数，编译失败后只修正该数据流；
- 对照正式计划补入旧缓存、未知字段、过期时间、共享品牌合同、损坏 JSON、Cookie 和 Windows 数据目录测试；新增断言分别先因缺少展示合同、过期 API、分类错误和路径模型而失败，再进入实现。

### 实现

- `UsageSnapshot`、Provider、状态、指标、重置券、本机活动和每日历史均使用与共享 JSON Schema 对齐的强类型 Rust 模型；
- `Ratio` 只接受 `0...1` 的有限数值，越界值不会被静默钳制；未知主 Schema 只保留身份和时间元数据，并强制降级为不显示数值的 `Unavailable`；
- Provider 名称、Logo、品牌色和 DeepSeek 圆环语义直接从嵌入的根级展示合同读取，不在 Windows UI 重复维护；
- Settings 使用 `%APPDATA%\AI Token Meter\settings.json`，缓存使用 `%LOCALAPPDATA%\AI Token Meter\cache\`；
- JSON 先完整序列化，再写同目录临时文件、flush/sync 并替换目标；Windows 使用 `MoveFileExW` 的 replace + write-through 语义，失败会清理临时文件且不改动旧内容；
- 持久化错误分为 serialize、decode、I/O、invalidPath 和 missingEnvironment，便于 UI 给出可恢复提示；
- 日志脱敏覆盖 API Key、Bearer、Cookie、邮箱、中国大陆手机号和 Windows 用户目录，同时保留普通诊断文本。

### 验证

- Rust 16 项测试全部通过：1 项元数据、7 项共享领域、5 项持久化、3 项脱敏；
- `cargo fmt --check`、完整 `cargo test` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过；
- macOS 完整回归为 357 项普通测试 + 11 项独立 PTY 测试，共 368 项、72 个测试组；前端 2 项测试与生产构建、共享合同、128 份 Markdown 和公开发布安全门禁同时通过；
- Windows 专属 `MoveFileExW` 分支仍需随后由 Windows GitHub runner 编译和真机验证，当前未用 macOS 结果冒充 Windows 证据。

### 验证命令勘误

实施计划原先将 `scripts/check-public-release.sh .` 写成仓库扫描示例，但该脚本的位置参数代表 Release ZIP，因此会正确报告“归档不存在”。新增 Swift 文档门禁测试先复现此误导，随后把计划修正为 `scripts/check-public-release.sh --repository .`，并让文档检查器持续拒绝歧义写法；聚焦测试、完整回归和正确的公开仓库扫描均通过。此项由 `REQ-20260903-006` 独立留档。

## Phase 2：DeepSeek 凭据与余额采集

### 失败证据

- 先写 Fake Credential Store 与回环 HTTP server 集成测试；补齐测试运行依赖后，首次有效编译因 `accounts`、`collectors`、Credential Store 和 Secret 类型不存在而失败；
- 测试覆盖无 Key、401、超时缓存、候选 Key 成功替换、验证失败保留旧 Key、凭据写入中断回滚、响应上限和任意网络来源拒绝；
- 沙箱内不允许绑定回环端口，故网络测试使用受控的本机临时端口权限运行；没有访问真实 DeepSeek 账号或 Key。

### 实现

- `CredentialStore` 只暴露 read、replace verified 和 delete；`SecretString` 使用 `zeroize` 容器，Debug 永远只输出 `[REDACTED]`；
- DeepSeek 正式客户端固定 `https://api.deepseek.com/user/balance`，禁止重定向，使用 10 秒总超时和 64 KiB 流式响应上限；调试测试只额外允许 `/user/balance` 的回环 HTTP 地址；
- HTTPS 使用 Windows 原生 Schannel/系统证书栈，避免把自带 CA 或跨平台 C 加密后端引入 Windows 包；
- 401 映射为 `AuthenticationRequired`，缺 Key 映射为 `SetupRequired`，超时或可恢复网络失败保留并明确标记缓存；
- DeepSeek 圆环仍以用户基准余额计算已消耗比例，默认 ¥100，余额和本地基准指标保持分离；
- 新 Key 先通过官方余额调用验证，再写 Windows Credential Manager；写入失败会恢复旧 Key 或删除首次失败的残留；所有错误是固定安全文案；
- Windows Generic Credential target 固定为 `AI Token Meter/DeepSeek API Key`，测试使用进程/时间隔离 target 并在结束前删除。

### 验证与环境边界

- 当前 Rust 全套 24 项通过，其中 DeepSeek 行为/安全测试 8 项；格式和零警告 Clippy 通过；
- Credential Manager 生产源文件通过 `x86_64-pc-windows-msvc` 最小外壳编译，确认 Win32 API 类型和调用签名有效；
- 完整 Tauri Windows 交叉检查已越过 Rust 网络栈，因 macOS 缺少 Windows Resource Compiler `llvm-rc` 停止；Credential Manager 真正读写测试必须由后续 Windows CI/真机执行，当前不标记为运行时验收通过。

## Phase 2：原生 Windows 与 WSL CLI 发现

### 失败证据与合同边界修复

- 先加入候选优先级、无效文件、Node shebang、CMD 包装器、UTF-16 WSL 列表和参数隔离测试；首次编译按预期因账户类型、环境输入、发现器和 WSL 模块不存在而失败；
- 实施计划原拟把 `windows-cli-locations.json` 直接放进 `contracts/fixtures/`，会被“该目录恰好四份用量快照”的合同门禁正确拒绝；辅助发现 fixture 改放 `contracts/fixtures/auxiliary/`，既保留四份用量快照的严格约束，也能版本化非用量测试数据；
- 复核首轮绿灯时发现只实现了解码器，尚未接入 `wsl.exe --list --quiet` 和原生优先/WSL 兜底顺序；新增失败测试准确暴露两个缺失 API 后再补齐，没有把局部实现标为完成。

### 实现

- 候选顺序固定为：已验证自定义路径、当前进程 PATH、用户注册表 PATH、系统注册表 PATH、npm/nvm/fnm/Volta 约定、桌面应用目录、WSL；
- Windows 环境采集读取 HKCU/HKLM PATH，并覆盖 `%APPDATA%\\npm`、`.volta\\bin`、`NVM_HOME`、`NVM_SYMLINK`、`FNM_MULTISHELL_PATH` 与 fnm 会话目录，不写死用户名或 Node 版本；
- 只接受名称匹配的普通文件；目录、错误名称、符号链接循环、缺少运行时或健康检查失败的候选会被跳过；
- `.cmd` 必须由已验证的 `%SystemRoot%\\System32\\cmd.exe` 启动；`#!/usr/bin/env node` 脚本必须在同一安装树找到明确的 `node.exe`，不依赖 Finder/GUI 进程 PATH；
- WSL 列表调用固定为 `wsl.exe --list --quiet`，支持 UTF-8/UTF-16LE 和 NUL/BOM 清理；发行版及 Provider 参数始终分开传递，不拼接 shell 字符串；
- 原生候选始终优先；WSL 只在原生候选均无效时参与选择，运行时来源和发行版保留在强类型结果中，后续不会默默合并账户或活动。

### 验证与环境边界

- 8 项发现器测试通过，完整 Rust 套件共 32 项通过；`cargo fmt --check` 和零警告 Clippy 通过；
- 前端 2 项测试与生产构建、四份共享用量 fixture、128 份 Markdown 和公开发布安全门禁通过；辅助 fixture 不再进入四份用量快照计数；
- 环境、发现器、WSL 和既有 Credential Manager 的同一生产源文件通过 `x86_64-pc-windows-msvc` 最小编译外壳；
- 当前阶段只生成结构化进程调用并注入健康检查，真正的超时、进程树回收、`wsl.exe` 往返和 Windows 注册表运行时证据属于下一任务及 Windows runner，仍明确保留为未验收。

## Phase 2：受限进程、ConPTY 与登录入口（进行中）

### 失败证据

- 固定 Node 测试夹具先覆盖参数元字符、输出洪泛、睡眠、UTF-16/ANSI、工作目录与最小环境；首次编译按预期因进程、ConPTY 和登录模块不存在而失败；
- 首轮 Windows 目标编译暴露 `HPCON` 在绑定中是整数句柄而非裸指针，按实际 Win32 类型修正；
- 加入输入/输出后，Windows 目标编译进一步暴露 `ReadFile` / `WriteFile` 需要显式 `Win32_System_IO` feature，补齐最小 API feature 后通过；
- 测试夹具最初使用 CommonJS `require`，但 Windows 前端包声明 ESM；有效运行因此统一失败，改为 Node ESM import 后恢复，未掩盖成 runner 缺陷。

### 当前实现

- 非交互 runner 只接收显式 executable 与 argument 数组，不经过 PowerShell 或 shell；工作目录和环境均显式设置，Windows 仅继承 SystemRoot/WINDIR/TEMP/TMP，并把所选 executable 目录作为子进程 PATH；
- stdout/stderr 由有界队列并发排空，总输出默认限制 64 KiB；超限、超时、取消、启动和进程控制错误只返回固定分类，错误 Debug 不含原始输出；
- Windows 子进程进入带 `KILL_ON_JOB_CLOSE` 的 Job Object，父进程完成、超时或取消都会终止残留进程树；
- ANSI 控制序列在内存中清理，UTF-8/UTF-16LE 输出统一后再交给 parser；原始输出不进入错误对象；
- ConPTY 使用 `CreatePseudoConsole` 创建默认 120×40 终端，支持 resize、最大 4 KiB 固定输入、限时/限量 pattern 等待；进程以 `STARTUPINFOEXW` 附加伪终端，先 suspended 创建并加入 Job Object，再恢复主线程，消除子进程树逃逸窗口；
- Windows 命令行按 `CommandLineToArgvW` 反向规则引用；ConPTY 子进程使用显式 executable、最小 Unicode 环境块和可选工作目录；
- Claude Code 登录固定为 `claude auth login`，OpenAI Codex 固定为 `codex login`；WSL 发行版和所有参数保持独立；Node launcher 显式传脚本，CMD wrapper 启用 `/d` 且拒绝路径或固定参数中的 CMD 元字符。

### 当前验证

- macOS 上 8 项进程行为测试与 1 项平台前置校验通过，零警告 Clippy 通过；
- 同一生产源文件通过 `x86_64-pc-windows-msvc` 最小编译外壳，覆盖 Job Object、CreateProcessW、ConPTY、pipe I/O 与账户命令；
- `.github/workflows/windows-ci.yml` 提前落地真实 Windows 核心验证，后续分支推送将执行前端、格式、Clippy、完整 Rust、Credential Manager、进程树和 ConPTY 往返；
- Task 6 的最终检查点仍等待 Windows CI 真实运行结果，当前不会提前标为完成。

### 首次 Windows runner 反馈

- Windows CI [33714907252](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33714907252) 已通过前端测试/构建、Rust 格式、零警告 Clippy、Windows 源码编译，以及 ConPTY 非法尺寸和创建/缩放两项真实运行测试；
- 唯一失败为 ConPTY 固定输入往返：测试向控制台写入 `hello\r\n` 后等待 `received:hello`，3 秒内未收到目标响应；这证明失败边界在真实终端输入语义，而不是 ConPTY 创建、窗口尺寸或 Windows 编译；
- 后续修正使用 Windows 控制台 Enter 的单 `CR` 序列，并临时允许捕获受控夹具的 `hello` 回显，以便下一轮 CI 明确区分“输入已到伪终端但未到 Node stdin”和“完整响应成功”；最终断言仍要求 `received:hello`，不会把回显误判为成功；
- CI 同步升级到 `actions/setup-node@v6`，消除旧 Node 20 action runtime 的弃用告警。

### ConPTY 输入停滞根因

- Windows CI [33715538395](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33715538395) 再次通过前端、格式、Clippy、Rust 编译和 ConPTY 创建/缩放，但即便等待终端回显也没有出现任何受控文本，排除了 CRLF/CR 差异；
- 微软 Terminal 的 ConPTY 实现说明：伪终端创建后会立即发出 Device Status Report 光标位置查询 `ESC[6n`，若终端宿主不回复 `ESC[row;columnR`，ConPTY 将暂停处理输入；此前 `read_until` 会读取该控制序列但没有实现回复，正好解释“创建/缩放成功，进程输入输出停滞”；
- `read_until` 现会按原始字节流识别完整光标查询，并以保守的 `ESC[1;1R` 回复；分片查询不会被误判，多次查询会逐一应答；
- Windows 运行测试改为先等待夹具主动输出 `ready`（验证启动、输出管道和 DSR 握手），再发送固定输入并等待 `received:hello`（验证输入管道和子进程 stdin），避免单一超时掩盖故障层。
- Windows CI [33716133098](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33716133098) 在运行测试前由零警告 Clippy 发现条件编译后的 `items_after_test_module`；macOS 会裁掉后续 Windows 项而未触发该 lint，测试模块现已移至文件最终位置，并将 Windows 目标静态检查纳入本地复核。

## 安全与隐私

- 当前新增合同和 fixture 不含真实身份、路径、API Key、OAuth Token、Cookie、手机号或原始 Provider 响应；
- Windows 前端尚无秘密接口，也没有任意命令执行能力；
- Windows Credential Manager、CLI 输出边界和 WebView2 allowlist 将在后续任务以独立测试实现。

## 尚未完成

- Credential Manager Windows runner 往返测试与 DeepSeek 真机 Key 验收；
- Windows runner 上的 Credential Manager、进程树与 ConPTY 往返，以及 Claude Code/OpenAI Codex 采集；
- 系统托盘、贴边窗口、详情、完整 Settings 和真机交互；
- DeepSeek WebView2 30 日图表；
- NSIS、Tauri Updater、Windows CI 和双平台 Preview Release。

上述项目保持 `进行中`，不会因为 macOS 上的骨架编译通过而提前标记完成。
