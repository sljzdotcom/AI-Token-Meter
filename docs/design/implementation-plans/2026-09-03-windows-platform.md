# AI Token Meter Windows 平台实现计划

> **面向 AI 代理的工作者：** 使用 `executing-plans` 按任务顺序执行；每个任务遵循测试先行、最小实现、完整回归和独立 Git 检查点。不得在缺少 Windows runner 或真机证据时把 Windows 原生能力标记为完成。

**目标：** 在保留现有 macOS 原生应用的前提下，为 AI Token Meter 新增 Windows 11 x64 版本，并建立版本、功能语义、文档、CI 与 GitHub Release 的双平台同步门禁。

**架构：** macOS 继续使用 SwiftUI/AppKit；Windows 使用 Tauri 2、Rust 与 React/TypeScript。两端通过根级版本文件、JSON Schema、脱敏解析夹具、展示合同和功能对等矩阵共享产品语义，不共享平台 UI 源码。Windows 前端只调用窄化的 Tauri commands，进程、网络、缓存、WebView2 和凭据均由 Rust 后端负责。

**技术栈：** Swift 6、Swift Testing、Tauri 2、Rust stable、Cargo、React 19、TypeScript、Vite、Vitest、Testing Library、Windows App SDK/Win32 API、WebView2、Windows Credential Manager、NSIS、Tauri Updater、GitHub Actions。

---

## 交付阶段与完成定义

| 阶段 | 可交付结果 | 完成证据 |
| --- | --- | --- |
| Phase 1 | 共享合同、唯一版本源、Windows 可构建骨架、跨平台 CI | macOS 全量测试；Windows TypeScript/Rust 测试和 Tauri build；合同门禁通过 |
| Phase 2 | 三个 Provider 的 Windows 领域层、持久化、凭据与原生/WSL CLI 发现 | Rust 单元/集成测试；脱敏夹具；Windows runner 原生 smoke test |
| Phase 3 | 托盘、贴边浮动条、详情与 Settings 的完整交互 | 前端组件测试；Win32 窗口策略测试；Windows 11 真机截图与交互记录 |
| Phase 4 | DeepSeek 30 日 WebView2、手动更新、NSIS 安装与同步 Release | WebView2 真机验收；签名更新演练；双平台 Release 资产和校验文件 |
| Phase 5 | 公共预览与稳定发布 | `v0.3.0-preview.1` 反馈关闭；双平台稳定 CI；`v0.3.0` Release |

Windows Widget 不在本计划范围内。macOS Widget 的证书与真实桌面验收继续由 `REQ-20260901-003` 管理。

## 计划文件布局

### 新建

- `VERSION`：两个平台唯一的产品版本来源，初始内容与当前 macOS `0.2.2` 一致。
- `contracts/schemas/usage-snapshot.schema.json`：版本化 Provider 快照合同。
- `contracts/presentation/providers.json`：名称、排序、品牌色、Logo 键与进度语义。
- `contracts/parity/features.yml`：双平台功能对等和证据矩阵。
- `contracts/fixtures/*.json`：不含身份与凭据的共享正常、缓存、登录失效、未安装和解析异常样本。
- `scripts/check-cross-platform-contracts.rb`：版本、Schema、展示合同、fixture 和对等矩阵门禁。
- `Tests/AIMeterCoreTests/CrossPlatformContractTests.swift`：Swift 对共享合同的消费测试。
- `windows/`：Tauri/Rust/React Windows 应用、测试和资源。
- `.github/workflows/windows-ci.yml`：Windows 11 x64 构建、测试和安装包门禁。
- `scripts/package-cross-platform-release.sh`：仅编排经过验证的双平台产物，不在本机伪造 Windows 包。
- `docs/development/2026-09-03-windows-platform.md`：实现、验证、限制与发布证据日志。

### 修改

- `Package.swift`、`Sources/AIMeterApp/Resources/Info.plist`、构建脚本：从 `VERSION` 校验或注入 macOS 版本。
- `.github/workflows/ci.yml`：增加共享合同门禁，保留 macOS 测试职责。
- `README.md`、`CHANGELOG.md`、`docs/README.md`、`docs/project-status.md`、架构/用户/发布/测试文档：同步平台能力与真实状态。
- `docs/requirements-backlog.md`：每个阶段记录状态和 Git 证据。

---

## 任务 1：建立唯一版本源与共享产品合同

**文件：**

