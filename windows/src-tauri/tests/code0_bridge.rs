use ai_token_meter_windows::collectors::code0_bridge::{
    CODE0_BRIDGE_SOURCE, Code0BridgeContext, Code0BridgeEvent, Code0BridgeReason,
    Code0BridgeValidator,
};
use serde_json::{Value, json};

const EPOCH: &str = "11111111-1111-4111-8111-111111111111";
const ATTEMPT: &str = "22222222-2222-4222-8222-222222222222";
const NONCE: &str = "33333333-4444-4555-8666-777777777777";
const DOCUMENT: &str = "88888888-9999-4000-8111-222222222222";

fn context() -> Code0BridgeContext {
    Code0BridgeContext {
        connection_epoch: EPOCH.into(),
        attempt_id: ATTEMPT.into(),
        source_url: "https://console.code0.ai/console".into(),
        is_main_frame: true,
    }
}
fn validator() -> Code0BridgeValidator {
    Code0BridgeValidator::new(EPOCH, ATTEMPT, NONCE, DOCUMENT)
}
fn message(kind: &str, sequence: u64, fields: Value) -> Vec<u8> {
    let mut value =
        json!({"version":1,"nonce":NONCE,"document":DOCUMENT,"sequence":sequence,"kind":kind});
    for (key, field) in fields.as_object().unwrap() {
        value[key] = field.clone();
    }
    serde_json::to_vec(&value).unwrap()
}
fn capture(sequence: u64, request: u64, revision: u64) -> Vec<u8> {
    message(
        "capture",
        sequence,
        json!({"request":request,"revision":revision,"quota":1250000,"usedQuota":750000,"divisor":500000}),
    )
}
fn rejected(event: Code0BridgeEvent) -> bool {
    matches!(event, Code0BridgeEvent::Rejected(_))
}
fn receive(
    subject: &mut Code0BridgeValidator,
    kind: &str,
    sequence: u64,
    fields: Value,
) -> Code0BridgeEvent {
    subject.receive(&message(kind, sequence, fields), &context())
}
fn refresh_trace(subject: &mut Code0BridgeValidator) -> Vec<Code0BridgeEvent> {
    vec![
        receive(subject, "ready", 1, json!({})),
        receive(subject, "selfStart", 2, json!({"request":1,"revision":0})),
        receive(
            subject,
            "refreshStart",
            3,
            json!({"request":2,"revision":1}),
        ),
        receive(
            subject,
            "refreshStart",
            4,
            json!({"request":3,"revision":2}),
        ),
        receive(subject, "refreshEnd", 5, json!({"request":2,"revision":2})),
        receive(subject, "selfStart", 6, json!({"request":4,"revision":2})),
        receive(subject, "refreshEnd", 7, json!({"request":3,"revision":2})),
    ]
}
#[test]
fn complete_trace_captures_exactly_once_and_rejects_replay() {
    let mut subject = validator();
    let ready = receive(&mut subject, "ready", 1, json!({}));
    let started = receive(
        &mut subject,
        "selfStart",
        2,
        json!({"request":1,"revision":0}),
    );
    let accepted = subject.receive(&capture(3, 1, 0), &context());
    // Assert the final observable capture before control details: the deny-only
    // scaffold must fail for missing permission, not a missing type or panic.
    assert!(
        matches!(accepted, Code0BridgeEvent::Capture(_)),
        "valid trace must capture: {accepted:?}"
    );
    if let Code0BridgeEvent::Capture(values) = accepted {
        assert_eq!((values.balance, values.cumulative_usage), (2.5, 1.5));
    }
    assert_eq!(ready, Code0BridgeEvent::Control);
    assert_eq!(started, Code0BridgeEvent::Control);
    assert!(rejected(subject.receive(&capture(4, 1, 0), &context())));
}
#[test]
fn overlapping_refreshes_require_a_new_self_after_all_completions() {
    let mut premature = validator();
    refresh_trace(&mut premature);
    assert!(rejected(premature.receive(&capture(8, 4, 2), &context())));
    let mut clean = validator();
    let controls = refresh_trace(&mut clean);
    let started = receive(
        &mut clean,
        "selfStart",
        8,
        json!({"request":5,"revision":2}),
    );
    let accepted = clean.receive(&capture(9, 5, 2), &context());
    assert!(
        matches!(accepted, Code0BridgeEvent::Capture(_)),
        "post-refresh trace must capture: {accepted:?}"
    );
    assert!(
        controls
            .iter()
            .all(|event| *event == Code0BridgeEvent::Control)
    );
    assert_eq!(started, Code0BridgeEvent::Control);
}
#[test]
fn terminal_controls_have_distinct_meaning_and_dispose_revokes() {
    for (kind, reason, expected) in [
        (
            "invalidate",
            "authenticationChanged",
            Code0BridgeEvent::Invalidated(Code0BridgeReason::AuthenticationChanged),
        ),
        (
            "failure",
            "invalidResponse",
            Code0BridgeEvent::Failure(Code0BridgeReason::InvalidResponse),
        ),
    ] {
        let mut subject = validator();
        receive(&mut subject, "ready", 1, json!({}));
        assert_eq!(
            receive(&mut subject, kind, 2, json!({"reason":reason})),
            expected
        );
        assert!(rejected(subject.receive(&capture(3, 1, 0), &context())));
    }
    let mut subject = validator();
    subject.dispose();
    assert!(rejected(receive(&mut subject, "ready", 1, json!({}))));
}
#[test]
fn callback_context_cannot_be_claimed_by_page_json() {
    let mut contexts = Vec::new();
    let mut old = context();
    old.connection_epoch = ATTEMPT.into();
    contexts.push(old);
    let mut old = context();
    old.attempt_id = EPOCH.into();
    contexts.push(old);
    let mut frame = context();
    frame.is_main_frame = false;
    contexts.push(frame);
    for url in [
        "",
        "http://console.code0.ai/console",
        "https://sub.console.code0.ai/console",
        "https://console.code0.ai:444/console",
        "https://x@console.code0.ai/console",
    ] {
        let mut candidate = context();
        candidate.source_url = url.into();
        contexts.push(candidate);
    }
    for candidate in contexts {
        assert!(rejected(
            validator().receive(&message("ready", 1, json!({})), &candidate)
        ));
    }
}
#[test]
fn exact_envelope_rejects_extra_fields_types_identifiers_and_unsafe_counters() {
    for fields in [
        json!({"version":2}),
        json!({"version":true}),
        json!({"nonce":EPOCH}),
        json!({"document":EPOCH}),
        json!({"sequence":0}),
        json!({"sequence":-1}),
        json!({"sequence":1.5}),
        json!({"sequence":9007199254740992_u64}),
        json!({"sequence":true}),
        json!({"kind":"unknown"}),
        json!({"quota":1}),
        json!({"reason":"raw-response"}),
        json!({"request":1}),
    ] {
        assert!(rejected(
            validator().receive(&message("ready", 1, fields), &context())
        ));
    }
    let normal = String::from_utf8(message("ready", 1, json!({}))).unwrap();
    for raw in [
        format!("{{\"version\":1,{}", &normal[1..]),
        format!("{normal}{}", " ".repeat(2049)),
        "[]".into(),
        "{}".into(),
    ] {
        assert!(rejected(validator().receive(raw.as_bytes(), &context())));
    }
}
#[test]
fn reordered_sequences_and_forged_request_revision_cannot_publish() {
    for variant in [
        "noReady",
        "sequence",
        "olderSelf",
        "request",
        "revision",
        "refreshInFlight",
        "unknownEnd",
        "reusedRequest",
        "revisionJump",
    ] {
        let mut subject = validator();
        if variant != "noReady" {
            receive(&mut subject, "ready", 1, json!({}));
        }
        receive(
            &mut subject,
            "selfStart",
            2,
            json!({"request":1,"revision":0}),
        );
        let last = match variant {
            "sequence" => capture(2, 1, 0),
            "olderSelf" => {
                receive(
                    &mut subject,
                    "selfStart",
                    3,
                    json!({"request":2,"revision":0}),
                );
                capture(4, 1, 0)
            }
            "request" => capture(3, 2, 0),
            "revision" => capture(3, 1, 1),
            "refreshInFlight" => {
                receive(
                    &mut subject,
                    "refreshStart",
                    3,
                    json!({"request":2,"revision":1}),
                );
                capture(4, 1, 1)
            }
            "unknownEnd" => message("refreshEnd", 3, json!({"request":9,"revision":0})),
            "reusedRequest" => message("selfStart", 3, json!({"request":1,"revision":0})),
            "revisionJump" => message("refreshStart", 3, json!({"request":2,"revision":2})),
            _ => capture(3, 1, 0),
        };
        assert!(rejected(subject.receive(&last, &context())), "{variant}");
    }
}
#[test]
fn capture_rejects_ambiguous_or_invalid_numeric_envelopes() {
    let normal = String::from_utf8(capture(3, 1, 0)).unwrap();
    for raw in [
        normal.replace("1250000", "9007199254740991.1"),
        normal.replace("1250000", "true"),
        normal.replace("500000", "0"),
        format!("{{\"quota\":1,{}", &normal[1..]),
        format!("{{\"identity\":1,{}", &normal[1..]),
    ] {
        let mut subject = validator();
        receive(&mut subject, "ready", 1, json!({}));
        receive(
            &mut subject,
            "selfStart",
            2,
            json!({"request":1,"revision":0}),
        );
        assert!(rejected(subject.receive(raw.as_bytes(), &context())));
    }
}
#[test]
fn embedded_source_is_the_canonical_bridge_expression() {
    assert_eq!(
        CODE0_BRIDGE_SOURCE,
        include_str!("../../../web/code0-bridge.js")
    );
}

