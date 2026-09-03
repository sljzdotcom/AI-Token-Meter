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
- Windows CI [33716538082](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33716538082) 通过 Windows Clippy 后，夹具的 `ready` 出现在父 CI 日志而 ConPTY 输出 pipe 仍超时；这直接证明子进程启动成功、但标准输出仍绑定父控制台；
- Windows 在 `bInheritHandles = false` 且未设置 `STARTF_USESTDHANDLES` 时仍可能复制父进程标准句柄。当前 `STARTUPINFOEXW` 已按生产级 ConPTY 实现设置 `STARTF_USESTDHANDLES`，并继续使用清零的 stdin/stdout/stderr，由伪终端建立新的控制台连接。

### Windows 运行边界复验

- Windows CI [33717040377](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33717040377) 已在真实 `windows-latest` runner 上通过 ConPTY 的创建、缩放、`ready → input → received` 完整往返，证明标准句柄隔离修复有效；Credential Manager 真实写入/读取/删除、Job Object 后代回收、超时、取消、输出上限、编码归一化和固定登录命令也全部通过；
- 该轮唯一失败不是运行时功能，而是工作目录测试把 Node 报告的 8.3 短路径 `C:\Users\RUNNER~1\...` 与 Rust `canonicalize` 得到的 verbatim 长路径 `\\?\C:\Users\runneradmin\...` 直接按字符串比较；两者指向同一目录；
- 测试现先把子进程报告的路径重新规范化，再与预期规范路径比较，验证的是目录身份而非 Windows 可变的文本表示。本机聚焦测试、格式、零警告 Clippy 与 Windows 目标静态检查均通过；
- 最终 Windows CI [33717589098](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33717589098) 在同一提交 `432d728` 上完整通过前端测试/构建、Rust 格式、零警告 Clippy 和全部 Windows 运行测试。Task 6 的进程、凭据、ConPTY 与登录入口检查点据此关闭。

## 安全与隐私

- 当前新增合同和 fixture 不含真实身份、路径、API Key、OAuth Token、Cookie、手机号或原始 Provider 响应；
- Windows 前端尚无秘密接口，也没有任意命令执行能力；
- Windows Credential Manager 与 CLI 输出边界已有独立测试和真实 runner 证据；WebView2 allowlist 将在 DeepSeek 图表阶段实现。

## Phase 2：Claude Code 与 OpenAI Codex 采集（进行中）

### 跨平台解析合同

- 先新增 Windows Claude 终端与 Codex app-server JSONL 脱敏 fixture；失败测试确认采集模块和统一错误类型尚不存在后再实现；
- Claude 解析器支持 ANSI 清理、used/remaining 转换、两级额度和相邻重置说明；促销中的 `50% higher` 不会被误识别为额度，未登录、工作区未信任和未知输出不会伪装成 `0%`；
- Codex 解析器按请求 ID 选取响应并忽略未知通知；解析主/周额度、Unix 重置时间和仅 `available` 的 Full Reset Credit，时间统一输出 RFC 3339；
- 6 项解析测试、格式与零警告 Clippy 通过。下一步接入真实 Claude ConPTY 会话、Codex app-server 进程及本机 30 日活动。

### Provider 进程接入

- Claude 采集先以受限非交互进程执行 `auth status`，再通过 120×40 ConPTY 启动固定的 `--ax-screen-reader --safe-mode`，只发送固定 `/usage`，并在解析前识别登录和工作区信任阻断；
- Codex app-server 以显式 executable/argument 启动，完成 initialize、initialized、account/read 与 account/rateLimits/read；每个响应按单调 ID 匹配，未知通知被忽略，单行 1 MiB、100 行和逐请求超时构成硬边界；
- app-server 子进程使用同一最小环境和 Windows Job Object，完成或失败都会终止进程树；跨平台 Node fixture 已完成真实 stdin/stdout 会话测试；
- 当前 Claude Windows-only fixture 将在下一轮 runner 验证真实 ConPTY 端到端采集；macOS 交叉构建仍会因缺少 `llvm-rc` 停在 Tauri 资源阶段，此边界不冒充 Windows 结果。

### Provider runner 首轮反馈

- Windows CI [33718718700](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33718718700) 通过前端、构建、格式和 Windows Clippy，但 Claude 端到端 fixture 在 0.26 秒内返回 Transport；耗时排除了 8 秒 ConPTY 等待超时；
- 检查 fixture 发现其按 ESM 执行却包含顶层 `return`，Node 会在进入 `auth status` 分支前语法退出。fixture 已改为完整 `if / else if / else`，保留非预期参数退出码 2，下一轮 runner 将验证产品路径。

