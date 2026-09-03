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

## 安全与隐私

- 当前新增合同和 fixture 不含真实身份、路径、API Key、OAuth Token、Cookie、手机号或原始 Provider 响应；
- Windows 前端尚无秘密接口，也没有任意命令执行能力；
- Windows Credential Manager、CLI 输出边界和 WebView2 allowlist 将在后续任务以独立测试实现。

## 尚未完成

- Windows 强类型领域、缓存和设置；
- Credential Manager 与 DeepSeek API；
- 原生/WSL CLI 发现、ConPTY、Claude Code 和 OpenAI Codex 采集；
- 系统托盘、贴边窗口、详情、完整 Settings 和真机交互；
- DeepSeek WebView2 30 日图表；
- NSIS、Tauri Updater、Windows CI 和双平台 Preview Release。

上述项目保持 `进行中`，不会因为 macOS 上的骨架编译通过而提前标记完成。
