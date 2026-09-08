# 关于页精简与 Telegram 链接

关联需求：REQ-20260908-009。日期：2026-09-08。派工基线：`866ce41`。用户已授权按推荐方案直接实施，暂不发布。

## 范围与设计

Windows 设置→关于移除中英文作者整行（Author · Miller / 作者 Miller）及可见 Author links / 作者链接标题；保留 section 的本地化无障碍名称。Twitter 和 GitHub 原目标、点击路由及失败提示不变，新增 Telegram @sljzdotcom，固定目标 `https://t.me/sljzdotcom`。三入口复用现有图标/文本链接样式、系统字体字号和焦点边框，等高对齐，窄宽度自然换行、不截断。纸飞机为本地矢量图标，装饰性且不重复朗读。

macOS 仅在原作者行下新增相同 Telegram 名称和目标，与现有 15pt 社交图标按钮风格一致，保留作者信息和横向不足时的纵向回退。复用 NSWorkspace 点击打开边界和可恢复错误提示。

推荐扩展现有品牌链接组件及 Rust 固定目标枚举；不采用可配置任意 URL（不符合固定链接范围），也不新增依赖或重做 About 布局。仅用户激活时打开默认浏览器，不访问账号、发送 Telegram 消息或加入频道。

## 验收

- 渲染时无打开副作用；Twitter → `https://twitter.com/MillerPanYue`、GitHub → `https://github.com/sljzdotcom/AI-Token-Meter`、Telegram → `https://t.me/sljzdotcom`，按键/点击激活均走既有边界。
- Windows 中英文实际渲染无上述作者文字与社交可见标题；无障碍组名称、链接名称和 Tab/Enter 激活保留；窄宽度换行且图文完整、入口等高。
- Rust 只接受固定 target；拒绝任意 URL、路径和命令字符串。macOS 实际 NSButton 渲染/点击/错误恢复覆盖 Telegram，原作者行不变。
- Settings 系统字体字号、Windows available 深红粗体提示及其他设置保持不变。
- 运行针对性红绿测试、平台测试和构建、真实宿主浏览器布局检查、文档检查及独立审查。宿主浏览器不冒充 Windows WebView2/真机 DPI 验收。

## 交付约束

不改版本、更新源、签名、Release，不发布；REQ-007 继续暂不发布，历史受限事项不重开。只在隔离工作区提交，由协调入口核验整合，不直接写主工作区。维护唯一需求台账中 REQ-009 及相关文档/索引。