### 本机 30 日活动与刷新协调

- Claude 活动只解码 timestamp、sessionId 和四个 usage 计数字段；对话内容、cwd 与其他字段不会进入模型；JSONL 枚举拒绝符号链接，并限制 16,384 个目录项、4,096 个文件、单文件 256 MiB、总计 512 MiB 和单行 2 MiB；
- Codex 活动以 SQLite `READ_ONLY | NO_MUTEX`、`query_only=ON` 和 1 秒 busy timeout 查询 `threads` 的 tokens_used/created_at/updated_at，最多 100,000 行，不读取对话文本；
- 两种来源都输出共享的 period、sessions、tokens、active days 和 longest session；原生与 WSL 路径由调用方分别传入，不会合并为一个统计；
- 刷新协调器默认 300 秒，三个 Provider 使用独立线程并行；计划刷新对同 Provider 去重，手动刷新取消旧 token 后替换，旧任务结束时不能误删新任务；单项失败只进入该 Provider 结果；
- 本机完整 Rust 回归共 56 项通过，新增 3 项活动和 3 项协调器测试；格式与全目标零警告 Clippy 通过。

### Windows runner 第二轮 Provider 反馈

- Windows CI [33719315118](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33719315118) 已证明 ESM 语法修复有效，但 Claude 端到端 fixture 仍在约 0.07 秒返回 Transport，继续排除 ConPTY 的 8/12 秒读取超时；
- 根因是 ConPTY 为完成终端握手先向子进程 stdin 写入 `ESC[1;1R`，fixture 使用 `stdin.once("data")` 把这段协议回复误当成唯一业务输入，未见 `/usage` 即移除监听并退出；生产 Claude CLI 本身会正确处理终端协议，缺陷位于测试替身；
- fixture 改为有界累计输入并在看到固定 `/usage` 后才输出脱敏额度，随后移除监听并退出；本机以“终端握手回复 + `/usage`”组合序列复验成功；
- 修复提交 `16258c8` 已触发 Windows runner，结果未通过前仍不关闭 Task 7。

### Windows runner 第三轮 Provider 反馈

- Windows CI [33720000356](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33720000356) 再次通过前端、格式、Windows Clippy 和全部先行编译；Claude 端到端测试仍在约 0.09 秒返回 `Transport`，故障继续只存在于测试替身生命周期；
- 替身在输出额度后固定 100ms 主动退出，而采集器正在从 ConPTY 读取第二段输出；管道关闭与读取形成竞态。真实 Claude Code 是持续交互进程，其生命周期由调用方结束，不具备该提前退出行为；
- 替身现只输出一次额度并保持交互会话，由生产代码已有的 Job Object 在采集返回时回收。这样同时验证稳定读取与无残留进程清理，不再依赖任意 100ms 墙钟窗口。

### Windows runner 第四轮 Provider 反馈

