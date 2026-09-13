# macOS 悬浮条烟熏 Liquid Glass 实现计划

> **面向 AI 代理的工作者：** 使用测试驱动开发逐项执行；每项完成后保留可审查提交与验证证据。

**目标：** 在不改变Windows行为和既有悬浮条几何的前提下，为macOS增加可即时切换并持久化的B款烟熏Liquid Glass外观，然后发布0.10.0/build28。

**架构：** 偏好层增加向后兼容的外观枚举；SwiftUI表面根据外观、系统版本与辅助功能环境选择原生玻璃或烟熏回退。展开态与折叠态共用选择规则，现有`floatingAppearanceHandler`负责即时刷新。

**技术栈：** Swift 6、SwiftUI、AppKit、Swift Testing、macOS 14–26、GitHub Actions。

---

### 任务1：锁定偏好与设置合同

- [x] 在`AppModelDisplaySettingsTests`和设置结构测试中先添加失败回归，要求默认/旧值为Deep Sea、Liquid Glass可持久化、未知值回退、设置变更触发即时外观通知，并且Picker只有两个材质选项。
- [x] 运行专项测试，确认旧实现因外观类型和设置入口不存在而失败。
- [x] 在`FloatingStripPreferences`增加外观枚举和容错迁移，在macOS Floating Strip页增加三语言Picker。
- [x] 重跑专项测试，确认偏好和即时刷新通过。

### 任务2：实现展开与折叠材质

- [x] 先添加失败回归，锁定Deep Sea、macOS 26原生玻璃、旧系统烟熏回退和减少透明度实色四种解析结果，并覆盖两种密度、左右边缘及折叠轮廓。
- [x] 运行专项测试并确认材质解析器缺失导致失败。
- [x] 在`FloatingStripBackground.swift`实现可测试的材质解析和展开/折叠表面；macOS 26对现有Shape应用烟熏基底与系统`tint`玻璃，旧系统使用材质叠色，辅助功能环境增强边界或关闭透明度。
- [x] 把`FloatingStripView`的展开与折叠分支接到当前偏好，重跑渲染、几何和真实窗口即时更新专项测试。

### 任务3：候选验证与审查

- [x] 更新用户指南、CHANGELOG、项目状态、版本合同和0.10.0发布说明；Windows仅更新统一版本元数据。
- [x] 运行完整Swift、macOS正式构建、Windows现有前端/Rust/浏览器/生产构建、跨平台合同、`scripts/check-docs.sh`和公开安全门禁。
- [x] 使用macOS 26真实窗口检查Deep Sea与Liquid Glass的两种密度、左右贴边和展开/折叠状态，并保存证据。
- [ ] 完成独立审查，修复全部发现并重跑受影响门禁；形成可合并候选。

### 任务4：合并并发布0.10.0

- [ ] 推送候选并创建PR，等待精确候选双平台CI通过后合入main。
- [ ] 等待精确main双平台CI通过，从干净且与远端一致的main创建`v0.10.0`。
- [ ] 等待正式发布workflow生成并公开双平台签名资产，推进macOS稳定appcast、Windows stable latest.json和旧Preview兼容入口。
- [ ] 匿名重下资产并核对SHA、签名、篡改拒绝、版本元数据和更新源；提交发布证据，等待最终CI并关闭两项需求。
