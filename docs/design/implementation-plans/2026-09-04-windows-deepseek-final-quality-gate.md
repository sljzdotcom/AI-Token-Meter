# Windows DeepSeek 最终质量门禁修复实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）跟踪进度。

**目标：** 关闭 `REQ-20260904-006` 最终复审留下的四项 Important，并以新的代码审查、全量本机门禁和 Windows runner 证据恢复合并资格。

**架构：** 后端运行时为 ready、transfer、session 三个生命周期分别持有 generation 绑定计时器，coordinator 保持唯一终态所有权并在真实销毁事件中核销残留清理。前端以单调 attempt 约束初始查询、命令返回、轮询与事件，所有状态通过同一 generation 接收边界；图表和清理失败恢复提示可以同时存在。

**技术栈：** Rust、Tauri 2、Tokio、React、TypeScript、Vitest、Testing Library、Vite、GitHub Actions。

---

## 文件职责

- `windows/src-tauri/src/platform/windows/deepseek_history_window.rs`：纯状态机；暴露分片传输是否启动、传输超时终态和残窗销毁核销。
- `windows/src-tauri/src/platform/windows/deepseek_webview.rs`：拥有三个异步计时器并把 WebView2 回调转为状态机事件。
- `windows/src-tauri/tests/deepseek_history.rs`：对 generation、超时和清理恢复执行真实纯 Rust 行为测试。
- `windows/src/Shell.tsx`：拥有详情实例的 attempt/generation 接收规则以及失败恢复逻辑。
- `windows/src/details/DeepSeekDetail.tsx`：在有无历史两种布局中显示可操作同步状态。
- `windows/src/App.test.tsx`：通过真实 React 组件与 Tauri 边界替身复现异步交错和可达恢复动作。
- `docs/development/2026-09-04-windows-deepseek-history-and-density.md`：记录新周期红绿证据、审查结论和最终验收状态。

### 任务 1：分片传输停滞与残窗核销

**文件：**
- 修改：`windows/src-tauri/src/platform/windows/deepseek_history_window.rs:170-465`
- 修改：`windows/src-tauri/src/platform/windows/deepseek_webview.rs:31-563`
- 测试：`windows/src-tauri/tests/deepseek_history.rs:650-735`

- [x] **步骤 1：编写分片停滞失败测试**

新增测试，点名它要抓的破坏：删除 transfer deadline 后，首片进入 `Waiting` 的 generation 在 20 秒后仍保持 active。

```rust
#[test]
fn stalled_fragment_transfer_claims_failed_and_allows_retry() {
    let mut coordinator = active_coordinator();
    let generation = coordinator.current_generation().unwrap();
    assert!(matches!(
        coordinator.accept_chunk(generation, first_of_two_chunks(), transfer_start()).unwrap(),
        DeepSeekHistoryChunkOutcome::Waiting { transfer_started: true }
    ));
    let terminal = coordinator.claim_transfer_timeout(generation).expect("stalled transfer must fail");
    let execution = execute_window_actions(terminal.actions(), &mut FakeWindowExecutor::successful());
    coordinator.finish_terminal(terminal, &execution);
    assert_ne!(coordinator.open(NEXT_NONCE, retry_time()).generation, generation);
}
```

- [x] **步骤 2：编写完成/旧代次取消测试**

新增测试证明完整分片取得 completed 后，旧 transfer timeout 不能再取得终态；旧 generation 的 timeout 也不能影响重开会话。

- [x] **步骤 3：运行定向 Rust 测试并确认 RED**

运行 `cargo test --manifest-path windows/src-tauri/Cargo.toml --test deepseek_history stalled_fragment_transfer_claims_failed_and_allows_retry` 和完成态定向用例。预期因缺少 `transfer_started`/`claim_transfer_timeout` 接口或行为而失败，不接受测试拼写错误作为 RED。

- [x] **步骤 4：实现最小状态机边界**

让 `DeepSeekHistoryChunkOutcome::Waiting` 携带 `transfer_started`；只在 assembler 第一次进入未完成传输时为 true。增加 `claim_transfer_timeout(generation)`，它只能终止当前、已开始且未进入 Terminating 的会话。增加 `reconcile_destroyed(generation)`，仅在相同 generation 的 `cleanup_pending` 为 true 时释放 session。

- [x] **步骤 5：实现 generation 绑定异步计时器**

在 runtime 增加 `transfer_timeout: Option<(DeepSeekHistoryGeneration, JoinHandle<()>)>`。首个 `Waiting { transfer_started: true }` 后启动 `HISTORY_TRANSFER_TIMEOUT = 20s`；到期调用 `claim_transfer_timeout`。`cancel_timeouts` 同时取消 ready、transfer、session。`Destroyed` 事件先尝试 `reconcile_destroyed`，不能核销时再走正常关闭终态。

- [x] **步骤 6：运行定向与完整 Rust 门禁**

运行完整 `deepseek_history` 测试、`cargo test --all-targets --all-features`、`cargo fmt --check` 和严格 Clippy。预期全部通过且零警告。

- [x] **步骤 7：提交任务 1**

提交信息：`fix(windows): bound DeepSeek history transfer (REQ-20260904-006)`。

### 任务 2：前端 attempt/generation 所有权

**文件：**
- 修改：`windows/src/Shell.tsx:112-284`
- 测试：`windows/src/App.test.tsx:180-470`

- [x] **步骤 1：编写旧初始查询交错测试**