- Windows CI [33722059809](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33722059809) 在替身保持存活后仍于约 0.08 秒返回 `Transport`，因此否定“100ms 退出竞态”是主因；此前改动仍保留，因为它让替身更符合真实交互式 CLI；
- 与已通过的通用 ConPTY Node 测试逐项比较后，唯一剩余差异是 `ExecutableLocator` 为安全校验保存了 Windows `canonicalize` 产生的 `\\?\` 扩展路径，并把该路径原样作为 Node/CMD 的脚本参数。Win32 API 接受该形式，但解释器入口并不保证接受；
- 新增失败先行测试，分别覆盖本地 `\\?\C:\...` 与网络 `\\?\UNC\...`。内部候选继续保留规范路径用于身份/文件校验，只在生成 Node/CMD 参数时转换为普通 DOS/UNC 路径，且所有参数仍保持分离、不经过 shell 拼接。

## Phase 4：应用生命周期、服务账号与 Windows 更新

### 真实应用生命周期

- 启动先加载每个 Provider 的独立缓存，再立即触发计划刷新；后续按设置周期刷新，托盘 Refresh 使用手动优先级并取消同 Provider 的旧任务；
- 刷新开始、成功与失败通过同一 `snapshot-updated` 事件同步浮动条、详情和托盘；刷新世代阻止已取消旧任务回写，DeepSeek 余额更新保留独立网页会话写入的 30 日历史；
- 首轮 Windows CI `33725559642` 只暴露父模块和文件重复 `cfg(windows)` 的零警告 Clippy 错误；最小修复后 Windows CI `33726552828` 全绿，Task 7 据此关闭。

### Settings Services

- Claude Code 使用官方 `auth status --json`，显示可用的账号、登录方式与订阅类型；OpenAI Codex 使用官方 app-server `account/read`，容忍未来新增的 plan 名称；DeepSeek 只显示 Credential Manager 中 Key 的末四位；
- CLI 来源只显示 `Native Windows` 或 `WSL · 发行版` 及经过约束的版本号，不把用户路径返回前端；登录按钮只允许固定的 `claude auth login` / `codex login`，在新控制台窗口中由官方 CLI 完成认证；
- DeepSeek 替换先请求固定官方余额端点验证候选 Key，成功后才替换旧凭据；失败保留旧 Key，前端密码框不回显已存储内容；
- 解析与界面测试覆盖登录、未登录、未知 Codex plan、外部 Provider、末四位掩码、原生/WSL 来源和显式按钮行为。Codex app-server 行为以官方文档为准：<https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md>。

### Tauri Updater 与 NSIS

- About 只在用户点击 `Check for Updates` 时访问固定 GitHub `latest.json`；发现新版后才启用独立的 `Update Now`，无后台检查或静默安装；
- updater 公钥进入应用，私钥留在维护者机器/GitHub Actions Secret；日常配置不生成签名更新资产，`tauri.release.conf.json` 仅在正式 Release 开启 `createUpdaterArtifacts`；
- NSIS 使用 current-user 安装与 passive 更新，更新开始前取消全部在途采集进程；Tauri 自身强制验证 updater signature。实现依据：<https://v2.tauri.app/plugin/updater/>、<https://v2.tauri.app/distribute/windows-installer/>；
- 本机 4 项更新状态测试、10 项前端测试、零警告 Clippy、TypeScript 与 Vite build 已通过；签名产物与隔离升级仍必须由 Windows Release runner 和真机完成，不能用 macOS 构建结果替代。

## Phase 3：Windows 浮动条、详情和 Settings 前端

### 测试先行范围

- 组件测试覆盖三枚 Logo 统一 34×34 视觉框、三个等尺寸圆环、Claude Code 黄色、OpenAI Codex 紫色和 DeepSeek 蓝色合同来源；
- DeepSeek 以 ¥100 基准和 ¥77.99 余额显示 22.01% 已消耗；不可用、未安装、待登录和格式变化均不提供伪造 `aria-valuenow=0`；
- 点击 Provider 只打开一张详情；再次点击、外部指针和设置秒数到期均关闭；指针或键盘交互暂停自动隐藏计时；
- Settings 使用 Appearance、Monitoring、Services、About 四个 tab，自身及全部子控件强制 Windows 系统字体；展示字体列表包含 System、Antonio、DIN Condensed、Alimama、Fira Code、Leigo 和 Menlo。

### 实现

- 浮动条复用仓库既有 `floating-strip-deep-sea.png`，黑蓝背景覆盖主体及上下肩部，不新增第二套视觉资源；圆环只显示放大的 Provider Logo，不放 Provider 名称或数值文字；
- 详情保留官方额度优先：Codex 显示 Reset Credit 数量和到期时间，Claude/Codex 显示明确标注 `This PC` 的 30 日本机统计，DeepSeek 预留官方 30 日图表区域；Claude 不恢复用户已要求移除的 Token composition、Top models 和隐私说明；
- 前端单向状态桥只调用固定 `usage_snapshots` 并订阅 `snapshot-updated`；对 Provider、状态、比例和必需字段再次做边界校验，无任意 executable、路径或 shell 接口；
- 默认展示字体为 Antonio，所有交互有可访问名称、键盘焦点和 reduced-motion/high-contrast 规则；200% 缩放使用自适应详情高度和响应式卡片。

### 验证与边界

- Vitest 5 项通过，TypeScript 无输出检查与 Vite production build 通过；构建包含 27 个模块；
- 本机 Chromium 800×900 视觉预览确认统一 Logo、深海背景、贴右布局和透明外围；预览图只用于样式自检，没有作为 Windows WebView2 或真实透明窗口证据；
- Win32 无任务栏窗口、左右贴边、全屏隐藏、拖动、详情临时 topmost、系统托盘和实际窗口 S 曲线仍属于下一阶段。

## 尚未完成

- DeepSeek 真机 Key 验收；
- Win32 左右贴边/DPI/全屏/拖动的 Windows 真机交互验收；
- DeepSeek WebView2 独立登录和 30 日图表真机验收；
- NSIS、Tauri Updater、Windows CI 和双平台 Preview Release。

上述项目保持 `进行中`，不会因为 macOS 上的骨架编译通过而提前标记完成。

## Phase 3：托盘与 Win32 窗口策略（实现完成，等待 runner/真机）

### 测试先行与窗口几何

- 纯策略测试覆盖左右工作区贴边、DPI 后物理尺寸、归一化垂直位置、详情向屏幕内展开并钳制、显示器断开后回退、同显示器全屏判定和详情 topmost 生命周期；
- 浮动条轮廓用同一组 Bezier 采样点生成 Win32 `HRGN`；右侧与左侧互为严格镜像，CSS `clip-path` 同步使用相同 116×450 坐标，避免透明窗口点击区和视觉边界不一致；
- 新增 Windows-only GDI 集成测试，要求左右轮廓都能创建有效区域，并验证 runner 存在非空显示器和工作区。macOS 只编译测试壳，不把零项执行冒充 Windows 结果。

### 原生窗口行为

- Tauri 应用拆分为 `meter`、`detail`、`settings` 三个最小权限窗口；capability 只授权这三个窗口及拖动，不引入 shell 插件或任意命令执行；
- meter 使用无标题、透明、跳过任务栏、不可聚焦及 Win32 `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`，默认主屏右侧；非按钮区域可拖动，松手后选择最近边缘并保存显示器标识与归一化 Y；
- Settings 可即时切换左右边缘；重新显示和重启均恢复持久化边缘、显示器与高度，显示器已断开时回退主屏；字体和详情自动隐藏秒数也从界面真实写入原子设置并通过事件同步所有窗口；
- 同显示器存在全屏前台应用时每 500ms 隐藏 meter，离开全屏后恢复；用户在托盘主动隐藏时不会被监控线程误恢复；
- detail 每次打开都定位在 meter 内侧、临时置顶并获得焦点；失去焦点、外部点击或自动隐藏后先清除 topmost 再隐藏；Settings 关闭只隐藏窗口，不退出托盘应用。

### 系统托盘与验证边界

- 托盘包含三个 Provider 脱敏摘要、Refresh、Settings、Show/Hide Meter、About 和 Quit；摘要按事件动态更新，DeepSeek 显示可用余额，Claude Code/OpenAI Codex 显示已用比例，不可用状态不会伪造 `0%`；
- 本机 7 项窗口策略、1 项托盘摘要、6 项设置持久化、完整 65 项 Rust 回归、5 项前端、TypeScript/Vite 构建、格式和零警告 Clippy 已通过；128 份 Markdown、共享合同与公开安全门禁通过；
- Windows CI 增加完整 `tauri build --debug --no-bundle`，下一轮将同时验证真实 Win32 条件编译、GDI 区域、显示器 API、多窗口 capability 与桌面壳构建；左右贴边、125%/200% DPI、全屏 Edge、普通窗口覆盖和真实指针拖动仍需 Windows 真机视觉验收，Task 9 步骤 5 因此保持未完成。

### Windows runner 桌面壳与 Provider 终验

- Windows CI [33722886734](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33722886734) 在提交 `04da07b` 上完整通过；Claude Code 的真实 ConPTY 替身、Credential Manager、Job Object、GDI 窗口区域、显示器 API和完整 Tauri 多窗口桌面壳均在 `windows-latest` 编译/运行通过；
- 该结果验证了 Node/CMD 解释器参数边界将 `\\?\` 本地/UNC 路径转换为解释器可接受形式，同时内部规范路径仍用于身份校验；Task 7 据此关闭；
- Task 9 的 runner 部分已完成，但左右贴边、125%/200% DPI、全屏 Edge 与真实指针拖动仍需要交互式 Windows 11 桌面，步骤 5 继续保持未完成。

## Phase 4：DeepSeek 独立 WebView2 历史（实现完成，等待真机登录）

### 数据来源与安全边界

- 官方公开 API 目前只提供余额 `/user/balance`；最近 30 天费用、请求和 Token 来自官方控制台使用的私有 dashboard 端点，属于可能变化的网页接口。实现同时支持当前 `usage/by_api_key/amount` 与 `usage/by_api_key/cost` 响应对，并把格式变化视为历史不可用，绝不影响余额快照；
- 历史窗口使用 `%LOCALAPPDATA%\AI Token Meter\WebView2\DeepSeek` 独立目录，不读取 Edge/Chrome Cookie，也不复用外部浏览器会话；导航只允许 HTTPS 的 `platform.deepseek.com` 精确主机，仿冒子域、HTTP、userinfo 和其他 DeepSeek 主机均被拒绝；
- 注入桥接只拦截官方页面已经发起的用量响应，在页面内先丢弃 `api_key`、tracking ID、名称和其他身份字段，再输出日期、人民币费用、请求数和三类 Token 汇总；Rust 边界继续验证 128-bit nonce、20 秒会话、64 分片、16 KiB 单片、256 KiB 总量、严格 DTO 和敏感字段黑名单；
- 桥接通过应用私有、不可导航的 `aimeter-deepseek://history` 回调传递分片，不向远程网页开放 Tauri commands、文件、进程或凭据能力。历史失败不写余额；成功只更新 `dailyHistory` 和独立的 `historyFetchedAt`。

