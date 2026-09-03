# 浮动条位置稳定持久化实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 让浮动条跨重启、休眠和显示器拓扑变化稳定恢复用户保存的物理屏幕、侧边与高度，并在目标屏缺失时只无损回退当前主屏。

**架构：** 把屏幕身份和选择规则从窗口控制器提取为纯策略：macOS 以 Core Graphics UUID 为稳定身份、兼容旧 `NSScreenNumber`；Windows 以哈希后的设备接口路径为稳定身份、兼容旧运行时名称。两平台都监听屏幕拓扑变化，任何系统重排都不写回临时回退；只有明确用户操作和带旧值守卫的无损标识迁移能更新偏好。

**技术栈：** Swift 6、AppKit/CoreGraphics、Swift Testing、UserDefaults；Rust、Tauri 2、Serde、Cargo test；Markdown 文档与现有项目门禁。

---

## 文件结构

- 创建 `Sources/AIMeterApp/System/FloatingStripScreenIdentity.swift`：将 `NSScreen` 转换为稳定 UUID、旧数字编号和主屏标志。
- 修改 `Sources/AIMeterApp/System/FloatingStripScreenResolver.swift`：纯函数选择保存目标、旧标识迁移或主屏临时回退。
- 修改 `Sources/AIMeterApp/System/FloatingPanelController.swift`：所有重排复用稳定解析；只在迁移或用户操作时写入位置。
- 修改 `Sources/AIMeterApp/AppModel.swift`：提供带旧值守卫的显示器标识迁移入口，删除会覆盖位置的屏幕丢失恢复入口。
- 修改 `Tests/AIMeterAppTests/FloatingStripLayoutTests.swift`：覆盖稳定目标、多屏、单屏回退和旧标识迁移。
- 修改 `Tests/AIMeterAppTests/FloatingPanelPositioningPolicyTests.swift`：把旧“普通重排写回默认”断言替换为“只允许旧标识迁移写入”的合同。
- 修改 `Tests/AIMeterAppTests/AppModelStartupTests.swift`：验证迁移只更新显示器身份。
- 修改 `windows/src-tauri/src/platform/windows/monitor.rs`：实现稳定物理身份、旧标识迁移、目标屏优先、主屏回退与拓扑变化跟踪。
- 创建 `windows/src-tauri/src/platform/windows/display_topology.rs`：运行中监听显示器连接、断开、主屏角色与工作区变化并重新定位。
- 修改 `windows/src-tauri/src/platform/windows/window_controller.rs`、`windows/src-tauri/src/lib.rs`：接通稳定身份、启动迁移、用户设置写入与拓扑恢复。
- 修改 `docs/user-guide/settings.md`、`docs/user-guide/troubleshooting.md`、`docs/development/testing.md`：更新用户可见行为、排障与双屏验收口径。
- 创建 `docs/development/2026-09-03-floating-strip-placement-persistence.md`：记录根因、红绿测试、迁移、安全边界和验证证据。
- 修改 `docs/requirements-backlog.md`：持续记录阶段状态，最终只在证据完整后标记完成。

### 任务 1：建立稳定屏幕身份和纯解析策略

**文件：**
- 创建：`Sources/AIMeterApp/System/FloatingStripScreenIdentity.swift`
- 修改：`Sources/AIMeterApp/System/FloatingStripScreenResolver.swift`
- 测试：`Tests/AIMeterAppTests/FloatingStripLayoutTests.swift`

- [x] **步骤 1：编写失败的多屏与迁移测试**

将旧的“缺失显示器回到右侧中点”测试替换为以下合同，并补充旧标识迁移：

