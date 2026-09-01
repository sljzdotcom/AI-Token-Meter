# AI Token Meter Settings 字体隔离与内容字号提升设计规格

**日期：** 2026-09-01

**状态：** 已实施；自动化、Release 构建、签名和安装指纹已验证。Settings 实机验收已记录；菜单点击面板及 Claude/Codex 非激活详情的像素级人工视觉验收仍待具备全屏捕获能力的环境补验。

**基础分支：** `codex/desktop-only-floating-strip`

**关联规格：** `2026-08-31-display-font-selection-design.md`

## 1. 背景

AI Token Meter 当前通过根级字体环境把用户选择的 System Default、Antonio 或 DIN Condensed 应用到菜单面板、浮动条、详情页和 Settings。Settings 中的字体选项还会用对应字体直接预览名称。这个结构使 Settings 也随显示字体变化，不符合“设置界面始终稳定、清晰”的新要求。

同时，浮动条、浮动条详情以及点击菜单栏图标后显示的面板文字整体偏小。用户要求这些内容界面的现有文字统一增大一级，并确认“一号”在本阶段定义为精确增加 `1pt`，而不是把 `body` 跳成 `headline` 等不等距语义字号。

本规格覆盖原字体规格中“Settings 跟随显示字体”和“字体选项使用自身字体预览”的规则；原规格的字体选择、缺失字体回退、即时生效与持久化规则继续保留。

## 2. 已确认需求

1. Settings 的全部文字永久使用 macOS 系统默认字体。
2. Settings 不受当前或未来显示字体选择影响。
3. Settings 不受本次或未来内容字号增减功能影响。
4. Settings 内 System Default、Antonio、DIN Condensed 选项只显示名称，不再用对应字体预览。
5. 浮动条、浮动条详情页、菜单栏图标点击后显示的全部内容页面，当前字号统一增加 `1pt`。
6. Antonio 继续作为当前用户在非 Settings 内容界面中的已选显示字体；本功能不擅自改写字体偏好。

## 3. 设计原则

### 3.1 Settings 是隔离的系统界面

Settings 根视图建立独立字体作用域：

- 字体家族固定为 macOS 系统字体；
- 字号偏移固定为 `0pt`；
- 不读取显示字体偏好来决定 Settings 的实际渲染字体；
- 字体选择器仍可读取和修改偏好，但选项本身始终以系统字体显示；
- `Restore Default Font` 继续只恢复非 Settings 内容界面的显示字体。

这条边界必须由根级作用域保证，不能依赖每个 Settings 子视图自行记住 `.font(.system(...))`。未来增加 Settings Tab、字段、提示、按钮或字号功能时，新内容应自动继承同一隔离规则。

### 3.2 内容界面统一增加 1pt

建立独立的内容字号偏移，当前值固定为 `+1pt`，应用于：

- 桌面浮动条中所有可见或异常状态文字；
- Claude、Codex、DeepSeek 浮动详情中的标题、数值、标签、日期、余额、图表和错误说明；
- 点击菜单栏图标后显示的主面板、Provider 摘要、状态、按钮和辅助说明；
- 上述页面包含的复用子视图。

以下不应用 `+1pt`：

- Settings 及其所有子视图；
- macOS 系统对话框和系统菜单；
- Logo、SF Symbols、圆环、图表几何、背景图片等非文字内容；
- DeepSeek 官方网页 WebView 内由网站绘制的文字。

## 4. 字体架构

### 4.1 双作用域模型

现有显示字体环境扩展为两个相互独立的参数：

1. `displayFontChoice`：System Default、Antonio 或 DIN Condensed；
2. `fontPointOffset`：在语义基础字号上增加或减少的点数。

提供两个明确入口：

- **Settings 作用域：** `displayFontChoice = .system`，`fontPointOffset = 0`；
- **内容作用域：** `displayFontChoice = AppModel.displayFontChoice`，`fontPointOffset = +1`。

菜单面板和浮动窗口分别在自己的根视图安装内容作用域。Settings 在自己的根视图安装 Settings 作用域。禁止在 App 场景最外层继续用一个共享作用域同时包住两类界面。

