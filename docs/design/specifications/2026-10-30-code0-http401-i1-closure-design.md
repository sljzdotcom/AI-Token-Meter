# Code0 HTTP401 原生夹具 I1 缺口闭合设计（判定规格）

**登记需求：** REQ-20261030-003（接续 REQ-20261026-004）
**基线：** 提交 `5e03674b10ec0911d424f4c1d3f0539de7052419`，`windows/src-tauri/src/platform/windows/code0_webview/http401_native.rs`（248528B，6880 行）
**目标口径来源：** 另一台机器上的 `frozen-fix1` 候选（N 238118B/sha256 `fb5c0c12…`、core 152821B/`692be863…`）与现行台账 2026-10-30 记录；**该候选在本机不可达**（全库对象扫描无对应 blob），故本文件按台账语义把判定条件写成可实施规格。

**本文档性质：只写判定规格与清单，不含实现。** 不修改产品源码，不替代独立审查，不构成 GREEN 结论。文中所有行号对应上述基线提交。

---

## 1. 三项 I1 缺口与现有源码坐标

现行 `Facts::native_positive()`（`http401_native.rs:942`）与 `Facts::positive()`（`:1048`）对服务端只有**单连接、总计**判据，因此下列三处缺口无法被现有断言发现。

### 缺口 1 — 首包与总字节关系缺判据

- 接收记账：`:3637-3639`，`Operation::Receive` 成功时 `s.received = s.received.saturating_add(outcome.length)`，并把 `s.facts.server.request_bytes = s.received.min(4097)`。
- 单次接收上界：`:3497-3500`，`s.received < 4096` 时预留 `Operation::Receive(handle, 512.min(4096 - s.received))`，即单次最多 512 字节 scratch。
- 发送记账：`:3642-3645`，`sent_bytes` 累加并 `.min(140)` 截断。
- **缺口实质**：没有任何字段记录「首个非空接收 chunk 的大小」，也没有 `first_chunk_bytes <= received_bytes` 的约束。当前 `request_bytes > 0 && <= 4096`（`:1029` 附近列表）无法排除「首包记为 4096、实际只收到 1 字节」或「首包被重复计数」这类记账错误；`sent_bytes == 139` 也无法区分「一次 139 字节」与「139 次 1 字节」之外更严重的错误分配。

### 缺口 2 — 收发/非进展次数的物理可行分配缺判据

- 非进展判定：`:3627-3631`，`matches!(outcome.error, Some(WSAEWOULDBLOCK | WSAEINTR))` 且操作为 Accept/Receive/Send。
- IO 预算：`:3505-3520`，`s.budget.reserve(...)` 失败即 `overflow = true`；`s.facts.server.io_attempts = s.budget.io`。
- 现有判据仅 `(3..=2048).contains(&s.io_attempts)`（`:1029` 列表附近），**没有** per-连接/per-操作类别的次数分配，也没有「次数 × 单次上界 >= 记账字节」这类可行性约束。
- **缺口实质**：`io_attempts` 是一个混合总数，无法排除「recv 次数为 0 却 recorded 了字节」「nonprogress 次数超过总 IO 次数」「recv 字节数超过 512 × recv 次数」等物理不可能的组合，也无法证明收发预算按 2048 上限真实分配。

### 缺口 3 — 辅助零字节连接（recv = nonprogress + EOF）完全缺失

- 结构：只有单个 `s.stream: Option<SocketLease>`（`:3488` 分支 `s.stream.is_none()` 才预留 `Operation::Accept`），accept 成功即 `s.stream = Some(...)`、`s.facts.server.accepts += 1`（`:3560-3565`）。
- 终态：仅按单 socket 归零判据 `sockets_remaining == 0`。
- **缺口实质**：没有第二个 accept、没有 aux 连接的读次数/非进展/EOF 记账，也没有「listener 在真实责任峰值后才 close」与「两个连接各自 exactly-once close」的判据。台账要求的 `aux one-byte seal 与 once close`、`N 最终双 Closed` 在当前代码里无落点。

---

## 2. 需要新增/修改的 predicate 清单

命名约定：`T` = 传输观测（transport observation，两 slot）；`T.slots[i]` = 第 i 个 accept 的 slot（`i ∈ {0,1}`）；`P` = primary（首次产生非空 chunk 的连接，一旦确定不可改）；`A` = aux（另一连接）。

### 2.1 需要新增的 schema 字段（对应 RED core 已有的观测集合）

