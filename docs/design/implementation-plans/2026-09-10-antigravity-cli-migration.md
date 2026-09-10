# Google Antigravity CLI 接入迁移实施计划

**需求：** REQ-20260909-007  
**规格：** [Google Antigravity CLI 接入迁移](../specifications/2026-09-09-antigravity-cli-migration-design.md)  
**确认：** 用户于2026-09-10回复“按照你的推荐继续执行”。  
**发布边界：** 实现、测试、审查与整合已授权；本需求不自动发布新版本。

## 目标与架构

把现有第四服务从旧 Gemini CLI 0.58.0 迁移到官方 Antigravity CLI `agy`。应用使用 CLI 自身处理的 `agy -p /usage` 读取四个官方额度窗口，不发送模型提示。macOS 用有界普通进程替换旧 PTY `/model` 会话；Windows 用有界隐藏进程替换旧 ConPTY 会话。持久化 provider ID 继续为 `gemini`，用来无损保留排序、隐藏、缓存和 Widget 数据，所有用户可见名称统一为 Google Antigravity。

## Task 1：额度合同与解析器

**范围：** Swift/Rust额度模型、解析器、共享fixture与合同。

- [x] 先添加脱敏四行 `/usage` fixture，以及成功、顺序变化、0/100、缺行、重复、未知列、非法百分比与时间的失败测试。
- [x] Swift 与 Rust 把 Remaining 转为现有已用比例，同时保留明确的额度标签与重置时间。
- [x] 严格拒绝不完整或歧义输出，失败时不产生0%快照。
- [x] 共享合同证明两端对相同fixture产生相同四窗顺序和数值。

## Task 2：官方命令、发现与生命周期

**范围：** macOS `GeminiCollector`/命令执行环境，Windows discovery/runtime，账户状态。

- [x] 测试先行把可执行文件从`gemini`改为`agy`，覆盖官方默认路径与Native Windows路径，不回退旧命令。
- [x] 固定执行`agy -p /usage --print-timeout <上限>`；使用专用空目录、受控环境、输出上限、总超时、取消和进程树清理。
- [x] 版本1.1.28作为现场基线；同一主版本仍须通过严格输出合同，未来主版本失败关闭。
- [x] 识别未安装、认证失败、网络/进程失败、超量输出和取消，任何错误展示前脱敏。
- [x] 合成运行器证明没有自然语言、模型、工具、登录/退出或配置写入参数。

## Task 3：双平台产品界面与无损迁移

**范围：** SwiftUI、React/Tauri、Widget、Settings、详情、可访问名称、安装与登录引导。

- [x] 所有用户可见`Gemini`/`Gemini CLI`迁移为`Google Antigravity`/`Antigravity CLI quota`，历史文档和兼容代码键除外。
- [x] 详情展示Gemini models与Claude and GPT models各自的Five hour和Weekly额度；环表示已用，卡片明确Remaining与重置时间。
- [x] Settings改用官方Antigravity安装页、macOS/Linux脚本、Windows PowerShell说明和`agy`登录/重试步骤。
- [x] 现有`gemini` provider ID、用户排序/隐藏选择、缓存和Widget解码保持兼容，不创建第五个按钮。
- [x] 沿用已有四角星Logo，直到有明确可分发的Antigravity品牌资产证据。

## Task 4：文档、全量验证与整合

- [x] 更新README、CHANGELOG、用户指南、架构数据源、安全说明、测试说明、开发日志和索引。
- [x] 运行`scripts/check-docs.sh`、跨平台合同、Swift/前端/Rust完整测试、真实浏览器、Release App与公开安全检查。
- [x] 完成范围、隐私、兼容、平台差异和测试缺口复核；缺陷按严重度修正后重新验证。
- [ ] 提交并通过PR/main双平台原生CI，更新需求为已完成并回传证据；保持版本0.6.3且不创建tag或Release。
