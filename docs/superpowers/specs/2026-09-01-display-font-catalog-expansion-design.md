# AI Token Meter 显示字体目录扩充设计规格

**日期：** 2026-09-01  
**状态：** 待用户审查  
**范围：** Appearance 中的显示字体选择、字体解析与回退  
**需求：** `REQ-20260901-007`

## 1. 方案与目标

采用用户确认的方案 A：延续现有“只使用 macOS 已安装字体”的机制，不下载、不安装、不在 App Bundle 或 Git 仓库分发字体文件。

在现有 System Default、Antonio、DIN Condensed 后新增：

1. Alimama FangYuanTi VF；
2. Fira Code；
3. Leigo；
4. Menlo；
5. Alimama DaoLiTi。

其中 Leigo 明确指 Ricardo Medina 的 Leigo Regular。Menlo 由 macOS 提供；其余字体需要用户自行合法安装。字体安装后重新打开 Settings 即可变为可选，不需要修改配置文件。

## 2. 稳定标识与家族映射

`DisplayFontChoice` 增加以下稳定 raw value：

| 显示名称 | raw value | 首选 macOS 家族名 | 兼容候选 |
| --- | --- | --- | --- |
| Alimama FangYuanTi VF | `alimama-fangyuanti-vf` | `Alimama FangYuanTi VF` | 无 |
| Fira Code | `fira-code` | `Fira Code` | `Fira Code VF` |
| Leigo | `leigo` | `Leigo` | `Leigo Regular` |
| Menlo | `menlo` | `Menlo` | 无 |
| Alimama DaoLiTi | `alimama-daoliti` | `Alimama DaoLiTi` | 无 |

字体目录把“可用性检测”和“最终家族解析”统一到同一组候选映射，避免选项显示可用但渲染时找不到家族。找到多个候选时始终选择表中靠前者，保证结果确定。

现有 `system`、`antonio`、`din-condensed` raw value 保持不变，升级不改写用户当前选择。

## 3. Settings 行为

- Settings 整个窗口继续无例外使用 macOS 系统字体和系统字号；
- 字体菜单只显示名称，不使用对应字体预览；
- 已安装字体可立即选择；未安装字体显示 `Not installed` 并禁用；
- `Restore Default Font` 继续恢复 `System Default`；
- 字体在 Settings 打开期间被系统安装后，重新进入 Appearance 或重新打开 Settings 时刷新检测结果；本阶段不增加常驻字体目录监听器；
- 当前保存的字体后来被移除时，内容界面安全回退到系统字体，但保留原偏好，字体重新安装后自动恢复。

## 4. 应用范围与字形回退

新字体沿用现有语义字体系统，只影响：

- 浮动条内文字；
- Claude、Codex、DeepSeek 详情页；
- 菜单栏点击后的内容面板。

以下区域始终使用系统字体：

- Settings；
- macOS Widget；
- 系统菜单、通知和操作系统绘制内容。

Fira Code、Leigo 和 Menlo 不提供完整中文覆盖。中英文混排时由 macOS 字体级联为缺失的中文字形选择系统中文字体；不得显示空白方框，也不得为了统一外观内置第二套中文字体。

## 5. 资源与授权边界

本功能不复制或重新分发字体，因此仓库不新增 TTF/OTF 文件，也不增加 App 体积。文档只提供字体名称、安装状态说明和权利归属链接，不内置第三方下载器。

- Fira Code 的上游许可为 SIL OFL 1.1；
- 两款阿里妈妈字体由其权利人声明许可使用，用户应从官方或可信渠道安装；
- Leigo 使用 Ricardo Medina 发布的 Leigo Regular；
- Menlo 随 macOS 提供。

AI Token Meter 不声称拥有、销售或再授权上述字体。

## 6. 错误处理

- 字体未安装：选项禁用并显示 `Not installed`；
- 家族别名变化：依次尝试兼容候选，均失败时回退系统字体；
- 保存值损坏或来自未知未来版本：继续回退 System Default；
- 自定义字体缺少某些字重：由 macOS 合成或选择可用字重，不导致详情页崩溃；
- 中文字形缺失：依赖系统字体级联，不替换用户选择。

## 7. 测试与验收

自动化测试覆盖：

1. 八个选项的顺序、显示名称与稳定 raw value；
2. 五个新增字体的首选家族和兼容候选解析；
3. Fira Code VF、Leigo Regular 别名可用时正确启用；
4. 缺失字体禁用、保存字体缺失时系统回退且偏好不被覆盖；
5. 旧版 Antonio 偏好升级后保持；
6. Settings 继续使用系统字体，菜单名称不做字体预览；
7. Widget 不读取新增字体偏好；
8. 所有自定义字体在内容字号偏移下仍走统一语义规则。

真实验收依次安装并切换新增字体，确认菜单、浮动条和三个详情即时生效；重点检查百分比、人民币金额、长日期、英文模型名和中文说明的可读性与截断。验收结束后恢复用户原有 Antonio 选择，除非用户明确指定新的最终字体。