```swift
@Test("A saved physical display wins even when another display is primary")
func savedDisplayWins() {
    let result = FloatingStripScreenResolver.resolve(
        savedIdentifier: "uuid:target",
        screens: [
            .init(stableIdentifier: "uuid:primary", legacyIdentifier: "1", isMain: true),
            .init(stableIdentifier: "uuid:target", legacyIdentifier: "3", isMain: false),
        ]
    )
    #expect(result.selectedIdentifier == "uuid:target")
    #expect(!result.usesFallbackScreen)
    #expect(result.migratedIdentifier == nil)
}

@Test("A missing display falls back to the primary without replacing its identity")
func missingDisplayUsesPrimaryTemporarily() {
    let result = FloatingStripScreenResolver.resolve(
        savedIdentifier: "uuid:disconnected",
        screens: [
            .init(stableIdentifier: "uuid:secondary", legacyIdentifier: "2", isMain: false),
            .init(stableIdentifier: "uuid:primary", legacyIdentifier: "1", isMain: true),
        ]
    )
    #expect(result.selectedIdentifier == "uuid:primary")
    #expect(result.usesFallbackScreen)
    #expect(result.migratedIdentifier == nil)
}

@Test("A matching legacy display number migrates to the stable identity")
func matchingLegacyIdentifierMigrates() {
    let result = FloatingStripScreenResolver.resolve(
        savedIdentifier: "3",
        screens: [
            .init(stableIdentifier: "uuid:target", legacyIdentifier: "3", isMain: true),
        ]
    )
    #expect(result.selectedIdentifier == "uuid:target")
    #expect(result.migratedIdentifier == "uuid:target")
    #expect(!result.usesFallbackScreen)
}
```

另加：旧数字标识在单屏但编号改变时安全迁移；稳定 UUID 缺失时即使只剩单屏也不得迁移；屏幕集合为空返回 `nil`；没有保存目标时优先主屏。

- [x] **步骤 2：运行定向测试并确认按预期失败**

运行：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-placement-red-clang \
AI_METER_TEST_BUILD_DIR=/private/tmp/ai-meter-placement-red-build \
bash scripts/test.sh --filter "Floating strip layout"
```

预期：编译失败，指出新 `screens` 参数、`FloatingStripScreenIdentity` 或新结果字段尚不存在；不能接受与目标无关的失败。

- [x] **步骤 3：实现最小纯身份与解析模型**

在新文件定义：

```swift
struct FloatingStripScreenIdentity: Equatable, Sendable {
    let stableIdentifier: String
    let legacyIdentifier: String?
    let isMain: Bool
}

struct FloatingStripScreenResolution: Equatable {
    let selectedIdentifier: String
    let usesFallbackScreen: Bool
    let migratedIdentifier: String?
}
```

解析器严格按“稳定标识命中 → 旧标识命中并迁移 → 单屏旧数字安全迁移 → 当前主屏回退 → 第一块屏幕”执行。仅用 `Int(savedIdentifier) != nil` 识别升级前数字格式，稳定标识必须使用 `uuid:` 前缀。

AppKit 身份适配使用：

```swift
let displayID = (screen.deviceDescription[.init("NSScreenNumber")] as? NSNumber)?.uint32Value
let stableIdentifier = displayID
    .flatMap { CGDisplayCreateUUIDFromDisplayID($0)?.takeRetainedValue() }
    .flatMap { CFUUIDCreateString(nil, $0) as String? }
    .map { "uuid:\($0.lowercased())" }
```

UUID 不可用时用 `legacy:<number>` 作为本次可选择身份，同时保留裸数字 `legacyIdentifier` 供旧配置匹配；不得返回空字符串。

- [x] **步骤 4：运行定向测试确认绿色**

运行与步骤 2 相同的筛选命令。预期：`Floating strip layout` 全部通过，0 失败。

- [x] **步骤 5：提交身份和解析策略检查点**

```bash
git add Sources/AIMeterApp/System/FloatingStripScreenIdentity.swift \
  Sources/AIMeterApp/System/FloatingStripScreenResolver.swift \
  Tests/AIMeterAppTests/FloatingStripLayoutTests.swift
