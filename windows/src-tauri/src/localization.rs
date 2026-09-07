use crate::persistence::Locale;

pub fn text(locale: Locale, key: &str) -> &str {
    if locale != Locale::SimplifiedChinese {
        return key;
    }
    match key {
        "Refresh" => "刷新",
        "Settings" => "设置",
        "Show / Hide Meter" => "显示 / 隐藏浮动条",
        "Show Floating Strip Now" => "立即显示浮动条",
        "About AI Token Meter" => "关于 AI Token Meter",
        "Quit AI Token Meter" => "退出 AI Token Meter",
        "Refresh now" => "立即刷新",
        "Hide for 1 hour" => "隐藏 1 小时",
        "Settings…" => "设置…",
        "Unavailable" => "暂不可用",
        "Not installed" => "未安装",
        "Sign in required" => "需要登录",
        "Setup required" => "需要设置",
        "Needs update" => "需要更新",
        "AI Token Meter Settings" => "AI Token Meter 设置",
        "AI Token Meter Details" => "AI Token Meter 详情",
        "DeepSeek Usage · AI Token Meter" => "DeepSeek 用量 · AI Token Meter",
        "Paste the DeepSeek API Key into the Password field. It stays inside this protected Windows dialog." => {
            "请将 DeepSeek API 密钥粘贴到密码字段中。密钥仅保存在此受保护的 Windows 对话框内。"
        }
        "Session" => "当前会话",
        "Session limit" => "会话额度",
        "Weekly limit" => "每周额度",
        "Available balance" => "可用余额",
        _ => key,
    }
}

pub fn threshold_notice(
    locale: Locale,
    provider: &str,
    metric: &str,
    level: u8,
) -> (String, String) {
    if locale == Locale::SimplifiedChinese {
        (
            format!("{provider} 用量已达 {level}%"),
            format!("{}现已达到或超过 {level}%。", text(locale, metric)),
        )
    } else {
        (
            format!("{provider} usage reached {level}%"),
            format!("{metric} is now at or above {level}%."),
        )
    }
}