- 创建：`VERSION`
- 创建：`contracts/schemas/usage-snapshot.schema.json`
- 创建：`contracts/presentation/providers.json`
- 创建：`contracts/parity/features.yml`
- 创建：`contracts/fixtures/claude-fresh.json`
- 创建：`contracts/fixtures/codex-reset-credit.json`
- 创建：`contracts/fixtures/deepseek-balance.json`
- 创建：`contracts/fixtures/authentication-required.json`
- 创建：`scripts/check-cross-platform-contracts.rb`
- 创建：`Tests/AIMeterCoreTests/CrossPlatformContractTests.swift`
- 修改：`scripts/test.sh`
- 修改：`docs/design/README.md`

- [x] **步骤 1：先写失败的 Swift 合同测试**

测试从仓库根目录读取 `VERSION` 和合同文件，验证：

```swift
@Test("Root version matches the macOS bundle version")
func sharedVersionMatchesBundle() throws {
    let root = try RepositoryRoot.locate()
    let version = try String(contentsOf: root.appending(path: "VERSION"))
        .trimmingCharacters(in: .whitespacesAndNewlines)
    let plist = try PropertyListSerialization.propertyList(
        from: Data(contentsOf: root.appending(path: "Sources/AIMeterApp/Resources/Info.plist")),
        format: nil
    ) as? [String: Any]
    #expect(version == plist?["CFBundleShortVersionString"] as? String)
}

@Test("Every provider has a fixture and stable presentation identity")
func providerContractIsComplete() throws {
    let contract = try SharedProviderContract.load()
    #expect(contract.providers.map(\.id) == ["claude", "codex", "deepseek"])
    #expect(contract.providers.map(\.displayName) == ["Claude Code", "OpenAI Codex", "DeepSeek"])
}
```

- [x] **步骤 2：运行聚焦测试并确认失败**

运行：`bash scripts/test.sh --filter CrossPlatformContractTests`

预期：因 `VERSION` 和 `contracts/` 尚不存在而失败。

- [x] **步骤 3：实现版本、Schema 和展示合同**

`usage-snapshot.schema.json` 固定 `schemaVersion: 1`，要求：

- `providerId` 只接受 `claude`、`codex`、`deepseek`；
- `status` 只接受规格定义的九种状态；
- `usedRatio` 为空或 `0...1`；
- 时间使用 RFC 3339；
- 未知附加字段允许，未知主版本由消费者拒绝显示数值；
- Reset Credit 只保存类型、数量、到期时间；
- 本机活动只保存日期、会话数、Token 聚合和最长时长。

`providers.json` 将 DeepSeek 的 `progressSemantics` 固定为 `consumedFromBalanceBaseline`，避免 Windows 误画成余额剩余。

- [x] **步骤 4：实现 Ruby 合同门禁**

脚本使用 Ruby 标准库，不增加全局依赖；验证 JSON 语法、必填字段、ID 唯一性、版本同步、fixture 脱敏、对等矩阵状态值和每项 REQ 证据字段。秘密扫描至少拒绝：Bearer Token、Telegram Bot Token、OpenAI/Anthropic/DeepSeek Key、Cookie 和手机号。

- [x] **步骤 5：把合同检查接入总测试**

在 `scripts/test.sh` 的文档与公开安全检查之前执行：

```bash
ruby scripts/check-cross-platform-contracts.rb "$PROJECT_DIR"
```

- [x] **步骤 6：运行验证**

运行：

```bash
bash scripts/test.sh --filter CrossPlatformContractTests
ruby scripts/check-cross-platform-contracts.rb .
bash scripts/check-docs.sh .
git diff --check
```

预期：聚焦测试、合同门禁和文档检查均通过。

- [x] **步骤 7：提交检查点**

```bash
git add VERSION contracts scripts/check-cross-platform-contracts.rb scripts/test.sh Tests/AIMeterCoreTests/CrossPlatformContractTests.swift docs/design docs/requirements-backlog.md
git commit -m "feat: establish cross-platform product contracts"
```

---

## 任务 2：创建可在 Windows 构建的 Tauri 工作区

**文件：**

- 创建：`windows/package.json`
- 创建：`windows/package-lock.json`
- 创建：`windows/tsconfig.json`
- 创建：`windows/vite.config.ts`
- 创建：`windows/index.html`
- 创建：`windows/src/main.tsx`
- 创建：`windows/src/App.tsx`
- 创建：`windows/src/test/setup.ts`
- 创建：`windows/src/App.test.tsx`
- 创建：`windows/src-tauri/Cargo.toml`
- 创建：`windows/src-tauri/build.rs`
- 创建：`windows/src-tauri/tauri.conf.json`
- 创建：`windows/src-tauri/capabilities/default.json`
- 创建：`windows/src-tauri/src/lib.rs`
- 创建：`windows/src-tauri/src/main.rs`
- 创建：`windows/src-tauri/icons/*`
- 修改：`.gitignore`

