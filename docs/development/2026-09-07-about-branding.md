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
