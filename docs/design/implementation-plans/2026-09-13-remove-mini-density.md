# 删除 Mini 悬浮条密度实现计划

> **面向 AI 代理的工作者：** 使用测试驱动开发逐项执行；每项完成后保留可审查提交与验证证据。

**目标：** 双平台只保留 Comfortable 与 Compact，并将旧 Mini 偏好安全迁移到 Compact。

**架构：** 在持久化边界完成旧值迁移，在类型与UI层彻底删除 Mini；共享合同继续作为双平台尺寸真源。现有两档几何和窗口更新路径保持不变。

**技术栈：** Swift 6/SwiftUI/AppKit、React/TypeScript、Rust/Tauri、Vitest、Swift Testing。

---

### 任务1：用失败回归锁定两档与迁移合同

- [x] 修改macOS偏好、设置结构与共享合同测试，要求只存在两档且旧`mini`解码为Compact。
- [x] 修改Windows前端、浏览器与Rust偏好测试，要求设置只显示两档且旧`mini`归一为`compact`。
- [x] 运行专项测试并确认旧实现因仍暴露Mini而失败。

### 任务2：删除双平台Mini产品路径

- [x] 从macOS密度枚举、设置Picker和三语言资源删除Mini，保留容错解码迁移。
- [x] 从Windows类型、设置Select、本地化、尺寸与轮廓分支删除Mini；原生允许列表将旧值归一为Compact。
- [x] 从共享合同和浏览器样例删除Mini，清理只服务于Mini的断言和类型转换。
- [x] 运行专项测试，确认两档设置、旧值迁移、几何与即时切换通过。

### 任务3：完整验证与独立审查

- [x] 运行完整Swift及屏幕测试、Windows前端/浏览器/生产构建/Rust/格式/严格Clippy、跨平台合同、文档与公开安全门禁。
- [x] 构建并验证macOS Release App的版本、便携资源和Sparkle嵌套签名。
- [x] 完成独立审查，修复全部Critical、Important和Minor发现后重跑受影响门禁。
- [x] 更新开发记录、CHANGELOG、测试基线和需求台账，形成0.9.1/build27候选。

### 任务4：发布0.9.1

- [x] 推送候选并创建PR，等待精确候选双平台CI通过后合入main。
- [x] 等待精确main双平台CI通过，从干净且与远端一致的main创建`v0.9.1`。
- [x] 等待正式发布workflow生成并公开双平台签名资产，推进三个稳定更新入口。
- [ ] 匿名重下资产并核对SHA、签名、篡改拒绝、版本元数据和更新源；提交发布证据并等待最终CI。
