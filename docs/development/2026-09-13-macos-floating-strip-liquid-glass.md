# 2026-09-13：macOS 悬浮条烟熏 Liquid Glass

关联REQ-20260913-005与REQ-20260913-006。[设计规格](../design/specifications/2026-09-13-macos-floating-strip-liquid-glass-design.md)、[实现计划](../design/implementation-plans/2026-09-13-macos-floating-strip-liquid-glass.md)。

用户从三款同尺度预览中选择B款烟熏深色玻璃，并明确限定只在macOS实现；Windows不增加设置或渲染分支。用户同时授权直接开发、合并并发布下一稳定版。

## 当前实现

- macOS悬浮条偏好schema升级到5，新增Deep Sea与Liquid Glass；缺失或未知外观回退Deep Sea，保存后重启保持。
- Floating Strip设置页以英语、简体中文和繁体中文提供外观Picker；沿用现有同步外观通知，展开或折叠时都会立即重绘。
- macOS 26以烟熏基底叠加系统Liquid Glass，macOS 14–15使用`ultraThinMaterial`与同色烟熏层；减少透明度改用不透明渐变，高对比度增强轮廓。
- 展开与折叠表面复用同一材质策略，原Deep Sea路径、两档尺寸、左右轮廓、圆环、拖动和自动收起状态机保持。

## 测试驱动证据

首轮新增偏好、即时通知、设置结构和材质策略测试因类型与入口不存在而编译失败。实现后，真实渲染回归发现纯透明玻璃层在无边框窗口缓存中内部透明，随后增加B款烟熏基底，回归转绿。

允许原生窗口和临时大小写敏感卷的环境中，120项悬浮条、偏好、设置与本地化相关测试全部通过；另有12项真实`NSPanel`密度、自动收起、转换反转、拖动延后和即时设置回归通过。两套材质的两种密度、左右边缘和14pt折叠轮廓已生成2x渲染证据，完整Compact四环合成图通过人工检查，并确认Liquid Glass不绘制Deep Sea图片。

版本合同已升级为双平台0.10.0、macOS build28。当前完整门禁通过570项Swift、132项Windows前端、25项浏览器进程生命周期、16组四产品轮廓、2组折叠轮廓、608个文字角色、Windows生产构建及完整Rust宿主测试；Rust格式与全目标严格Clippy、6份跨平台合同、310份Markdown、公开安全、无Widget macOS Release App便携资源及Sparkle嵌套签名验证通过。首次Windows浏览器门禁在沙箱内因127.0.0.1监听权限被拒绝，在允许回环端口的受控环境重跑原命令后通过；没有修改代码或测试期限。

完整候选门禁、独立审查、PR/main、0.10.0/build28签名发布与公网验收将在同一记录继续补充。
