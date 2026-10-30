# Code0 HTTP401 双连接 core 实施设计（Phase 1 / B 方案）

**登记需求：** REQ-20261030-003
**判定基线：** `docs/design/specifications/2026-10-30-code0-http401-i1-closure-design.md`（12260B，sha256 `9c840f08…`）
**源码基线：** 提交 `5e03674b`，`windows/src-tauri/src/platform/windows/code0_webview/http401_native.rs`（248528B，6880 行）
**本文档性质：** 实施设计 + 待裁定规格变更点。**尚未修改产品源码，尚未编译。**

---

## 1. 现状约束（可证事实）

1. **客户端是真实 WebView2，不由夹具控制**：`:3889` `core.Navigate(PCWSTR(url))` 导航到 `http://127.0.0.1:{port}/code0-http-auth`（URL 组装在 `:3692-3695`）。
2. **端口由系统分配**：`:3089` `sin_port: 0`，`:3222` `bind`，`:3692` 再从 `getsockname` 读回。夹具/测试进程没有任何获取该端口的通道。
3. **模块只在夹具构建中编译**：`code0_webview.rs:503-505` `#[cfg(all(debug_assertions, feature = "code0-native-fixture"))] pub(super) mod http401 { include!(...) }`。
4. **测试进程没有自己的 TCP 客户端**：`tests/code0_native_fixture.rs` 与 `code0_webview.rs` 全文无 `TcpStream`/`TcpListener`。
5. **现状只有单 accept 结构**：`:3488` `s.stream.is_none()` 才预留 `Operation::Accept`；`:3560-3565` accept 成功即写入唯一 `s.stream` 并 `accepts += 1`；`:2527-2532` `inventory()` 也只统计 listener + 单 stream。
6. **既有实际观测**：Windows 红门 `40988017283` 的诊断显示 `accept1/io1/recv0`、`requestValid` 缺失、`auth.callbacks == 0`；台账明确记录「旧 status0 原因不填 preconnect」（即**不得**把首因归因于预连接）。
7. **RED core 的绿色预期文本**（只读参考 `http401_transport_fix2.rs:2280-2312`）：`s.accepts == 2`、`chosen_slot == Some(2)`、`auxiliary_count == 1`、`listener_retirement == Some("capacityReached")`、`slots[0].recv_bytes == 0`、`slots[0].sent_bytes == 0`、`slots[0].seal_final_count == 1`、`slots[0].close_successes == 1`、`slots[1].chosen`、`io_attempts == accept + recv + send`。

**结论：** 在现结构下，「第二条连接」没有任何可确定的物理来源。若直接把 `acceptedTotal == 2` 写成 GREEN 硬判据，该门禁将依赖 Chromium 是否额外开 socket——不可复现，等价于用运气当证据。

---

## 2. 需要裁定的规格变更点

### 变更 A：受控 client-only 连接的来源（必须裁定）

**提议**：在**同一窄差异**内加入受控客户端，使第二条连接真实存在且可复现：

- 新增夹具环境契约 `AI_METER_CODE0_HTTP401_PORT`（可选，仅 fixture 构建读取）：设置时 listener `bind` 到该端口（`:3089` 的 `sin_port` 改为从该值取），未设置时保持 `sin_port: 0` 现状。
- 夹具进程在 WebView2 导航前用非阻塞 `connect` 探测该端口（**真** socket，不是内部记账），作为 aux：连接成功后不发任何字节，随后 `recv` 得到 `WSAEWOULDBLOCK`（nonprogress）再 `recv` 得到 `0`（EOF），再一次性 seal 与一次性 close。
- WebView2 的连接仍是请求载体（primary）。
- 台账措辞「受控 client-only 非阻塞 connect 真实完成原语/资格/预算/typed receipts」「peer 只 AF_INET 127.0.0.1、local 须 exact owned port」与该提议一致；「旧 status0 原因不填 preconnect」也保持遵守：该 probe 是**新门的验证手段**，不是对历史首因的解释。

**未采用方案**：把 aux 记录为「未发生也置 2」（伪造）；或依赖 Chromium 预连接（不可复现）。两者都违反证据分层。

**若不同意 A**：则 I1-3a 需退为 `acceptedTotal ∈ {1,2}`，两连接语义只在受控 transport 测试里证；代价是 native WebView2 门禁不再证明「两连接所有权」。**请选择 A 或退让方案**。

### 变更 B：seal 的物理语义（建议按此落地）

证据：RED core oracle 要求 `slots[0].sent_bytes == 0` 与 `seal_final_count == 1` 同时成立；台账 M2 记录「每次 seal 最多 1 byte，Interrupted 消耗同 global budget 并后续 turn 继续，只一个合格终 seal」。

**提议定义**：seal = 一次长度 `0..=1` 的 `send`（0 字节合法），
- `seal_attempt_count` 每次预留 +1；
- `seal_final_count` 在「该次 send 返回成功（含 0 字节）」或「最后一次合格尝试」时置 1；
- `seal_class ∈ {sealedZero, sealedOne, wouldBlock, interrupted, failed}`；
- seal 走同一 2048 全局预算与 25ms 节奏，不另开通道。

**若不同意 B**：alternative 是用 `shutdown(SD_SEND)` 作 seal（`sent_bytes` 保持 0），但那是另一类原语，需明确写入规格。

---

## 3. 状态机（两 slot）

`Slot[i]`（i ∈ {0,1}）：`lease: Option<SocketLease>`、`recv_bytes/recv_attempts/send_attempts/nonprogress/eof/seal_attempts/seal_final/sent_bytes/close_attempts/close_successes`、`chosen: bool`、`owned_at_final: bool`。
Instance 新增：`accepted_total`、`chosen_slot: Option<u8>`（**1-based 序号**，与 RED core 的 `chosen_slot == Some(2)` 一致）、`first_chunk_bytes`、`socket_peak`、`listener_retirement: Option<&'static str>`。

