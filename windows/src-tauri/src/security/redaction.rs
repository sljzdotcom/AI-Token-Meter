use std::sync::OnceLock;

use regex::Regex;

pub struct SensitiveTextRedactor;

impl SensitiveTextRedactor {
    pub fn redact(input: &str) -> String {
        let mut output = cookie_regex()
            .replace_all(input, "Cookie: [cookie]")
            .into_owned();
        output = bearer_regex()
            .replace_all(&output, "[authorization]")
            .into_owned();
        output = secret_regex().replace_all(&output, "[secret]").into_owned();
        output = email_regex().replace_all(&output, "[email]").into_owned();
        output = phone_regex().replace_all(&output, "[phone]").into_owned();
        windows_user_regex()
            .replace_all(&output, "${1}[user]")
            .into_owned()
    }
}

fn cookie_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?im)^Cookie:\s*[^\r\n]*").expect("valid regex"))
}

fn bearer_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{16,}").expect("valid regex"))
}

fn secret_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\bsk-[A-Za-z0-9_-]{16,}\b").expect("valid regex"))
}

fn email_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")
            .expect("valid regex")
    })
}

fn phone_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\b1[3-9][0-9]{9}\b").expect("valid regex"))
}

fn windows_user_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)([A-Z]:\\Users\\)[^\\\s]+").expect("valid regex"))
}