git commit -m "fix: resolve floating strip by stable display identity"
```

### 任务 2：阻止系统重排覆盖保存位置

**文件：**
- 修改：`Sources/AIMeterApp/System/FloatingPanelController.swift`
- 修改：`Sources/AIMeterApp/AppModel.swift`
- 测试：`Tests/AIMeterAppTests/FloatingPanelPositioningPolicyTests.swift`
- 测试：`Tests/AIMeterAppTests/AppModelStartupTests.swift`

- [x] **步骤 1：编写失败的无损重排和迁移测试**

把旧策略测试改成任何系统触发的临时回退都不持久化，只有 resolver 明确给出的旧标识迁移可以写入：

```swift
@Test("System repositioning never persists a fallback display")
func systemRepositioningPreservesSavedPlacement() {
    let resolution = FloatingStripScreenResolution(
        selectedIdentifier: "uuid:primary",
        usesFallbackScreen: true,
        migratedIdentifier: nil
    )
    #expect(
        FloatingStripPositionPersistencePolicy.action(
            savedIdentifier: "uuid:missing",
            resolution: resolution
        ) == .preserve
    )
}
```

另加迁移 resolution 返回 `.migrate(from: "3", to: "uuid:display")` 的断言。

在 AppModel 测试中先保存 `.automatic + .left + 0.48 + "3"`，调用新迁移入口后断言只有标识变为 `uuid:display`；再用错误旧值调用，断言不覆盖较新的位置：

```swift
model.migrateFloatingStripScreenIdentifier(from: "3", to: "uuid:display")
#expect(model.floatingStripPosition.preference == .automatic)
#expect(model.floatingStripPosition.lastResolvedEdge == .left)
#expect(model.floatingStripPosition.normalizedCenterY == 0.48)
#expect(model.floatingStripPosition.screenIdentifier == "uuid:display")
```

- [x] **步骤 2：运行测试确认旧行为失败**

运行：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-placement-controller-red-clang \
AI_METER_TEST_BUILD_DIR=/private/tmp/ai-meter-placement-controller-red-build \
bash scripts/test.sh --filter FloatingPanelPositioningPolicyTests
bash scripts/test.sh --filter AppModelStartupTests
```

预期：新迁移方法与 `FloatingStripPositionPersistencePolicy` 不存在，旧策略仍允许普通重排写回默认位置。

- [x] **步骤 3：接通稳定身份和无损回退**

实施以下最小变更：

1. `placementContext()` 构造当前 `NSScreen` 与 `FloatingStripScreenIdentity` 的一一映射，并调用新 resolver；
2. 无论命中目标还是临时回退，侧边统一使用 `resolvedEdgeForCurrentPreference()`，高度统一使用已保存 `normalizedCenterY`；
3. 临时回退只移动面板，不调用任何保存方法；
4. resolver 返回迁移标识时调用 `migrateFloatingStripScreenIdentifier(from:to:)`；
5. 拖动结束和辅助功能移动保存稳定 UUID，而不是 `NSScreenNumber`；
6. 删除 `recoverFloatingStripAfterScreenLoss`、旧 reposition reason 和 `persistScreenLossRecovery`；新增的 persistence policy 只输出 `.preserve` 或 `.migrate(from:to:)`，活动 Space 与普通重排共享同一决策；
7. 解析不到任何屏幕时保持现有 frame，并等待下一次系统通知。

迁移入口必须带旧值守卫：

```swift
func migrateFloatingStripScreenIdentifier(from old: String, to new: String) {
    guard floatingStripPosition.screenIdentifier == old, old != new else { return }
    floatingStripPosition.screenIdentifier = new
    floatingStripPositionStore.save(floatingStripPosition)
}
```

- [x] **步骤 4：运行位置相关测试确认绿色**

运行：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-placement-controller-green-clang \
AI_METER_TEST_BUILD_DIR=/private/tmp/ai-meter-placement-controller-green-build \
bash scripts/test.sh --filter FloatingStripPositionTests
bash scripts/test.sh --filter FloatingStripLayoutTests
bash scripts/test.sh --filter FloatingPanelPositioningPolicyTests
bash scripts/test.sh --filter AppModelStartupTests
```

预期：位置、解析、迁移与启动测试全部通过，0 失败。

- [x] **步骤 5：提交控制器检查点**

```bash
git add Sources/AIMeterApp/System/FloatingPanelController.swift \
  Sources/AIMeterApp/AppModel.swift \
  Tests/AIMeterAppTests/FloatingPanelPositioningPolicyTests.swift \
  Tests/AIMeterAppTests/AppModelStartupTests.swift
git commit -m "fix: preserve floating strip placement across display changes"
```

### 任务 3：锁定 Windows 对等行为并补齐文档

**文件：**
- 修改：`windows/src-tauri/src/platform/windows/monitor.rs`
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/user-guide/troubleshooting.md`
- 修改：`docs/development/testing.md`
- 创建：`docs/development/2026-09-03-floating-strip-placement-persistence.md`
- 修改：`docs/requirements-backlog.md`

- [x] **步骤 1：增加 Windows 目标屏与回退测试**

在 `monitor.rs` 增加：

