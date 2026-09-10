use std::io::{BufRead, Write};

use serde_json::Value;

fn main() {
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
