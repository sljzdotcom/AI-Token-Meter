# macOS-only 发布计划（REQ-20261030-004）

**登记需求：** REQ-20261030-004（Windows 降为最低优先级；macOS 全做完并在 GitHub 发布后，才恢复 Windows）
**依据：** 用户裁定（2026-10-30 13:0x SGT，PM 转述）＋ 提交 `5e03674b` 实测 ＋ 本机环境实测
**本文性质：** 只出计划，**不动代码、不动 CI、不动 PR#67**。所有结论标注证据来源；未验证项明确列为未验证。

---

## 0. 三个先决事实（决定后面所有方案）

1. **两个平台是两棵独立的树**：macOS 是 Swift（`Sources/`、`Tests/`、`Package.swift`），Windows 是 Tauri（`windows/`）。`a97c2a58..5e03674b` 共 208 提交 / 168 文件 / +86494；其中 macOS+共享子集为 **71 文件 / +11730 / −168**，其余全在 `windows/`。
2. **唯一强耦合是版本契约**：`scripts/check-cross-platform-contracts.rb:40-59` 要求 `VERSION` == macOS `Info.plist` == `windows/package.json` == `windows/src-tauri/tauri.conf.json` == `tauri.release.conf.json` == `tauri.preview.release.conf.json`（+ Cargo 版本），并检查两个锁文件存在。**因此任何 macOS 版本升级必须同时改 `windows/` 的版本元数据（仅数字），这不是"启用 Windows"。**
3. **`release.yml` 目前是双平台同步发布**：`publish` 的 `needs: [macos-preflight, windows-release]`，且 publish 步骤包含上传 Windows 资产、更新 Windows preview feed。**macOS-only 发布必须给它加模式，否则不可能只发 macOS。**

另有两条本机实测（影响 ⑤）：`security find-identity -v -p codesigning` → **0 valid identities**；`swift --version` → **Apple Swift 6.4 / Xcode 27.1**（CI 用 macos-15 + Xcode 26.3）。

---

## ① 剩余范围：macOS 侧还差什么

| # | 事项 | 证据 | 分级 |
| --- | --- | --- | --- |
| 1 | Code0 macOS 功能层（AppRuntime/WebSession/NativeWebDriver/BridgeProtocol/SnapshotStore/ProfileLease + 详情页/服务页/logo/三语本地化）已在 `5e03674b` 落地，App 接线与 12 项窗口回归在 `4838bbc` 已定向验收 | `docs/design/implementation-plans/2026-10-26-code0-monitor.md:91` | 已完成（源码层+定向实际） |
| 2 | 真实媒体权限委托与 TCC 分支未覆盖：隐藏文档延后媒体/定位、可见媒体系统先行拒绝→WebKit 委托、无视频设备前置条件 | `docs/design/specifications/2026-10-26-code0-monitor-design.md:64`（原文明确"媒体权限委托与TCC分支明确未覆盖，不据此声称真实拒绝"） | 【已知限制】写进 release notes |
| 3 | 真实 TLS 握手 / client-cert 回调先后未观察；受控路径只到 loopback 401 | 同 spec `:64` 与 plan `:124` | 【已知限制】 |
| 4 | 真实账号、真实余额/累计消耗读取未验收（全程合成夹具） | plan `:91`「不视为真实账户验收」 | 【已知限制】＋发布说明写明"首次需用户自行登录官方网页" |
| 5 | macOS 原生 401/redirect/popup/download/dialog/permission 的组合验收：N1–N5 已有限验收，非"完整原生覆盖" | plan `:286`/`:336` 的有限 C0/I0/M0 记录 | 【已知限制】（不得说成完整覆盖） |
| 6 | Task5 最终全量独立审查（macOS 部分：源码 + 测试 + 文档 + 门禁） | 未做 | **【阻塞发布】** |
| 7 | 完整测试门禁：`bash scripts/test.sh`、`ruby scripts/check-cross-platform-contracts.rb .`、`bash scripts/check-docs.sh "$(pwd)"`、`node --test web/code0-bridge.test.mjs`、`bash scripts/test-code0-native-fixture.sh`、`scripts/check-public-release.sh` | `.github/workflows/release.yml:54-58` 即发布前必跑项 | **【阻塞发布】** |
| 8 | 版本与物料：`VERSION`、`Info.plist`（shortVersion+build）、README 徽章、`appcast.xml`、release notes | 现为 0.10.11 / build 39 | **【阻塞发布】** |
| 9 | 签名与公证 + zip/dmg 产物（含 Sparkle EdDSA 签名） | `scripts/build-app.sh` 需 codesign 身份；`scripts/package-update-release.sh` 需 Sparkle `generate_keys` | **【阻塞发布】**，且本机 0 identities |
| 10 | `release.yml` macOS-only 模式（去掉 windows-release 与 Windows 资产步骤） | `release.yml:173` `needs: [macos-preflight, windows-release]` | **【阻塞发布】** |
| 11 | Windows 侧一切（HTTP401 双连接 GREEN、S5 工厂六路径、`40988017283` 红门） | 本轮已 STOP | 不阻塞 macOS；**不得进入本版产物** |

