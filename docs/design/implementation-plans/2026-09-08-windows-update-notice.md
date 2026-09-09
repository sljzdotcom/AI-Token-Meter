# Windows 新版本提示强调实现计划

> 面向 AI 代理的工作者：使用 subagent-driven-development 实现并审查此单项任务。

**目标：** 仅强化 Windows 关于页发现新版的提示，不发布。

**架构：** 继续使用 UpdateState 驱动 SettingsWindow；available 追加局部 CSS 状态类，状态离开后自动移除。测试真实组件和生产 CSS。

**技术栈：** React、TypeScript、Vitest、Chromium。

## 全局约束

- 仅 available 使用 `#991B1B`、`font-weight: 700`；其他状态不强调，字号/字体不变。
- 中英文一致；保留 aria-live 和按钮逻辑；macOS、版本与发布文件不改。
- 本次不发布，CHANGELOG 列入 Unreleased。

### Task 1: 更新提示样式和回归

文件：修改 `windows/src/settings/SettingsWindow.tsx`、`windows/src/styles.css`；新增 `windows/src/settings/SettingsUpdateNotice.test.tsx`；扩展 `windows/src/test/density-browser-entry.tsx` 和 `windows/scripts/test-density-browser.mjs` 中的浏览器计算样式验证。文档由主代理维护。

- [x] 先写真实组件测试：中英文 available，检查→available→其余六种状态；断言提示、强调 class 和更新按钮 enabled 状态。缺失 available 分支或无条件强调必须导致失败。运行 `npm test -- src/settings/SettingsUpdateNotice.test.tsx` 留存 RED。
- [x] 最小实现：段落 class 为 `update-status` 加 available 条件类 `update-status--available`；生产 CSS 添加：

```css
.update-status--available {
  color: #991b1b;
  font-weight: 700;
}
```

- [x] 运行聚焦测试 GREEN。扩展现有真实浏览器门禁，以实际 getComputedStyle 检查中英文 available 颜色为 `rgb(153, 27, 27)`、字重 `700`；对比其余状态，字体字号保持一致，非 available 不继承强调。现有报表和字号断言保留。
- [x] 先 `bash scripts/check-docs.sh`，然后 Windows `npm test`、`npm run build`、`npm run test:density`，`git diff --check`；不调用发布命令。
- [x] 独立规格/质量审查，保存实现测试证据，提交代码；主代理完成文档/最终审查后按既有授权合入 main，暂不发布。