### 4.2 字号解析

统一字体解析层按以下顺序计算最终字号：

1. 从 `AIMeterTextStyle` 或明确点数读取基础字号；
2. 加上当前作用域的 `fontPointOffset`；
3. 保留调用方的字重与设计意图；
4. 按最终字体选择渲染，并在字体缺失时回退系统字体。

内容作用域中的语义字体需要得到精确 `+1pt` 结果，不能依赖 Dynamic Type 档位跳级。Settings 仍走原生系统语义字体且偏移为零。

最终字号必须设置安全下限，防止未来负偏移产生零或负字号；本阶段不新增用户可编辑字号控件。

### 4.3 清理绕过统一系统的字体

浮动条、详情和菜单面板中直接使用 SwiftUI `.font(...)`、固定 `Font.system(size:)` 或由组件尺寸临时推导文字大小的代码，都要逐项分类：

- 属于产品文字的，迁移到统一的 `aiMeterFont` 或等价的偏移感知接口；
- 属于图形内部标记且确实需要按几何尺寸缩放的，使用统一解析器提供的明确点数入口；
- 属于 Logo 或符号图形的，不纳入文字字号偏移。

不得通过对整个视图做 `scaleEffect` 来放大文字，因为这会同时改变圆环、间距、点击区域和像素清晰度。

## 5. Settings 行为

Settings 中的字体设置继续提供：

- System Default；
- Antonio；
- DIN Condensed；
- Restore Default Font。

变化如下：

- 三个选项均使用系统默认字体显示名称；
- 不再提供字体名称的字形预览；
- 选中状态、缺失字体标记、禁用状态和即时切换能力不变；
- 用户切换选项时，浮动条、详情和菜单面板立即变化，Settings 自身外观不变化；
- 用户恢复默认字体时，Settings 自身外观仍不变化。

## 6. 布局与可访问性

`+1pt` 可能增加文字宽度和行高，实现时必须保持：

- 百分比、金额、充值券到期日和重置时间不被截断；
- 英文错误说明和中文本地化文本可正常换行；
- 菜单面板和详情页现有自适应高度继续工作；
- 浮动条外形、圆环直径、Logo 尺寸和点击区域不改变；
- VoiceOver 标签、排序和可点击语义不改变；
- 字体不可用时的系统回退也应用相同内容字号偏移。

如果个别固定宽度区域在 `+1pt` 后无法容纳内容，应优先允许合理换行或提高容器自适应能力，不得单独把该文字偷偷缩回旧字号。

## 7. 状态与持久化

本阶段不新增用户偏好键：

- 显示字体选择继续使用现有持久化字段；
- 内容字号偏移当前是产品级常量 `+1pt`；
- Settings 的系统字体与零偏移是界面契约，不保存为用户选择；
- 以后若增加字号设置，只允许影响内容作用域，Settings 继续固定零偏移。

## 8. 错误处理

- Antonio 或 DIN Condensed 不可用：内容界面回退系统字体，并保持 `+1pt`；Settings 继续系统字体、零偏移。
- 保存的字体值未知或损坏：内容界面回退系统字体，并保持 `+1pt`。
- 新增视图遗漏内容作用域：根级作用域应让其自动继承；测试同时扫描或覆盖常见绕过路径。
- 固定字号迁移导致布局变化：通过目标视图快照/结构测试和实机视觉验收发现，不通过降低局部字号掩盖。

## 9. 测试策略

### 9.1 单元与结构测试

至少覆盖：

1. Settings 作用域永远解析为系统字体与 `0pt` 偏移；
2. Settings 不因 AppModel 选择 Antonio 或 DIN Condensed 而改变实际字体；
3. 字体选项名称不再调用字体预览 modifier；
4. 内容作用域对所有语义角色和明确点数字号精确增加 `1pt`；
5. 内容字体缺失回退时仍保留 `+1pt`；
6. 负偏移安全下限；
7. 菜单面板和浮动窗口根视图均安装内容作用域；
8. Settings 根视图安装隔离作用域；
9. 目标内容视图不存在绕过统一字体系统的产品文字；
10. 现有字体偏好、恢复默认、Provider 配色、浮动条轮廓和窗口行为测试继续通过。