| 字段 | 类型 | 语义 |
| --- | --- | --- |
| `acceptedTotal` | `u16` | 累计成功 accept 次数，目标 **2** |
| `chosenSlot` | `Option<u8>` | primary 的 slot 序号，immutable |
| `auxiliaryCount` | `u16` | aux 连接数量，目标 **1** |
| `firstChunkBytes` | `u16` | 首个非空接收 chunk 的实际字节数 |
| `socketPeak` | `u16` | 真实责任峰值（accept 2 时的在管 socket 数） |
| `listenerRetired` / `listenerRetirement` | `Option<bool>` / `Option<&'static str>` | listener 在峰值之后才关闭，并记录关闭类别 |
| `slots[0..1].accepted` / `.chosen` / `.ownedAtFinal` | `bool` | slot 归属与终态责任 |
| `slots[i].recvAttempts` / `.sendAttempts` / `.nonprogressCount` / `.eofCount` | `u16` | per-连接次数分配 |
| `slots[i].recvBytes` / `.sentBytes` | `u16` | per-连接字节记账 |
| `slots[i].sealAttemptCount` / `.sealFinalCount` / `.sealClass` | `u16` / `u16` / `Option<&'static str>` | aux one-byte seal |
| `slots[i].closeAttempts` / `.closeSuccesses` | `u16` | exactly-once close |

> 影响面提示：这些字段进入 `record!` 结构会改变夹具 schema 版本（现行诊断行 `Code0NativeHttp401FirstFailure schema=1`）。schema 号变更必须与 CI 验收脚本（fixture binding / v3 规则）在同一窄差异内同步，不得只改一侧。

### 2.2 新增 predicate（编号沿用缺口分组）

**I1-1 首包与总字节关系**
- `I1-1a` `first_chunk_bytes >= 1`：primary 必须存在首个非空 chunk（空 chunk 不算首包）。
- `I1-1b` `first_chunk_bytes <= received_bytes`：首包计入总量，不得重复计数。
- `I1-1c` `received_bytes == request_bytes`：接收记账与对外报告的请求字节一致（`request_bytes` 不得用 `min(4097)` 掩盖越界）。
- `I1-1d` `request_bytes >= 1 && request_bytes <= 4096`（保留现值，改为与 1c 联立）。
- `I1-1e` `sent_bytes == 139` 且 `sent_bytes == Σ slots[i].sentBytes`：139 字节必须落在真实 slot 上，不得只留总数。
- `I1-1f` `first_chunk_bytes <= first_recv_bound`，其中 `first_recv_bound = min(512, 4096)`：首包不得超过 scratch 上界。

**I1-2 收发/非进展次数的物理可行分配**
- `I1-2a` `Σ slots[i].recvAttempts + Σ slots[i].sendAttempts <= io_attempts`：per-连接次数不得超出混合总数。
- `I1-2b` `io_attempts <= 2048`（保留现值 `3..=2048` 作下界语义不变）。
- `I1-2c` `slots[i].recv_attempts >= 1`（两个连接都必须被真实读取过）。
- `I1-2d` `slots[i].recvBytes <= 512 * slots[i].recvAttempts`：字节数必须能被次数 × scratch 上界解释。
- `I1-2e` `slots[i].sentBytes <= 139 * slots[i].sendAttempts`（成功发送每次至少 1 字节）。
- `I1-2f` `slots[i].nonprogressCount <= slots[i].recvAttempts + slots[i].sendAttempts`：非进展不可能多于尝试总数。
- `I1-2g` `slots[i].eofCount <= slots[i].recvAttempts`：EOF 只能由接收观察到。
- `I1-2h` `receivedBytes == Σ slots[i].recvBytes`，`sendAttempts == Σ slots[i].sendAttempts`，`recvAttempts == Σ slots[i].recvAttempts`：总计与分配必须闭合。
- `I1-2i` primary 的 `recvAttempts >= 1` 且 `recvBytes >= 1`；aux 见 I1-3。

**I1-3 辅助零字节连接（recv = nonprogress + EOF）**
- `I1-3a` `acceptedTotal == 2` 且 `auxiliaryCount == 1`：真实发生第二个 accept（不是伪造 slot）。
- `I1-3b` `chosenSlot` 恰好指向一个 slot，且该 slot 与 `first_chunk_bytes > 0` 的观测一致；一旦确定不得改（immutable winner）。
- `I1-3c` aux：`recvBytes == 0`。
- `I1-3d` aux：`nonprogressCount >= 1` 且 `sealClass` 记录为 would-block/interrupted 一类，**至少一次非进展读**。
- `I1-3e` aux：`eofCount == 1`，即读到真实 EOF（`recv == 0`）。
- `I1-3f` aux：`recvAttempts == nonprogressCount + eofCount`（台账的 `R = N + EOF`）。
- `I1-3g` aux：`sealFinalCount == 1` 且 `sealAttemptCount >= 1`（one-byte seal 恰好一次落定）。
- `I1-3h` aux：`closeAttempts >= 1 && closeSuccesses == 1`（exactly-once close）。
- `I1-3i` primary：`closeSuccesses == 1`（同样 exactly-once）。
- `I1-3j` `listenerRetired == Some(true)` 且 `socketPeak >= 2`：listener 在真实责任峰值之后才退役。
- `I1-3k` 终态双 Closed：`slots[0].ownedAtFinal == false && slots[1].ownedAtFinal == false`，且 `sockets_remaining == 0`、`server_threads == 0`、`server_joins == 0` 保留现值。
- `I1-3l` 两个 slot 的 `accepted` 均为真、`accepted_local_matched`/`accepted_peer_matched` 逐 slot 复核（不得只验一个）。

