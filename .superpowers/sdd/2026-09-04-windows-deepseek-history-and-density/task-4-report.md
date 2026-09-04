# 任务 4：Windows 详情与 Settings 紧凑密度

关联需求：`REQ-20260904-006`。

## RED

- 在 `windows/src/App.test.tsx` 先增加两个组件级测试：Provider 详情必须声明 `provider-detail--compact-density`，Settings 必须同时声明紧凑密度与系统字体语义，且 Display font 控件保持可访问名称。
- 未修改生产代码时，两个测试分别因 Provider 缺少 `provider-detail--compact-density`、Settings 缺少 `settings-window--compact-density` 失败；其余 22 项前端测试仍通过。
- 这些用例会捕获详情或 Settings 在未来意外失去 Windows 专属紧凑边界的回归，而非检查样式文件文本。

## GREEN

- Provider 详情添加显式紧凑密度类；`styles.css` 以集中 CSS token 收紧正文至 14px、身份标题至 20px、主数值至 24px、区块标题至 13px、卡片关键数值至 18px，并小幅收紧卡片/区块 padding 与 gap；440px 宽和 760px 高上限保持不变，展示字体仍由既有 `--display-font` 控制。
- Settings 添加显式紧凑密度类，仍以 `settings-window--system-font` 锁定 Segoe UI；基准 14px、标题 20px、控件 13px 与最小高度 32px 由集中 token 控制。
- Settings 声明 `color-scheme: light`，紧凑范围内 `select` 与 `option` 均明确使用 `#151821` 深色文字与 `#fff` 背景；全局 `:focus-visible` 规则未改动。

## 生产突变覆盖

- 删除任一组件的紧凑密度类会使对应组件测试失败；删除 Settings 系统字体类亦会失败。
- 详情 CSS token 已由本机 Chromium 渲染检查为正文 14px、身份标题 20px、主数值 24px、区块标题 13px；其余卡片 token 固定为 18px，并由相同紧凑类继承。
- 自动化 DOM 断言覆盖了可访问的 Display font 控件和四个 Settings tab；没有以全文件 grep 取代组件行为测试。

## 视觉检查

- 通过现有 Vite 浏览器入口逐项打开 Claude Code、OpenAI Codex 与 DeepSeek 详情：三个详情均无溢出或截断；DeepSeek 官方历史区块仍完整显示。
- Chromium 实测 Claude 详情为 440px 宽、最大 672px（当前视口约束）且上述字号生效；卡片颜色、层级和 Antonio 展示字体保持不变。
- 浏览器入口刻意只承载桌面浮动条/详情预览，不能模拟 Tauri 独立 Settings 窗口；四个 Settings tab 以真实 `SettingsWindow` 组件渲染和可访问性测试覆盖。Windows 11 真机仍需在完整需求验收阶段确认原生下拉框弹出菜单的系统渲染。

## 自审

- 仅改动 `windows/` 前端和任务需求台账；未改 macOS 源码。
- 未改变 Provider 数据、DeepSeek 同步状态、自动隐藏或窗口生命周期。
- 保留原有焦点可见性、深浅层级、显示字体机制和 Provider 详情尺寸上限。

## 文件与验证

- `windows/src/App.test.tsx`
- `windows/src/details/ProviderDetail.tsx`
- `windows/src/settings/SettingsWindow.tsx`
- `windows/src/styles.css`
- `docs/requirements-backlog.md`

已验证：`npm test`（2 个测试文件、24 项通过）、`npm run build`（TypeScript 检查和 Vite production build 通过）。

Git 提交：`fix(windows): compact detail and Settings density (REQ-20260904-006)`。

## 审查修复第 1 轮：浏览器级 CSS 门禁

### 根因与 RED

- 原有 Vitest 只验证组件的紧凑密度 class、节点和可访问名称；其 JSDOM 入口不加载 production `styles.css`，因此错误字号、错误控件高度、浅色 scheme 或 select/option 配色仍会通过。
- 增加 `density-browser.html`、`src/test/density-browser-entry.tsx` 和 `scripts/test-density-browser.mjs` 后，门禁通过 Vite 提供实际前端模块、Chrome headless 的 `getComputedStyle()` 计算样式和 DOM 报告验证可见结果，不读取 CSS 源文本或 grep。
- 突变证明：临时把 `--provider-detail-body-size` 从 `14px` 改为 `15px`，`npm run test:density` 按预期以 `detailBody: expected 14px, received 15px` 失败；恢复 `14px` 后同一命令转绿。

### 覆盖

- Provider：正文、身份标题、主数值、区块标题、卡片关键数字分别为 14/20/24/13/18px；详情展示字体仍为 Antonio。
- Settings：基准、主标题、控件字号/最小高度分别为 14/20/13/32px；字体仍为 Segoe UI，未泄漏到 meter 或 Provider detail。
- 原生控件：Settings `color-scheme` 为 `light`，Display font `select` 和首个 `option` 的前景/背景均为 `rgb(21, 24, 33)` / `rgb(255, 255, 255)`。

### 文件与验证

- `windows/density-browser.html`
- `windows/src/test/density-browser-entry.tsx`
- `windows/scripts/test-density-browser.mjs`
- `windows/package.json`
- `docs/requirements-backlog.md`

已验证：`npm run test:density`（Chrome headless CSS 计算样式通过）、`npm test`（2 个测试文件、24 项通过）、`npm run build`（TypeScript 检查和 Vite production build 通过）。

Git 提交：`test(windows): verify compact density in browser (REQ-20260904-006)`。

## 审查修复第 2 轮：隔离、CI 与进程韧性

### RED 与生产突变覆盖

- 原浏览器 fixture 的根默认字体也是 Segoe UI，因此删除 `.settings-window--system-font` 后仍会错误地通过。fixture 现将 `#root` 显式设为 Antonio；meter 与 Provider detail 同时保留 Antonio 断言。
- 临时将 Settings 系统字体 CSS selector 改为不存在的 class 后，Chrome headless 报 `Settings did not retain its system font`，证明专属覆盖规则被真正的计算样式门禁保护；恢复 selector 后同一命令转绿。
- 断言由宽松的“包含 Segoe fallback”收紧为实际首选字体 `"Segoe UI Variable"`，避免 Antonio 的 fallback 栈造成假阳性。

### CI 与脚本

- `.github/workflows/windows-ci.yml` 和 Release 的 Windows job 均在 `npm test` 后运行 `npm run test:density`，使 PR 和发布构建都阻止 CSS 级联回归。
- 浏览器探测按 `BROWSER_BIN`、`CHROME_BIN`、macOS Chrome/Edge、Windows Chrome/Edge 的 ProgramFiles、ProgramFiles(x86)、LOCALAPPDATA 和 Linux 常见路径依次检查；找不到时列出全部已检查路径。
- Vite 改为 `--port 0` 并解析实际监听地址，消除探测端口释放后的竞争窗口；清理阶段等待 Vite 退出，Windows 使用 `taskkill /T /F` 终止子进程树，Unix 在超时后升级为 SIGKILL。

### 验证

- `npm run test:density`：Chrome headless 通过全部计算样式断言；Settings 字体突变时按预期失败，恢复后再次通过。
- `npm test`：2 个测试文件、24 项通过。
- `npm run build`：TypeScript 检查与 Vite production build 通过。
- `ruby -e 'require "yaml"; ...' .github/workflows/windows-ci.yml .github/workflows/release.yml`：两份工作流 YAML 解析通过。
- `ruby scripts/check-cross-platform-contracts.rb .`：4 份 fixture 合同检查通过。

Git 提交：`test(windows): harden density browser gate (REQ-20260904-006)`。