#[test]
fn capture_rechecks_late_callback_context_and_message_association() {
    for variant in ["epoch", "attempt", "frame", "source", "nonce", "document"] {
        let mut subject = validator();
        receive(&mut subject, "ready", 1, json!({}));
        receive(
            &mut subject,
            "selfStart",
            2,
            json!({"request":1,"revision":0}),
        );
        let mut callback = context();
        let mut fields =
            json!({"request":1,"revision":0,"quota":1250000,"usedQuota":750000,"divisor":500000});
        match variant {
            "epoch" => callback.connection_epoch = ATTEMPT.into(),
            "attempt" => callback.attempt_id = EPOCH.into(),
            "frame" => callback.is_main_frame = false,
            "source" => callback.source_url = "https://other.invalid/console".into(),
            _ => fields[variant] = json!(EPOCH),
        }
        assert!(
            rejected(subject.receive(&message("capture", 3, fields), &callback)),
            "{variant}"
        );
    }
}

#[test]
fn control_kinds_reject_mixed_fields_and_invalid_fixed_reasons() {
    for (kind, fields) in [
        ("refreshStart", json!({"request":1,"revision":1,"quota":1})),
        ("selfStart", json!({"request":true,"revision":0})),
        ("selfStart", json!({"request":1.5,"revision":0})),
        ("selfStart", json!({"request":1,"revision":-1})),
        ("invalidate", json!({"reason":"invalidResponse"})),
        ("failure", json!({"reason":"authenticationChanged"})),
        ("failure", json!({"reason":"raw-exception-message"})),
    ] {
        let mut subject = validator();
        receive(&mut subject, "ready", 1, json!({}));
        assert!(rejected(receive(&mut subject, kind, 2, fields)));
    }
}

