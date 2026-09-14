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

独立审查首轮Critical/Important/Minor为`0/0/2`：README测试徽章与实现计划复选框落后于已经记录的570项完整门禁。两项文档一致性问题已修正；精确候选`29fe0c5..c3b169d`最终复审为`0/0/0`，产品实现没有审查发现。修正后的完整Swift、合同、310份Markdown和公开发布安全门禁再次通过。

## 合并与正式发布

[PR #46](https://github.com/sljzdotcom/AI-Token-Meter/pull/46)候选的macOS CI [`34793138251`](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34793138251)用时2分33秒，Windows CI [`34793138222`](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34793138222)用时12分42秒。合并提交`ddb531a891b8aeb25262d27d5d89f4632b51ac2d`的main macOS CI [`34793819831`](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34793819831)用时2分36秒，Windows CI [`34793819847`](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34793819847)用时11分58秒；两端完整门禁、NSIS、GUI subsystem和安装器上传均通过。

注解标签`v0.10.0`精确指向上述main合并提交。正式发布workflow [`34794605355`](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34794605355)三项job全部成功：macOS标签与签名资产复核2分20秒，Windows签名发布17分49秒，同步公开任务17秒。稳定appcast由提交`20681aa3bac864598eb0b1ac091de4e085b82fbe`推进；[GitHub Release v0.10.0](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.10.0)于2026-09-14公开，包含七项资产。

## 公网验收

- macOS ZIP为3,907,171字节，SHA-256 `c90991b7eff5666a7f15aa1c21c4a559e07ab93c620031400bed65eb7087e44b`；匿名下载与公开SHA清单一致，Sparkle EdDSA验签通过，追加字节的篡改副本被拒绝。
- Windows安装器为5,259,791字节，SHA-256 `f7e4320d2b7b3d3204fb603b68da1b36493b4e5dd01286c4505b33455a43d0a1`；`.sig`为428字节，SHA-256 `c3b816186ca1e3f6bf79923e04932fe2b8449a03e618fe01b1cc8035f08e9749`。匿名下载与公开SHA清单一致，应用内置Tauri公钥验签通过，追加字节后以`InvalidSignature`拒绝。
- Release `appcast.xml`与仓库稳定appcast的SHA-256均为`ff48f97632c0719589fb2e024e6c99137dde82194c1636d531d4d6708f81dab3`；Windows stable `latest.json`与旧Preview兼容入口的清单SHA-256均为`c25b94859916ba0bb22f47ea972aab3d30eba553974e05cb80b186e7b9c8dfdc`。三个公开更新入口均解析为0.10.0并包含对应平台签名。

0.10.0/build28已完成实现、审查、PR/main双平台CI、签名发布、七项公开资产与三个更新入口的公网验收。Windows只同步统一版本与签名安装包，未增加Liquid Glass设置或渲染分支。