- [x] **步骤 1：先写失败的前端 smoke test**

```tsx
it("renders the three providers using the shared names", async () => {
  render(<App />)
  expect(await screen.findByLabelText("Claude Code usage")).toBeVisible()
  expect(screen.getByLabelText("OpenAI Codex usage")).toBeVisible()
  expect(screen.getByLabelText("DeepSeek usage")).toBeVisible()
})
```

- [x] **步骤 2：创建固定依赖的最小前端**

使用 `npm ci` 可重现的 lockfile；脚本固定为：

```json
{
  "dev": "vite",
  "build": "tsc --noEmit && vite build",
  "test": "vitest run",
  "tauri": "tauri"
}
```

首屏只渲染三个无文字 Logo 的语义按钮和可访问名称；不得提前伪造真实额度。

- [x] **步骤 3：先写失败的 Rust 启动合同测试**

```rust
#[test]
fn product_metadata_matches_shared_contract() {
    let metadata = app_metadata().expect("metadata");
    assert_eq!(metadata.product_name, "AI Token Meter");
    assert_eq!(metadata.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(metadata.providers, ["claude", "codex", "deepseek"]);
}
```

- [x] **步骤 4：实现最小 Tauri 应用**

Rust library 暴露可测试的 `app_metadata()`；`main.rs` 只调用 library 的 `run()`。Tauri capability 仅允许窗口和事件所需能力，不安装通用 shell 插件。`tauri.conf.json` 的版本由检查脚本与根 `VERSION` 强制同步。

- [x] **步骤 5：运行本机可行验证**

运行：

```bash
cd windows && npm ci
npm test
npm run build
```

若当前 macOS 未安装 Rust，只记录 Rust 本机验证未执行；不得因此跳过随后 Windows CI。安装依赖前先核对 lockfile 和官方 registry 来源。

- [x] **步骤 6：提交检查点**

```bash
git add windows .gitignore
git commit -m "feat: scaffold Windows Tauri application"
```

---

## 任务 3：实现 Windows 领域模型、缓存与设置

**文件：**

- 创建：`windows/src-tauri/src/domain/mod.rs`
- 创建：`windows/src-tauri/src/domain/usage.rs`
- 创建：`windows/src-tauri/src/domain/presentation.rs`
- 创建：`windows/src-tauri/src/persistence/mod.rs`
- 创建：`windows/src-tauri/src/persistence/atomic_json.rs`
- 创建：`windows/src-tauri/src/persistence/cache.rs`
- 创建：`windows/src-tauri/src/persistence/settings.rs`
- 创建：`windows/src-tauri/src/security/redaction.rs`
- 创建：`windows/src-tauri/tests/shared_contract.rs`
- 创建：`windows/src-tauri/tests/persistence.rs`

- [x] **步骤 1：写共享 fixture 和兼容性失败测试**

覆盖正常快照、旧缓存缺字段、未知附加字段、未知主版本、损坏 JSON、过期快照和原子写入中断：

```rust
#[test]
fn unknown_major_schema_never_surfaces_a_number() {
    let value = fixture_with_schema_version(99);
    let snapshot = UsageSnapshot::decode_compatible(&value);
    assert_eq!(snapshot.status, UsageStatus::Unavailable);
    assert_eq!(snapshot.used_ratio, None);
}
```

- [x] **步骤 2：实现强类型领域模型**

Rust enum 使用 `serde(rename_all = "camelCase")` 与 Schema 对齐；比例构造器钳制有限数值并拒绝 NaN/Infinity。Provider 名称和颜色从嵌入的共享展示合同加载，不在 React 中复制。

- [x] **步骤 3：实现原子持久化**