### 详情与测试证据

- DeepSeek 详情现在显示 30 日费用柱状图、总费用、请求数、Token、官方网页来源和独立更新时间；没有历史时提供“Sync official history”，点击 DeepSeek 且无历史也会自动打开独立登录/同步窗口；
- 先行测试覆盖官方 origin、仿冒 origin、乱序分片、超时、nonce、分片/总量上限、敏感字段、严格格式、30 日裁剪、重复日期合并、溢出和余额隔离；7 项 Rust 历史测试通过；
- 真实注入脚本用当前官方控制台 `by_api_key` 脱敏响应 fixture 验证：金额、请求、Token 聚合正确，API Key 名称与 tracking ID 不会进入 payload；2 项桥接测试与 7 项界面测试通过；
- 本机完整 Rust 73 项、前端 9 项、TypeScript/Vite production build、格式及零警告 Clippy 均通过。独立 WebView2 的真实登录持久化、网页响应兼容性和自动隐藏仍须 Windows 11 用户会话验收，Task 10 步骤 4 保持未完成。
- Windows CI [33724498418](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33724498418) 在 `a80f72b` 上通过前端、Rust、Windows 条件编译与完整 Tauri 桌面壳构建；这关闭了代码/runner 风险，但不能替代用户账户下的 WebView2 登录和网页数据真机验收。