```rust
#[test]
fn saved_monitor_wins_even_when_another_monitor_is_primary() {
    let monitors = vec![
        MonitorIdentity::new("primary", true),
        MonitorIdentity::new("saved", false),
    ];
    assert_eq!(choose_monitor(&monitors, Some("saved")), monitors.get(1));
}

#[test]
fn missing_monitor_falls_back_only_to_primary() {
    let monitors = vec![
        MonitorIdentity::new("secondary", false),
        MonitorIdentity::new("primary", true),
    ];
    assert_eq!(choose_monitor(&monitors, Some("missing")), monitors.get(1));
}
```

同时验证运行时断开与重新接入都会触发重新定位，而临时回退不返回迁移标识；只有用户拖动、用户在 Settings 主动改边，或旧运行时名称无损迁移时更新 `meter_monitor_id`。

- [x] **步骤 2：运行 Windows Rust 定向测试**

运行：

```bash
cargo test --manifest-path windows/src-tauri/Cargo.toml platform::windows::monitor
```

预期：目标屏优先与主屏回退测试通过；若非 Windows 主机条件编译裁掉模块，则运行 `cargo test --manifest-path windows/src-tauri/Cargo.toml` 并由现有 Windows CI 补充真实目标验证。

- [x] **步骤 3：更新当前文档与开发证据**

文档必须明确：

- macOS 配置位于系统 `UserDefaults`，Windows 配置位于 `%APPDATA%\AI Token Meter\settings.json`；
- 已保存目标在线时不会因主屏角色变化跳屏；
- 目标离线时只临时回当前主屏并沿用侧边/高度，不把回退写回；
- 用户在回退屏主动移动才建立新目标；
- 记录本任务红测试、绿测试、完整回归、物理双屏未覆盖时的诚实限制和每个 Git 提交。

同步把 `REQ-20260903-009` 的阶段证据写入 backlog；在完整验证与 CI 之前保持 `进行中`。

- [x] **步骤 4：运行双平台和文档完整门禁**

运行：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-placement-final-clang \
AI_METER_TEST_BUILD_DIR=/private/tmp/ai-meter-placement-final-build \
bash scripts/test.sh
cargo fmt --manifest-path windows/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path windows/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path windows/src-tauri/Cargo.toml
npm --prefix windows test
npm --prefix windows run build
git diff --check
```

预期：Swift 主测试与独立 PTY 测试、跨平台合同、文档/安全门禁、Rust format/clippy/test、前端测试与生产构建全部通过。

- [x] **步骤 5：提交文档与对等合同检查点**

```bash
git add windows/src-tauri/src/platform/windows/monitor.rs \
  docs/user-guide/settings.md docs/user-guide/troubleshooting.md \
  docs/development/testing.md \
  docs/development/2026-09-03-floating-strip-placement-persistence.md \
  docs/requirements-backlog.md
git commit -m "docs: record stable floating strip placement"
```

### 任务 4：合并前验证、集成和公开 CI

**文件：**
- 修改：`docs/development/2026-09-03-floating-strip-placement-persistence.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：执行完成前独立检查**

调用 `verification-before-completion`，重新检查当前分支状态、提交范围、敏感信息、设计覆盖和新鲜测试输出。确认没有把真实显示器 UUID、用户名、屏幕配置或个人路径写入仓库。

- [ ] **步骤 2：建立最终分支提交并推送**

```bash
git status --short
git log --oneline main..HEAD
git push -u origin codex/floating-strip-placement
```

- [ ] **步骤 3：核对分支双平台 CI**

等待 macOS 与 Windows 工作流都针对同一精确提交成功。任一失败必须回到系统化调试，不能以本机通过代替远端证据。

- [ ] **步骤 4：合并回 main 并复验精确合并头**

采用非破坏性 Git 合并，把已验证分支并入 `main`，推送后再次等待 macOS 与 Windows main CI。若当前环境没有物理双屏，把真实拔插验收保留为 `受环境限制` 的补验条目，但代码、自动化和文档完成后可以关闭本缺陷。

- [ ] **步骤 5：回填最终证据**

把最终提交、合并提交、两条 main CI 链接和测试数量写入开发日志与 backlog，再次运行文档检查和 `git diff --check`，提交证据更新。精确证据提交的 CI 也必须成功后，才把 `REQ-20260903-009` 标记为 `已完成`。
