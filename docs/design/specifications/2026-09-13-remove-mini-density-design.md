# 删除 Mini 悬浮条密度规格

关联需求：REQ-20260913-001。日期：2026-09-13。状态：用户已确认推荐方案并要求直接发布。

## 目标行为

- macOS 与 Windows 的悬浮条尺寸设置只显示 Comfortable 和 Compact，顺序保持不变。
- 运行时密度类型、跨平台合同、轮廓和尺寸逻辑只保留两档：Comfortable 为108pt/px，Compact 为78pt/px。
- 升级前保存为 `mini` 的偏好在首次读取时归一为 Compact；其他设置、产品顺序、隐藏状态、屏幕位置和自动收起参数保持不变。
- 新保存的数据不会再写出 `mini`，未知密度继续安全回退到 Compact。
- 历史发布记录保留当时的 Mini 事实，不改写历史文档。

## 实现边界

macOS 从 `FloatingStripDensity` 删除 `.mini`，现有容错解码把旧字符串自动映射为 `.compact`；设置 Picker 和本地化资源删除 Mini。Windows 将前端联合类型和原生允许列表收窄到两档，原生反序列化归一化旧 `mini` 为 `compact`，设置 Select、SVG尺寸分支、浏览器密度样例和共享合同同步收窄。

不改变 Compact 或 Comfortable 的宽度、圆环、间距、高度、轮廓、自动隐藏状态机或实时切换入口。删除仅与 Mini 存在相关的实现和测试，保留两档的左右贴边、1–4产品、自动隐藏和即时调整覆盖。

## 验收

- 双平台设置只出现 Comfortable、Compact，默认仍为 Compact。
- 旧 `{"density":"mini"}` 与未知密度都读取为 Compact，并在再次保存后写出 `compact`。
- `allCases`、共享合同和浏览器报告只包含两档；源码产品路径不再包含 Mini 分支或文案。
- Compact/Comfortable 的尺寸、轮廓、左右镜像、圆环包含、自动隐藏展开和设置即时生效回归全绿。
- 完整 Swift、Windows 前端、Rust、跨平台合同、生产构建、文档与发布安全门禁通过，独立审查无未关闭发现。

## 发布

用户已明确授权直接发布。交付版本为0.9.1、macOS build27，包含REQ-20260913-001及已完成的REQ-20260913-002；按现有稳定发布事务完成PR、main、双平台原生CI、标签、签名资产、GitHub Release与更新源验收。
