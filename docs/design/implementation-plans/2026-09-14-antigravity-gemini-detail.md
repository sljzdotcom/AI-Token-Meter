# Google Antigravity Gemini 优先详情实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 test-driven-development 逐任务完成红灯、绿灯和重构；完成后使用 verification-before-completion 与 requesting-code-review。

**目标：** 让双平台 Antigravity 详情、汇总、圆环和提醒只使用 Gemini 两个额度窗口，并以可选的真实 CLI 模型信息充实详情。  
**架构：** 额度解析层验证可选 Claude/GPT 行但只产出 Gemini 指标，从源头统一所有下游口径；可选 `AntigravityCLIInfo` 由独立有界命令采集并随快照缓存。详情层只消费统一快照。  
**技术栈：** Swift 6/SwiftUI/Swift Testing、Rust/Serde、React/TypeScript/Vitest、JSON Schema

---

### 任务 1：确定 Gemini 额度与旧缓存归一化

**文件：**
- 修改：`Tests/AIMeterCoreTests/GeminiUsageParserTests.swift`
- 修改：`Tests/AIMeterCoreTests/GeminiCacheTests.swift`
- 修改：`windows/src-tauri/tests/gemini_collector.rs`
- 修改：`windows/src-tauri/tests/gemini_cache.rs`
- 修改：`Sources/AIMeterCore/Collectors/GeminiUsageParser.swift`
- 修改：`Sources/AIMeterCore/Domain/UsageModels.swift`
- 修改：`Sources/AIMeterCore/Persistence/SnapshotCache.swift`
- 修改：`windows/src-tauri/src/collectors/gemini.rs`
- 修改：`windows/src-tauri/src/domain/usage.rs`
- 修改：`windows/src-tauri/src/persistence/usage_runtime.rs`

- [ ] **步骤 1：写解析红灯。** 四行与两行输出都断言只得到 `Gemini · Five hour`、`Gemini · Weekly`；四行 fixture 中 Claude/GPT 80% 已用不能成为 primary/usedRatio。增加半组额外行、缺 Gemini 和重复行拒绝测试。
- [ ] **步骤 2：运行 Swift/Rust Gemini 解析测试并确认因旧四项输出而失败。**
- [ ] **步骤 3：实现最小解析变更。** 严格验证两条必需 Gemini 行及零或完整两条额外行，随后只保存、排序 Gemini 指标。
- [ ] **步骤 4：写旧缓存红灯。** 四项缓存归一化并重算；不完整 Gemini 缓存清空可见额度。
- [ ] **步骤 5：运行缓存测试确认旧实现保留 Claude/GPT 或旧主指标而失败。**
- [ ] **步骤 6：实现 Swift/Rust 缓存归一化并重跑定向测试至通过。**
- [ ] **步骤 7：提交额度口径检查点。**

### 任务 2：增加独立容错的 CLI 补充信息

**文件：**
- 修改：`Sources/AIMeterCore/Domain/UsageModels.swift`
- 新建：`Sources/AIMeterCore/Collectors/AntigravityCLIInfoParser.swift`
- 修改：`Sources/AIMeterCore/Collectors/GeminiCollector.swift`
- 修改：`Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`
- 修改：`Sources/AIMeterCore/Security/SensitiveTextRedactor.swift`
- 修改：`Tests/AIMeterCoreTests/GeminiCollectorTests.swift`
- 新建：`Tests/AIMeterCoreTests/AntigravityCLIInfoParserTests.swift`
- 修改：`windows/src-tauri/src/domain/usage.rs`
- 修改：`windows/src-tauri/src/collectors/gemini.rs`
- 修改：`windows/src-tauri/src/collectors/gemini_environment.rs`
- 修改：`windows/src-tauri/src/collectors/gemini_runtime.rs`
- 修改：`windows/src-tauri/tests/gemini_collector.rs`