设置保存到 `%APPDATA%\AI Token Meter\settings.json`，缓存保存到 `%LOCALAPPDATA%\AI Token Meter\cache\`。写入流程为同目录临时文件、flush、rename；权限或损坏错误返回分类错误且保留旧文件。

- [x] **步骤 4：实现日志脱敏器**

测试覆盖 API Key、Authorization、Cookie、邮箱、手机号、用户目录；日志允许 Provider、阶段、耗时、错误分类和已泛化路径，不允许原始 CLI 响应。

- [x] **步骤 5：运行测试并提交**

```bash
cd windows/src-tauri && cargo test domain persistence security
git add windows/src-tauri
git commit -m "feat: add Windows usage domain and persistence"
```

---

## 任务 4：实现 Windows 凭据和 DeepSeek 余额采集

**文件：**

- 创建：`windows/src-tauri/src/security/credential_store.rs`
- 创建：`windows/src-tauri/src/platform/mod.rs`
- 创建：`windows/src-tauri/src/platform/windows/mod.rs`
- 创建：`windows/src-tauri/src/platform/windows/credential_manager.rs`
- 创建：`windows/src-tauri/src/collectors/mod.rs`
- 创建：`windows/src-tauri/src/collectors/deepseek.rs`
- 创建：`windows/src-tauri/src/accounts/deepseek.rs`
- 创建：`windows/src-tauri/tests/deepseek_collector.rs`
- 创建：`windows/src-tauri/tests/credential_manager_windows.rs`

- [x] **步骤 1：写 Fake Credential Store 与 HTTP server 测试**

验证：无 Key 为 `setupRequired`；401 为 `authenticationRequired`；超时使用可辨识缓存；新 Key 必须先成功调用余额接口才能替换；失败时旧 Key 保留且任何错误文本不含候选 Key。

- [x] **步骤 2：实现窄协议和 DeepSeek collector**

```rust
#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn read(&self, account: CredentialAccount) -> Result<Option<SecretString>>;
    async fn replace_verified(&self, account: CredentialAccount, secret: SecretString) -> Result<()>;
    async fn delete(&self, account: CredentialAccount) -> Result<()>;
}
```

HTTP 客户端固定官方 HTTPS origin、10 秒总超时、响应大小上限和结构化解析。Secret 使用零化容器，不能实现 `Debug` 明文输出。

- [x] **步骤 3：实现 Windows Credential Manager**

仅 `cfg(windows)` 编译 `CredReadW`、`CredWriteW`、`CredDeleteW`；target 名固定为 `AI Token Meter/DeepSeek API Key`。Windows 集成测试写入随机测试 target 并在 `drop`/清理阶段删除，不接触真实 Key。

- [x] **步骤 4：运行测试和秘密扫描后提交**

```bash
cd windows/src-tauri && cargo test deepseek credential
ruby ../../scripts/check-cross-platform-contracts.rb ../..
git add windows/src-tauri
git commit -m "feat: add secure Windows DeepSeek collector"
```

Windows Credential Manager 集成测试源码和目标隔离清理已实现；同一生产源文件已通过 `x86_64-pc-windows-msvc` 最小编译外壳检查。由于 macOS 不提供 Tauri 所需的 `llvm-rc` 和真实 Windows Credential Manager，往返测试必须在任务 11 建立的 Windows runner 与后续真机上执行，未把本机检查记录为运行时通过。

---

## 任务 5：实现原生 Windows 与 WSL CLI 发现

**文件：**

- 创建：`windows/src-tauri/src/platform/windows/environment.rs`
- 创建：`windows/src-tauri/src/platform/windows/executable_locator.rs`
- 创建：`windows/src-tauri/src/platform/windows/wsl.rs`
- 创建：`windows/src-tauri/src/accounts/cli_account.rs`
- 创建：`windows/src-tauri/tests/executable_locator.rs`
- 创建：`windows/src-tauri/tests/wsl_locator_windows.rs`
- 创建：`contracts/fixtures/auxiliary/windows-cli-locations.json`

- [x] **步骤 1：写候选优先级和安全失败测试**

覆盖自定义路径、进程 PATH、用户/系统注册表 PATH、npm/nvm/fnm/Volta 常见目录、桌面应用候选和 WSL；拒绝目录、符号链接循环、错误文件名、超时健康检查与 shell 元字符注入。

- [x] **步骤 2：实现候选模型**

```rust
pub enum RuntimeSource {
    NativeWindows,
    Wsl { distribution: String },
}

