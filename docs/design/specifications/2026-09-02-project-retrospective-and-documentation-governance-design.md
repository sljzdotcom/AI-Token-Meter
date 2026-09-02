# AI Token Meter 全项目复盘与文档治理规格

**日期：** 2026-09-02  
**需求：** `REQ-20260902-013`  
**状态：** 已确认，进入实施  

## 目标

把当前仓库整理为即使数月后重新打开，也能仅凭仓库回答以下问题的状态：产品现在能做什么、数据从哪里来、为什么这样设计、怎样开发和发布、哪些事项仍未完成、每次改动如何被验证。

## 权威信息层级

1. `README.md`：产品入口与最短上手路径；
2. `docs/project-status.md`：当前版本、能力、验证基线和未完成事项的事实快照；
3. `docs/user-guide/`：当前用户行为；
4. `docs/architecture/`、`docs/security-and-privacy.md`：当前技术事实与边界；
5. `docs/requirements-backlog.md`：需求状态唯一来源；
6. `docs/development/`：维护、测试、发布与实际验收证据；
7. `docs/design/`：历史规格和计划，只解释当时的决定。

历史设计不得覆盖当前用户指南。已完成事项不得从需求台账删除。

## 文档整理

- 把仍位于 `docs/superpowers/specs` 与 `docs/superpowers/plans` 的资料迁入 `docs/design/specifications` 与 `docs/design/implementation-plans`；
- 更新仓库中的有效相对链接和现行维护文档；历史计划中的命令路径也同步到新位置，避免复制命令时重新制造旧目录；
- 删除已被 `docs/requirements-backlog.md` 完整取代的 `docs/next-phase-requirements.md`；
- 删除两个已被正式开发日志完整吸收、且没有任何入口引用的 `.superpowers/sdd` 临时报告；
- 保留所有有独立决策价值的设计、开发日志和验收记录，不以“精简”为由抹掉历史。

## 新增长期文档

- `docs/project-status.md`：当前产品状态、支持矩阵、版本/签名事实、已知限制与恢复入口；
- `docs/architecture/decisions.md`：记录影响长期维护的架构决策、原因、代价与重新评估条件；
- `docs/development/maintenance-playbook.md`：日常诊断、变更影响面、发布、安装、回滚及敏感信息处理；
- `docs/development/2026-09-02-project-retrospective.md`：本轮盘点范围、发现、删除证据、测试和 Git 节点。

## 自动化防回退

新增文档治理检查，至少验证：

- Markdown 本地相对链接指向存在的文件或目录；
- README 版本与主应用 `Info.plist` 一致；
- README 测试徽章与测试指南基线一致；
- 旧 `docs/superpowers` 和 `docs/next-phase-requirements.md` 不会重新出现；
- 文档索引包含状态、决策、维护手册和复盘日志入口。

检查必须能够从仓库根目录单独运行，并纳入 `scripts/test.sh`，从而让未来每次完整测试都验证文档。

## 本地生成物清理

以下对象只有在只读检查证明未被 Git 跟踪、可由脚本再生或已经合并后才删除：

- 根检出的 `.build/` SwiftPM 缓存；
- `dist/` 中旧名和当前名的本机构建产物；
- `.superpowers/brainstorm/` 的临时原型服务状态与输出；
- 已完全合入 `main` 的 `feat/initial-app` 工作树和分支；
- 本轮完成后使用的临时复盘工作树和分支。

安装在 `/Applications` 的当前应用不属于仓库垃圾，不在清理范围。

## 不擅自决定的事项

- 不替用户选择开源许可证；
- 不伪造远程仓库、Release、Git tag、Developer ID、公证或 CI；
- 不把 Widget 证书验收、Mission Control、多普通 Space 和多显示器验收写成已完成；
- 不修改兼容身份 `com.millerpan.AIMeter`、`AIMeterApp`、Keychain 服务名和 `Application Support/AI Meter`。

## 验收

1. 文档只有一个需求队列和一个设计资料根目录；
2. 新维护者能从文档索引找到当前状态、架构决策、数据口径、维护流程和历史证据；
3. 自动文档检查、完整 Swift 测试、Release 构建和严格签名验证通过；
4. 删除清单逐项记录“为何可删、如何恢复”；
5. `main` 合并后工作区干净，需求状态与 Git 证据一致。