- [ ] **步骤 1：写补充解析红灯。** `/model` 的 Gemini TSV 返回显示名，第三方或多行省略；`models` 忽略固定 banner，过滤第三方，计算数量并把 High/Medium/Low 变体折叠为系列。
- [ ] **步骤 2：运行 Swift/Rust 定向测试并确认缺少解析器或字段而失败。**
- [ ] **步骤 3：实现跨平台 `AntigravityCLIInfo` 与纯解析函数。** 限制字符串、条目和系列数量，保持顺序。
- [ ] **步骤 4：写采集生命周期红灯。** 断言固定 `/usage`、`/model`、`models` 命令；补充命令各自失败仍返回 fresh 额度；取消继续传播。
- [ ] **步骤 5：运行采集测试确认旧实现没有附加命令而失败。**
- [ ] **步骤 6：实现两个有界可选命令，保留私有目录、受控环境、输出上限和取消语义。**
- [ ] **步骤 7：更新快照复制、隐私清洗及 Rust 所有结构字面量，重跑定向测试至通过。**
- [ ] **步骤 8：提交 CLI 信息检查点。**

### 任务 3：更新共享合同与双平台详情

**文件：**
- 修改：`contracts/fixtures/gemini-fresh.json`
- 修改：`contracts/schemas/usage-snapshot.schema.json`
- 修改：`Tests/AIMeterCoreTests/CrossPlatformContractTests.swift`
- 修改：`Sources/AIMeterApp/Views/GeminiDetailView.swift`
- 修改：`Sources/AIMeterApp/System/GeminiDetailPanelLayout.swift`
- 修改：`Tests/AIMeterAppTests/GeminiDetailPanelLayoutTests.swift`
- 修改：`Sources/AIMeterApp/Resources/en.lproj/Localizable.strings`
- 修改：`Sources/AIMeterApp/Resources/zh-Hans.lproj/Localizable.strings`
- 修改：`Sources/AIMeterApp/Resources/zh-Hant.lproj/Localizable.strings`
- 修改：`windows/src/state/usage.ts`
- 修改：`windows/src/state/usageBridge.ts`
- 修改：`windows/src/details/ProviderDetail.tsx`
- 修改：`windows/src/details/GeminiDetail.test.tsx`
- 修改：`windows/src/localization.ts`
- 修改：`windows/src/styles.css`
- 修改：`windows/src/test/density-browser-entry.tsx`

- [ ] **步骤 1：写合同和界面红灯。** Fixture 只含两条 Gemini 指标及 CLI 信息；详情断言两额度、模型摘要、版本与更新时间可见，Claude/GPT 与 Credits 不可见；无补充字段仍可渲染。
- [ ] **步骤 2：运行合同、SwiftUI 与 Vitest 定向测试，确认旧四卡界面和缺字段失败。**
- [ ] **步骤 3：更新 JSON Schema、Swift/Rust/TypeScript 合同验证。**
- [ ] **步骤 4：实现 macOS 紧凑 CLI 信息卡和收紧高度，补齐三语言。**
- [ ] **步骤 5：实现 Windows 紧凑 CLI 信息卡和样式，补齐双语。**
- [ ] **步骤 6：重跑定向测试和真实浏览器详情场景至通过，保持青绿强调色与深色底板。**
- [ ] **步骤 7：提交详情与合同检查点。**

### 任务 4：完整验证、审查与整合

**文件：**
- 新建：`docs/development/2026-09-14-antigravity-gemini-detail.md`
- 修改：`docs/development/README.md`
- 修改：`docs/security-and-privacy.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：记录红绿证据、实现范围、隐私边界、平台限制和提交索引。**
- [ ] **步骤 2：运行 Swift 完整测试和 Release App 构建。**
- [ ] **步骤 3：运行 Windows 前端完整测试/生产构建、Rust完整测试/严格 Clippy。**
- [ ] **步骤 4：运行共享合同、真实浏览器、文档、公开安全和差异检查。**
- [ ] **步骤 5：请求独立代码审查，修复全部 Critical/Important，记录 Minor 结论并重跑受影响验证。**
- [ ] **步骤 6：更新台账为已完成并提交最终证据。**
- [ ] **步骤 7：按已有本地整合授权合入最新 main，并在合并结果上重新运行必要门禁；不推送、不建 Tag、不发布。**
- [ ] **步骤 8：向协调任务回传实现、测试/限制、审查、提交/合并 SHA 和发布状态。**

