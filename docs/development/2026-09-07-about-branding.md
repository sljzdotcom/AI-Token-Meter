# 2026-09-07：安装包核对与关于品牌展示

关联 REQ-20260907-008/009；[规格](../design/specifications/2026-09-07-about-branding-design.md)、[计划](../design/implementation-plans/2026-09-07-about-branding.md)。

## 安装包核对结论

macOS CLIInstallationScriptBuilder 生成 curl HTTPS 下载脚本，Windows installation::powershell_script 用 Invoke-WebRequest 下载临时安装脚本；用户点击后执行，完整下载失败不执行。Package.swift 与 Tauri bundle 资源列表没有 Claude Code/Codex 本体或官方安装器归档。已有实现就是在线下载，无需重复修改。磁盘上另行安装的 CLI 占用不等于本应用安装包体积；本轮不编造具体 MB 差值。

## 设计与当前阶段

用户授权常规决定直接执行。采用紧凑带图标社交链接，复用本地软件 Logo，不增加图标依赖/网络图片。规格自检无占位或冲突；独立工作树 codex/about-branding 实施，暂未发布。

登记 `e8db7c9`，规格/计划 `a10481e`。开始前 React 75 项和 Swift AppBrand 4 项通过，合同/便携发布 fixture、文档与公开安全门禁通过。后续按测试驱动实现并独立审查。

控制者在准备视觉检查时指出初版 GitHub 的通用按键符号不具品牌识别性、macOS 单行链接缺少窄宽回退、前端同步抛错未被 Promise.resolve 参数求值捕获；作为同一需求的实现边界交回实现者补齐可辨识矢量图标、宽度回退和同步/异步失败测试。

## 验证与审查

首轮代码 `d3e55cf`：React 80 项、宿主 Rust 全套、21 项生命周期和 632 个浏览器文本角色检查通过，Swift 完整门禁通过。沙箱首次拒绝监听本机回环端口（EPERM），仅为相关测试开放回环监听后原断言通过，未修改产品逻辑掩盖环境问题。

本地浏览器 About 的中文文案、两个语义链接和内联 SVG 已核对；Windows 头部图片实际尺寸 40×40，来源为既有本地资源。自动化浏览器不等于 Windows 真机视觉验收，本轮不改写既有真机限制。

任务审查发现 macOS 首轮仍以 GH 文字代替图标，且仅测打开操作包装器，遗漏真实视图交互覆盖；两项均交回实现者修复，不把首轮绿色测试当成规格完整的证据。

`580b674` 改为本地 15px 图形并加入真实 NSHostingView/NSButton 点击与提示断言；定向复审进一步指出视图自有模型应使用 StateObject，GitHub 点击还需要精确 URL 断言。`5e5e0e4` 完成模型生命周期、目标断言和失败文案换行修正。

最终本地基线：Swift 419+13=432 项（82 组），React 80 项，Rust 208 项，密度生命周期 21 项和浏览器文本角色 632 项通过；格式、严格 Clippy、production 前端、174 份文档、合同、发布 fixture 和公开安全检查通过。GitHub 原生 CI 和整分支审查通过后方可集成；当前公开版本仍为 0.4.0，无更新源改动。

整分支审查对 `fb0cf55` 无 Critical/Important/Minor；最终 Release 资源与签名验证通过。但原生 macOS CI `34088536876` 在 BrandLinksViewTests 的失败提示视图断言失败，主测试 419 项中仅该项失败。本机正常，CI 日志未报告点击目标或 openingFailed 状态错误，差异集中在 SwiftUI 更新后的视图树读取。登记 REQ-20260907-010，继续定位有界等待主线程视图提交；保留原断言，不以复跑绿色替代修正证据。

`48cbd0f` 仅修正测试对异步渲染的等待：两秒 ContinuousClock 截止，10ms 异步挂起让主线程提交更新，实际视图条件满足即返回；不重试点击，不跳过断言，不改产品行为。本地完整 432 项再通过，定向最终复审无 Critical/Important/Minor，等待该精确提交的原生 CI。

原生 CI `34088933770` 已通过两个 About 真实交互测试；此次仅既有 `CLICollectorTests.codexEarlyTimeoutIsReplayed` 的一秒耗时断言失败（1.006975s），timedOut/PID/进程退出断言没有报告错误，相关产品与测试文件本轮没有改动。归入既有 REQ-20260906-003 保留追踪；记录一次同提交完整门禁复验，不放宽原截止时间，也不宣称历史时序问题已修复。

## 完成与集成

- [macOS CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34088933770) 同提交第二次完整门禁通过；上文保留初次失败，不把偶发时序问题改成已修复。
- [Windows CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34088933785) 通过：80 项前端、21 项生命周期/632 个样式角色、218 项原生 Rust、严格 Clippy、NSIS 安装器、PE GUI 子系统与上传。
- macOS 最终 Release App 资源、嵌套签名与 Sparkle 验证通过；构建用于验收，未安装、启动或发布。
- [PR #11](https://github.com/sljzdotcom/AI-Token-Meter/pull/11) 合并为 `12ad5e2`；已核对合并树与验证头 `48cbd0f` 无差异。
- REQ-20260907-008/009/010 完成，需求、指南、CHANGELOG、测试基线及索引同步。公开版本仍为 0.4.0/build12，本轮没有发布 Release 或修改更新源。
- 已关闭本轮预览服务/临时浏览器页，移除工作树临时依赖符号链接；误启动预览产生的 .vite 缓存移到临时目录保留，可恢复，源代码和实际 node_modules 未删除。
