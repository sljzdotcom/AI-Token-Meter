# Antigravity安全恢复独立审查

需求：`REQ-20261009-001`。审查对象为本地候选分支`codex/antigravity-safe-auto-refresh`。审查均为只读，不运行真实`agy`、用户账号、浏览器、Terminal或更新源。

## 对候选 `6b014a7` 的补充审查

协调方指出原审查没有解释全量生产采集路径。本次独立复核沿AppModel启动/唤醒/定时刷新追到`GeminiCollector.collect()`，发现候选解锁暂停后会在`agy -p /usage`之外运行`agy -p /model`和非headless `agy models`；补充命令失败仍可能被当作可选结果，不能证明没有图形副作用。还发现普通失败的暂停写盘结果被忽略，因此一次性清暂停后若后续重新暂停写入失败，重启时可能再次尝试。Review也确认`/dev/null` stdin、无PTY和独立进程组并不限制真实CLI使用LaunchServices打开浏览器；版本门槛覆盖1.1.28起的整个1.x范围，仓库未验证此范围的行为。启动与唤醒可产生串行紧邻刷新；活跃重叠由MainActor/coordinator去重。

上述发现使“成功单次检查后恢复周期采集”的结论不成立。本次差异撤回持久暂停清除，并将规格和提示改为单次额度更新后仍暂停。更正后的复审由`/root/final_recovery_review`只读完成，确认暂停语义、测试预期、本地化提示和文档边界与本次更正一致；审查者要求补全测试/提交证据并修正暂停写盘措辞。该复审没有运行测试，验证证据单独列于开发记录。

## 更正差异复审

审查范围包括`RefreshCoordinator`、`AppModel`、macOS设置/详情提示、三种语言、定向测试及规格/计划/开发记录。独立复审未发现需要改动代码的问题；先前指出的文档证据占位和暂停写盘措辞已补齐。纠正已提交在本地分支`codex/antigravity-safe-auto-refresh`。
