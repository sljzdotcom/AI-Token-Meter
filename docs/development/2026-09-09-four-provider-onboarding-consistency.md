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
- 用户验收确认Gemini按钮实际打开额度说明页而非安装方法。两端现改为明确的0.58.0安装指南并固定直达官方安装页；未安装或未知状态显示Node.js 20+、精确npm命令、Google登录和回到应用检查的顺序，待登录时跳过重装，连接后隐藏安装步骤。

## 自动化验证

- Windows失败先行覆盖恢复按钮、DeepSeek历史误导、重复检查按钮、首次保存文案及带认证原因的缓存；最终前端完整回归为122项，并通过TypeScript检查。
- macOS失败先行覆盖四服务恢复矩阵、Settings固定页签和首次DeepSeek失败文案；实现后相关3个Suite共26项通过。
- 完整macOS门禁为461项常规测试、3项独立刷新调度、18项PTY runner和6项Gemini PTY，共488项；6份跨平台合同、发布脚本回归、210份Markdown文档和公开安全扫描通过。
- Windows为122项前端和5项终端输入协议测试、25项密度进程生命周期、16组四服务布局及608个真实浏览器文字角色；production build、258项宿主Rust、`cargo fmt --check`和严格Clippy通过。两次未获权限的Rust全套仅有6项回环服务因沙箱禁止bind失败，同一命令在获准本机环境全部通过。
- 无Widget的macOS Release App完成arm64构建；可移植资源、Sparkle framework/helper、`@rpath`、嵌套组件和严格签名结构通过。
- 初次独立审查为Critical 0、Important 2、Minor 1：Windows原生Key提示框关闭后的网络验证没有真正进入busy状态；有旧官网历史时恢复入口排在历史之后；两个TypeScript文件还带EOF空行。失败先行补充真实Settings生命周期、恢复顺序和后端单事务测试后，验证期间的替换、检查与窗口聚焦刷新均锁定，Rust命令也拒绝并发替换；恢复入口调整到历史之前，格式检查恢复通过。
- 首轮修复复核关闭全部原有发现，并指出一项Minor：普通DeepSeek状态读取与真实Key替换共用验证文案。新增失败测试后以独立的真实替换标志控制文案，普通读取仍禁用按钮但保持“Save API Key”；最终复核精确提交`978a170`为Critical 0、Important 0、Minor 0。
- REQ-20260909-003扩展先让Swift安装策略、Windows缺失/待登录/连接视图和Rust固定URL测试失败；实现后定向Swift、47项相关前端、TypeScript production build与3项Rust链接测试通过。扩展后的完整本机门禁为462项常规Swift、3项刷新调度、18项PTY runner和6项Gemini PTY，共489项；Windows为123项前端、5项终端输入协议、25项密度进程生命周期、8个Gemini详情场景、16组四服务布局、608个浏览器文字角色和259项宿主Rust；严格Clippy、Rust格式、production build、6份跨平台合同、210份Markdown、公开安全与diff检查均通过。无Widget macOS Release App再次通过资源、Sparkle嵌套组件和严格签名验证；扩展候选独立审查仍待提交后完成。
- 扩展候选`ad45fc3`的独立审查为Critical 0、Important 1、Minor 0：macOS把全部Gemini缓存当未知状态而提示重装，Windows详情把全部缓存当连接状态而隐藏过期登录恢复。双平台认证缓存与普通网络缓存视图测试先分别复现错误；现统一为认证缓存保留旧额度并只提示Google登录，普通缓存保持被动展示且不提示安装或登录。修复后完整本机门禁为463项常规Swift、3项刷新调度、18项PTY runner和6项Gemini PTY，共490项；Windows为124项前端、5项终端输入协议、25项密度进程生命周期、8个Gemini详情场景、16组四服务布局和608个浏览器文字角色；production build、6份跨平台合同、210份Markdown、公开安全、Rust格式及无Widget Release构建全绿。Rust生产代码未变，沿用同一候选259项完整宿主回归与严格Clippy证据。修复提交`a5a4aa4`的增量复核确认原发现关闭且没有新增问题，最终为Critical 0、Important 0、Minor 0。
- 实现检查点为`80da33f`，并发、顺序与格式修复为`4149086`，最终文案修复为`978a170`。完整本机门禁再次通过；原生双平台CI、main整合和发布证据将在后续阶段补齐，未完成前不把需求标为完成。

## 安全、隐私与现场边界

本轮没有新增自动安装、自动登录、真实账号操作或凭据读取路径。DeepSeek候选Key仍只进入macOS Keychain流程或Windows原生受保护提示框，固定Settings页签路由不接收路径、命令或凭据。Windows前端与Rust后端都阻止同一时间启动第二次Key替换，避免验证或回滚相互覆盖。

真实Gemini普通OAuth账号、Windows交互式终端/WebView2、物理DPI与官网登录仍是既有现场边界；自动化结果不会替代这些真机检查。

## Git证据

- 接单与审计开始：`57a8293`
- 审计、规格与实施计划：`7886a63`
- 双平台实现与本地候选门禁：`80da33f`
- DeepSeek事务串行化、恢复顺序与格式修复：`4149086`
- 最终复核候选：`978a170`，独立审查Critical/Important/Minor为`0/0/0`
- Gemini安装引导扩展：`ad45fc3`；缓存分流修复：`a5a4aa4`，最终独立复核Critical/Important/Minor为`0/0/0`。
- 整合与发布证据：完成对应阶段后回写。
