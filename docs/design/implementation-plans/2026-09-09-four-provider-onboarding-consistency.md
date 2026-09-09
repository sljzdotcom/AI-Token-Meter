# 四服务商新用户接入一致性实施计划

**需求：** REQ-20260908-015；完成后接续已授权的REQ-20260909-001稳定版发布。

**规格：** [四服务商新用户接入一致性：审计与设计](../specifications/2026-09-09-four-provider-onboarding-consistency-design.md)

**目标：** 修正双平台四服务商在浮动条、详情和Settings之间的恢复路径与文案，同时保持现有发现、安装、登录、采集和凭据安全边界。

## Task 1：锁定跨表面恢复策略

- [x] Swift和TypeScript先增加四服务商×快照状态的失败测试。
- [x] 实现纯恢复策略：Claude工作区直达、Claude/Codex/DeepSeek打开Services、Gemini保留专用重试/文档。
- [x] 验证fresh/普通cached/refreshing不产生错误的配置提示，unavailable不被改成notInstalled；带登录、凭据或设置失败原因的cached保留旧额度并给出恢复动作。

## Task 2：修正Settings首次接入和重复动作

- [x] 先增加双平台Claude/Codex不可用状态只出现一个“Check Status”的视图断言。
- [x] Windows DeepSeek先增加无Key时`Save API Key`、已有Key时`Replace API Key`的测试。
- [x] 双平台DeepSeek失败文案按首次保存/替换区分，保留先验证后写入与失败回滚。
- [x] 运行Settings、账户控制器、安装/登录及凭据定向测试。

## Task 3：打通详情到Services的恢复路径

- [x] macOS先测试Settings路由选择Services及详情动作策略，再实现固定页签通知和打开动作。
- [x] macOS DeepSeek在API Key缺失时展示配置说明与Services按钮，不启动无关的官网历史登录；已有凭据/数据时保留历史功能。
- [x] Windows先测试详情恢复按钮和固定Services路由，再扩展`open_settings`命令及详情调用。
- [x] Gemini两端继续使用重试与用户触发的官方安装指南，不新增自动安装或登录。
- [x] 按REQ-20260909-003失败先行核对链接，确认旧按钮误开额度页；改为官方安装页，并按未安装、待登录、已连接状态呈现固定0.58.0步骤与回流检查。

## Task 4：审计证据、完整验证与整合

- [x] 更新开发日志、README/文档索引（如新增文档需要）及需求台账证据。
- [x] 运行`scripts/check-docs.sh`、Swift完整测试/Release构建、Windows前端/Rust/构建及相关浏览器门禁。
- [x] 请求独立审查，关闭Critical/Important发现后重新运行受影响门禁。
- [x] 对REQ-20260909-003扩展候选重新运行完整门禁和独立审查，关闭全部发现。
- [ ] 由开发入口安全整合到main并回传提交、测试、审查和现场边界。

## Task 5：发布下一稳定版本

- [ ] 将REQ-20260909-001设为进行中，按仓库发布合同选择下一补丁版本并同步双平台元数据。
- [ ] 完成发布前门禁、公开推送、Tag、签名Release、双平台CI和更新源切换。
- [ ] 匿名重下资产，核对SHA-256、Sparkle/Tauri签名、稳定更新源和应用内可发现性。
- [ ] 回写发布记录与需求台账，向协调入口回传最终证据。
