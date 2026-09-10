use std::io::{BufRead, Read, Write};

use serde_json::Value;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("conpty-stdin") {
        run_conpty_stdin();
        return;
    }

    run_codex_app_server();
}

fn run_codex_app_server() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { return };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(id) = message.get("id").and_then(Value::as_u64) else {
            continue;
        };
        let response = match message.get("method").and_then(Value::as_str) {
            Some("initialize") => serde_json::json!({
                "id": id,
                "result": { "userAgent": "native-rust-fixture" }
            }),
            Some("account/read") => serde_json::json!({
                "id": id,
                "result": {
                    "account": {
                        "type": "chatgpt",
                        "email": "private@example.com",
                        "planType": "pro"
                    },
                    "requiresOpenaiAuth": true
                }
            }),
            Some("account/rateLimits/read") => serde_json::json!({
                "id": id,
                "result": {
                    "rateLimits": {
                        "primary": {
                            "usedPercent": 17,
                            "windowDurationMins": 300,
                            "resetsAt": 1788422400
                        },
                        "secondary": {
                            "usedPercent": 4,
                            "windowDurationMins": 10080,
                            "resetsAt": 1788652800
                        }
                    },
                    "rateLimitResetCredits": { "availableCount": 0, "credits": [] }
                }
            }),
            _ => continue,
        };
        if serde_json::to_writer(&mut output, &response).is_err()
            || output.write_all(b"\n").is_err()
            || output.flush().is_err()
        {
            return;
        }
    }
}

fn run_conpty_stdin() {
    let mut output = std::io::stdout().lock();
    if output.write_all(b"ready\r\n").is_err() || output.flush().is_err() {
        return;
    }

    let mut input = std::io::stdin().lock();
    let mut received = Vec::new();
    let mut chunk = [0_u8; 256];
    loop {
        let Ok(count) = input.read(&mut chunk) else {
            return;
        };
        if count == 0 {
            return;
        }
        received.extend_from_slice(&chunk[..count]);
        let received_line = [b"hello\r".as_slice(), b"hello\n".as_slice()]
            .iter()
            .any(|line| received.windows(line.len()).any(|window| window == *line));
        if received_line {
            let _ = output.write_all(b"received:hello\r\n");
            let _ = output.flush();
            return;
        }
        if received.len() > 4096 {
            return;
        }
    }
}