pub struct ExecutableCandidate {
    pub executable: PathBuf,
    pub launcher: Option<PathBuf>,
    pub source: RuntimeSource,
}
```

Node shebang 脚本必须解析同目录或安装树中可验证的 `node.exe`，并以显式 executable + arguments 启动；不得依赖 GUI 进程 PATH 中的 `env node`。

- [x] **步骤 3：实现 WSL 探测**

调用 `wsl.exe --list --quiet`，逐行清理 UTF-16/空字符；发行版名称作为独立参数传给 `wsl.exe --distribution <name> --exec`。原生与 WSL 结果分别展示，不自动合并账户或活动。

- [x] **步骤 4：运行跨平台单测和 Windows 集成测试后提交**

```bash
cd windows/src-tauri && cargo test executable_locator
cargo test --test wsl_locator_windows
git add contracts/fixtures windows/src-tauri
git commit -m "feat: discover native and WSL AI CLIs on Windows"
```

发现器纯逻辑测试、完整 Rust 回归、零警告 Clippy 与同一 Windows 专属源文件的 `x86_64-pc-windows-msvc` 最小编译外壳均已通过。`wsl.exe --list --quiet` 和 Provider 调用已建模为分离参数；真正启动进程、超时健康检查及 Windows/WSL 运行时往返由任务 6 的受限 runner 和任务 11 的 Windows runner 完成，未以 macOS 结果冒充真机验收。

---

## 任务 6：实现安全进程执行、ConPTY 和登录入口

**文件：**

- 创建：`windows/src-tauri/src/platform/windows/process.rs`
- 创建：`windows/src-tauri/src/platform/windows/conpty.rs`
- 创建：`windows/src-tauri/src/accounts/claude.rs`
- 创建：`windows/src-tauri/src/accounts/codex.rs`
- 创建：`windows/src-tauri/tests/process_runner.rs`
- 创建：`windows/src-tauri/tests/conpty_windows.rs`

- [x] **步骤 1：写进程边界失败测试**

覆盖参数数组、最大输出、超时、取消、父进程退出后子进程树终止、UTF-8/UTF-16 解码、ANSI 清理和日志脱敏。测试命令使用仓库内固定 fixture，不加载 PowerShell profile 或 CMD AutoRun。

- [x] **步骤 2：实现非交互 runner**

使用 Job Object 管理整个进程树；创建进程时显式 executable、argument quoting、工作目录和最小环境；超时后先取消读取再终止 Job，最终排空有限输出。

- [x] **步骤 3：实现 ConPTY adapter**

通过 `CreatePseudoConsole` 建立 120×40 终端；暴露发送固定输入、等待模式、超时和 resize。原始终端输出只传给 parser，不落盘。

- [x] **步骤 4：实现固定登录动作**

- Claude Code：固定 `claude auth login`；
- OpenAI Codex：固定 `codex login`；
- 对 WSL 使用同一已选择发行版；
- 自定义路径只作为 executable，不接受额外参数文本。

- [x] **步骤 5：运行 Windows 进程集成测试并提交**

```bash
cd windows/src-tauri && cargo test process_runner
cargo test --test conpty_windows
git add windows/src-tauri
git commit -m "feat: run Windows AI CLIs through bounded processes"
```

---

## 任务 7：实现 Claude Code 与 OpenAI Codex 采集

**文件：**

- 创建：`windows/src-tauri/src/collectors/claude.rs`
- 创建：`windows/src-tauri/src/collectors/codex.rs`
- 创建：`windows/src-tauri/src/collectors/claude_activity.rs`
- 创建：`windows/src-tauri/src/collectors/codex_activity.rs`
- 创建：`windows/src-tauri/src/collectors/codex_app_server.rs`
- 创建：`windows/src-tauri/tests/claude_collector.rs`
- 创建：`windows/src-tauri/tests/codex_collector.rs`
- 创建：`contracts/fixtures/claude-usage-windows.txt`
- 创建：`contracts/fixtures/codex-app-server-windows.jsonl`

- [ ] **步骤 1：移植脱敏解析 fixture 并写失败测试**

同一 fixture 在 Swift 与 Rust 中应得到相同 session/weekly 比例、重置时间、促销说明和 Reset Credit。错误文本、登录提示和版本变化必须映射到合同状态而非零值。

- [ ] **步骤 2：实现 Claude collector**

在应用专属空目录启动交互式 `/usage`；首次信任、未登录、超时和格式未知分别返回明确状态；30 日本机活动只读取 Windows/WSL 对应环境的白名单聚合字段。

- [ ] **步骤 3：实现 Codex app-server collector**

完成 initialize、account/read、rateLimits/read 和 thread/list 所需最小 JSON-RPC；请求 ID 单调递增；忽略未知通知；对 Reset Credit 到期日做 RFC 3339 归一化。原生与 WSL 的 `.codex` 活动不混合。

- [ ] **步骤 4：实现并发刷新协调器**

三个 Provider 并发、同 Provider 去重、单项失败隔离、300 秒默认周期、前台手动刷新可取消旧任务；更新检查不能复用此网络周期。

- [ ] **步骤 5：运行共享解析、集成测试并提交**

```bash
cd windows/src-tauri && cargo test claude_collector codex_collector refresh
ruby ../../scripts/check-cross-platform-contracts.rb ../..
git add contracts/fixtures windows/src-tauri
git commit -m "feat: collect Claude Code and OpenAI Codex usage on Windows"
```

---

## 任务 8：实现前端状态、浮动条、详情和 Settings

**文件：**

- 创建：`windows/src/state/*`
- 创建：`windows/src/components/UsageRing.tsx`
- 创建：`windows/src/components/ProviderLogo.tsx`
- 创建：`windows/src/components/FloatingStrip.tsx`
- 创建：`windows/src/details/ClaudeDetail.tsx`
- 创建：`windows/src/details/CodexDetail.tsx`
- 创建：`windows/src/details/DeepSeekDetail.tsx`
- 创建：`windows/src/settings/SettingsWindow.tsx`
- 创建：`windows/src/settings/{Appearance,Monitoring,Services,About}Settings.tsx`
- 创建：`windows/src/theme/*`
- 创建：`windows/src/**/*.test.tsx`
- 创建：`windows/src/styles.css`

- [ ] **步骤 1：先写组件和交互失败测试**

覆盖：三个 Logo 视觉尺寸一致；品牌色区分；DeepSeek 77.99/100 显示约 22.01% 已消耗；缓存/不可用不显示为 0%；详情标题正式命名；Settings 永远系统字体；字体与字号只作用于浮动条、菜单和详情。

- [ ] **步骤 2：建立单向状态流**

前端只持有脱敏 `UsageSnapshot` 和非秘密设置；Tauri command 返回 Rust 领域 DTO；事件只包含 `snapshot-updated`、`account-status-updated`、`update-state-changed` 和窗口可见性。禁止把任意路径/命令执行能力暴露给页面。

- [ ] **步骤 3：实现已确认视觉**

- 深海黑蓝背景覆盖 S 曲线全部肩部；
- 三个圆环仅显示放大的 Logo；
- Claude Code 使用黄色基调，OpenAI Codex 使用紫色，DeepSeek 保留蓝色；
- 正常、warning、critical、cached、unavailable 优先采用语义视觉；
- Settings 分类使用 tab，所有 Settings 文本保持 Windows 系统字体；
- 可访问名称、键盘焦点、200% 字号和高对比度下仍可操作。

- [ ] **步骤 4：实现详情自动隐藏行为**

同一时间一张详情；点击当前 Logo 切换关闭；点击外部关闭；按设置秒数自动关闭；鼠标/键盘/WebView2 交互暂停计时；关闭时清空 topmost 请求。

- [ ] **步骤 5：运行前端测试、构建和视觉快照后提交**

```bash
cd windows && npm test
npm run build
git add windows/src
git commit -m "feat: build Windows meter interface"
```

---

## 任务 9：实现托盘和 Windows 窗口层级策略

**文件：**

- 创建：`windows/src-tauri/src/platform/windows/tray.rs`
- 创建：`windows/src-tauri/src/platform/windows/window_controller.rs`
- 创建：`windows/src-tauri/src/platform/windows/desktop_visibility.rs`
- 创建：`windows/src-tauri/src/platform/windows/monitor.rs`
- 创建：`windows/src-tauri/tests/window_policy.rs`
- 创建：`windows/src-tauri/tests/window_integration_windows.rs`
- 修改：`windows/src-tauri/src/lib.rs`

- [ ] **步骤 1：写纯窗口策略失败测试**

覆盖左右贴边、工作区、DPI、任务栏变化、显示器拔插、拖动归一化位置、前台全屏隐藏、桌面恢复、详情临时 topmost 和关闭后撤销 topmost。

- [ ] **步骤 2：实现系统托盘**

菜单包括三项摘要、Refresh、Settings、Show/Hide Meter、About、Quit；浅/深/高对比资源均有单色可辨识轮廓。托盘生命周期独立于窗口，关闭 Settings 不退出应用。

- [ ] **步骤 3：实现贴边浮动条**

Win32 窗口无标题、透明、无任务栏按钮、不抢焦点；默认主显示器右侧，可切左侧；不注册 AppBar。非 Logo 区域拖动并保存显示器/归一化 Y；屏幕变化后钳制回可见工作区。

- [ ] **步骤 4：实现桌面可见性与详情置前**

检测当前显示器全屏前台窗口时隐藏；返回桌面恢复。点击 Logo 时详情获得焦点并短暂 topmost，关闭后立即撤销。若标准层级在 Windows 构建号上不稳定，使用“非桌面前台时隐藏”的安全回退，并记录真机证据。

- [ ] **步骤 5：运行 Windows 集成测试和真机验收后提交**

```powershell
cargo test --manifest-path windows/src-tauri/Cargo.toml window_policy
cargo test --manifest-path windows/src-tauri/Cargo.toml --test window_integration_windows
npm --prefix windows run tauri build -- --debug
```

真机记录至少覆盖：左右侧、125%/200% DPI、全屏 Edge、普通窗口覆盖、详情置前、外部点击关闭、真实指针拖动。

```bash
git add windows/src-tauri docs/development/2026-09-03-windows-platform.md
git commit -m "feat: add Windows tray and edge window behavior"
```

---

## 任务 10：实现 DeepSeek WebView2 最近 30 天详情

**文件：**

- 创建：`windows/src-tauri/src/collectors/deepseek_history.rs`
- 创建：`windows/src-tauri/src/platform/windows/deepseek_webview.rs`
- 创建：`windows/src-tauri/tests/deepseek_history.rs`
- 创建：`windows/src/deepseek-web/bridge.ts`
- 修改：`windows/src/details/DeepSeekDetail.tsx`

- [ ] **步骤 1：写 payload 分片、来源和降级测试**

覆盖官方 HTTPS allowlist、错误 origin、分片重组、超时、格式变化、30 天裁剪、重复日期合并和费用/请求/Token 汇总。历史失败不覆盖余额 API 快照。

- [ ] **步骤 2：实现独立 WebView2 会话**

用户数据目录固定为应用 LocalAppData；不读取浏览器 Cookie；只允许 DeepSeek 官方登录与控制台域名；脚本桥只接收带 nonce 的结构化统计消息，拒绝 Authorization、Cookie、DOM 文本和非 allowlist 导航。

- [ ] **步骤 3：实现详情图表**

提供最近 30 天费用柱状图、总费用、请求数和 Token；图表明确标注官方网页来源和更新时间；失败时显示可重试历史区域，不影响余额及圆环。

- [ ] **步骤 4：Windows 真机登录/刷新验收并提交**

验收日志不得截图或记录手机号、验证码、Cookie、Key、邮箱；仅记录成功/失败状态和脱敏图表。

```bash
git add windows/src-tauri windows/src docs/development/2026-09-03-windows-platform.md
git commit -m "feat: add Windows DeepSeek usage history"
```

---

## 任务 11：实现 Windows 手动更新、NSIS 与双平台发布门禁

**文件：**

- 创建：`windows/src-tauri/src/updater/mod.rs`
- 创建：`windows/src-tauri/src/updater/state.rs`
- 创建：`windows/src-tauri/tests/updater.rs`
- 创建：`.github/workflows/windows-ci.yml`
- 创建：`.github/workflows/release.yml`
- 创建：`scripts/package-cross-platform-release.sh`
- 修改：`windows/src-tauri/tauri.conf.json`
- 修改：`.github/workflows/ci.yml`
- 修改：`scripts/check-public-release.sh`

- [ ] **步骤 1：写更新状态与签名失败测试**

覆盖 idle、checking、up-to-date、available、downloading、ready、failed；只有用户点击 Check 才联网；只有 available 可 Update Now；错误不删除当前版本；无签名、错版本、错 target 和 hash 不一致全部失败。

- [ ] **步骤 2：接入 Tauri Updater**

公钥进入应用配置；私钥只由 GitHub Actions Secret 提供。`latest.json` 同时描述 `windows-x86_64` 和现有 macOS appcast 的同版本事实；Windows 更新前保存非敏感状态、终止采集子进程并退出旧实例。

- [ ] **步骤 3：配置当前用户 NSIS 安装器**

产物名固定：

```text
AI-Token-Meter-X.Y.Z-windows-x64-setup.exe
AI-Token-Meter-X.Y.Z-windows-x64-setup.exe.sha256
AI-Token-Meter-X.Y.Z-windows-x64-setup.exe.sig
```

首个 Preview 可无 Authenticode，但 README 和 Release Notes 必须明确 SmartScreen 提示；稳定版取得证书前不得声称“已签名发布者”。

- [ ] **步骤 4：建立 Windows CI**

`windows-ci.yml` 在 `windows-latest` 执行：

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: 24
    cache: npm
    cache-dependency-path: windows/package-lock.json
- uses: dtolnay/rust-toolchain@stable
- run: npm ci
  working-directory: windows
- run: npm test
  working-directory: windows
- run: npm run build
  working-directory: windows
- run: cargo test --locked
  working-directory: windows/src-tauri
- run: npm run tauri build
  working-directory: windows
```

