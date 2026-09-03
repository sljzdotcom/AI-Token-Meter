use ai_token_meter_windows::security::SensitiveTextRedactor;

#[test]
fn removes_credentials_and_personal_identity_from_logs() {
    let api_key = "sk-".to_owned() + &"a".repeat(32);
    let bearer = "Bearer ".to_owned() + &"b".repeat(32);
    let input = format!(
        "request {api_key} {bearer} user@example.com 13812345678 C:\\Users\\Miller\\.codex\\sessions"
    );

    let output = SensitiveTextRedactor::redact(&input);

    assert!(!output.contains(&api_key));
    assert!(!output.contains(&bearer));
    assert!(!output.contains("user@example.com"));
    assert!(!output.contains("13812345678"));
    assert!(!output.contains("Miller"));
    assert_eq!(
        output,
        "request [secret] [authorization] [email] [phone] C:\\Users\\[user]\\.codex\\sessions"
    );
}

#[test]
fn removes_cookie_headers_without_erasing_the_following_diagnostic() {
    let input = "Cookie: session=private-value; path=/\nprobe timed out";

    assert_eq!(
        SensitiveTextRedactor::redact(input),
        "Cookie: [cookie]\nprobe timed out"
    );
}

#[test]
fn ordinary_diagnostics_remain_readable() {
    let input = "OpenAI Codex probe timed out after 10 seconds";

    assert_eq!(SensitiveTextRedactor::redact(input), input);
}
