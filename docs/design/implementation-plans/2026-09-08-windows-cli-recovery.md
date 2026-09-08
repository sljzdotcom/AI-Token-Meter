# Windows CLI 恢复实施计划

> 执行方式：subagent-driven-development，依序实现与独立审查；按用户授权直接执行推荐方案。

目标：修复 REQ-20260908-001 中 npm Codex 启动、发现错误分类与 Claude 额度初始化链路。
规格：[Windows CLI 恢复设计](../specifications/2026-09-08-windows-cli-recovery-design.md)。

## 全局约束

不改凭据、不自动登录/信任、不重装、不硬编码个人路径；只改 Windows。所有实现先红后绿；保留受限环境、超时、取消、进程清理。所有任务在 codex/windows-cli-recovery 工作树中依序执行；不覆盖他人编辑。文档设计仅存 docs/design；实现后更新原开发日志、需求台账和当前文档。发布与真机验收不能因本地测试通过而宣称完成。

## Task 1: 显式 npm Codex 与 Node 启动

所有权：windows/src-tauri/src/platform/windows/executable_locator.rs、必要的同目录 npm 辅助模块及 mod.rs、tests/executable_locator.rs 和新增 npm 运行回归测试/fixture。不得修改账户或采集业务。

1. 阅读现有定位器、process.rs、DiscoveryInputs、测试。先验证官方 Codex npm 包入口形态（优先本机包、其次官方来源）。
2. 写失败回归：用户 npm 包装器与 Program Files/nodejs/node.exe 分离；路径空格；缺 Node/包入口/错误包身份不能算健康；原生/自定义/WSL 兼容。
3. 最小实现：对验证过的标准 Codex npm 安装定位真实 JS 入口，以 Node 加结构化参数启动；Node 从有界候选目录定位，不硬编码用户名，不靠宽松继承 PATH。候选返回保留实际启动信息，所有进程调用者可复用。
4. 新增 Windows-only 原生执行测试，在清理 PATH 的环境证明入口真实运行；本机不可执行部分明确注明，交 CI。
5. 运行 executable_locator、cli_discovery、process_runner 与相关测试，格式化、自审、提交。报告文件记录先红后绿命令与结果、Windows 验证边界。

## Task 2: 统一发现结果与失败分类

所有权：windows/src-tauri/src/collectors/application.rs、accounts/windows_service.rs、必要的共享运行时发现模块及 mod.rs、相关发现/采集测试。保留 Task 1 实现。

1. 阅读纯策略 accounts/cli_discovery.rs、账户和采集当前重复发现代码。
2. 写失败回归：候选存在但启动失败为 Unavailable，不为 Missing；取消不折叠；真正缺失仍为未安装；显式路径与 WSL 规则一致。
3. 提取/复用统一有界发现执行：账户、额度、登录均使用同一策略；保留 8 秒/12 进程等预算、取消与单独进程超时。仅 Missing 生成 NotInstalled；Unavailable 生成可重试失败，不提示重装。
4. 避免账户模块与采集模块循环依赖，不复制整套策略；保留原生 exe/npm/WSL 路径选择与自定义路径验证。
5. 运行相关 Rust 测试、自审与提交。报告覆盖真实生产映射，不只有纯枚举测试。

## Task 3: Claude 隔离工作区与初始化入口

所有权：Windows Claude 工作区辅助模块、collectors/application.rs、accounts/windows_service.rs、lib.rs、src/Shell.tsx、settings/SettingsWindow.tsx、localization 与对应测试；必要的 onboarding 集成。保留前两任务实现，不改 macOS。

1. 阅读 macOS ClaudeUsageWorkspace/ClaudeSetupScriptBuilder 作为已验证行为参考，Windows 已有状态/刷新/onboarding 流程与 WSL 运行接口。
2. 先写失败测试：采集与初始化共用应用专用空工作区，非用户主目录；WSL 目录属于相应发行版；启动参数不自动信任；已登录也可初始化；忙碌/失败/完成检查提示；状态检查触发重新额度采集而不伪造成功。
3. 新增明确的用户触发初始化命令，原生 CLI 在新终端打开相同专用目录；WSL 使用固定可信脚本和结构化发行版参数创建并进入隔离目录，不插入任意用户命令。不自动接受 CLI 提问。
4. Settings Claude 服务添加中英文“初始化额度读取”按钮和必要说明，复用忙碌控制；初始化启动仅提示用户完成后检查。检查状态刷新实际额度并清除已有阻塞，不重复登录。保留账户信息，不把额度错误覆盖成账户登出。
5. Rust/前端交互测试及构建通过，检查 Tauri 权限和命令接线；自审、提交。

## 最终验证与交付

独立分支总审；运行 scripts/check-docs.sh、Rust 全套、前端测试/构建/现有 UI 门禁及 Windows 原生 CI。更新原调查日志、用户故障排查、项目状态、CHANGELOG 和需求证据。检查隐私与 git diff。按既有集成授权在验证通过后合并；没有公开发布时明确告知更新渠道尚未变化。用户真机问题尚未复验时如实保留边界。
