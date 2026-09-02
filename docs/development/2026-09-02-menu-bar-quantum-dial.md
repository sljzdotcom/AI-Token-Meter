# 菜单栏 Quantum Dial 图标开发与验收记录

**日期：** 2026-09-02  
**需求：** `REQ-20260902-010`  
**状态：** 已完成

## 1. 背景与目标

菜单栏原先使用系统通用的 `gauge.with.dots.needle.50percent`，功能明确但辨识度不足。用户在实尺寸原型中选择方案 A「Quantum Dial」，并确认图标应动态表示 Claude、Codex、DeepSeek 三项服务中最高的有效已用比例。

目标是把顶部图标替换为 18×18pt 单色自绘仪表，同时保留精确百分比、辅助功能和原菜单行为。应用 Icon、Provider Logo、浮动条、Widget、额度算法和弹出面板内部无数据图标均不在变更范围内。

## 2. 实现范围

- `MenuBarSummary` 新增 `usageFraction`，与百分比文字、辅助功能文案和风险语义共享同一个规范化最高比例；
- `MenuBarMeterGeometry` 把 `0...1` 映射到批准的 270° 断环和指针角度，对负数、超限、非有限值及无数据做安全归一化；
- `MenuBarMeterIcon` 绘制低亮度轨道、进度弧、三枚刻度、指针和中心轴；固定为 18×18pt，使用系统前景色，无背景、阴影、彩色状态或动画；
- `MenuBarLabel` 接入动态图标，保留原百分比文字、字体作用域及单一辅助功能标签；
- 删除仅探测旧 SF Symbol 源码字符串的过时测试，改由真实数据、几何、像素渲染和编译验证保护行为。

## 3. 测试驱动证据

### Red

1. 展示模型测试先引用 `MenuBarSummary.usageFraction`，编译按预期失败，证明测试覆盖的是尚未存在的接口；
2. 图标测试先引用 `MenuBarMeterGeometry` 与 `MenuBarMeterIcon`，编译按预期失败，证明新组件并非在测试之前预置。

### Green

- `AppPresentationTests`：15 项通过，覆盖最高值、不可用 Provider 排除、夹紧和无数据；
- `MenuBarMeterIconTests`：3 项通过，覆盖 0%、23%、70%、90%、100%、越界、NaN、Infinity 与无数据；
- `TypographyTests`：16 项通过，确认菜单栏字体作用域与其他符号字体契约未被破坏；
- 完整回归：**299 tests / 59 suites / 0 failures**，耗时约 15.1 秒；
- `git diff --check` 无空白错误。

渲染测试以 Retina 2× 输出 36×36 像素的浅色与深色图标，验证四角透明、中心轴可见、有效像素数量处于合理区间，并确认没有背景方块或画布裁切。

## 4. Release、安装与本机验收

- 构建命令：`AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh`；
- 候选应用：`dist/AI Token Meter.app`；
- 候选与安装版均通过 `codesign --verify --deep --strict`；
- 安装版可执行文件：Mach-O 64-bit arm64；
- 候选与 `/Applications/AI Token Meter.app` 的可执行文件 SHA-256 完全一致：
  `1ab9e102b648ce95e33e3b3593e0e7981b3fd79816a341897f2b22083edf932d`；
- 旧安装已可恢复地保存在：
  `/private/tmp/AI Token Meter-pre-quantum-dial-20260902-103220.app`。

已安装应用成功启动并完成真实刷新。Computer Use 辅助功能状态读取到 Claude 0%、Codex 35%、DeepSeek ¥76.91；因此运行时最高比例为 35%，与 `MenuBarSummary` 驱动 Quantum Dial 和精确文字的同源逻辑一致。浮动条三项按钮和移动控制保持可用。

macOS 的辅助功能快照不暴露独立 `MenuBarExtra` 状态项的像素截图，因此没有把缺失截图伪装成手工视觉证据；18×18pt 的浅色/深色清晰度、透明边界和中心可见性由真实 SwiftUI `ImageRenderer` 测试覆盖，安装版由与候选一致的哈希保证包含同一实现。

## 5. 安全、隐私与兼容性

- 未新增网络请求、计时器、进程、文件或持久化字段；
- 未读取或记录 API Key、OAuth Token、Cookie 或会话内容；
- 图标只消费现有脱敏展示比例，不解析菜单栏百分比字符串；
- VoiceOver 继续只朗读 `MenuBarSummary.accessibilityLabel`，内部绘图标记为装饰性；
- 无数据仍显示 `—`，非有限和越界输入不会造成异常角度或溢出。

## 6. Git 节点

| 提交 | 内容 |
| --- | --- |
| `9662480` | 确认动态 Quantum Dial 设计规格 |
| `5c5c2d9` | 进入实现阶段并更新需求状态 |
| `836fbd7`、`71a0635` | 编写并强化测试驱动实施计划 |
| `e4a7552` | 输出与菜单文字同源的规范化最高比例 |
| `1374a44` | 实现并测试 18×18pt Quantum Dial |
| `8375a3f` | 接入菜单栏并移除过时源码探测 |
| `00bc80a` | 同步用户文档、验收记录与需求完成状态 |

功能分支已通过合并前审查与完整测试，随后以 fast-forward 方式把 `main` 推进至 `4839cdb`。

合并前逐项审查了同源比例、无数据与越界输入、18×18pt 像素边界、辅助功能去重和变更范围；未发现 Critical、Important 或成立的 Minor 问题。

## 7. 已知边界

- Widget 真实安装仍依赖 Apple Development 证书，属于已延期的独立需求，不影响本次菜单栏图标；
- 当前版本不提供图标主题选择、彩色菜单栏状态或动画，符合已确认的低能耗单色规格。
