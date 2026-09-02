# Settings 字体隔离与内容字号提升开发日志

**日期：** 2026-09-01

**分支：** `codex/desktop-only-floating-strip`

**规格：** [`docs/design/specifications/2026-09-01-settings-font-isolation-and-content-size-step-design.md`](../design/specifications/2026-09-01-settings-font-isolation-and-content-size-step-design.md)

**计划：** [`docs/design/implementation-plans/2026-09-01-settings-font-isolation-and-content-size-step.md`](../design/implementation-plans/2026-09-01-settings-font-isolation-and-content-size-step.md)

## 1. 交付节点

- `92c88dd feat: isolate typography by interface surface`：定义 Settings、内容与菜单栏标签三类字体作用域；内容精确 `+1pt`，Settings 固定 `.system + 0pt`。
- `5890658 feat: enlarge content fonts outside settings`：将 Settings、浮动条/详情和菜单点击面板接到明确作用域，并移除字体名称预览。
- `bb73fc4 fix: keep symbols at content scope size`：初步审计内容作用域中的 SF Symbols。
- `4a0d6b3 fix: preserve semantic symbol baselines`：最终审查修复。删除无参 body 默认的 Symbol helper，逐项标注语义基线；Codex 两个 reset 图标恢复 caption2，`ContentUnavailableView` 保持系统大号空状态图标。

本日志对应的文档提交在本阶段完成时另见 Git 历史；没有合并 `main` 或推送远端。

## 2. TDD 红绿证据

### 任务 1：字体作用域与精确偏移

红灯命令：

```bash
swift test --filter TypographyTests
```

首次执行先被受限的 Swift/Clang 用户缓存目录阻断；在允许所需构建缓存后，测试编译暴露 `aiMeterFontPreview` 旧调用点的集成依赖。该阶段最小实现保留了临时的兼容包装器，以便只修改 `AIMeterTypography.swift` 与 `TypographyTests.swift` 时仍可编译。

绿灯命令：

```bash
swift test --filter TypographyTests
```

结果：`AI Meter typography` 12 个测试通过，0 失败。之后任务 2 完成根视图迁移并删除兼容 API；最终字体套件扩展为 14 个测试，仍为 0 失败。

### 任务 2：根视图接线与符号尺寸

红灯命令：

```bash
swift test --filter TypographyTests.rootScopeWiring
```

结果：5 个源码契约预期失败：三个根视图仍使用旧作用域调用，Settings 仍引用 `aiMeterFontPreview`。

绿灯命令：

```bash
swift test --filter TypographyTests.rootScopeWiring
swift test --filter TypographyTests
swift test --filter VisualSystemTests
```

结果：根接线测试 1 个通过；`TypographyTests` 13 个、`VisualSystemTests` 18 个通过，0 失败。后续针对 SF Symbols 的红灯 `swift test --filter TypographyTests.symbolFontsIgnoreContentOffset` 在四个经审计视图缺少集中式符号样式时失败；增加 `aiMeterSymbolFont` 后，聚焦测试、14 个字体测试和 18 个视觉系统测试均通过，0 失败。

### 最终审查：Symbol 语义基线

红灯命令：

```bash
swift test --filter TypographyTests
```

结果：新增逐项映射在旧实现记录 15 个错误（每个无参 helper 都不能证明其原始语义，两个 Codex reset 符号应为 caption2）；初版空状态对照也显示 `ContentUnavailableView` 自身把两种写法都渲染为 `36 × 36`，因此把该断言收紧为“实际系统空状态渲染大于 body 基线”并以源映射禁止产品 13pt 覆盖。根因是 `aiMeterSymbolFont()` 无条件固定 body 13pt，接口本身无法要求调用方声明语义。

绿灯命令：

```bash
swift test --filter TypographyTests
swift test --filter VisualSystemTests
```

结果：`TypographyTests` 16 个、`VisualSystemTests` 18 个通过，0 失败。新测试逐个列出 9 个图标调用点；`ImageRenderer` 确认 caption2 小于 body，且真实 `ContentUnavailableView` 空状态图标仍大于 body 基线。

## 3. 完整自动化回归

```bash
bash scripts/test.sh
```

2026-09-01 10:11 +0800 最终结果：187 个测试、38 个套件通过，0 失败。

环境门控跳过共 4 项，不影响脚本退出码：

- Keychain 隔离读写测试：受当前钥匙串环境限制；
- 已安装 Claude auth 状态；
- 已安装 Claude CLI 额度快照；
- 已安装 Codex CLI 额度快照。

测试中 SwiftPM/WebKit 对受限缓存目录的诊断不影响退出码或通过结论。

## 4. Release 构建、签名和安装身份