### 9.2 实机视觉验收

1. 当前选择 Antonio 时，打开 Settings，全部文字仍是 macOS 系统字体；
2. 依次选择 System Default、Antonio、DIN Condensed，Settings 外观完全不变；
3. 三个字体选项只用系统字体显示名称；
4. 浮动条、菜单面板以及 Claude、Codex、DeepSeek 详情中的文字均比当前版本大 `1pt`；
5. 长日期、充值券、余额、错误说明和图表标签无截断；
6. Settings 的窗口布局、Tab、按钮和说明无意外位移；
7. 重启后字体选择仍保持，Settings 仍固定系统字体；
8. Release 构建、签名、安装和候选包指纹验证通过。

## 10. 文档与版本记录

实施完成后同步更新：

- `README.md`：说明字体选择只影响内容界面；
- `CHANGELOG.md`：记录 Settings 字体隔离与内容字号提升；
- `docs/user-guide/settings.md`：更新字体选项行为；
- 对应开发日志与 `docs/development/README.md`；
- 原字体规格的状态和被覆盖条款；
- 实施计划完成状态及关键 Git 提交。

代码、测试、文档、构建和实机验收分别保留清晰 Git 检查点。未经用户明确要求，不合并 `main`、不推送远端。

## 11. 非目标

- 本阶段不新增字号选择器、步进器或恢复字号按钮；
- 不修改 DeepSeek 官网 WebView 的字体；
- 不改变 Logo、圆环、浮动条轮廓、背景、配色或窗口层级；
- 不重新设计 Settings 布局或 Tab；
- 不更改现有字体文件安装方式或随 App 分发字体；
- 不将 `+1pt` 应用于 macOS 系统绘制的控件文字。

## 12. 完成标准

Settings 在所有字体选择和未来内容字号调整下都稳定使用 macOS 系统默认字体，字体选项不再做字形预览；浮动条、三个 Provider 详情和菜单面板的产品文字通过统一作用域精确增加 `1pt`，没有局部遗漏、截断或非文字元素被误放大；自动化测试、Release 构建、签名、安装和实机视觉验收全部留下证据。

## 13. 实施与验收状态

- `aea7e15` 建立字体表面作用域与精确字号偏移；`d350af9` 将 Settings、浮动窗口和菜单面板接入显式作用域；`d334321` 将内容作用域中继承字体的 SF Symbols 固定回原尺寸。
- 2026-09-01 完整回归通过 185 个测试、38 个套件、0 失败；Keychain 隔离读写、已安装 Claude 状态、Claude 认证状态和已安装 Codex 快照共 4 个环境门控检查跳过。
- Release 候选及 `/Applications/AI Meter.app` 均通过 `codesign --verify --deep --strict`；实际可执行文件为 `Contents/MacOS/AIMeterApp`，候选与安装 SHA-256 同为 `b6505ae1ab6fd7c5688615af7a81b4b1705ff24d4f20ccd211f95d2aa2efe359`。安装前版本保留在 `/private/tmp/AI-Meter-font-scope-backup-20260901/AI Meter.app`。
- 实机已观察 Settings 在 System Default、Antonio、DIN Condensed 下保持系统字体与不变字号；Restore Default、选择状态和重启后的 Antonio 持久化均有辅助功能树和截图证据。
- DeepSeek 详情可直接观察，当前金额、30 天统计、图表和日期没有截断。Computer Use 的窗口捕获不显示菜单点击面板或 Claude/Codex 非激活 `NSPanel` 的像素内容；三者只完成了打开/关闭辅助功能交互验证，字体、截断、自动隐藏、外部点击关闭与真实指针拖动仍不能宣称已完成。
- 详细命令、截图、辅助功能结果、环境限制和补验清单见 [开发日志](../../development/2026-09-01-settings-font-isolation-and-content-size-step.md)。
