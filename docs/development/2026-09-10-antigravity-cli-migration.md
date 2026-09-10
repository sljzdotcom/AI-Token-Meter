# Google Antigravity CLI 迁移

关联 `REQ-20260909-007`，规格见[设计说明](../design/specifications/2026-09-09-antigravity-cli-migration-design.md)，执行步骤见[实施计划](../design/implementation-plans/2026-09-10-antigravity-cli-migration.md)。

## 结果

第四项服务的可见名称改为 **Google Antigravity**，内部继续使用 `gemini` Provider ID，旧设置中的显示开关、排序、缓存文件名和 Widget 身份不变。Windows 加载旧缓存时会把可见名称迁移为 Google Antigravity，并保留上次成功额度直到新采集完成。

生产采集只发现 `agy`，不回退到旧 `gemini`。macOS 与 Windows 都在私有空目录运行固定的 `agy -p /usage`，保留真实 HOME 供官方 CLI 恢复既有登录，不读取或修改用户配置。进程环境拒绝 API Key、代理、动态库和启动脚本等覆盖，版本检查与额度读取分别受超时和输出大小限制；临时日志及 `.env` 随采集目录销毁。

解析器要求四行完整的制表符输出：

- Gemini · Five hour；
- Gemini · Weekly；
- Claude/GPT · Five hour；
- Claude/GPT · Weekly。

CLI 的剩余百分比统一换算为内部已用百分比，详情卡显示原始剩余口径并保留每个窗口的 RFC 3339 重置时间，顶部百分比、进度条和圆环取已用口径。首期不展示 AI Credits。旧 Gemini CLI 0.58.0 的 `/model` 终端观察、按键注入、ConPTY 会话及 npm 启动适配均已从生产与当前测试链移除；历史契约仍保留为 0.6.0 发布证据。

## 官方能力证据

本机官方 `agy` 1.1.28 已在用户授权的只读范围内完成脱敏探测。`agy -p /usage` 由 CLI 自身处理斜杠命令，没有自然语言模型提示；输出结构包含 Gemini 与 Claude/GPT 两组 Weekly、Five Hour 窗口。仓库只保存使用虚构百分比和时间戳的[无账号合成契约](../../contracts/antigravity-cli/1.1.28/README.md)，不保存个人额度、邮箱、Credits 或原始日志。

官方参考：

- [安装](https://antigravity.google/docs/cli/install/)
- [`/usage` 命令](https://antigravity.google/docs/cli/commands/usage)
- [Headless 模式](https://antigravity.google/docs/cli/headless/)
- [CLI 参考](https://antigravity.google/docs/cli/reference/)

## 验证

- Swift 共482项通过：461项普通组、3项独立刷新调度和18项PTY runner；Antigravity 新增普通 Pipe 有界进程、解析、采集、缓存和引导回归；
- Windows 宿主 Rust 241项通过，格式与严格 Clippy 通过；原生 Windows 条件编译、Job Object、安装器与GUI子系统等待PR流水线复验；
- 前端18个测试文件、124项测试和production build通过；真实Chrome密度门禁通过25项进程生命周期、8个Antigravity详情状态、16个浮动条布局和608个文字角色；
- 六份跨平台快照契约及破坏性回归、232份Markdown、发布辅助脚本、公开安全和差异检查通过；
- 无Widget的macOS 0.6.3 arm64 Release App完成资源、Sparkle嵌套组件与严格签名验证；版本、标签、Release和更新源未改变。

提交前范围、隐私、缓存兼容和平台差异复核发现并关闭四项缺口：macOS改用普通有界进程、详情明确显示剩余口径、Windows四窗口缓存状态避免底部裁切、架构概览移除旧PTY/ConPTY现状描述。最终自审无遗留的Critical、Important或Minor发现；PR原生双平台CI仍是合并前门禁。

真实 Windows 11 GUI/DPI 与用户设备上的四窗口显示仍属于现场验收；本项不自动发布。

PR #22 的实现候选 `5dc6be0` 已通过原生双平台门禁：macOS workflow `34420886177` 用时2分29秒；Windows workflow `34420886221` 用时11分42秒，包含真实 Microsoft Edge 密度、完整 Rust runtime、NSIS、GUI subsystem 与安装器上传。首轮 Windows workflow `34420661996` 曾准确发现340×760缓存详情越界，修复后同一门禁通过；最终证据提交与合并后 main CI 仍待完成。