## Phase 2 补验：应用刷新生命周期

### 审计发现

- Provider 解析器、进程采集器与 `RefreshCoordinator` 已分别通过测试及 Windows CI，但 `RuntimeState` 启动时仍写入三份 `Unavailable` 占位；托盘 Refresh 也只发出前端事件，没有后端订阅者；
- 因此此前证据只能证明采集库可运行，不能证明安装后的应用会自动取得数据。实施计划新增 Task 7 步骤 6，原“Task 7 关闭”表述收紧为采集器和 runner 边界完成。

### 测试先行修复

- 新增 `UsageRuntime`，先用 5 项失败测试固定缓存启动、刷新中保留旧数值、Provider 失败隔离、认证错误不伪装成零用量、DeepSeek 余额刷新保留 30 日历史；
- 新增协调器回归，证明手动刷新取消后，即使旧采集器忽略取消并返回成功，也只能得到 `Cancelled`，不能发布旧快照；重复计划刷新不会运行开始生命周期，避免把界面永久留在 `Refreshing`；
- Windows 应用现在启动时加载 `%LOCALAPPDATA%` 独立缓存并发刷新三项，后台读取设置的刷新周期，托盘 Refresh 使用手动优先级；每次开始和有效完成发出同一 `snapshot-updated` 脱敏 DTO，浮动条、托盘及已打开详情同步更新；
- Native/WSL CLI 在运行时按既定优先级发现并用固定 `--version` 健康检查；Claude/Codex 成功快照附加白名单 30 日本机活动，DeepSeek 从 Credential Manager 读取 Key；缓存写入只接受成功的 fresh 快照，格式/认证/安装失败不制造数值。

### 当前验证

- 新增 6 项运行时/协调器回归后，两组聚焦测试共 10 项通过；本机完整 Rust 80 项通过（DeepSeek loopback 测试经允许运行），全目标零警告 Clippy 和 rustfmt 通过；
- 前端 9 项测试、TypeScript 检查及 Vite production build 通过；
- Windows 专属生命周期模块已进入下一轮 `windows-latest` CI。由于 macOS 缺少 MSVC C 工具链，`rusqlite bundled` 的 Windows 交叉检查停在 C 编译器环境，不把该结果视为源码失败或 Windows 编译成功。

## Phase 4 补验：Services 与 Windows 更新器

- Windows CI [33728220354](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33728220354) 已通过前端测试、生产构建和 Rust 格式检查，随后在严格 Clippy 阶段发现两个仅由 Windows 条件分支暴露的多余 `return`；该运行尚未执行 Rust 测试或 NSIS 打包，因此不作为安装包成功证据；
- 问题仅位于 Tauri command 的返回表达式风格，不涉及账号、凭据或更新状态逻辑；按 Clippy 精确建议移除多余 `return`，其余实现保持不变，进入完整 runner 复验。