挂起首次 `deepseek_history_status`，点击同步并观察 opening，然后返回旧 idle；断言仍为 opening。它要抓的破坏是初始查询没有保存 attempt 快照。

- [x] **步骤 2：编写命令拒绝但 active 已确认测试**

监听事件先确认 `{ generation: 12, status: "active" }`，随后让 open 命令和恢复查询都 reject；断言仍显示 **Sync in progress**，轮询继续，详情自动隐藏没有恢复。

- [x] **步骤 3：编写合法终态查询统一接收测试**

表驱动覆盖 completed、cancelled、failed，证明命令 reject 后的查询结果保留其 generation/status；failed 显示重试，completed/cancelled 恢复普通状态。

- [x] **步骤 4：运行前端定向测试确认 RED**

运行 `npm --prefix windows test -- --run windows/src/App.test.tsx`。预期至少一个新增用例因现有生产行为失败。

- [x] **步骤 5：实现 attempt 快照与统一失败边界**

初始查询捕获 `const initialAttempt = deepseekHistoryAttempt.current`，返回时要求 attempt 未变化。增加 `hasConfirmedLiveSession()`，仅当当前 generation 非空且状态为 opening/active 时返回 true。命令或查询异常只有在当前 attempt 没有已确认 live session 时写本地 failed；所有合法查询结果先进入 generation 感知接收函数。

- [x] **步骤 6：运行前端测试确认 GREEN**

运行定向 App 测试、完整 `npm --prefix windows test` 和 `npm --prefix windows run build`。预期全部通过。

- [x] **步骤 7：提交任务 2**

提交信息：`fix(windows): own DeepSeek sync attempts (REQ-20260904-006)`。

### 任务 3：已有历史时提供清理恢复入口

**文件：**
- 修改：`windows/src/details/DeepSeekDetail.tsx:5-68`
- 测试：`windows/src/App.test.tsx:160-490`

- [x] **步骤 1：编写图表与失败恢复共存测试**

使用非空 `dailyHistory` 渲染详情并发送 failed 状态；断言图表仍可见、固定错误具有 `role=alert`、**Try again** 可见且点击会调用 `open_deepseek_history`。该测试要抓的破坏是 history 分支提前 return 导致恢复入口消失。

- [x] **步骤 2：运行定向测试确认 RED**

运行 `npm --prefix windows test -- --run windows/src/App.test.tsx`。预期找不到 **Try again** 或 alert。

- [x] **步骤 3：实现共用同步状态组件**

提取仅负责消息和按钮的内部组件。有无历史都使用同一状态规则；有历史时只在 failed、opening、active 或 status channel 不可用时显示紧凑状态条，idle/completed/cancelled 不显示附加卡片。按钮继续遵循 `syncing || !statusPathAvailable` 禁用条件。

- [x] **步骤 4：运行前端与真实密度门禁确认 GREEN**

运行 `npm --prefix windows test`、`npm --prefix windows run test:density` 和 `npm --prefix windows run build`。预期组件测试、production fixture 和计算样式通过。

- [x] **步骤 5：提交任务 3**

提交信息：`fix(windows): expose DeepSeek cleanup recovery (REQ-20260904-006)`。

### 任务 4：整分支审查、全量验证与状态收敛

**文件：**
- 修改：`docs/development/2026-09-04-windows-deepseek-history-and-density.md`
- 修改：`docs/requirements-backlog.md`
- 修改：`docs/project-status.md`
- 修改：`.github/workflows/windows.yml`（仅当 Windows runner 暴露真实构建问题时）

- [x] **步骤 1：逐项自审设计合同**

检查任务 1–3 的 diff，逐项验证四项 Important 的失败路径、状态所有权、计时取消、重试可达性和安全边界；任何 Critical/Important 在继续前修复并添加能抓住该破坏的测试。

- [x] **步骤 2：请求独立代码审查**

以 `3da157e` 为问题基线、当前 HEAD 为修复头，重点模拟迟到查询、命令 reject、缺片、窗口 destroy failure 和重开交错。Critical/Important 必须清零。

- [x] **步骤 3：运行新鲜完整本机门禁**

依次运行 Windows 前端测试、密度浏览器门禁与 production build，Rust 全测试、rustfmt、严格 Clippy，`scripts/test.sh`、无 Widget macOS App 构建、`scripts/check-public-release.sh --repository .` 和 `git diff --check`。所有命令必须退出码 0，并记录真实计数。

- [x] **步骤 4：运行 Windows runner 门禁**

推送功能分支并触发既有 Windows CI，确认 Windows-only Rust/Win32/Tauri Release/NSIS 构建通过。CI 失败必须先定位根因，不得跳过或冒充真机验收。

- [x] **步骤 5：更新文档和需求状态**

把 RED/GREEN、提交、审查和验证证据追加到开发日志。若代码审查与自动化均通过，将 `REQ-20260904-006` 改为 `待用户确认`，下一步限定为 Windows 11 真实登录与交互验收；不得在真机证据前标 `已完成`。

- [x] **步骤 6：提交文档检查点**

提交信息：`docs: record final DeepSeek repair evidence (REQ-20260904-006)`。

- [x] **步骤 7：按分支收尾流程处理集成**

在合并前重新运行完整测试并确认基础分支为 `main`。根据用户既有“修复完成后并入 main”授权，验证全绿且无 Critical/Important 后本地合并；合并结果再次运行必要门禁，再推送并由 Windows main CI 复核。若任何条件不成立，保留分支并准确记录阻塞。