**2.3 不得改动的既有约束（回归面）**

`45s attempt`、`10s Stop`、`75s 外层看门狗`、`2048` 全局次数、`25ms` 服务节奏、请求 `4096`、接收 scratch `512`、响应 `139`、完整摘要 `8192` 字节；现有 `positive()` 的 `cleanup` 六项、`target_at_ms < 45000`、`elapsed < 10_000`、`final - close ∈ {elapsed, elapsed+1}`、`post_target.final_order` 全部保留；旧 46 项测试定义与 17 项边界定义一条不得删。

---

## 3. 受控 RED 判决定式

RED 的目的是**按预期暴露目标缺陷**，不是随便失败。在仅含现行单 accept core 的源码上运行目标夹具时，必须同时满足：

1. 进程以语义退出码 **1** 结束（`2` = 不完整、`124` = 看门狗强退，均不算 RED 成立）。
2. 唯一失败断言落在 I1 组内，且诊断输出点名具体谓词，例如 `I1-3a acceptedTotal == 2` 或 `I1-3d aux nonprogressCount >= 1`。
3. 失败原因是**观测值不足**（`acceptedTotal == 1`、`auxiliaryCount == 0`、`firstChunkBytes` 字段缺失），而不是超时、panic、环境缺失或断言自相矛盾。
4. 其余既有断言（含 `sent_bytes == 139`、`request_bytes ∈ (0,4096]`、cleanup 六项）在本轮**仍然通过**，证明 RED 是窄的、不是整体崩溃。

判定式：`RED_VALID := (exit==1) ∧ (failing ⊆ I1) ∧ (reason_class == missing_field|missing_observation) ∧ (existing_assertions_pass==true)`。

---

## 4. GREEN 判决定式

1. `RED_VALID` 已在**同一源码谱系**上先成立（先红后绿，禁止跳过）。
2. `I1-1*`、`I1-2*`、`I1-3*` 全部为真。
3. 既有 `native_positive()` 与 `positive()` 全部为真（含 `phase == "Closed"`、`session.failures == 1`、`reason == "PolicyRejected"`）。
4. 旧 46 项测试 + 17 项新增边界定义全绿；无断言删除、无记录截断、无字段省略、无超时放宽。
5. 物理侧证（`ACTUAL`）：Windows runner 上真实原生 binary 执行，`acceptedTotal == 2` 与 aux 的 `R = N + EOF` 由真实 socket 行为产生，不是夹具注入的常量。
6. 终态双 Closed：两 slot 均无 owner、无遗留 socket/线程/join。

判定式：`GREEN := RED_VALID ∧ I1_all_true ∧ existing_positive_true ∧ tests_all_green ∧ physical_evidence ∧ dual_closed`。

`C0/I0/M0` 只对报告明确范围有效；本文档为 `SOURCE` 规格，不构成任何 `ACTUAL`/`GREEN` 结论。

---

## 5. 与 F 候选窄差异的组合终态要求

- F 的 `schema15|16` 完整 `8192` 长度限制已单独源码接受，本设计**不修改**该限制。
- 组合时两侧差异必须窄：I1 组只新增字段/谓词与两连接策略，不得顺手改 `cleanup`、`navigation`、`auth` 语义。
- schema 版本变更（若引入）与 CI 验收脚本（fixture binding、v3 规则）必须在同一提交内一致，避免"改一侧就绿"。
- 组合后必须重跑 `RED_VALID` 与 `GREEN` 判定式，不接受"分别通过即组合通过"。

---

## 6. 未决项与风险

1. **本机无法编译**：`http401_native.rs` 只在 `cfg(all(debug_assertions, feature = "code0-native-fixture"))` 下编译，且本机 rustc 1.64.0、无 `x86_64-pc-windows-msvc` target、无 Windows 环境；因此本文档所有判据目前只能是 `SOURCE`，编译/Clippy/ACTUAL 必须走 Windows runner。
2. **基包缺失**：本规格以台账语义为准，与另一台机器上的 `frozen-fix1` 逐字差异尚未做过（不可达）。若后续取回基包，需以本文档为对照做一次窄差异核对。
3. **RED core 已有可复用字段**：`http401_transport.rs`（RED 分支 `99504B`/`606f3818…`）已具备 `firstChunkBytes`、per-slot 计数与 `slots:[2]`，可作为实现参照，但它是单 accept 的 legacy core（文件头自述），不能直接当作 GREEN。
