# macOS Services 产品 Logo

关联需求：REQ-20260912-003。规格见[设计文档](../design/specifications/2026-09-12-macos-services-provider-logos-design.md)，实施步骤见[计划](../design/implementation-plans/2026-09-12-macos-services-provider-logos.md)。

## 结果

候选实现`eeaf215`在macOS Settings → Services的Claude Code、OpenAI Codex、DeepSeek和Google Antigravity四个分组标题前显示对应Logo。四组标题复用同一组件，Logo使用18pt布局框、6pt名称间距和系统`.primary`前景色；浅色与深色外观自动适配。装饰Logo继续隐藏于辅助功能树，产品名称只朗读一次。

现有ProviderLogo默认白色外观、资源映射和光学校正保持兼容，浮动条与详情调用无需修改。账户状态、安装登录、DeepSeek Key、按钮、导航、额度采集和数据展示逻辑均未改变；Core与Windows代码没有差异。

## 实现与审查

- `912f130`为ProviderLogo增加默认`.white`的可选tint；`6303279`补齐四产品精确资源、默认白色、18pt布局和`1.28 / 1.0 / 0.92 / 1.0`光学校正的可判别回归。
- `eeaf215`把四个Services分组标题迁移到共享`ServiceSectionHeader`，统一使用`.primary`、18pt和6pt，并加入真实`NSWindow`/`NSHostingView`像素与辅助功能测试。
- 任务1和任务2独立审查最终均为Critical/Important/Minor `0/0/0`。本阶段再次核对四产品映射、尺寸、浅深色、辅助功能与差异范围，未发现需修改产品或测试的问题；最终完整分支审查与本地main整合仍由后续任务完成。

## 验证

- 真实窗口专项使用串行执行：`AI_METER_SCREEN_TESTS=1 scripts/test.sh --no-parallel --filter 'Services|Settings|Localization|ProviderLogo|FloatingStrip'`通过147项、18个suite，0问题；四个Logo在英文、简体中文、繁体中文及浅深色外观的真实宿主中均可见、匹配、未裁切，辅助功能标题各出现一次。
- 首次在受限环境运行同一专项时出现9个环境问题：7项因`NSScreen.screens`为空，1项因大小写敏感卷报`Device not configured`，1项真实宿主返回空窗口。允许真实桌面与磁盘设备访问后，原命令全部通过；两份原始日志均保留，没有删测、改断言或修改产品来获得绿灯。
- `scripts/test.sh`通过：主测试538项、独立刷新调度3项、PTY runner 18项，共559项Swift测试；6份跨平台合同、合同可移植性、Windows发布资产归一化、更新源探针、当时292份Markdown及公开发布安全检查同时通过。本文档加入索引后，最终文档门禁为293份Markdown通过。
- `scripts/build-app.sh`通过，生成并验证`dist/AI Token Meter.app`；主App、Sparkle framework及嵌套helper签名有效。当前机器没有Apple Development身份与Team ID，按既有规则跳过Widget，主App使用有效ad-hoc签名。
- Release App继续包含`claude.png`、`codex.svg`、`deepseek.svg`、`gemini.svg`四份Logo，以及`en.lproj`、`zh-hans.lproj`、`zh-hant.lproj`三份`Localizable.strings`。
- 产品差异只包含`ProviderLogo.swift`与`ServicesSettingsView.swift`；测试差异只包含两组AIMeterAppTests。没有AIMeterCore、Windows、资源、网络、账户或数据逻辑变更。
- 原始专项、完整测试、构建、产物清单、签名和范围日志保存在本地任务目录`.superpowers/sdd/2026-09-12-macos-services-provider-logos/final-gates/`。

## 交付边界

需求保持`进行中`，等待最终完整分支审查及本地`main`整合。本阶段没有合并`main`、推送、创建版本、修改更新源或发布；用户未授权本需求发布。