#[test]
fn counter_gaps_max_request_and_page_routes_accept_zero_fractional_values() {
    let mut subject = validator();
    let mut routed = context();
    routed.source_url = "https://console.code0.ai:443/console?tab=usage#quota".into();
    assert_eq!(
        subject.receive(&message("ready", 7, json!({})), &routed),
        Code0BridgeEvent::Control
    );
    assert_eq!(
        subject.receive(
            &message(
                "selfStart",
                10,
                json!({"request":9007199254740991_u64,"revision":0})
            ),
            &routed
        ),
        Code0BridgeEvent::Control
    );
    let payload = message(
        "capture",
        11,
        json!({"request":9007199254740991_u64,"revision":0,"quota":0,"usedQuota":1,"divisor":0.5}),
    );
    let event = subject.receive(&payload, &routed);
    match event {
        Code0BridgeEvent::Capture(value) => {
            assert_eq!((value.balance, value.cumulative_usage), (0.0, 2.0))
        }
        _ => panic!("valid boundary capture: {event:?}"),
    }
    assert_eq!(
        subject.receive(&payload, &routed),
        Code0BridgeEvent::Rejected(
            ai_token_meter_windows::collectors::code0_bridge::Code0BridgeRejection::Disposed
        )
    );
}

#[test]
fn decoded_duplicate_keys_and_raw_fractional_counters_revoke() {
    use ai_token_meter_windows::collectors::code0_bridge::Code0BridgeRejection;
    let normal = String::from_utf8(message("ready", 1, json!({}))).unwrap();
    for raw in [
        format!("{{\"sequen\\u0063e\":1,{}", &normal[1..]),
        normal.replace("\"sequence\":1", "\"sequence\":1.00000000000000001"),
        normal.replace("\"sequence\":1", "\"sequence\":9007199254740991.1"),
        normal.replace("\"sequence\":1", "\"sequence\":null"),
        normal.replace("\"sequence\":1", "\"sequence\":\"1\""),
        format!("{normal}{{}}"),
    ] {
        let mut subject = validator();
        assert!(rejected(subject.receive(raw.as_bytes(), &context())));
        assert_eq!(
            receive(&mut subject, "ready", 2, json!({})),
            Code0BridgeEvent::Rejected(Code0BridgeRejection::Disposed)
        );
    }
}

