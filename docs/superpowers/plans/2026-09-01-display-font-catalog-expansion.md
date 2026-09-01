# AI Token Meter 显示字体目录扩充实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在不打包字体文件的前提下，为显示字体选择器增加 Alimama FangYuanTi VF、Fira Code、Leigo、Menlo 和 Alimama DaoLiTi，并保持既有回退、Settings 隔离和 Widget 系统字体规则。

**架构：** `DisplayFontChoice` 继续保存稳定偏好；`DisplayFontCatalog` 使用每个选项的有序家族候选完成可用性检测和最终解析。Settings 只消费统一的 availability 结果，内容视图继续通过现有语义字体环境即时切换。

**技术栈：** Swift 6、SwiftUI、AppKit `NSFontManager`、Swift Testing、Swift Package Manager

---

## 文件结构

- 修改 `Sources/AIMeterCore/Preferences/DisplayFontChoice.swift`：新增稳定选项和显示名称。
- 修改 `Sources/AIMeterApp/Views/AIMeterTypography.swift`：集中维护字体家族候选、检测和解析。
- 修改 `Tests/AIMeterCoreTests/DisplayFontPreferenceTests.swift`：验证选项顺序、标签、raw value 和持久化兼容。
- 修改 `Tests/AIMeterAppTests/TypographyTests.swift`：验证首选家族、别名、缺失回退、Settings 选项与语义字体。
- 修改 `docs/user-guide/settings.md`：说明新增字体、安装要求和中文级联。
- 修改 `docs/development/README.md`，创建 `docs/development/2026-09-01-display-font-catalog-expansion.md`：记录实现、测试与真实验收。
- 修改 `docs/requirements-backlog.md`：更新需求状态和证据链接。

### 任务 1：扩充稳定字体偏好

**文件：**
- 修改：`Tests/AIMeterCoreTests/DisplayFontPreferenceTests.swift`
- 修改：`Sources/AIMeterCore/Preferences/DisplayFontChoice.swift`

- [x] **步骤 1：编写失败的选项和持久化测试**

将选项断言更新为：

```swift
#expect(DisplayFontChoice.allCases == [
    .system, .antonio, .dinCondensed,
    .alimamaFangYuanTiVF, .firaCode, .leigo, .menlo, .alimamaDaoLiTi,
])
#expect(DisplayFontChoice.alimamaFangYuanTiVF.rawValue == "alimama-fangyuanti-vf")
#expect(DisplayFontChoice.firaCode.rawValue == "fira-code")
#expect(DisplayFontChoice.leigo.rawValue == "leigo")
#expect(DisplayFontChoice.menlo.rawValue == "menlo")
#expect(DisplayFontChoice.alimamaDaoLiTi.rawValue == "alimama-daoliti")
#expect(DisplayFontChoice.alimamaFangYuanTiVF.displayName == "Alimama FangYuanTi VF")
#expect(DisplayFontChoice.firaCode.displayName == "Fira Code")
#expect(DisplayFontChoice.leigo.displayName == "Leigo")
#expect(DisplayFontChoice.menlo.displayName == "Menlo")
#expect(DisplayFontChoice.alimamaDaoLiTi.displayName == "Alimama DaoLiTi")
```

在 round-trip 测试中依次保存并加载每个新增选项，同时保留未知 raw value 回退 `.system` 的测试。

- [x] **步骤 2：运行测试验证失败**

运行：`swift test --filter DisplayFontPreferenceTests`

预期：FAIL，编译器报告五个新增 enum case 不存在。

- [x] **步骤 3：实现最小稳定枚举**

在 `DisplayFontChoice` 增加：

```swift
case alimamaFangYuanTiVF = "alimama-fangyuanti-vf"
case firaCode = "fira-code"
case leigo
case menlo
case alimamaDaoLiTi = "alimama-daoliti"
```

并在 `displayName` switch 返回规格中的名称，不改变三个既有 raw value。

- [x] **步骤 4：运行测试验证通过**

运行：`swift test --filter DisplayFontPreferenceTests`

预期：PASS。

- [x] **步骤 5：Commit**

```bash
git add Sources/AIMeterCore/Preferences/DisplayFontChoice.swift Tests/AIMeterCoreTests/DisplayFontPreferenceTests.swift
git commit -m "feat: expand display font preferences"
```

### 任务 2：统一字体候选解析与缺失回退

**文件：**
- 修改：`Tests/AIMeterAppTests/TypographyTests.swift`
- 修改：`Sources/AIMeterApp/Views/AIMeterTypography.swift`

- [x] **步骤 1：编写失败的家族映射测试**

新增断言：