**结论：** macOS 侧功能已完成，剩下的是"验证 + 物料 + 发布管线"三件事，没有未知的功能缺口；媒体/TCC/TLS/真实账号四项是**已知限制**，不是阻塞项。

---

## ② 发布内容：Code0 是否随本次 macOS 版本发布

**建议随发布（第五个服务）**，理由与边界：

- 用户 003 的顺序本就是"macOS 全做完并发布"，Code0 是这套 macOS 工作的唯一新增服务；不发布等于这套工作没有交付物。
- 已有证据支持可以发布：功能层源码 + App 接线定向验收 + 12 项窗口回归 + 有限原生夹具（N1–N5）+ 合成账号边界。
- 发布时必须同时满足三条：
  1. **Windows 侧不随本版启用**：`windows/` 只做版本数字同步，不合并任何 Code0 功能代码；Tauri 前端 `windows/src/**Code0*` 不进入本版。
  2. **release notes 明写**：Code0 为 macOS 新服务；Windows 本版未发布、无 Code0；媒体/TCC/TLS/真实账号为已知未覆盖边界。
  3. **服务默认不自动采集**：首启不自动连接，需用户在设置里显式连接官方网页并登录（设计即如此），release notes 说明。

---

## ③ 分支与合并路径（最关键）

**不采用整支合并 `5e03674b → main`**，理由是可证的：该提交携带 Windows 红门（HTTP401 native fixture 在 `40988017283` 已接受 ACTUAL 失败）与 S5 未完成路径，整支合并会让 `main` 的 `windows-ci`（`windows-ci.yml` 对 `main` push 触发，`:4-8`）转红，且直接把未完成 Windows 路径送进发布物。

**采用"路径级子集合并"（方案 A）**：

1. 从 `origin/main`（`a97c2a58`）建发布分支，例如 `release/macos-0.11.0`。
2. 把 macOS+共享子集按路径取入：`Sources/`、`Tests/`、`Package.swift`、`contracts/`、`web/`、`docs/`、`.ci/`、`.github/workflows/ci.yml`、`.github/workflows/code0-dispatcher-mutations.yml`、`scripts/`（含 `sync-code0-bridge.py`、`test-code0-native-fixture.sh`、`test-code0-dispatcher-mutations.py`、`test.sh`、`check-cross-platform-contracts.rb`）。实现方式：`git checkout 5e03674b -- <这些路径>`（或 `git merge --no-commit` 后把 `windows/` 路径 reset 回 main），**历史不回写、不 squash**，提交信息注明"macOS-only 子集，Windows 未包含"。
3. **版本元数据只同步数字**：`VERSION`、`Sources/AIMeterApp/Resources/Info.plist`、`windows/package.json`、`windows/src-tauri/tauri.conf.json`、`windows/src-tauri/tauri.release.conf.json`、`windows/src-tauri/tauri.preview.release.conf.json`、`windows/src-tauri/Cargo.toml`。
4. **`release.yml` 增加 macOS-only 模式**（`workflow_dispatch` 输入 `platform: macos|both`）：
   - `windows-release` job 加 `if: inputs.platform == 'both'`；
   - `publish` 的 `needs` 改为在 macos-only 下只依赖 `macos-preflight`；
   - Windows 资产下载/校验/上传、Windows preview feed 更新等步骤按同一输入跳过；
   - **不跳过任何 macOS 门禁**（版本/tag 校验、`scripts/test.sh`、契约检查、原生夹具、已签名资产校验全部保留）。