#[test]
fn rejection_dispose_and_bad_configuration_cannot_be_reused() {
    use ai_token_meter_windows::collectors::code0_bridge::Code0BridgeRejection;
    for source in [
        "https://@console.code0.ai/console",
        "https://console.code0.ai\\console",
        " https://console.code0.ai/console",
    ] {
        let mut subject = validator();
        let mut untrusted = context();
        untrusted.source_url = source.into();
        assert_eq!(
            subject.receive(&message("ready", 1, json!({})), &untrusted),
            Code0BridgeEvent::Rejected(Code0BridgeRejection::InvalidContext)
        );
        assert_eq!(
            receive(&mut subject, "ready", 2, json!({})),
            Code0BridgeEvent::Rejected(Code0BridgeRejection::Disposed)
        );
    }
    let mut subject = validator();
    assert_eq!(
        receive(&mut subject, "ready", 1, json!({})),
        Code0BridgeEvent::Control
    );
    assert_eq!(
        receive(&mut subject, "ready", 2, json!({})),
        Code0BridgeEvent::Rejected(Code0BridgeRejection::InvalidState)
    );
    assert_eq!(
        receive(
            &mut subject,
            "selfStart",
            3,
            json!({"request":1,"revision":0})
        ),
        Code0BridgeEvent::Rejected(Code0BridgeRejection::Disposed)
    );
    let mut invalid = Code0BridgeValidator::new(EPOCH, ATTEMPT, "not-a-uuid", DOCUMENT);
    assert_eq!(
        receive(&mut invalid, "ready", 1, json!({})),
        Code0BridgeEvent::Rejected(Code0BridgeRejection::ValidationRequired)
    );
}
