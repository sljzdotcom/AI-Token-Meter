# macOS三语言即时切换

关联需求：REQ-20260912-002。日期：2026-09-12。

## 当前结果

macOS Settings → Appearance新增Language选择，固定提供English、简体中文和繁體中文。未保存偏好、空值和未知值都以English启动；用户的显式选择即时写入偏好并在重启后恢复，同值选择不重复写入。

语言状态由`AppModel`统一发布。菜单栏内容与标签、Settings、悬浮条及其详情窗口使用同一可观察语言根，因此现存窗口无需重建即可切换Locale。Settings页签、菜单动作、五个设置页面、四个Provider详情、账户与安装状态、瞬时消息、通知、日期数字和辅助功能文案都按当前语言解析。Provider品牌、具体字体名、CLI命令、外部身份和未知诊断保持原文。

本地实现候选位于`codex/macos-language-selection`，实现头为`5b230aa4e3be0348ab3a142738b6d7c9266cf3b1`。专项、完整测试、Release构建、资源、安全和文档门禁已经通过；需求仍为进行中，等待最终独立审查和本地`main`整合。本轮没有推送、创建版本、修改更新源或发布。

## 实现

- `AppLanguage`定义三个稳定存储值、显式Locale和自称显示名；`AppLanguagePreferenceStore`只接受支持值，其他情况回退English。
- `AppLocalizer`从可移植App资源根按实际磁盘目录查找语言表，缺译时先回退English，再返回原键。三张表拥有相同键集合与格式参数合同。
- SwiftUI/AppKit根入口都读取共享语言状态。Language Picker位于Display font之前，按English、简体中文、繁體中文的固定顺序显示。
- Settings最终消息保存为带参数的`SettingsNotice`语义值，显示时再按当前语言生成，避免切换后保留旧语言字符串。
- App层presentation适配已覆盖四Provider状态、额度与重置说明、本机统计、账户回退名和已知CLI日期格式；不修改Core协议数据、额度计算或Windows实现。
- `NotificationService`在每次发送时读取当前语言；数字、百分比、日期和辅助功能完整句由显式Locale格式化。

## TDD与分阶段审查

| 任务 | 失败证据 | 最终专项 | 独立审查与修正 |
| --- | --- | --- | --- |
| 1：偏好与模型 | 缺少语言类型、存储与模型API时按预期编译失败；移除同值保护后写入断言失败 | 2项通过 | `0/0/0` |
| 2：资源与本地化器 | 缺少`AppLocalizer`红灯；大小写敏感卷与格式合同分别产生可观察失败 | 8项通过 | 初审`0/2/0`；`1301448`修正磁盘大小写与格式参数门禁，复审`0/0/0` |
| 3：根传播与Picker | 旧Appearance缺Language，未实现根传播时真实宿主Locale与简繁页签断言失败 | 17项通过 | `0/0/0` |
| 4：Settings与浮动界面 | 23项测试产生20个预期问题；固定英文和绕过菜单本地化的变异产生8个失败 | 28项通过 | 初审`0/0/1`；`System Default`遗漏交任务5关闭 |
| 5：Provider与动态状态 | 语义消息、四Provider状态、动态模板、原始诊断边界与重置语义均保留分组红灯；审查修正前6项测试产生44个问题 | 修正后80项、9个suite通过 | 初审`0/2/1`；`5a9723e`修正真实Claude标签、CLI日期与无邮箱账户名，复审`0/0/0` |
| 6：通知、格式与辅助功能 | 通知7个、真实AX树11个、计数格式6个及重置券2个预期失败 | 61项、6个suite通过 | `0/0/0` |

任务4的Minor已由任务5关闭；任务5的两项Important与一项Minor也已补真实解析器、实际视图/OCR和跨时区探针后复审关闭。上述是分阶段审查，最终整项独立审查仍由后续整合步骤执行。

## 完整验证

- 资源与界面专项：`AI_METER_SCREEN_TESTS=1 swift test --filter 'AppLanguageTests|AppLocalizationTests|SettingsStructureTests|FloatingStripRenderingTests|GeminiDetailPanelLayoutTests|NotificationServiceTests'`通过，51项、6个suite，0失败。
- `scripts/test.sh`通过：主测试525项、独立刷新调度3项、PTY runner 18项，共546项Swift测试；6份跨平台合同fixture、合同可移植性、Windows资产规范化、更新源探针、289份Markdown与公开发布安全检查同时通过。
- `scripts/build-app.sh`通过，生成并验证`dist/AI Token Meter.app`；主App、Sparkle framework及嵌套helper签名有效。当前机器没有Apple Development身份与Team ID，按既有规则跳过Widget。
- Release App包含`en.lproj`、`zh-hans.lproj`、`zh-hant.lproj`三份`Localizable.strings`，各298项；三份均通过`plutil -lint`。资源bundle的`CFBundleDevelopmentRegion`为`en`，Package默认本地化与缺失偏好回退也都是English。
- 公共发布安全检查通过；对Release App全文件的高置信凭据、常见真实邮箱、本机用户路径扫描均无命中。测试使用隔离偏好、fixture和合成状态，没有读取真实服务账号。
- 从计划前产品基线`029c15b`到实现头的新增行没有`URLSession`、`URLRequest`、任务或Network API；出现的URL仅是原有官方帮助链接的本地化迁移。Core、Windows与跨平台合同无差异。
- 文档改动前`check-docs`为289份Markdown通过；新增本记录后最终复验为290份Markdown通过。`git diff --check`在产品门禁阶段和文档完成后都通过。

## Git证据

- `08f7945ba491145703590a4cd968c15851a93a24`：语言偏好与模型状态。
- `bb6e415afb331f0c1e119f31a5522429a551cb67`：三语言资源与显式本地化器。
- `1301448a5ed9c83f69fa71a83dfdc37c4dc2f55e`：资源大小写与格式合同审查修正。
- `5fab2785f988e48087d22a645049c86af12fd96f`：Appearance选择器与根Locale传播。
- `e1f33eade95fb01220f658e724b0e31ad73ae3a8`：Settings、菜单栏与悬浮条静态界面。
- `ab96ff05d4375599ceded1c57333eb786131e6c2`：Provider详情、状态与语义消息。
- `5a9723e5ad1f1bb5943915ec8546de80b118451f`：真实解析器输出的本地化审查修正。
- `5b230aa4e3be0348ab3a142738b6d7c9266cf3b1`：通知、日期数字与辅助功能。

任务7文档提交的精确SHA记录在同目录实现者报告中；最终独立审查与本地`main`合并SHA将在后续步骤完成后补记，当前不编造整合证据。

## 验收边界

自动化覆盖真实`NSHostingView`/`NSPanel`、实际菜单、Apple Vision OCR、AppKit辅助功能树、真实通知请求对象和打包资源；没有触发真实Notification Center权限/展示、人工VoiceOver朗读或真实Provider账号访问。它们不阻断本次代码与资源门禁，也不被表述为已完成的现场验收。

当前产物为本地未发布候选。用户没有单独授权发布，因此不创建标签、GitHub Release或改动appcast/Windows更新源。
