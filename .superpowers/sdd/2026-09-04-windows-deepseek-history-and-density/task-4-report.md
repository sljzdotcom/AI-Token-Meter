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
