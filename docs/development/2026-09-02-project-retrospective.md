# 2026-09-02 全项目复盘

- **需求：** `REQ-20260902-013`
- **状态：** 已完成
- **基线：** `main` 的 `99f3788`

## 审计范围

- 251 个 Git 提交、全部本地分支/worktree、tag、stash 与 remote；
- 298 个跟踪文件：168 个 Swift、101 个 Markdown，以及 Shell、plist、entitlements 和图片资源；
- `AIMeterCore`、`AIMeterApp`、`AIMeterWidgetExtension` 三个 target 与对应测试；
- README、用户指南、架构、安全、设计、开发、发布、需求和变更记录；
- 根目录构建缓存、发行包、视觉原型与历史工作树。

## 基线结论

- `main` 没有未提交代码，所有历史功能分支均已是 `main` 祖先；
- 无 remote、tag、stash 或第三方 Swift Package 依赖；
- 应用元数据为 `0.1.0` build `1`、macOS 14+、LSUIElement 菜单栏 App；
- 首次完整测试运行 305 项中 `PTYCommandRunnerTests.sendsInputAndPreservesExitStatus` 在 2 秒门槛偶发超时；立即单独复跑完整 PTY suite 为 9/9 通过。该事实保留，最终验证必须再次完整复跑；
- Markdown 相对链接初检为 0 个断链，但文档有两个状态入口和两个设计资料根目录，属于信息架构问题。

## 发现与处理

### 1. 需求状态分叉

`docs/next-phase-requirements.md` 是建立正式台账前的过渡文件，其中 R1/R4/R5/R6 已完成，R2 和 R3 已分别进入正式的受环境限制/延期事项。继续保留会形成第二需求队列，因此删除；当前事实迁入[项目状态](../project-status.md)，状态只由[需求台账](../requirements-backlog.md)维护。

### 2. 设计资料分叉

2026-08-31 曾开始从内部名 `docs/superpowers` 迁移到 `docs/design`，但后续新增资料又写回旧目录。已把 13 份规格和 15 份计划移动到统一设计目录、更新有效链接，并为全部 28 组规格/计划建立[完整索引](../design/README.md)。Git 将其识别为高相似度 rename，内容历史保留。

### 3. 临时报告被正式日志覆盖

两个 `.superpowers/sdd` 文件没有任何入口引用：

- 反向半圆 task report 的提交、测试和疑虑已在 `2026-08-31-visual-system-edge-docking.md` 与提交历史记录；
- Symbol 最终修复报告的 `4a0d6b3`、红绿灯、验收和限制已在 `2026-09-01-settings-font-isolation-and-content-size-step.md` 及对应规格记录。

因此删除临时副本；恢复方式是从复盘前提交 `99f3788` 读取，不丢失 Git 历史。

### 4. 缺少长期接手入口

新增：

- [当前项目状态](../project-status.md)：当前能力、数据、版本、验证与未完成事项；
- [架构决策](../architecture/decisions.md)：长期约束、动机、代价和重新评估条件；
- [维护手册](maintenance-playbook.md)：变更影响矩阵、三服务诊断、构建、安装和回滚；
- 文档一致性检查：未来自动阻止断链、版本/测试基线分叉和旧目录复发。

### 5. 本地可再生残留

盘点时根检出共约 644 MB，其中 `.build/` 约 439 MB、工作树约 179 MB、`dist/` 约 9.4 MB、视觉原型状态约 2 MB。它们均被 `.gitignore` 排除。最终合并后将删除：

- 可由 SwiftPM/构建脚本重建的根 `.build/`；
- 同时包含旧 `AI Meter.app` 和新 `AI Token Meter.app` 的根 `dist/`；
- 已完全合入 `main` 且干净的 `feat/initial-app` 工作树/分支；
- 已被正式规格、日志和资源吸收的 `.superpowers/brainstorm/` 临时服务器状态与原型输出；
- 本轮完成后的复盘工作树/分支。

当前 `/Applications/AI Token Meter.app` 不属于仓库残留，不删除。清理后的恢复方式分别是重新测试/构建，或从对应 Git 提交读取正式记录。

## 保留但明确标记的事实

- Widget 源码已实现，真实 Gallery/桌面验收仍因证书决定延期；
- Mission Control、第二普通 Space、真实指针、多显示器仍受环境限制；
- 项目没有远程备份、tag、CI、许可证、Developer ID 签名和公证；这些不能在复盘中凭空补造；
- 旧兼容 Bundle ID、可执行文件名、Keychain 服务和数据目录有意保留。

## 最终验证

### 分支验证

- 文档检查测试先在缺少 `scripts/check-docs.sh` 时按预期 3/3 失败；实现后定向复跑 3/3 通过；
- `scripts/check-docs.sh` 检查 104 份 Markdown：必备入口、相对链接、README/Info.plist 版本、README/测试文档基线和禁用旧路径全部通过；
- `scripts/test.sh`：308 项测试、61 个测试组通过，2 个需要真实 Keychain/已安装 CLI 的门控测试按设计跳过；此前偶发失败的 PTY 用例也在完整并发运行中通过；
- `AI_METER_INCLUDE_WIDGET=0 scripts/build-app.sh`：Release 构建成功；产物为 arm64、版本 `0.1.0`，`codesign --verify --deep --strict --verbose=2` 通过；
- 敏感模式扫描仅命中四个隐私回归测试中的合成 `sk-...` 字符串，没有命中真实 Telegram Token 或 Bearer 凭证；
- 最大跟踪文件是主应用与 Widget 各自的 272 KB 深海背景资源，没有跟踪 `.app`、构建缓存或异常大二进制；
- `git fsck --full` 未报告对象损坏；存在若干不可达历史对象，保留给 Git 自身回收，不把正常对象库维护冒充仓库内容清理。

### 合并与清理

- 复盘分支以 `d7748c1` 合入 `main`；合并树重新运行 `scripts/test.sh`，308 项测试、61 个测试组和 104 份 Markdown 检查全部通过；
- 合并树重新构建无 Widget Release，严格签名验证通过；随后按“产物可重建”原则删除仓库内临时 `dist/`，不影响 `/Applications/AI Token Meter.app`；
- 删除根 `.build/`（约 439 MB）、根 `dist/`（约 9.4 MB）、`.superpowers/brainstorm/`（约 2 MB）、已完全合并的 `feat/initial-app` 工作树（约 179 MB）和本轮复盘工作树；
- 删除 `feat/initial-app` 与 `codex/project-retrospective` 本地分支并执行 worktree prune；最终只有 `main` 和主工作目录；
- 仓库磁盘占用由约 644 MB 降至约 16 MB，约回收 628 MB。数值受文件系统计量影响，恢复方式是重新测试/构建或从 Git 历史重新建立工作树；
- 根 `.build`、`dist`、`.superpowers`、`.worktrees` 均已不存在，`git status --short --branch` 为干净 `main`（本收尾记录提交前除外）。