5. **Windows 红门在合并后为何不阻塞**：因为 `windows/` 保持 `origin/main` 的版本，本分支**不引入**任何 Windows 源码改动，`windows-ci` 的既有结论不变（仍是通过的旧版本）；HTTP401/S5 继续留在 PR#67（Draft）作为后续批次载体，恢复 Windows 时再处理。**不允许**通过删断言、跳过测试、`continue-on-error`、放宽超时让它"变绿"。
6. `pr#67` 保持 Draft，不合并、不改状态。

**执行记录（2026-10-30，实际落地与本文的差异）**：

- 实际取用路径 = `.github/workflows/ci.yml`、`.github/workflows/release.yml`、`AGENTS.md`、`Package.swift`、`Sources/`、`Tests/`、`contracts/`、`docs/`、`web/`、`scripts/check-cross-platform-contracts.rb`、`scripts/sync-code0-bridge.py`、`scripts/test-code0-native-fixture.sh`、`scripts/test.sh`（15 项，`windows/` 零源文件改动）；首取 84 文件 +15525/−220，版本物料与发布说明另计。
- 偏离 1：`.ci/**`、`scripts/collect-code0-native-loader.ps1` 未取——二者仅被 `.github/workflows/windows-ci.yml` 使用，而该 workflow 保留 `main` 版本。
- 偏离 2：`.github/workflows/code0-dispatcher-mutations.yml` 与 `scripts/test-code0-dispatcher-mutations.py` 曾按第 2 步取入，随后移除。原因：该 workflow 精确校验 `windows/src-tauri/src/platform/windows/code0_dispatcher.rs` 的已评审 SHA，而本发布分支不含该源码，Draft PR 首跑（run `41006300512`）以 `Checkout bytes are not the reviewed committed source; mutation opt-in refused` 退出 1；该脚本 docstring 本身限定 Windows CI 使用；两文件在 `main` 均不存在，故本发布不新增，Windows 批次再随源文件一并引入。
- 偏离 3：版本数字实际落地范围以 `scripts/check-cross-platform-contracts.rb` 为准，除第 3 步所列外还需 `windows/package-lock.json`（两处）、`windows/src-tauri/Cargo.lock`，以及 `Sources/AIMeterWidgetExtension/Resources/Info.plist`（Widget 与宿主版本一致性断言）。
- 偏离 4（含一次被证伪的假设，必须记录）：第 5 步「`windows/` 保持 `origin/main` ⇒ `windows-ci` 结论不变」一度被证伪。`5e03674b` 把 Code0 条目写进了共享 `contracts/presentation/providers.json`，并新增 `contracts/fixtures/code0-site-credits.json`；而 Windows 运行时用 4 变体 `ProviderId` 解码这份共享契约（两处 `include_str!`：`windows/src-tauri/src/lib.rs:32`、`windows/src-tauri/src/domain/presentation.rs:7`），于是 Windows CI 连红四次（`41006482385`、`41007442895`、`41009318287`、`41010012618`）。中途曾用「测试侧显式排除 code0 夹具 + 目录期望跟随」修补（`18faf8be`、`c8d4f31e`），被独立审查判定为**用测试掩盖产品缺陷**并否决；该修补已整体 revert，不保留其写法。
- 偏离 5（契约按平台拆分，取代上述修补）：共享契约只描述两个平台都实现的服务——`contracts/presentation/providers.json` 回到 `main` 的 4 项（两侧运行时都解码这一份），Code0 展示条目移到 `contracts/presentation/providers.macos.json`（`schemaVersion 1` + `platform: "macos"`）；`contracts/fixtures/code0-site-credits.json` 移到 `contracts/fixtures/macos/` 子目录，共享夹具回到 6 个。`scripts/check-cross-platform-contracts.rb` 随之分两段校验：共享 6 fixture + 4 provider 与 `main` 一致；macOS-only 1 fixture + 1 provider 且**只允许 `code0`**；再用合并结果断言仍是 5 项、顺序/displayName/进度语义/logoKey 唯一性与 snapshot schema enum 不变。该门禁用两个负例验证过会红（macOS 文件混入第二个 provider、共享夹具多加一个）。`windows/src-tauri/tests/shared_contract.rs` 全量回退 `main`；`app_metadata.rs` 只保留 `0.11.0` 版本断言与此前新增的 `windows_service_set_never_accepts_code0` 守卫（断言 Windows 域只有四个服务、`code0` 快照解码必须报错）。影响面结论：`Sources/` 对 `providers.json` 零引用，macOS 生产代码不读它，故本次拆分对 macOS 运行时零影响，是纯契约/测试层动作。
- 第 4 步（`release.yml` 的 `platform: macos|both`）已实现：`windows-release` job 整体 `if: inputs.platform == 'both'`（显式跳过，非删除）；`publish` 改为 `!cancelled() && needs.macos-preflight.result == 'success' && (inputs.platform != 'both' || needs.windows-release.result == 'success')`；`publish` 内四个 Windows 步骤逐个加 `if: inputs.platform == 'both'`；并新增 `Publish the verified macOS draft release`（仅 `platform == 'macos'`）——原流程只有 Windows 步骤才会把 draft 转为公开，macOS-only 必须补这一步，且它只在 macOS 资产（zip、`.sha256`、`appcast.xml`）齐备时公开。macOS 门禁一项未减少。
- Draft PR #68 仅用于在 macOS runner 上取得权威门禁结论，保持 Draft、不合并、不转 Ready。