macOS CI 同时运行共享合同检查。正式 Release workflow 必须等待两个平台 job 成功，再发布双方产物。

- [ ] **步骤 5：执行隔离升级演练**

在 Windows 11 干净用户中从 `0.3.0-preview.0` 手动检查并升级到 `0.3.0-preview.1`；确认下载签名、安装器置前、原位替换、重新启动和设置保留。失败场景用错误签名 feed 证明旧版本仍能启动。

- [ ] **步骤 6：提交检查点**

```bash
git add windows .github scripts
git commit -m "feat: add Windows installer and signed updates"
```

---

## 任务 12：补齐公开文档、验收与 Preview Release

**文件：**

- 修改：`README.md`
- 修改：`CHANGELOG.md`
- 修改：`SECURITY.md`
- 修改：`docs/README.md`
- 修改：`docs/project-status.md`
- 修改：`docs/architecture/overview.md`
- 修改：`docs/architecture/decisions.md`
- 修改：`docs/architecture/repository-structure.md`
- 修改：`docs/security-and-privacy.md`
- 修改：`docs/user-guide/getting-started.md`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/user-guide/troubleshooting.md`
- 修改：`docs/development/testing.md`
- 修改：`docs/development/release-process.md`
- 修改：`docs/development/maintenance-playbook.md`
- 修改：`docs/development/commit-history.md`
- 创建或更新：`docs/development/2026-09-03-windows-platform.md`
- 创建：`docs/assets/screenshots/windows-floating-strip.png`
- 创建：`docs/assets/screenshots/windows-provider-detail.png`
- 创建：`docs/assets/screenshots/windows-settings.png`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：先扩展文档门禁**

要求 README 同时列出 macOS 14+ Apple Silicon 与 Windows 11 x64；安装说明区分 DMG/ZIP 与 NSIS；截图无账户、路径、Key、手机号、邮箱；每个平台的限制、更新签名和 SmartScreen 边界真实可查。

- [ ] **步骤 2：补齐面向用户和维护者的文档**

记录：三 Provider 数据口径；原生/WSL 选择；Credential Manager；CLI 找不到时的一键检测/登录；左右贴边；全屏隐藏；Settings；手动更新；日志目录；卸载与隐私；双平台发版和回滚。

- [ ] **步骤 3：运行完整门禁**

macOS：

```bash
bash scripts/test.sh
bash scripts/build-app.sh release
bash scripts/verify-app-resources.sh dist/AI\ Token\ Meter.app
bash scripts/check-public-release.sh --repository .
```

Windows：

```powershell
npm --prefix windows ci
npm --prefix windows test
npm --prefix windows run build
cargo test --locked --manifest-path windows/src-tauri/Cargo.toml
npm --prefix windows run tauri build
```

共享：

```bash
ruby scripts/check-cross-platform-contracts.rb .
bash scripts/check-docs.sh .
git diff --check
```

- [ ] **步骤 4：执行代码审查与修复循环**

审查正确性、安全、升级回滚、Windows GUI 环境下 CLI 发现、原生/WSL 隔离、无障碍、DPI、文档准确性和秘密泄露。所有 P0/P1/P2 问题关闭后重新跑完整门禁。

- [ ] **步骤 5：发布首个 Preview**

把根 `VERSION`、macOS plist、Windows Cargo/Tauri/package 版本统一为 `0.3.0-preview.1`；创建同一 tag 和 Release，附双平台安装包、SHA-256、Windows updater signature、已签名 macOS 更新资产和 `latest.json`。公开下载后再次核对 hash 与签名。

- [ ] **步骤 6：记录真实状态并提交**

只有 Windows runner 和真机验收均有证据时，`REQ-20260903-004` 才能进入 `待用户确认`；只有用户完成 Windows Preview 验收且双平台稳定 Release 发布后才能标记 `已完成`。

```bash
git add README.md CHANGELOG.md SECURITY.md docs VERSION contracts windows .github scripts
git commit -m "docs: complete Windows preview delivery"
git push -u origin codex/windows-platform
```

---

## 每个任务的统一停止条件

出现以下任一情况时，不继续扩大改动范围：

1. 共享 fixture 在 Swift 与 Rust 中产生不同数据语义；
2. 前端能取得明文 API Key、Cookie、CLI 原始输出或通用 shell 能力；
3. 原生 Windows 与 WSL 的身份或活动被静默合并；
4. Windows 应用在全屏应用之上持续显示，且安全隐藏回退无效；
5. 更新包无法通过 Tauri signature 验证或失败后旧版本不能启动；
6. 正式 Release 只有一个平台产物，或版本号不一致；
7. Windows 原生能力只有 mock 通过、缺少 Windows runner/真机证据。

这些情况必须保留失败证据，把需求保持为 `进行中`、`受环境限制` 或 `待用户确认`，不得使用“已完成”。
