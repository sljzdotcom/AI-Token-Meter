# Windows发布前端测试文件调度可靠性

关联`REQ-20260910-005`，阻塞`REQ-20260910-004`的稳定版公开发布。

## 失败证据

v0.7.0首次签名发布workflow [`34455443116`](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34455443116)中，Windows任务的`npm test`有122/124项通过，但`ProviderRecovery.test.tsx`与`App.test.tsx`各自的首项分别耗时9.679秒与10.848秒，超过Vitest默认5秒上限。两个首项并行受阻后，同文件的其余44项均在毫秒级完成；完整前端步骤总耗时23.43秒。macOS标签验证通过，Windows后续浏览器、Rust、NSIS、签名和公开步骤全部按门禁跳过，Release保持草稿。

同一代码的本机、PR Windows CI与main Windows CI此前均通过124项；失败集中在多个jsdom文件并行冷启动时的runner资源竞争，并非产品行为等待或某一异步结果缺少结束条件。

## 修复边界

Windows前端测试文件改为串行调度，避免多个jsdom/React文件同时争用受限runner资源。每个测试仍保留Vitest默认5秒上限，测试内容、断言与产品代码均不修改；真实浏览器、Rust原生运行时和签名门禁继续独立执行。

## 验证与发布恢复

Swift合同先要求`windows/vite.config.ts`包含`fileParallelism: false`且不得增加`testTimeout`；修改前合同准确失败。加入串行文件调度后，合同转绿，Windows前端124项全部通过，单项5秒上限保持不变。

恢复候选本机门禁通过：Windows前端124项用时8.58秒，production build、25项浏览器生命周期、8个Antigravity状态、16组布局、608个文字角色、241项宿主Rust、格式与严格Clippy全部通过；macOS 463项普通测试、3项刷新调度和18项PTY runner也全部通过。6份合同、245份Markdown、公开安全检查，以及0.7.1/build20的arm64无Widget Release App资源、Sparkle嵌套组件与严格签名均通过。

提交前复核确认生产代码与测试内容没有改变，只调整Windows前端测试文件调度并同步恢复版本元数据；版本单调性、默认5秒期限、发布范围、更新兼容和凭据边界均无遗留发现，Critical/Important/Minor为`0/0/0`。

已经推送的v0.7.0标签不移动或重写；公开恢复使用0.7.1、macOS build20。双平台PR/main CI和新的标签发布workflow完成后再关闭本项。
