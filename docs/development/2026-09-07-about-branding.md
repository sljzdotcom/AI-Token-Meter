# 2026-09-07：安装包核对与关于品牌展示

关联 REQ-20260907-008/009；[规格](../design/specifications/2026-09-07-about-branding-design.md)、[计划](../design/implementation-plans/2026-09-07-about-branding.md)。

## 安装包核对结论

macOS CLIInstallationScriptBuilder 生成 curl HTTPS 下载脚本，Windows installation::powershell_script 用 Invoke-WebRequest 下载临时安装脚本；用户点击后执行，完整下载失败不执行。Package.swift 与 Tauri bundle 资源列表没有 Claude Code/Codex 本体或官方安装器归档。已有实现就是在线下载，无需重复修改。磁盘上另行安装的 CLI 占用不等于本应用安装包体积；本轮不编造具体 MB 差值。

## 设计与当前阶段

用户授权常规决定直接执行。采用紧凑带图标社交链接，复用本地软件 Logo，不增加图标依赖/网络图片。规格自检无占位或冲突；独立工作树 codex/about-branding 实施，暂未发布。

登记 `e8db7c9`，规格/计划 `a10481e`。开始前 React 75 项和 Swift AppBrand 4 项通过，合同/便携发布 fixture、文档与公开安全门禁通过。后续按测试驱动实现并独立审查。
