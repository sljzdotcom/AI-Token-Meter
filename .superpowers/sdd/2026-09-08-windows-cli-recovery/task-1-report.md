# Task 1 报告：Windows npm Codex 显式 Node 恢复

日期：2026-09-08
关联：REQ-20260908-001

## 结果

- 对 `codex.cmd` 只认可官方全局 npm 布局：包目录必须为 `node_modules/@openai/codex`，`package.json` 必须声明 `name = @openai/codex` 且 `bin.codex = bin/codex.js`，真实入口必须存在且为文件。
- 包元数据读取限制为 64 KiB；Node 只在已捕获的 process/user registry/system registry/conventional/desktop 有界候选目录中查找，不扩大 PATH，不解析 CMD 内容。
- 定位结果复用现有 `ExecutableCandidate`/`CommandInvocation` 约定：`launcher = node.exe`，`executable = bin/codex.js`；所有账户、登录和采集调用者因此自动获得同一结构化启动信息，未修改其业务逻辑。
- 原生 EXE、Claude CMD、显式自定义路径和 WSL 原有顺序/行为保持不变。

## 官方入口证据

OpenAI 官方 Codex 仓库的 `codex-cli/package.json` 声明包名 `@openai/codex`、`bin.codex = bin/codex.js`；官方打包脚本同样只将 `bin/codex.js` 纳入主包 files：

- https://github.com/openai/codex/blob/main/codex-cli/package.json
- https://github.com/openai/codex/blob/main/codex-cli/scripts/build_npm_package.py

本仓库未安装本地 `@openai/codex`，因此未读取或改动用户实际 npm 包、凭据或主目录。

## TDD 证据

### RED

```text
cargo test --manifest-path windows/src-tauri/Cargo.toml --test executable_locator official_codex_npm_wrapper -- --nocapture

running 1 test
official_codex_npm_wrapper_uses_verified_entry_and_separate_node ... FAILED
panic: verified npm candidate
test result: FAILED. 0 passed; 1 failed
```

失败原因与预期一致：旧定位器将 `codex.cmd` 交给 `cmd.exe`，无法产生分离目录的 Node + 真实 JS 入口候选。

### GREEN

```text
cargo test --manifest-path windows/src-tauri/Cargo.toml --test executable_locator --test cli_discovery --test process_runner --test cli_installation -- --nocapture

cli_discovery: 5 passed
cli_installation: 3 passed
executable_locator: 16 passed
process_runner: 9 passed
```

```text
cargo clippy --manifest-path windows/src-tauri/Cargo.toml --all-targets -- -D warnings
Finished `dev` profile; 0 warnings
```

`cargo fmt --manifest-path windows/src-tauri/Cargo.toml -- --check` 和 `git diff --check` 通过。

## Windows-only 真实进程回归

`process_runner::separated_official_npm_entry_runs_with_node_only_path` 仅在 Windows 编译运行。它会：

1. 在含空格的用户 npm 目录建立官方身份包和可执行 JS 入口；
2. 将真实 `node.exe` 放在分离的 `Program Files/nodejs` 目录；
3. 通过定位器和 `command_for_candidate` 生成 Node + JS 结构化命令；
4. 交给真实 `BoundedProcessRunner` 执行，并断言入口标记、参数及子进程 PATH 仅为 Node 目录。

这不是 health closure mock。当前 macOS 主机不能执行 Windows PE，因此该用例需 Windows 原生 CI/真机运行。尝试交叉目标 `cargo check --target x86_64-pc-windows-msvc --tests` 时，本机缺少 Windows MSVC C 头文件，`ring` 在 `assert.h` 处停止；这是工具链边界，不冒充原生验证。

## 安全与范围复核

- 未执行安装、登录、信任确认或凭据操作。
- 未硬编码用户名或个人路径；测试路径全部位于临时目录。
- 未扩大环境继承，未修改 `process.rs` 的环境白名单。
- 未修改账户、采集、界面或 macOS 逻辑。

## 首轮 Windows 原生 CI 跟进

Windows CI `34183253409` 已证明 npm 入口真实执行成功：标记、`--version` 参数、退出状态以及其他 10 项 `process_runner` 用例均通过。唯一失败为同一 PATH 目录的 Windows 路径表示差异：

```text
left:  "\\\\?\\C:\\Users\\runneradmin\\...\\Program Files\\nodejs"
right: "C:\\Users\\RUNNER~1\\...\\Program Files\\nodejs"
test result: FAILED. 10 passed; 1 failed
```

根因是 `tempfile`/Windows 文件系统同时暴露规范长路径和等价 8.3 短路径，不是 PATH 扩大。回归现将子进程报告的整个 PATH 与预期 Node 目录分别规范化后做全等比较；如果 PATH 含有分隔符和多个目录，它不是可规范化的单一目录，断言仍会失败。

本机 macOS 上 `cargo test --manifest-path windows/src-tauri/Cargo.toml --test process_runner` 修正后 9/9 通过；Windows-only 用例的修正后原生结果待下一轮 Windows CI，不在本地结论中冒充。
