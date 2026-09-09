# 2026-09-09：四服务商新用户接入一致性

关联需求：`REQ-20260908-015`。规格与实现基线见[审计与设计](../design/specifications/2026-09-09-four-provider-onboarding-consistency-design.md)和[实施计划](../design/implementation-plans/2026-09-09-four-provider-onboarding-consistency.md)。

## 背景与目标

双平台已经能发现Claude Code、OpenAI Codex、DeepSeek和Gemini的依赖与账户状态，但新用户从浮动条进入详情后，Claude/Codex/DeepSeek缺少统一的恢复入口；DeepSeek首次配置还混用了“替换旧Key”和“官网登录历史”的文案。本轮让每个可恢复状态都能到达真正解决问题的入口，同时保留现有安装、登录、采集和凭据安全边界。

## 问题证据

- Claude/Codex状态为`unavailable`时，Services同一卡片会出现两个同名的“Check Status”。
- Windows在没有DeepSeek Key时仍显示“Replace API Key”；双平台首次验证失败会错误提到不存在的旧Key。
- macOS在DeepSeek API Key缺失时先展示官网登录，但该登录只负责30天历史，不能恢复余额API。
- Claude/Codex/DeepSeek详情能说明失败，却不能直接打开Services。旧额度因登录或设置问题进入缓存时，详情也缺少恢复动作。

这些行为先由Swift和TypeScript失败测试锁定，再加入实现。普通网络超时缓存继续保持只读，不误标为需要安装或登录。

## 实现与关键决定

- 两端使用纯恢复策略覆盖四服务和全部快照状态。Claude在macOS工作区确认时直达一次性授权；其他Claude/Codex/DeepSeek问题打开Services；Gemini继续使用既有重试和官方文档。
- 详情打开Settings时固定选择Services。macOS通过应用内固定通知路由，Windows的Tauri命令只接受四个固定页签名。
- Claude/Codex主操作已经是“Check Status”时不再显示第二个同名按钮，其他状态仍保留独立检查。
- DeepSeek首次配置显示“Save API Key”，已有Key显示“Replace API Key”。候选Key仍先验证再写入；首次失败只说明新Key未保存，替换失败才说明旧Key保留。
- DeepSeek API Key缺失时先引导Services。官网登录继续只用于官网历史；已有历史数据和旧额度仍可保留显示。

## 自动化验证

- Windows失败先行覆盖恢复按钮、DeepSeek历史误导、重复检查按钮、首次保存文案及带认证原因的缓存；实现后前端完整回归为119项，并通过TypeScript检查。
- macOS失败先行覆盖四服务恢复矩阵、Settings固定页签和首次DeepSeek失败文案；实现后相关3个Suite共26项通过。
- Windows Rust固定页签白名单测试通过；`cargo fmt --check`和`git diff --check`通过。
- 完整Swift、Release构建、Windows Rust/浏览器密度、独立审查、原生双平台CI和发布证据将在本记录后续阶段补齐，未完成前不把需求标为完成。

## 安全、隐私与现场边界

本轮没有新增自动安装、自动登录、真实账号操作或凭据读取路径。DeepSeek候选Key仍只进入macOS Keychain流程或Windows原生受保护提示框，固定Settings页签路由不接收路径、命令或凭据。

真实Gemini普通OAuth账号、Windows交互式终端/WebView2、物理DPI与官网登录仍是既有现场边界；自动化结果不会替代这些真机检查。

## Git证据

- 接单与审计开始：`57a8293`
- 审计、规格与实施计划：`7886a63`
- 实现、复审、整合与发布证据：完成对应阶段后回写。