**风险与反例**：若有人把 `windows/` 一并带入（例如为了"契约一致"而整目录 checkout），则红门随之进入 main，方案失效——所以第 2 步必须逐路径列出，并在提交上做一次 `git diff --stat origin/main` 复核"无 `windows/src` 变更"。

---

## ④ 版本与物料清单

| 物料 | 现值 | 目标 | 产出方式 |
| --- | --- | --- | --- |
| `VERSION` | 0.10.11 | 0.11.0 | 手工提交（唯一真源） |
| `Sources/AIMeterApp/Resources/Info.plist` | 0.10.11 / 39 | 0.11.0 / 40 | 手工提交 |
| `windows/*` 版本元数据（5 处） | 0.10.11 | 0.11.0 | 同一提交，仅数字 |
| `README.md` 徽章 | version-0.10.11 / tests-653 | version-0.11.0 / tests-886 | 跑完 `scripts/test.sh` 后取实际通过数；本次实测 886 |
| `appcast.xml` | 最新 0.10.11/39 | 新增 0.11.0/40 条目 | `scripts/package-update-release.sh` 生成（含 `length` 与 `sparkle:edSignature`）；`release.yml` 在资产公开后提交该文件 |
| `preview-appcast.xml` | 同 | 不用于正式版 | 仅 preview 流程 |
| release notes | — | 新文档/Release body | 必须含：Code0 新服务（macOS）、Windows 本版未发布、四项已知限制、升级方式 |
| 签名 | — | Developer ID Application + notarytool | `scripts/build-app.sh`（本机 0 identities） |
| 产物 | `.zip` + `.sha256` + `appcast.xml` | 同名 0.11.0 | `scripts/build-app.sh` + `scripts/package-update-release.sh` |
| GitHub Release | `v0.10.11` | `v0.11.0`（tag = `v$VERSION`，`release.yml:44` 强制校验） | 先建 draft 并上传资产 → 跑 `release.yml`（macos-only）→ `publish` 置公开并提交 appcast |
| 公证 | — | notarytool 通过 + staple | 本机无凭据，见 ⑤ |

---

## ⑤ 本机限制与风险