执行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
shasum -a 256 "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
codesign --verify --deep --strict "/Applications/AI Meter.app"
shasum -a 256 "/Applications/AI Meter.app/Contents/MacOS/AIMeterApp"
```

Release 构建完成，候选与安装版的严格签名命令均无输出且退出码为 0。

| 项目 | SHA-256 |
| --- | --- |
| 最终候选 `dist/AI Meter.app/Contents/MacOS/AIMeterApp` | `ace8c9c9fde6dd46cf26b2eeb2ea303a9bf6363a48eb3883fe9423d16deb4f8c` |
| 最终安装 `/Applications/AI Meter.app/Contents/MacOS/AIMeterApp` | `ace8c9c9fde6dd46cf26b2eeb2ea303a9bf6363a48eb3883fe9423d16deb4f8c` |
| 最终安装前备份 `AI Meter.app/Contents/MacOS/AIMeterApp` | `b6505ae1ab6fd7c5688615af7a81b4b1705ff24d4f20ccd211f95d2aa2efe359` |

初始任务简报中使用的 `Contents/MacOS/AI-Meter` 路径不存在；`Info.plist` 的 `CFBundleExecutable` 是 `AIMeterApp`，因此指纹命令按实际 bundle 可执行文件执行并记录。实现计划已同步为该实际路径。安装前 bundle 已保留（未删除）于：

```text
/private/tmp/AI-Meter-font-symbol-fix-backup-20260901-1012/AI Meter.app
```

## 5. Settings 隔离实机验收

使用安装后的候选版与 Computer Use 辅助功能树完成；最终偏好读取为：

```bash
defaults read com.millerpan.AIMeter appearance.displayFont
# antonio
```

直接观察和辅助功能证据：

1. 在 Antonio 下打开 Settings：截图中标题、分区、字段、说明和按钮均为 macOS 系统字体；字体字段值为 Antonio。随后选择 System Default、Antonio、DIN Condensed，Settings 其余文本的字形与字号保持不变。
2. 字体弹出菜单的辅助功能树仅列出 `System Default`、`Antonio`、`DIN Condensed`，没有字体预览文字；三者均为可选状态（本机均已安装）。
3. System Default 时 `Restore Default Font` 禁用；Antonio 与 DIN Condensed 时启用；从 DIN Condensed 点击 Restore Default 后恢复为 System Default，再正常选回 Antonio。
4. 退出并启动候选版后，Settings 的字体字段仍为 Antonio，系统字体外观保持不变；最终浮动条辅助功能位置为 Right edge、vertical position 98 percent。

已保存的可视证据：

- `/private/tmp/AI-Meter-font-scope-backup-20260901/ui-evidence/settings-system-default.jpeg`
- `/private/tmp/AI-Meter-font-scope-backup-20260901/ui-evidence/settings-antonio.jpeg`
- `/private/tmp/AI-Meter-font-scope-backup-20260901/ui-evidence/settings-din-condensed.jpeg`
- `/private/tmp/AI-Meter-font-scope-backup-20260901/ui-evidence/settings-relaunch-antonio.jpeg`

本机两个自定义字体均已安装，因此没有把 `Not installed` 及禁用状态伪造为已完成的视觉检查；该状态由 `Settings always exposes three ordered choices and marks missing fonts` 自动化测试覆盖，仍需在缺少目标字体的环境中补做视觉验收。

## 6. 内容字号与布局实机验收

直接证据：

- 浮动条可见，当前 Claude、Codex、DeepSeek 按钮的辅助功能状态可读，位置和轮廓保持为 Right edge、98%。
- 依次打开 Claude、Codex、DeepSeek：辅助功能树依次报告相应 `Detail open`，切换会关闭上一个详情。DeepSeek 是可激活详情，直接截图和树中可见 `¥77.99`、Cost、API requests、Tokens、30 天图表、日期及官方链接；观察到当前文字、金额、统计值和日期没有截断。
- DeepSeek 截图显示产品文字使用 Antonio；Logo、圆环/图表几何、刷新 SF Symbol、深海背景和浮动条轮廓未出现随 `+1pt` 改变的可见放大。精确 `+1pt` 与 Symbol 语义基线另有 16 个 TypographyTests 的断言证据。
- 对 DeepSeek 按 Escape 后详情关闭，辅助功能树返回三个 `Detail closed`。

已保存证据：

- `/private/tmp/AI-Meter-font-scope-backup-20260901/ui-evidence/floating-strip-final-antonio.jpeg`
- `/private/tmp/AI-Meter-font-scope-backup-20260901/ui-evidence/deepseek-detail-antonio.jpeg`

### 未覆盖项（不能推断通过）

- Computer Use 的窗口捕获无法显示菜单栏点击面板，以及 Claude、Codex 非激活 `NSPanel` 的像素内容。三者的打开/关闭交互有辅助功能证据，但其 Antonio 字形、`+1pt` 视觉比较、长日期/重置券/错误说明的截断不能由此证明。
- 未对菜单点击面板或 Claude/Codex 详情完成视觉截图；不能声称这些面板的自适应高度、自动隐藏、外部点击关闭或真实鼠标拖动均已完成。
- Mission Control、左右两个普通 Space、真实指针拖动和多显示器均是此前桌面层功能的独立人工门禁，不是本字体任务完成项；普通/全屏 Edge 跨 App 层级已有系统整屏截图证明通过，本次不将其再写成待验。
- 当前可见 DeepSeek 数据没有出现充值券到期日或错误说明，因而只检查了实际呈现的金额、30 天统计和日期；其余长文本场景仍待具备相应数据的手工验收。

## 7. 自审与结论边界

- 文档明确 Settings 与内容作用域的边界，并将旧规格的相关历史条款标注为由新规格覆盖，未改写旧记录。
- Release 候选与安装包是同一 SHA-256，旧安装包保留可恢复。
- 自动化、签名、安装与 Settings 切换/持久化有新鲜证据。由于菜单与两种非激活详情的像素观察受工具限制，本阶段状态为 `DONE_WITH_CONCERNS`，而不是“所有实机视觉项通过”。
