# macOS 悬浮条烟熏 Liquid Glass 规格

关联需求：REQ-20260913-005、REQ-20260913-006。日期：2026-09-13。状态：用户已选择方案B并授权直接开发、合并和发布。

## 目标行为

- macOS Settings 的 Floating Strip 页增加“悬浮条外观”，可选现有 Deep Sea 与 Liquid Glass；默认仍为 Deep Sea。
- 选择后已展开或收起的悬浮条立即刷新，重启后保留选择。旧版偏好缺少该字段时迁移为 Deep Sea。
- Liquid Glass 使用用户确认的B款烟熏深色基调：深蓝黑半透明底、柔和边缘高光和足够的前景对比，不展示深海背景图。
- macOS 26及以上使用系统 Liquid Glass，材质裁切到现有自定义轮廓；macOS 14–15使用视觉接近的系统材质与烟熏叠色。开启“减少透明度”时改用不透明烟熏底板。
- Comfortable/Compact尺寸、圆环尺寸与品牌色、左右镜像肩弧、拖动、自动收起、点击详情和折叠把手均保持现有合同。
- Windows 不增加外观字段、设置控件、玻璃CSS或渲染分支。

## 设计与技术边界

偏好模型增加`FloatingStripAppearance`，原始值为`deepSea`与`liquidGlass`，schema升级后仍以容错解码处理旧数据。`AppModel.setStripPreferences`沿用现有外观通知，因此设置变化直接让所有屏幕的悬浮条重绘，不建立第二条窗口更新路径。

展开态和折叠态共用同一个材质选择规则：Deep Sea严格沿用现有图片、渐变和遮罩；Liquid Glass在macOS 26使用单个`glassEffect`覆盖完整自定义Shape，在旧系统使用`ultraThinMaterial`、深色烟熏叠层与细边缘高光。高对比度或“不依赖颜色区分”会增强描边；减少透明度会关闭透明材质并使用实色。玻璃不使用交互形变，防止破坏贴边肩弧与命中区域。

发布采用0.10.0、macOS build28。两端继续共享版本、标签和GitHub Release，Windows只更新版本元数据并生成既有签名安装包，产品行为不变。

## 验收

- macOS三种语言均能在Floating Strip设置页看到Deep Sea与Liquid Glass，并能即时切换。
- 新安装及旧偏好默认Deep Sea；保存Liquid Glass后重启仍为Liquid Glass；未知值安全回退Deep Sea。
- macOS 26走原生Liquid Glass，macOS 14–15和减少透明度走规定回退；两者都保留自定义轮廓及前景可读性。
- 两种密度、左右贴边、1–4个产品、展开/折叠与自动收起回归通过；Windows功能源码无玻璃外观改动。
- 完整Swift、Windows现有回归、跨平台合同、生产构建、文档与发布安全门禁通过，独立审查无未关闭发现。
- 0.10.0/build28的双平台签名资产、更新源、签名与篡改拒绝、公网下载均完成验证。
