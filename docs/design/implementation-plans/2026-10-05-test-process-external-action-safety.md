# 测试进程外部操作隔离实施计划

> **面向 AI 代理的工作者：** 按项目TDD、验证与独立审查规则执行；此计划优先建立无副作用证明，再扩大测试范围。

**目标：** 阻止测试中漏注入的登录/系统打开默认依赖触及真实Terminal、浏览器、agy或用户认证目录，同时保留生产显式操作。

**架构：** 以纯进程策略集中识别XCTest上下文，默认拒绝系统打开并将脚本目录导向临时目录；AppModel与低层launcher双层调用策略。测试通过假launcher和计数哨兵验证行为，不运行真实登录脚本。

**技术栈：** Swift 6、AppKit、Swift Testing、现有CLI launchers。

---

## 文件职责

- 新建 `Sources/AIMeterApp/System/SystemActionPolicy.swift`：纯测试上下文判定、打开决策和默认目录选择。
- 修改 `Sources/AIMeterApp/AppModel.swift`：默认认证、安装与帮助打开操作通过当前策略；保留显式注入覆盖。
- 修改 `Sources/AIMeterApp/System/CLIAuthenticationLauncher.swift`：拒绝时不写认证脚本；默认测试目录为临时目录，显式假打开回调仍可测试脚本合同。
- 修改 `Sources/AIMeterApp/System/CLIInstallationLauncher.swift`、`CodexInstallationGuideLauncher.swift`、`ClaudeWorkspaceSetupLauncher.swift` 与 `Sources/AIMeterApp/Views/BrandLinksView.swift`：保护所有相关默认系统打开入口，并允许测试哨兵注入。
- 修改 `Tests/AIMeterAppTests/CLIAuthenticationLauncherTests.swift` 与新增 `Tests/AIMeterAppTests/SystemActionPolicyTests.swift`：先验证合成XCTest策略纯函数与launcher拒绝边界。
- 修改 `Tests/AIMeterAppTests/GeminiAvailabilityTests.swift`：为两个原缺失测试补专用模拟登录闭包。
- 修改必要的 `ServiceAccountSettingsTests.swift` / `SettingsStructureTests.swift`：验证Claude/Codex及其他默认系统打开边界，并保持现有显式模拟。
- 更新 `docs/design/README.md`、`docs/requirements-backlog.md`，完成后新增开发记录并回传协调入口。

## 任务 1：安全策略纯函数红灯

- [x] 新建 `SystemActionPolicyTests.swift`，用字典 `['XCTestConfigurationFilePath': '/tmp/fake.xctest']` 构造策略，断言 `allowsExternalOpen == false`；用计数器传入模拟系统打开函数，断言调用策略后计数仍为0。
- [x] 为同一测试上下文断言认证目录解析到提供的临时目录之下；为普通进程字典断言系统目录解析仍使用Application Support。
- [x] 仅编译并运行该新策略测试，确认失败原因是缺少策略类型/行为；该测试不能引用 `NSWorkspace`、真实用户路径或认证脚本。

## 任务 2：建立低层 fail-closed 边界

- [x] 实现 `SystemActionPolicy`，默认读取当前进程环境；用Bundle扩展/已知XCTest环境键识别测试进程，不依赖 `DEBUG` 以免影响开发版生产登录。
- [x] 修改 `CLIAuthenticationLauncher`：默认路径遵循策略；显式 `openURL` 仅代表测试替身；默认测试操作在创建目录、写入文件、系统打开前抛出明确拒绝错误。
- [x] 增加launcher测试：合成测试策略+固定CLI定位器+打开哨兵，执行 `open(.gemini)` 后确认拒绝、哨兵为0、临时认证目录为空；旧脚本正文不被写入真实认证目录。
- [x] 运行 `swift test --filter SystemActionPolicyTests` 和 `swift test --filter CLIAuthenticationLauncherTests`，确认通过并阅读完整输出。

## 任务 3：AppModel默认保护及原遗漏修复

- [x] 先增加AppModel安全测试：注入一个只记录调用的假 `CLIAuthenticationLauncher`，不注入Claude/Codex/Gemini专用认证闭包，在XCTest上下文调用默认登录操作；断言launcher哨兵为0、Gemini无pending token且Claude/Codex不进入认证轮询。
- [x] 新测试只调用假的launcher与内存状态；运行前核实策略测试已通过，因此即便AppModel默认保护缺失，也没有真实系统依赖。
- [x] 为 `GeminiAvailabilityTests` 两处 `beginSignIn(.gemini)` 显式传入专用记录/拒绝闭包，断言不可用/已连接场景都没有发起登录。
- [x] 实现AppModel的默认认证、安装与指南操作策略门控，不更改用户显式点击后的生产操作顺序。
- [x] 运行新的AppModel安全测试与 `GeminiAvailabilityTests`，确认所有测试只触发假闭包且通过。

## 任务 4：其他外部打开入口与测试边界

- [x] 扩展纯策略表格测试覆盖测试进程拒绝、普通进程允许以及显式注入的fake opener照常被调用。
- [x] 将CLI安装脚本默认目录改为测试临时目录，并在写入前拒绝默认测试打开；将Codex指南、Claude workspace setup及品牌链接默认打开改为同一策略并注入打开哨兵。
- [x] 审计仓库全部 `NSWorkspace.shared.open` 与AppModel启动闭包；测试环境默认路径必须经过策略，测试显式假回调继续按预期记录。
- [x] 运行对应的launcher/设置测试筛选，确认Claude、Codex、Gemini行为与生产文案合同不变。

## 任务 5：门禁、全量验证和审查

- [x] 静态核实防护在目录创建、写入和调用系统打开之前执行；确认无测试入口会启动登录脚本、浏览器、`agy`或真实认证。
- [x] 运行 `scripts/check-docs.sh`、`git diff --check`，修复所有文档/格式问题。
- [x] 完整 `swift test` 与常规macOS非签名构建已运行；Core/Widget套件通过，App套件除大小写敏感本地化镜像用例外通过，该用例因`hdiutil: Device not configured`未能创建测试卷。针对性外部动作/诊断测试通过；未运行真实CLI或系统集成路径。
- [x] 运行全量 `NSWorkspace.shared.open` 静态盘点；进程元数据只读查询受本机`sysmond`服务错误限制，已记录，未清理真实会话。
- [x] 使用 `gpt-6-luna` / `high` 执行独立代码审查；最初发现安装专属错误类型Minor，已补齐启动器与AppModel守卫并针对性复验。最终复核REQ-003与REQ-002诊断版本补充，Critical/Important/Minor均为0。
- [ ] 更新REQ-20261001-003状态及开发记录并提交本地检查点。用户已另行授权公开发布，发布工作依赖REQ-20261001-002、003完成及双平台发布门禁，记录于REQ-20261005-001。

## 完成标准

红灯先行在纯函数/模拟边界中展示预期失败；通过后才允许运行原风险测试。所有Provider与系统打开默认路径均有fail-closed覆盖，显式模拟仍工作，生产显式登录无行为回归。文档、测试和独立审查有新鲜证据，本地Git提交可审查；M4 Max额度现场与旧会话归因明确保留未解决。公开发布须按REQ-20261005-001完成，不属于此计划的本地修复检查点。
