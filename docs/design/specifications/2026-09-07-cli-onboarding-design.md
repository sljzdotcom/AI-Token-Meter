# CLI 安装与登录引导、Windows 页签图标

关联 REQ-20260907-006/007；2026-09-07。用户授权常规决策按推荐执行，选择下述官方安装器方案，已登录保留账号与“重新登录”，不增加退出登录动作。

## 方案

1. 推荐并采用：官方独立安装器，用户点击后在可见终端执行，应用不收集安装器原始输出或认证信息；重新定位后由现有账户读取判断下一步。
2. 只开官方教程最简单，但不满足一键协助。
3. 内嵌二进制/自建包管理会扩大发行和信任维护范围，本轮不采用。

## 状态与交互

- Settings 打开/重新激活继续自动探测 CLI；不是只检查固定路径。现有 nvm、桌面内置 Codex 与 Windows Native/WSL 选择保持一致。
- 未安装：橙色 Install CLI / 安装 CLI，辅助官方安装说明；按钮旁告知将下载并运行官方安装器。
- 未登录：橙色 Sign in / 登录。
- 已登录：当前账号继续显示，主按钮 Sign in again / 重新登录，普通中性样式。不隐式 logout，不删除任何 CLI 凭据。
- checking / 操作中：不可重复触发；保留明确文字和进度，不能只用颜色传达状态。
- unavailable：不推断没装或没登录，保留 Check Status 重试及合适恢复提示。
- 安装点击必须重新检测；已存在则转为账户检查，不覆盖/升级现有 CLI。Windows 显式 WSL/自定义不可用路径不能偷偷改装 Native，提供对应官方教程及调整路径指引。
- 官方安装器在当前用户身份下运行，不用 sudo、不改变系统安全策略、不下载 Node/Homebrew/WSL；既有候选路径若不可执行不伪装为成功。
- 安装启动成功不等于安装完成；状态自动轮询有界，检测到 CLI 后更新账号卡，超时/关闭/失败可检查状态或重试，不永久锁按钮。安装期间不能并行登录同一服务。
- 不为此测试在维护者机器实际安装、重装或登录 CLI；脚本用本地受控 fixture 验证分支/退出与清理，真实服务验证标记边界。

## 官方来源（2026-09-07 核对）

- [OpenAI Codex CLI](https://learn.chatgpt.com/docs/codex/cli)：macOS `https://chatgpt.com/codex/install.sh`（sh），Windows `https://chatgpt.com/codex/install.ps1`（PowerShell）。
- [Claude Code 安装](https://code.claude.com/docs/en/setup)：macOS `https://claude.ai/install.sh`（bash），Windows `https://claude.ai/install.ps1`（PowerShell）。
- 下载地址在程序中按服务固定，不接受网页/前端传入任意命令或 URL。完整下载成功才执行；错误显示固定安全文案，远端文本不进入日志/账户 UI。
- 复用官方当前用户安装位置；必要时为官方新增路径补发现测试，不硬编码维护者用户名。

## Windows 页签

Appearance / 外观：调色板；Monitoring / 监测：活动波形；Services / 服务：连接；About / 关于：信息圆圈。统一 16px 线性 SVG、currentColor、随选中/焦点状态继承颜色，保留名称。图标 aria-hidden，不重复朗读，不加载网络图标或新字体。macOS 不变。

## 验收

覆盖缺失→启动安装→重新发现→未登录→登录→已连接、重复点击、启动失败、检查失败/超时、WSL/显式路径缺失、检查返回顺序。脚本无任意参数注入，下载失败不执行残缺脚本。Windows 英/中页签图标和名称、系统字体与键盘焦点保持。完整双平台测试、文档、安全检查及独立审查通过后合并；未经本轮发布不宣称 0.4.0 已含这些功能。
