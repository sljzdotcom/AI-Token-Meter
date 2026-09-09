# Windows 新版本提示强调

关联需求：REQ-20260908-007。用户已明确颜色、字重与不发布的范围；沿用授权直接实施。

## 设计

Windows Settings → About 的更新状态段落，仅在 `phase === "available"` 时使用深红色 `#991B1B` 和 `font-weight: 700`。保留原字号、系统字体、布局、版本号文案和 `aria-live="polite"`。中文与英文一致；状态离开 available 时恢复原外观。

比较仅变色、变色加粗和额外警示卡片：采用用户指定的变色加粗，强化可读性且不增加布局。无需新增图标、弹窗或通知。

## 边界与验收

- idle、checking、upToDate、downloading、installing、failed 保持现有外观。
- 不改检查更新/安装逻辑和按钮行为，不改 macOS。
- 真实 Settings 渲染测试验证中文、英文 available 提示及状态退出后的恢复；真实浏览器计算样式验证颜色、字重和原字号/字体。
- 不改版本、更新源、签名、安装包或 Release；列入 Unreleased 等后续统一发布。

规格自检：范围仅一个状态的样式，无待定取值或流程变更。
