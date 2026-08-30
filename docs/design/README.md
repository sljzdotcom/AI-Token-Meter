# 历史设计记录

本目录保存 AI Meter 开发过程中的设计规格和实施计划，用于解释“当时为什么这样决定”，而不是替代当前用户文档。

## 目录

- [`specifications/`](specifications/)：需求边界、交互、架构和安全设计。
- [`implementation-plans/`](implementation-plans/)：按任务拆分的实施与验证计划。

## 阅读顺序

1. 先看根目录 [README](../../README.md) 了解当前产品；
2. 使用问题查 [用户指南](../user-guide/)；
3. 当前代码边界查 [架构文档](../architecture/overview.md)；
4. 只有在追溯决策时再阅读本目录。

## 历史性说明

早期文档中的功能可能已被后续实现替代。例如第一版 DeepSeek 使用“本地月预算”概念，当前实现已改为“可配置余额基准 + 官方 30 天用量聚合”。这类原始文字会保留，避免改写历史；当前行为以源码、用户指南与 [CHANGELOG](../../CHANGELOG.md) 为准。