```swift
let catalog = DisplayFontCatalog(availableFamilies: [
    "Alimama FangYuanTi VF", "Fira Code VF", "Leigo Regular", "Menlo", "Alimama DaoLiTi",
])
#expect(AIMeterTypography.resolvedFamily(for: .alimamaFangYuanTiVF, catalog: catalog) == "Alimama FangYuanTi VF")
#expect(AIMeterTypography.resolvedFamily(for: .firaCode, catalog: catalog) == "Fira Code VF")
#expect(AIMeterTypography.resolvedFamily(for: .leigo, catalog: catalog) == "Leigo Regular")
#expect(AIMeterTypography.resolvedFamily(for: .menlo, catalog: catalog) == "Menlo")
#expect(AIMeterTypography.resolvedFamily(for: .alimamaDaoLiTi, catalog: catalog) == "Alimama DaoLiTi")
```

再用同时包含 `Fira Code`/`Fira Code VF` 和 `Leigo`/`Leigo Regular` 的目录，验证首选候选优先；用空目录验证五项全部回退 `nil`。更新 Settings 选项测试，断言八项顺序不变、缺失项显示 `Not installed`。

- [x] **步骤 2：运行测试验证失败**

运行：`swift test --filter TypographyTests`

预期：FAIL，新增 case 尚未被 availability 和 family switch 覆盖。

- [x] **步骤 3：实现有序候选映射**

在 `DisplayFontCatalog` 增加：

```swift
func resolvedFamily(_ choice: DisplayFontChoice) -> String? {
    DisplayFontFamilyCandidates.values(for: choice)
        .first(where: availableFamilies.contains)
}
```

定义集中映射：

```swift
enum DisplayFontFamilyCandidates {
    static func values(for choice: DisplayFontChoice) -> [String] {
        switch choice {
        case .system: []
        case .antonio: ["Antonio"]
        case .dinCondensed: ["DIN Condensed"]
        case .alimamaFangYuanTiVF: ["Alimama FangYuanTi VF"]
        case .firaCode: ["Fira Code", "Fira Code VF"]
        case .leigo: ["Leigo", "Leigo Regular"]
        case .menlo: ["Menlo"]
        case .alimamaDaoLiTi: ["Alimama DaoLiTi"]
        }
    }
}
```

`isAvailable` 对 `.system` 返回 true，其余调用 `resolvedFamily != nil`；`AIMeterTypography.resolvedFamily` 直接复用该结果，删除重复 switch。

- [x] **步骤 4：运行字体测试与完整测试**

运行：

```bash
swift test --filter TypographyTests
swift test
```

预期：全部 PASS；Swift 编译器无未穷尽 switch。

- [x] **步骤 5：Commit**

```bash
git add Sources/AIMeterApp/Views/AIMeterTypography.swift Tests/AIMeterAppTests/TypographyTests.swift
git commit -m "feat: resolve expanded display font catalog"
```

### 任务 3：文档、Release 与真实字体状态验收

**文件：**
- 修改：`docs/user-guide/settings.md`
- 修改：`docs/development/README.md`
- 创建：`docs/development/2026-09-01-display-font-catalog-expansion.md`
- 修改：`docs/requirements-backlog.md`

- [x] **步骤 1：更新用户文档**

把 Display font 列表扩展为八项，明确：AI Token Meter 不下载或分发字体；未安装项显示 `Not installed`；Leigo/Fira Code/Menlo 的中文由 macOS 系统字体级联；Settings 和 Widget 始终使用系统字体。

- [x] **步骤 2：执行静态与 Release 验证**

运行：

```bash
git diff --check
swift test
swift build -c release
```

预期：全部成功；`Sources` 和构建产物中没有新增 `.ttf`、`.otf`、`.woff` 或 `.woff2`。

- [x] **步骤 3：安装候选版并检查真实 Settings**

使用项目现有安装脚本生成并安装签名候选版。打开 Appearance，确认八个选项按规格排序；当前机器 Menlo 可选，未安装的四个新增第三方字体显示 `Not installed`；Antonio 当前偏好保持，Settings 字形不变化。

- [x] **步骤 4：记录验收结果与需求状态**

开发日志必须记录测试数量、Release 构建、安装指纹、八项真实状态、最终 Antonio 偏好，以及未安装字体无法做视觉字形验收这一事实。将 `REQ-20260901-007` 更新为已完成；未安装字体视觉检查只作为环境限制，不影响“目录扩充”完成。

- [x] **步骤 5：Commit**

```bash
git add docs/user-guide/settings.md docs/development/README.md docs/development/2026-09-01-display-font-catalog-expansion.md docs/requirements-backlog.md
git commit -m "docs: record expanded font catalog acceptance"
```
