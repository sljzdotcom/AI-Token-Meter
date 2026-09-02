# 2026-09-02 公开 GitHub 仓库与 v0.1.2 Release

- **需求：** `REQ-20260902-017`
- **产品版本：** `0.1.2`（build `3`）
- **目标平台：** Apple Silicon `arm64`、macOS 14.0+
- **许可证：** MIT
- **作者：** Miller

## 目标与边界

本阶段把完整项目历史整理为可公开审计的 GitHub 仓库，提供标准社区文档、脱敏产品截图、持续集成和可匿名下载的 Release。公开包继续使用 ad-hoc 签名且未经过 Apple notarization，不包含需要 Apple Development 证书的 Widget；这些限制必须在 README 与 Release Notes 中明确说明。

## 安全门禁

新增 `scripts/check-public-release.sh`，在发布前检查三层内容：

1. 当前跟踪与未跟踪文件；
2. 所有分支的完整 Git 历史；
3. 解压后的 Release ZIP。

门禁会阻止高置信度 API Key、Bot Token、Bearer 凭据、私钥和危险凭据文件名；安装 Gitleaks 后再对目录、历史和归档执行专业规则复核。失败输出只说明阶段，不打印命中的秘密。

测试使用合成凭据验证安全仓库通过、当前文件泄漏失败、仅存在于历史的泄漏仍失败，并断言日志不回显合成秘密。公开前的额外个人标记核对发现旧测试提交曾误用真实格式的个人号码；发布流程因此增加一次定向历史脱敏，不能把“当前文件已删除”误认为“公开历史已安全”。

## 公开文档与展示

- About 在版本信息后显示 `Author: Miller`；
- 根目录加入 MIT License、行为准则、支持说明、Issue/PR 模板和 GitHub Actions；
- README 增加英文摘要、三张脱敏截图、下载/安装、首次打开、作者和许可证；
- 截图来自 Release App 的 Demo Mode 或同一 SwiftUI 详情视图的确定性演示快照，不含真实邮箱、Key 后四位、用户名、路径或账户响应；
- 文档门禁校验截图存在、确为 PNG，并验证 README 的公开发布信息不会被后续修改意外移除。

## 测试与本地发布证据

- 自动化：**331 个测试、67 个测试组、0 失败**；
- 文档门禁：114 份 Markdown 通过断链、版本、测试基线和公开文件检查；
- 工具：GitHub CLI `2.99.0`，Gitleaks `8.30.1`，git-filter-repo `2.47.0`；
- Release 构建：`AI_METER_INCLUDE_WIDGET=0`；
- App：`0.1.2`（build `3`），Mach-O arm64；
- `scripts/verify-app-resources.sh` 与 `codesign --verify --deep --strict` 通过；
- ZIP 清单测试、独立目录解压、标准资源与严格签名复验通过；
- 最小 Finder 风格环境启动后保持运行，由验收进程主动停止，没有立即崩溃；
- 最终 ZIP 大小：2,358,998 bytes；
- 最终 ZIP SHA-256：`b9a71ebef40567c307913b54b9920b780d622c6572302ab0319b3baea19c254d`。

## GitHub 发布证据

- 公开仓库：[sljzdotcom/AI-Token-Meter](https://github.com/sljzdotcom/AI-Token-Meter)；
- 默认分支：`main`，本地脱敏历史已推送；
- 首次 CI 依次暴露并保留了跨工具链类型推断、严格并发、Xcode 版本和共享 runner 时序假设的问题；修复检查点为 `84e81f2`、`f1a0cc4`、`dc9f552`、`fe44847`、`e742f35`；
- [GitHub Actions 33637095658](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33637095658) 在 Xcode 26.3 上通过完整 Swift 测试、文档检查和公开发布安全门禁；
- `e742f35` 用行为探针替代绝对耗时并发断言、用实际时间窗口等待异步启动，并为 PTY 父进程退出后的延迟尾部输出增加先失败后通过的回归测试；
- 后续标签候选 [GitHub Actions 33637436534](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33637436534) 再次暴露完整并行负载下的 PTY 输出丢失和两项真实时钟竞态；根因是 PTY 退出等待通过 detached task 同步阻塞 Swift 并发线程池，而非排空毫秒数不足；
- 退出等待已改为 continuation 驱动的异步通知，同时保留同步 Codex app-server 清理入口；暂停/恢复自动隐藏测试改为手动控制睡眠事件，不再依赖共享 runner 的绝对时间；新增 32 路 PTY 并发输出回归；
- 最终修复检查点 `616ce76` 在本机通过 331 项测试、67 个测试组，并由 [GitHub Actions 33638241625](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33638241625) 在公开仓库完整复验通过；
- 隐私脱敏造成 Git 提交对象重写后，已将现行文档内 69 个失效短提交号全部迁移到当前公开历史中的可解析提交；发布前审计结果为 0 个悬空短提交号；
- `v0.1.2` 标签、Release 资产和匿名下载校验完成后继续回填本节；在这些证据存在前，`REQ-20260902-017` 保持 `进行中`。

## 已知限制

- GitHub 预览包为 ad-hoc 签名、未公证；首次打开可能需要 Finder 右键“打开”；
- 公共 ZIP 不含 Widget；Widget 源码保留，但桌面安装仍依赖 Apple Development 证书与 App Group；
- 当前只提供 Apple Silicon 构建；
- `REQ-20260902-016` 的 M4 Max nvm 实机界面确认仍是独立的 `待用户确认` 事项，不因 GitHub 发布而自动完成。
