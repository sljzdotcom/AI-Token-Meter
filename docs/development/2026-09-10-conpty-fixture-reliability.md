# Windows ConPTY 测试夹具稳定性

关联 `REQ-20260910-003`。本项修复原生 Windows ConPTY 综合测试的外部运行时依赖，不改变生产 ConPTY、Claude Code采集或任何行为期限。

## 失败证据与根因

PR #23最终证据候选`a0b0349`的Windows workflow `34424587418`中，安装脚本测试先用101.38秒完成。紧接着，已经改为原生夹具的Codex app-server握手仅用0.03秒通过；既有`conpty_attaches_a_process_sends_fixed_input_and_waits_for_a_pattern`仍通过外部Node.js启动测试脚本，在3.06秒内没有出现`ready`，按原3秒期限返回`TimedOut`。

失败发生在业务输入之前，且同一候选的前端、真实Edge、格式与严格Clippy均已通过。结合前一项Codex冷启动失败，根因是高负载Windows runner上的外部Node冷启动不属于ConPTY协议行为，却消耗了测试用于终端握手的全部期限。

## 实现

将原先Codex专用的Cargo fixture重命名为通用原生测试夹具，并增加固定`conpty-stdin`模式。该模式启动后立即输出并刷新`ready`，忽略ConPTY可能先送入的光标位置回复，收到测试固定写入的`hello`行后输出并刷新`received:hello`，随后以0退出。旧JavaScript夹具中已无引用的stdin分支删除；其余进程执行器模式保持。

测试继续验证真实ConPTY创建与缩放、进程附加、光标位置查询回复、3秒ready期限、固定`hello\r`输入、3秒业务响应期限和3秒正常退出。生产实现和全部期限未修改。

## 验证与边界

- 通用原生夹具在本机终端实际完成`ready → hello → received:hello`往返；
- Codex额度与账户定向测试继续通过；
- 宿主完整Rust测试、Cargo格式与全部目标严格Clippy通过；
- Windows专属ConPTY回归、完整runtime、NSIS、GUI subsystem和安装器上传等待PR及合并后`main` workflow复验。

夹具只处理固定合成文本和虚构JSON响应，不读取真实账号、凭证或额度。版本保持0.6.3，本项不创建标签、Release或更新源。
