# Antigravity 一次性受控恢复入口

**需求：** REQ-20261001-002  
**基线：** v0.10.6 / build34；本地产品修复从 0.10.6 源码基线开始  
**状态：** 已在独立 macOS 开发分支实现和自动化验证；未发布、未运行真实 `agy`、未登录或改动已安装应用及现场暂停状态。

## 背景与目标

用户再次提供的本机诊断仍为 `refreshPaused=true reason=unknown`、`lastQuotaAt=2026-10-01T05:18:21Z`、`stage/result=unknown`。这与 0.10.6 的持久暂停设计一致：旧版本迁移时原因缺失会保留为 `unknown`，该版本没有恢复入口，诊断环为空就不会编造阶段。它不证明账号已登出、额度耗尽或发生了新的失败。

本次已批准产品修复为未知/认证及其他持久暂停添加同一个用户主动入口。官方登录成功回执最多启动一轮只读额度查询；成功也保留持久暂停，避免回到无限周期重试。实现只在本地分支，不发布。

## 实现范围与决定

- 登录脚本将本次 `agy` 置于独立进程组，使用 300 秒看门狗；只对该组发出 TERM/KILL，并等待整个进程组结束，避免 CLI 主进程退出后遗留子进程。成功、非零退出、超时和终端关闭分别回送含 UUID token 和退出码的固定回执。UUID 只在内存和短暂的 0600 FIFO 中传递，不写入登录脚本。
- AppModel 使用内存 token、330 秒 deadline 和单次状态机。错误、取消、超时、过期或伪造回执均不会调用额度查询。`stop()` 会使 token 失效并取消任务。
- 独立 collector 路径只运行版本检查和 `/usage`；不运行 `/model`、`models`、全局刷新或其他 provider 命令。成功解析后只原子替换 Gemini 缓存并保留其他 provider 数据，不清除持久 `suspended` 记录。并发的普通 provider 刷新不会把较早缓存覆盖到磁盘或 UI/Widget。失败保留原缓存。
- Services 和 Antigravity 详情在所有持久暂停原因下显示“一次性检查额度”；英文、简体和繁体均说明 Terminal 登录、可能出现浏览器授权，以及成功后自动刷新仍暂停。界面不声称能阻止 CLI 打开浏览器。
- 登录结果和 collector 阶段继续写入脱敏的诊断白名单。无日志的旧暂停仍显示 unknown。
- Windows Rust 行为与跨平台 JSON 合同未修改。

## 验证证据

- 最终恢复相关定向回归：37 项通过，其中登录脚本/进程组/回执、协调器与 AppModel 刷新竞态均覆盖；包含 TERM-resistant 子进程、组长先退出、内存/磁盘缓存竞争。
- 完整 Swift suite：323 项、55 个 suite 通过，明确跳过 `AIMeterAppTests.AppLocalizationTests/caseSensitiveResourceLookup`。无跳过的 `scripts/test.sh` 运行中唯一失败仍是此大小写敏感 HFSX 磁盘映像测试：受限执行环境 `hdiutil` 返回 `Device not configured`。
- `scripts/check-docs.sh`：348 个 Markdown 文件通过；跨平台合同 6 fixtures、Windows 资产归一化、release feed 与公共发布安全检查通过；`bash -n` 和 `git diff --check` 通过。
- 独立代码复审发现并推动修复三类竞态/生命周期问题（超时后残留进程组、较早普通刷新覆盖新缓存或界面额度），以及启动失败遗留 FIFO 的清理问题；对应回归和最终定向复审均通过。
- 未运行真实登录、CLI、浏览器、Keychain、生产应用或 OS 隔离探针；没有现场验证或可更新安装包。

## 未完成边界

此本地提交不会改变现有安装版。用户当前仍会看到 0.10.6 的“已暂停”，因为这次产品修复不清除持久暂停；恢复入口须随之后另行授权的版本交付才会出现在应用里。即使新代码完成一次额度读取，诊断的 `refreshPaused` 仍应为 `true`，成功的 `stage=login/usage` 记录和更新后的 `lastQuotaAt` 才能显示单次恢复是否完成。未验证真实 M4 Max 额度恢复。
