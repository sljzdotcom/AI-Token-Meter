# Settings 字体隔离与内容字号：最终 Symbol 修复报告

**日期：** 2026-09-01
**分支：** `codex/desktop-only-floating-strip`
**修复提交：** `8b98744 fix: preserve semantic symbol baselines`

## RED / GREEN

- RED：先新增逐项 Symbol 映射与渲染测试，运行 `swift test --filter TypographyTests`。旧版记录 15 项映射错误：无参 `aiMeterSymbolFont()` 不能表达各图标原始语义，Codex 的 `info.circle` 与 `arrow.counterclockwise.circle` 本应是 caption2。初版 `ContentUnavailableView` 对照显示系统组件将两种图标写法都绘制为 `36 × 36`；测试据此改为验证真实空状态图标大于 body 基线，并由逐项源映射禁止 13pt helper 覆盖它。
- GREEN：删除无参 helper，改为只能接受 `AIMeterTextStyle`；body 图标显式 `.body`、Codex reset 图标显式 `.caption2`，空状态图标不应用 helper。`swift test --filter TypographyTests` 为 16/16，`swift test --filter VisualSystemTests` 为 18/18。

## 修改文件

- `Sources/AIMeterApp/Views/AIMeterTypography.swift`
- `Sources/AIMeterApp/Views/MenuBarPanel.swift`
- `Sources/AIMeterApp/Views/CodexDetailView.swift`
- `Sources/AIMeterApp/Views/CodexResetCreditsView.swift`
- `Sources/AIMeterApp/Views/DeepSeekAnalyticsView.swift`
- `Tests/AIMeterAppTests/TypographyTests.swift`
- `docs/development/README.md`
- `docs/development/2026-09-01-settings-font-isolation-and-content-size-step.md`
- `docs/superpowers/specs/2026-09-01-settings-font-isolation-and-content-size-step-design.md`
- `docs/superpowers/plans/2026-09-01-settings-font-isolation-and-content-size-step.md`

## 验证

- 聚焦：Typography 16/16、Visual system 18/18，均 0 失败。
- 全量：`bash scripts/test.sh` 为 187 个测试、38 个套件、0 失败；4 个既有本机环境门控按设计跳过（Keychain 隔离读写、已安装 Claude auth/额度、已安装 Codex 快照）。
- 构建：`bash scripts/build-app.sh` 完成；候选与安装版均通过 `codesign --verify --deep --strict`。
- 安装：安装前已完全退出 `AIMeterApp`，旧版本完整保留在 `/private/tmp/AI-Meter-font-symbol-fix-backup-20260901-1012/AI Meter.app`；未删除此前备份。

| 项目 | SHA-256 |
| --- | --- |
| 候选 `dist/AI Meter.app/Contents/MacOS/AIMeterApp` | `ace8c9c9fde6dd46cf26b2eeb2ea303a9bf6363a48eb3883fe9423d16deb4f8c` |
| 已安装 `/Applications/AI Meter.app/Contents/MacOS/AIMeterApp` | `ace8c9c9fde6dd46cf26b2eeb2ea303a9bf6363a48eb3883fe9423d16deb4f8c` |
| 新备份中的旧安装版 | `b6505ae1ab6fd7c5688615af7a81b4b1705ff24d4f20ccd211f95d2aa2efe359` |

候选与安装主可执行文件使用 `cmp -s` 逐字节核对一致。

## 自审与未覆盖项

- 所有内容作用域下的 helper 调用现在必须显式给出原始语义 token；测试逐项覆盖 9 个图标调用点，不再使用“每个文件至少出现一次”的弱检查。
- `ImageRenderer` 直接验证 caption2/body 的视觉尺寸差异，且空状态组件渲染仍明显大于 body 基线；这与结构映射共同避免 13pt 覆盖。
- 未重复难以捕获的菜单点击面板、Claude/Codex 非激活详情像素验收；其字体与截断的人工门禁继续保留。Mission Control、左右两个普通 Space、真实指针拖动和多显示器同样仍待人工环境补验；普通/全屏 Edge 层级已通过系统整屏截图。