| 限制 | 实测 | 影响 | 出路 |
| --- | --- | --- | --- |
| 无签名身份 | `security find-identity -v -p codesigning` → 0 valid identities | 无法本机签名/公证/产出可发布 zip | 在持有 Developer ID 的机器上打包，或用户安装证书后在本机打包 |
| 公证凭据 | 仓库无 `notarytool` 引用（`grep` 无命中）；凭据应在 keychain profile | 公证步骤无法在本机执行 | 需用户确认 notarytool profile 是否存在/提供 |
| Sparkle 私钥 | `scripts/package-update-release.sh:41` 需要 `$SPARKLE_TOOLS_DIR/generate_keys`；本机未验证存在 | `edSignature` 无法生成 → appcast 无法更新 | 同上：另一台机器或用户提供 |
| Xcode 版本差异 | 本机 Xcode 27.1 / Swift 6.4；CI `macos-15` + `Xcode_26.3` | 本机测试结果可能与 CI 不同 | 以 CI 为准；本机只做快速预检 |
| rustc 1.64 / 无 pnpm / 无 windows target | 已登记 | 与本次 macOS 发布无关（不构建 Windows） | 不需要处理 |
| Windows 红门 | `40988017283` 仍失败 | 不阻塞本方案，但不得带进 main | 见 ③.5 |

**需要用户提供/拍板的（只列清单，暂不索取）**：① 是否接受 Code0 随 0.11.0 发布；② 是否接受为 macOS-only 发布修改 `release.yml`；③ 签名与公证在哪台机器执行、由谁提供凭据；④ 版本号定为 0.11.0 / build 40 是否合适。

---

---

## ⑦ 补充实测：`main` 自身并非全绿（写计划时新发现）

- `gh run list --branch main`：`a97c2a58`（v0.10.11 合并提交）上 `CI` run **`40297237500` = failure**，同提交的 `Windows CI` run **`40297237183` = success**。
- 失败 job 为 `Swift tests and public-release checks`；日志结束行是 `✘ Suite "CLI authentication script builder" failed after 161.512 seconds with 3 issues`、`✘ Test run with 50 tests in 1 suite failed`、`##[error]Process completed with exit code 1`（该 job 的步骤名在日志里显示为 `Test and validate documentation`，但实际失败内容是 Swift 测试套件，不是 docs 检查）。已捕获的具体 issue 之一：`Test "Antigravity watchdog cleans descendants after the CLI leader exits" recorded an issue at CLIAuthenticationScriptBuilderTests.swift:1364:6: Caught error: NSCocoaErrorDomain Code=260 "callbacks.txt" ... No such file or directory`。
- 同一 job 在 `5e03674b` 上（CI `40988017311`，4 个 job 全 success）**通过**；`a97c2a58..5e03674b` 对该测试文件有多个修复类提交（`b017f036`、`0ed30ec3`、`299f6487`、`55bea462` 等）。
- 对本计划的影响与要求：
  1. 发布分支不能以"main 是绿的"为前提。合并 macOS 子集后必须**实跑** `scripts/test.sh` 与 CI 的 `test` job 并取实际结论，不得引用 main 的旧状态当证据。
  2. 该红是 main 既有缺陷、不是本次引入；合并 macOS 子集很可能顺带修复，但**必须实测确认**，不得假设已修复。
  3. 若实测仍红，按【阻塞发布】处理，在打包与发布前解决；不得跳过该 job、不得删断言、不得以重跑碰绿结项。
  4. 这也说明"Windows 红门只在 `codex/code0-monitor` 分支上"成立：main 的 `Windows CI` 是绿的，路径级子集合并不会把 `40988017283` 那类失败带进发布分支。

## ⑥ 执行顺序（等 PM 批准后开工）

1. macOS 子集合并到 `release/macos-0.11.0`（逐路径，`windows/src` 零改动）。
2. 版本与物料升级（VERSION / plist / windows 元数据 / README）。
3. 完整门禁：`scripts/test.sh`、`check-cross-platform-contracts.rb`、`check-docs.sh`、`node --test web/code0-bridge.test.mjs`、`test-code0-native-fixture.sh`。
4. 独立审查（Task5 macOS 部分；我不自审）。
5. 签名 + 公证 + zip/appcast（需 ⑤ 的凭据与机器）。
6. draft release `v0.11.0` → `release.yml`（macos-only）→ publish → 发布后验收。
7. 恢复 Windows 批次（PR#67）。
