# Gemini 四产品展示与迁移阶段

关联需求：REQ-20260908-012。日期：2026-09-08。本阶段交付四按钮及配置兼容；官方CLI额度采集仍在实施检查点，不能据此认定012完成。

## 实现与提交

- macOS `f3a3428`：Gemini四角星与青绿#3ED6B2，第四详情及Settings入口，1至4项窗口尺寸，schemaVersion2迁移，未知provider缓存逐条恢复。Widget继续默认显示原三项，类型与资源兼容Gemini。
- Windows `7b7ec5e`：Rust/React四产品合同、详情、设置排序/显隐与尺寸同步；旧完整设置缺失strip字段也按旧用户迁移。共享合同新增缺额度fixture。
- 两端全新用户四项显示；旧记录缺Gemini时末尾追加并隐藏，已有Gemini记录保留用户选择；最后可见项不能隐藏。
- 当前Gemini初始、Demo和刷新都明确额度不可用，指标为空，不显示模拟额度或宣称已检查登录。文档/重试入口不执行安装或登录。
- Logo来自LobeHub lobe-icons的Gemini描摹（MIT），两端资源附许可，不称为Google官方原始资产。

## 回归发现与修复

完整宿主Rust回归发现metadata解码仍要求三产品；独立审查发现snapshot schema名称漏加Gemini。`2124b78`移除固定三项容器并修复schema，新增实际schema删除Gemini和fixture未知名称的反例。定向8项及独立复审通过。

完整Swift回归发现刷新测试用allCases注册四个模拟采集器，却仍期望三并发和三结果。`8c632db`改为字面量四并发与四名称顺序，保留反序注册及真实重叠探针；8项定向测试通过。

## 集成验证

以下结果来自修复后的本地运行，不代表真实Windows或Google账号验收。

| 验证 | 结果 | 本机日志 |
| --- | --- | --- |
| macOS完整测试与仓库门禁 | 436项主测试、13项PTY通过；5 fixtures、合同行为反例、发布资产/源回归、文档和公开安全检查通过 | `/private/tmp/req012-swift-full-fixed.log` |
| Windows前端完整测试 | 105项、13文件通过 | `/private/tmp/req012-frontend-full.log` |
| 前端生产构建 | TypeScript/Vite通过 | `/private/tmp/req012-frontend-build.log` |
| 宿主Rust完整测试 | 225项、0失败、0忽略 | `/private/tmp/req012-rust-full-fixed.log` |
| 宿主Rust严格Clippy | all-targets、warnings为错误，通过 | `/private/tmp/req012-clippy.log` |
| macOS Release编译 | AIMeterApp通过，18.02秒 | `/private/tmp/req012-release-build.log` |
| 原生Swift离屏渲染 | 1至4项×左右×两密度，Logo方向与第四项几何点击区域通过 | `/tmp/gemini-task2-regression.log` |
| 实际浏览器门禁 | 16种数量/侧边/密度组合，裁切/点击/拖动隔离及632文字角色通过 | `/private/tmp/req012-windows-browser-loopback.log` |

Task2独立规格/质量审查无阻断；Task3两项P2经`2124b78`定向复审关闭。整个Gemini分支最终审查在实际采集接入后执行。

## 边界与下一阶段

真实macOS AX/鼠标事件、人工重排及物理多屏未完成；离屏几何和像素测试不替代这些操作。跨编译Windows因缺少SDK的assert.h在第三方ring依赖处失败，未证明Windows专用分支编译通过；宿主Rust与Chrome结果不等于原生WebView2/DPI验收。用户CLI、真实Google凭据、浏览器登录均未操作；未推送、未发布、未更新版本/feed、未安装覆盖应用。

下一步为[CLI能力报告](2026-09-08-gemini-cli-capability.md)中的Task4a完整启动链验证及通过后的采集实现。REQ-012保持进行中，REQ-015仍等待其功能完成。