操作序（`drive` 内）：

1. `setup 0..8`（9 次，现状不变）。
2. 若 `accepted_total < 2` 且 listener 仍拥有 → 预留 `Accept`。每次成功 accept 分配下一个空 slot，`accepted_total += 1`，`socket_peak = max(socket_peak, listener_owned + 已接受数)`。
3. **accept #2 成功后立即**：记录 `socket_peak`，`listener_retirement = Some("capacityReached")`，执行 listener 一次性 close（`listener_close_attempts/successes` 各 1）——「先记真实责任 peak 再 listener close」。
4. 每个 slot 独立推进自己的 setup 4 步 → 收请求 / 发响应：
   - 首次出现 `length > 0` 的接收且 `chosen_slot.is_none()` → 该 slot 成为 primary（`chosen = true`、`chosen_slot = Some(ordinal)`、`first_chunk_bytes = length`），**此后不可改**；
   - primary：`recv`（`512.min(4096 - received)`）→ `parser` 解析 → `send` 响应（139 字节总量）→ `response_complete`；
   - aux（非 primary 且 `recv_bytes == 0`）：`recv` 期望 would-block（`nonprogress += 1`）→ `recv` 期望 `0`（`eof = 1`）→ seal（见变更 B）→ 一次性 close。
5. primary `response_complete` 后：primary 一次性 close；若 listener 尚未退役（只来了 1 条连接）则在收尾路径退役，`listener_retirement = Some("drained")`。
6. 终态：两 slot `owned_at_final == false`、`sockets_remaining == 0`、`server_threads == 0`、`server_joins == 0`。

**预算分配**：accept / recv / send 共用 `budget`（2048 全局、25ms 节奏、45s attempt、10s Stop、75s outer 不变）；`io_attempts` 继续等于三类之和。

---

## 4. schema 与 CI 同步清单（同一窄差异）

1. `record!(Server {...})`（`:240`）：把单连接字段 `accepted_local_matched`/`accepted_peer_matched`/`stream_linger_off`/`stream_nonblocking`/`stream_close_attempts`/`stream_close_successes` 改为 per-slot 数组或保留合计 + 新增 `slots: [TransportSlot; 2]`；新增 `accepted_total`、`chosen_slot`、`first_chunk_bytes`、`socket_peak`、`listener_retirement`。
2. `Facts::native_positive()`（`:942`）/`positive()`（`:1048`）：加入第 2 节列出的 I1-1/2/3 谓词。
3. 诊断行：`Code0NativeHttp401FirstFailure schema=` 与 `Code0NativeDiagnostics schema=` 版本号递增，位域位宽复核。
4. **CI 验收脚本（同一提交）**：`.github/workflows/windows-ci.yml` schema16 HTTP401 v2 acceptance 的 `SERVER_TRUE`/`SERVER_ONCE`/`RANGES` 需与 per-slot 结构一致；`accepts` 由 once 变 per-slot 之和；`streamClose*` 变两 slot 之和且各 slot == 1。**禁止只改 Rust 让 CI 变绿，也禁止只改 CI。**
5. 旧 46 项测试定义 + 17 项边界定义全部保留；新增边界仅追加。

---

## 5. RED / GREEN 判据

**RED（当前单 accept core 上必须失败）**：`exit == 1` ∧ 失败断言 ⊆ I1 ∧ 原因类 ∈ {missing_field, missing_observation} ∧ 既有断言仍通过；典型失败应为 `acceptedTotal == 2`（观测 1）或 `aux eof_count == 1`（观测 0），不得是超时/panic/环境缺失。

**GREEN**：`RED_VALID` 先成立 ∧ I1 全真 ∧ 既有 `positive()` 全真 ∧ 旧 46+17 定义全绿 ∧ 物理 ACTUAL（Windows runner 原生 binary、真实 socket）∧ 双 Closed ∧ `io_attempts == accepts + recv + send ≤ 2048`。

**顺序**：实现 → `cargo fmt --check` → `cargo clippy`（Windows runner）→ 自测 → 回传 PM 另派独立审查 → RED → GREEN → 与 F 窄差异组合 → Windows 原生实测。

---

## 6. 风险

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| 本机无法编译（rustc 1.64、无 windows target） | 只能靠 CI 编译，迭代慢 | 小步提交、每步跑 Windows runner；不在本机声称编译通过 |
| WebView2 连接时序（导航 vs probe） | 可能 primary 先到、aux 后到 | primary 定义只看「首个非空 chunk」，与到达顺序无关；aux 只要求「非 primary 且零字节」 |
| 端口注入改动 bind 行为 | 可能影响现有 373 项门禁 | 仅在设置 env 时生效；未设置保持 `sin_port: 0` |
| 两连接同时读写 | 单 parser 冲突 | 保持单 parser 只由 primary 使用；aux 不进入 parser |
| CI acceptance 与 Rust 不同步 | 假绿 | 同一提交、同一审查 |

---

## 7. 待确认

1. **变更 A**（受控 client-only 连接 + 可选端口注入）是否批准？不批准则 I1-3a 退让为 `acceptedTotal ∈ {1,2}`。
2. **变更 B**（seal = ≤1 字节 send，0 字节合法）是否批准？不批准请指定 `shutdown(SD_SEND)` 版语义。
3. 端口注入成功后，aux 是否允许出现在 accept #1（probe 先连）并因此成为 slot 0（`chosen_slot == Some(2)`）？本设计按「slot 序号 = 接受顺序、primary 由首个非空 chunk 决定」处理，两种顺序都应成立。
