# AI Token Meter 全项目复盘与文档治理实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 `executing-plans` 逐任务实现此计划。步骤使用复选框跟踪进度。

**目标：** 建立单一、可验证、可长期维护的项目知识体系，并删除已证明冗余或可再生的仓库残留。

**架构：** 当前事实、需求状态、历史决策和过程证据分层存放；设计资料统一到 `docs/design`。以 `scripts/check-docs.sh` 把链接、版本、测试基线和目录规范纳入完整测试，阻止文档再次分叉。

**技术栈：** Markdown、POSIX shell、Swift Testing、SwiftPM、Git worktree、macOS codesign。

---

## 文件结构

- 创建 `docs/project-status.md`：当前事实唯一快照。
- 创建 `docs/architecture/decisions.md`：长期架构决策索引。
- 创建 `docs/development/maintenance-playbook.md`：维护与故障处置手册。
- 创建 `docs/development/2026-09-02-project-retrospective.md`：审计和清理证据。
- 创建 `scripts/check-docs.sh`：无网络文档一致性检查。
- 创建 `Tests/AIMeterAppTests/DocumentationCheckScriptTests.swift`：在受控临时仓库中运行真实检查脚本。
- 修改 `README.md`、`docs/README.md`、架构/开发/贡献文档：建立入口和维护规则。
- 移动 `docs/superpowers/{specs,plans}`：统一进入 `docs/design/`。
- 删除 `docs/next-phase-requirements.md` 与两个 `.superpowers/sdd` 临时报告：消除重复来源。

### 任务 1：锁定文档检查行为

- [ ] 新增失败测试，在临时目录运行真实检查器，分别验证完整文档集通过、断链失败、版本不一致失败。
- [ ] 运行 `./scripts/test.sh --filter DocumentationCheckScriptTests`，确认测试因检查脚本尚不存在而失败。
- [ ] 保留红灯证据，实施任务 2–4 后再让同一组测试转绿。
- [ ] 提交 `test: specify documentation checks` 检查点。

### 任务 2：统一设计资料与需求入口

- [ ] 将 `docs/superpowers/specs/*.md` 移到 `docs/design/specifications/`，将 `docs/superpowers/plans/*.md` 移到 `docs/design/implementation-plans/`。
- [ ] 把所有现行链接更新到新路径，并扫描确认仓库中除历史叙述外不存在有效的旧目录引用。
- [ ] 删除 `docs/next-phase-requirements.md`，把仍有效的 Widget、桌面环境验收、签名和发布限制保留在需求台账与状态页。
- [ ] 删除两个无引用 `.superpowers/sdd` 报告，确认其中独有的测试和验收事实已存在正式开发日志。
- [ ] 提交 `docs: consolidate project records` 检查点。

### 任务 3：补齐长期维护知识

- [ ] 编写 `docs/project-status.md`，列出版本、平台、三服务、Widget、签名、测试基线、持久化、未完成事项与权威入口。
- [ ] 编写 `docs/architecture/decisions.md`，记录原生菜单栏架构、统一快照、CLI 凭证边界、DeepSeek Keychain/WebKit 隔离、缓存降级、桌面层浮岛、模板菜单图标、Widget App Group 和兼容身份。
- [ ] 编写 `docs/development/maintenance-playbook.md`，给出按 Provider 的诊断顺序、变更影响矩阵、测试/构建/安装/回滚步骤和隐私红线。
- [ ] 更新 README、文档索引、架构目录、开发索引、贡献规则、发布与测试文档。
- [ ] 提交 `docs: add long-term maintenance knowledge` 检查点。

### 任务 4：自动验证文档

- [ ] 实现 `scripts/check-docs.sh`：使用系统自带 Ruby 检查相对链接、版本、测试基线、必备入口与禁用旧路径；接受可选仓库根路径以支持受控测试。
- [ ] 将检查脚本接入 `scripts/test.sh` 的 Swift 测试之后。
- [ ] 运行 `scripts/check-docs.sh`、`./scripts/test.sh --filter DocumentationCheckScriptTests` 和完整 `scripts/test.sh`，确认全部通过。
- [ ] 提交 `test: automate documentation consistency checks` 检查点。

### 任务 5：复盘、构建和清理

- [ ] 运行 `git diff --check`、完整测试、`scripts/build-app.sh`、`codesign --verify --deep --strict` 和 Markdown 检查。
- [ ] 在复盘日志记录一次基线 PTY 超时及独立复跑通过的事实，不把偶发失败隐瞒为一次全绿。
- [ ] 记录每个被删除对象的跟踪状态、替代来源、恢复方式和空间影响。
- [ ] 完成需求台账、CHANGELOG 与提交历史，标记 `REQ-20260902-013` 已完成。
- [ ] 合入 `main` 后删除已合并工作树/分支以及根目录可再生缓存与旧构建产物，最后确认仓库干净。
