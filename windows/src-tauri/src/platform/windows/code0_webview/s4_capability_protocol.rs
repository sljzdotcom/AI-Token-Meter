//! Finite S4 parsing only. No parsed value supplies native authority.
//! Tests were authored before implementation and remain UNRUN.

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ProtocolIdentity {
        ProtocolIdentity::new(
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
            Selector::OpenMain,
        )
        .unwrap()
    }

    const PARSED: &str = r#"{"p":"s4c1","n":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","c":"s4-capability-open-main","s":1,"k":"parsed","f":0}"#;
    const CONTEXT: &str = r#"{"context":{"id":4,"uniqueId":"unique-root","origin":"https://console.code0.ai","name":"","auxData":{"frameId":"root","isDefault":true,"type":"default"}}}"#;
    const FRAME: &str = r#"{"frame":{"id":"root","loaderId":"loader","url":"https://console.code0.ai/__ai_meter_fixture__/s4-capability-main","securityOrigin":"https://console.code0.ai","mimeType":"text/html"}}"#;

    // Mutation: replacing duplicate-aware scanning with Value/map last-wins.
    #[test]
    fn rejects_duplicate_decoded_keys_in_own_and_discarded_metadata() {
        let own = PARSED.replace("\"f\":0", r#""f":0,"\u0066":0"#);
        assert!(parse_probe(own.as_bytes(), &identity()).is_err());
        let metadata = CONTEXT.replace("\"name\":\"\"", r#""name":"","future":{"a":1,"\u0061":2}"#);
        assert!(parse_context_created(metadata.as_bytes()).is_err());
    }

    // Mutation: accepting extra own fields, float counters or bool-as-integer.
    #[test]
    fn own_protocol_is_closed_and_integer_typed() {
        assert_eq!(
            parse_probe(PARSED.as_bytes(), &identity()).unwrap().kind,
            ProbeKind::Parsed
        );
        for bad in [
            PARSED.replace("\"s\":1", "\"s\":true"),
            PARSED.replace("\"s\":1", "\"s\":1.0"),
            PARSED.replace("\"f\":0", "\"f\":false"),
            PARSED.replace("\"f\":0", "\"f\":32"),
            PARSED.replace("\"s\":1", "\"s\":0"),
            PARSED.replace("\"s\":1", "\"s\":9"),
            PARSED.replace("\"f\":0", "\"f\":0,\"extra\":0"),
        ] {
            assert!(parse_probe(bad.as_bytes(), &identity()).is_err(), "{bad}");
        }
        assert!(
            parse_probe(
                PARSED
                    .replace("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", "other")
                    .as_bytes(),
                &identity()
            )
            .is_err()
        );
    }

    // Mutation: conflating canonical ready with probes/numeric captures.
    #[test]
    fn canonical_ready_has_five_exact_keys_and_positive_safe_sequence() {
        let ready = r#"{"version":1,"nonce":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","document":"bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb","sequence":1,"kind":"ready"}"#;
        assert_eq!(
            parse_ready(ready.as_bytes(), &identity()).unwrap().sequence,
            1
        );
        for bad in [
            ready.replace("\"ready\"", "\"capture\""),
            ready.replace("\"version\":1", "\"version\":true"),
            ready.replace("\"sequence\":1", "\"sequence\":0"),
            ready.replace("\"sequence\":1", "\"sequence\":9007199254740992"),
            ready.replace("\"kind\":\"ready\"", "\"kind\":\"ready\",\"quota\":1"),
        ] {
            assert!(parse_ready(bad.as_bytes(), &identity()).is_err());
        }
        assert!(parse_ready(PARSED.as_bytes(), &identity()).is_err());
    }

    // Mutation: allowing arbitrary selectors, command names or parameters.
    #[test]
    fn exposes_only_seven_fixed_commands_and_seven_receivers() {
        let commands: Vec<_> = Method::ALL
            .iter()
            .map(|m| (m.name(), m.parameters()))
            .collect();
        assert_eq!(
            commands,
            vec![
                ("Runtime.enable", "{}"),
                ("Page.enable", "{}"),
                ("Runtime.addBinding", r#"{"name":"__aiMeterS4Capability"}"#),
                ("Page.getFrameTree", "{}"),
                (
                    "Runtime.removeBinding",
                    r#"{"name":"__aiMeterS4Capability"}"#
                ),
                ("Page.disable", "{}"),
                ("Runtime.disable", "{}")
            ]
        );
        assert_eq!(
            Event::ALL.map(Event::name),
            [
                "Runtime.bindingCalled",
                "Runtime.executionContextCreated",
                "Runtime.executionContextDestroyed",
                "Runtime.executionContextsCleared",
                "Page.documentOpened",
                "Page.frameNavigated",
                "Page.frameDetached"
            ]
        );
        assert_eq!(
            Selector::parse("s4-capability-open-child-prototype"),
            Some(Selector::OpenChildPrototype)
        );
        assert!(Selector::parse("Runtime.evaluate").is_none());
    }

    // Mutation: accepting arbitrary control fields or caller script transport.
    #[test]
    fn control_round_trip_is_closed_and_bounded() {
        let encoded = encode_control(&identity(), ControlKind::Detach).unwrap();
        assert_eq!(
            parse_control(encoded.as_bytes(), &identity()).unwrap(),
            ControlKind::Detach
        );
        assert!(parse_control(br#"{"p":"s4c1","n":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","c":"s4-capability-open-main","k":"detach","script":"x"}"#, &identity()).is_err());
        assert!(parse_control(br#"{"p":"s4c1","n":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","c":"s4-capability-open-main","k":"eval"}"#, &identity()).is_err());
    }

    // Mutation: accepting ID0, rounded doubles, oversized identities or numeric bool.
    #[test]
    fn engine_known_fields_have_strict_types_and_ranges() {
        for bad in [
            CONTEXT.replace("\"id\":4", "\"id\":0"),
            CONTEXT.replace("\"id\":4", "\"id\":2147483648"),
            CONTEXT.replace("\"id\":4", "\"id\":4e0"),
            CONTEXT.replace("\"isDefault\":true", "\"isDefault\":1"),
            CONTEXT.replace("unique-root", &"x".repeat(129)),
            CONTEXT.replace("unique-root", ""),
        ] {
            assert!(parse_context_created(bad.as_bytes()).is_err());
        }
        assert!(
            parse_context_created(CONTEXT.replace("\"id\":4", "\"id\":2147483647").as_bytes())
                .is_ok()
        );
        assert!(
            parse_frame(
                FRAME
                    .replace("\"loaderId\":\"loader\"", "\"loaderId\":false")
                    .as_bytes()
            )
            .is_err()
        );
    }

    // Mutation: treating name/default-ID/page marker as root provenance.
    #[test]
    fn root_eligibility_needs_actual_aux_unique_origin_and_root() {
        let root = Identity::new("root").unwrap();
        let context = parse_context_created(CONTEXT.as_bytes()).unwrap();
        assert_eq!(
            context.root_eligibility(&root),
            RootEligibility::EligibleMetadata
        );
        for other in [
            CONTEXT.replace("\"frameId\":\"root\"", "\"frameId\":\"child\""),
            CONTEXT.replace("\"isDefault\":true", "\"isDefault\":false"),
            CONTEXT.replace("https://console.code0.ai", "https://else.invalid"),
            CONTEXT.replace("\"uniqueId\":\"unique-root\",", ""),
            CONTEXT.replace(
                "\"auxData\":{\"frameId\":\"root\",\"isDefault\":true,\"type\":\"default\"}",
                "\"auxData\":{}",
            ),
        ] {
            assert_ne!(
                parse_context_created(other.as_bytes())
                    .unwrap()
                    .root_eligibility(&root),
                RootEligibility::EligibleMetadata
            );
        }
        let frame = parse_frame(FRAME.as_bytes()).unwrap();
        assert!(frame.matches_main_root(&root));
        assert!(
            !parse_frame(
                FRAME
                    .replace("\"id\":\"root\"", "\"id\":\"child\",\"parentId\":\"root\"")
                    .as_bytes()
            )
            .unwrap()
            .matches_main_root(&root)
        );
    }

    // Mutation: retaining/using unknown engine fields or rejecting bounded future data.
    #[test]
    fn bounded_unknown_engine_metadata_is_discarded() {
        let extra = CONTEXT.replace(
            "\"name\":\"\"",
            "\"name\":\"\",\"future\":{\"nested\":[1,true,null,{\"x\":\"ignored\"}]}",
        );
        assert_eq!(
            parse_context_created(extra.as_bytes()).unwrap(),
            parse_context_created(CONTEXT.as_bytes()).unwrap()
        );
        assert!(parse_contexts_cleared(br#"{"future":[1,true]}"#).is_ok());
        assert!(parse_contexts_cleared(br#"[]"#).is_err());
    }

    // Mutation: ignoring structural caps in discarded metadata.
    #[test]
    fn depth_and_value_limits_include_skipped_engine_metadata() {
        let depth16 = format!("{{\"x\":{}0{}}}", "[".repeat(14), "]".repeat(14));
        let depth17 = format!("{{\"x\":{}0{}}}", "[".repeat(15), "]".repeat(15));
        assert!(parse_contexts_cleared(depth16.as_bytes()).is_ok());
        assert!(parse_contexts_cleared(depth17.as_bytes()).is_err());
        let values512 = format!("{{\"x\":[{}]}}", vec!["0"; 510].join(","));
        let values513 = format!("{{\"x\":[{}]}}", vec!["0"; 511].join(","));
        assert!(parse_contexts_cleared(values512.as_bytes()).is_ok());
        assert!(parse_contexts_cleared(values513.as_bytes()).is_err());
    }

    // Mutation: applying just UTF16 or UTF8 cap, lossy surrogate decoding, third raw slot.
    #[test]
    fn raw_copy_caps_both_encodings_and_two_slots() {
        assert!(RawJson::from_utf16(&vec![u16::from(b' '); 8192]).is_ok());
        assert!(RawJson::from_utf16(&vec![u16::from(b' '); 8193]).is_err());
        assert!(RawJson::from_utf16(&vec![0x4e00; 2730]).is_ok());
        assert!(RawJson::from_utf16(&vec![0x4e00; 2731]).is_err());
        assert!(RawJson::from_utf16(&[0xd800]).is_err());
        let mut slots = RawSlots::new();
        assert_eq!(slots.copy_utf8(b"{}"), Ok(0));
        assert_eq!(slots.copy_utf8(b"{}"), Ok(1));
        assert!(slots.copy_utf8(b"{}").is_err());
        assert!(slots.release(0));
        assert_eq!(slots.copy_utf8(b"{}"), Ok(0));
    }

    // Mutation: accepting arbitrary command-result/root shape or losing child identities.
    #[test]
    fn frame_tree_ack_and_lifecycle_projections_are_distinct() {
        let tree = br#"{"frameTree":{"frame":{"id":"root","loaderId":"blank-loader","url":"about:blank","securityOrigin":"://","mimeType":"text/html"},"childFrames":[]}}"#;
        assert_eq!(
            parse_method_result(Method::GetFrameTree, tree).unwrap(),
            MethodResult::InitialRoot(Identity::new("root").unwrap())
        );
        assert!(parse_method_result(Method::RuntimeEnable, tree).is_err());
        assert!(parse_method_result(Method::RuntimeEnable, b"{}").is_ok());
        assert!(parse_method_result(Method::GetFrameTree, b"{}").is_err());
        assert!(parse_method_result(Method::GetFrameTree, &tree[..tree.len() - 1]).is_err());
        assert_eq!(
            parse_context_destroyed(
                br#"{"executionContextId":4,"executionContextUniqueId":"unique-root"}"#
            )
            .unwrap()
            .unique_id
            .unwrap()
            .as_str(),
            "unique-root"
        );
        assert!(parse_context_destroyed(br#"{"executionContextId":true}"#).is_err());
        assert_eq!(
            parse_frame_detached(br#"{"frameId":"child","reason":"remove"}"#)
                .unwrap()
                .reason,
            DetachReason::Remove
        );
        assert!(parse_frame_detached(br#"{"frameId":"root","reason":"unknown"}"#).is_err());
    }

    // Mutation: accepting binding name/context spoofing or ready as a probe.
    #[test]
    fn binding_projection_keeps_engine_context_separate() {
        let payload = serde_json::to_string(PARSED).unwrap();
        let event = format!(
            "{{\"name\":\"__aiMeterS4Capability\",\"payload\":{payload},\"executionContextId\":4}}"
        );
        let message = parse_binding(event.as_bytes(), &identity()).unwrap();
        assert_eq!(message.context_id.get(), 4);
        assert!(matches!(
            message.payload,
            PagePayload::Probe(Probe {
                kind: ProbeKind::Parsed,
                ..
            })
        ));
        assert!(
            parse_binding(
                event.replace("__aiMeterS4Capability", "other").as_bytes(),
                &identity()
            )
            .is_err()
        );
        assert!(
            parse_binding(
                event
                    .replace("\"executionContextId\":4", "\"executionContextId\":0")
                    .as_bytes(),
                &identity()
            )
            .is_err()
        );
    }

    // Mutation: forgetting per-payload size, UTF8 identity bounds or root sequence.
    #[test]
    fn payload_and_sequence_limits_are_independent() {
        let padded = format!("{}{}", PARSED, " ".repeat(512 - PARSED.len()));
        assert!(parse_probe(padded.as_bytes(), &identity()).is_ok());
        assert!(parse_probe(format!("{padded} ").as_bytes(), &identity()).is_err());
        assert!(Identity::new(&"界".repeat(42)).is_ok());
        assert!(Identity::new(&"界".repeat(43)).is_err());
        let mut sequence = ProbeSequence::new();
        let first = parse_probe(PARSED.as_bytes(), &identity()).unwrap();
        sequence.accept_root(&first).unwrap();
        assert!(sequence.accept_root(&first).is_err());
        let next =
            parse_probe(PARSED.replace("\"s\":1", "\"s\":8").as_bytes(), &identity()).unwrap();
        sequence.accept_root(&next).unwrap();
        assert!(
            parse_probe(
                PARSED
                    .replace("parsed", "childProbe")
                    .replace("\"s\":1", "\"s\":2")
                    .as_bytes(),
                &identity()
            )
            .is_err()
        );
        assert!(std::mem::size_of::<ExecutionContext>() <= 1024);
        assert!(std::mem::size_of::<Probe>() <= 512);
    }

    // Mutation: evicting/replacing mappings or silently overflowing bounded storage.
    #[test]
    fn fixed_context_and_message_storage_never_evicts() {
        let mut contexts = ContextTable::new();
        for id in 1..=8 {
            contexts
                .insert(
                    parse_context_created(
                        CONTEXT
                            .replace("\"id\":4", &format!("\"id\":{id}"))
                            .as_bytes(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        assert!(
            contexts
                .insert(
                    parse_context_created(CONTEXT.replace("\"id\":4", "\"id\":9").as_bytes())
                        .unwrap()
                )
                .is_err()
        );
        assert!(
            contexts
                .insert(parse_context_created(CONTEXT.as_bytes()).unwrap())
                .is_err()
        );
        assert_eq!(
            contexts
                .get(ContextId::new(4).unwrap())
                .unwrap()
                .unique_id
                .as_ref()
                .unwrap()
                .as_str(),
            "unique-root"
        );
        let mut messages = StagedMessages::new();
        let message = BindingMessage {
            context_id: ContextId::new(4).unwrap(),
            payload: PagePayload::Probe(parse_probe(PARSED.as_bytes(), &identity()).unwrap()),
        };
        for _ in 0..10 {
            messages.push(message.clone()).unwrap();
        }
        assert!(messages.push(message).is_err());
        assert_eq!(messages.pop_front().unwrap().context_id.get(), 4);
    }

    // Real binding projection for storage inputs; this grants no native/root authority.
    fn staged_binding(payload: &str, context_id: u32) -> BindingMessage {
        let payload = serde_json::to_string(payload).unwrap();
        let event = format!(
            "{{\"name\":\"__aiMeterS4Capability\",\"payload\":{payload},\"executionContextId\":{context_id}}}"
        );
        parse_binding(event.as_bytes(), &identity()).unwrap()
    }

    // Mutation: removing the ready latch, or resetting it when the first Ready is popped.
    #[test]
    fn staged_ready_is_unique_before_and_after_pop() {
        let ready = r#"{"version":1,"nonce":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa","document":"bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb","sequence":1,"kind":"ready"}"#;
        let first = staged_binding(ready, 4);
        assert_eq!(first.context_id.get(), 4);
        assert!(matches!(&first.payload, PagePayload::Ready(ready) if ready.sequence() == 1));
        let second = staged_binding(&ready.replace("\"sequence\":1", "\"sequence\":2"), 5);
        assert!(matches!(&second.payload, PagePayload::Ready(ready) if ready.sequence() == 2));
        let probe = staged_binding(PARSED, 6);
        let mut messages = StagedMessages::new();
        messages.push(first.clone()).unwrap();
        messages.push(probe.clone()).unwrap();
        assert_eq!(messages.push(second.clone()), Err(ParseError::Capacity));
        assert_eq!(messages.pop_front(), Some(first));
        assert_eq!(messages.push(second), Err(ParseError::Capacity));
        assert_eq!(messages.pop_front(), Some(probe));
        assert_eq!(messages.pop_front(), None);
    }

    // Mutation: recycling cumulative capacity after pop, or corrupting FIFO on failed push.
    #[test]
    fn staged_message_limit_is_cumulative_after_pop_and_preserves_fifo() {
        let mut messages = StagedMessages::new();
        for context_id in 1..=3 {
            messages.push(staged_binding(PARSED, context_id)).unwrap();
        }
        assert_eq!(messages.pop_front(), Some(staged_binding(PARSED, 1)));
        for context_id in 4..=10 {
            messages.push(staged_binding(PARSED, context_id)).unwrap();
        }
        assert_eq!(
            messages.push(staged_binding(PARSED, 11)),
            Err(ParseError::Capacity)
        );
        for context_id in 2..=10 {
            assert_eq!(
                messages.pop_front(),
                Some(staged_binding(PARSED, context_id))
            );
        }
        assert_eq!(messages.pop_front(), None);
        assert_eq!(
            messages.push(staged_binding(PARSED, 12)),
            Err(ParseError::Capacity)
        );
        assert_eq!(messages.pop_front(), None);
    }
}

use serde::Deserialize;
use serde_json::value::RawValue;
use std::collections::BTreeSet;

pub const MAX_RAW_UTF16: usize = 8192;
pub const MAX_RAW_UTF8: usize = 8192;
pub const MAX_DEPTH: usize = 16;
pub const MAX_VALUES: usize = 512;
pub const MAX_PROBE_UTF8: usize = 512;
pub const MAX_READY_UTF8: usize = 2048;
pub const MAX_CONTROL_UTF8: usize = 256;
pub const MAX_CONTEXTS: usize = 8;
pub const MAX_BINDING_MESSAGES: usize = 10;
pub const BINDING_NAME: &str = "__aiMeterS4Capability";
pub const MAIN_URL: &str = "https://console.code0.ai/__ai_meter_fixture__/s4-capability-main";
pub const CHILD_URL: &str = "https://console.code0.ai/__ai_meter_fixture__/s4-capability-child";
pub const OFFICIAL_ORIGIN: &str = "https://console.code0.ai";

/// Deliberately contains no raw text, native exception or page-supplied reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    Encoding,
    RawBound,
    Structure,
    DuplicateKey,
    DepthBound,
    ValueBound,
    Shape,
    Type,
    Range,
    IdentityMismatch,
    Capacity,
    DuplicateContext,
    Sequence,
}

type Parsed<T> = Result<T, ParseError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selector {
    OpenMain,
    WriteMain,
    OpenChildPrototype,
    DisposeMissing,
}

impl Selector {
    pub const ALL: [Self; 4] = [
        Self::OpenMain,
        Self::WriteMain,
        Self::OpenChildPrototype,
        Self::DisposeMissing,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::OpenMain => "s4-capability-open-main",
            Self::WriteMain => "s4-capability-write-main",
            Self::OpenChildPrototype => "s4-capability-open-child-prototype",
            Self::DisposeMissing => "s4-capability-dispose-missing",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|selector| selector.name() == value)
    }
    pub const fn control_limit(self) -> usize {
        if matches!(self, Self::DisposeMissing) {
            2
        } else {
            3
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    RuntimeEnable,
    PageEnable,
    AddBinding,
    GetFrameTree,
    RemoveBinding,
    PageDisable,
    RuntimeDisable,
}

impl Method {
    pub const ALL: [Self; 7] = [
        Self::RuntimeEnable,
        Self::PageEnable,
        Self::AddBinding,
        Self::GetFrameTree,
        Self::RemoveBinding,
        Self::PageDisable,
        Self::RuntimeDisable,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::RuntimeEnable => "Runtime.enable",
            Self::PageEnable => "Page.enable",
            Self::AddBinding => "Runtime.addBinding",
            Self::GetFrameTree => "Page.getFrameTree",
            Self::RemoveBinding => "Runtime.removeBinding",
            Self::PageDisable => "Page.disable",
            Self::RuntimeDisable => "Runtime.disable",
        }
    }
    pub const fn parameters(self) -> &'static str {
        match self {
            Self::AddBinding | Self::RemoveBinding => r#"{"name":"__aiMeterS4Capability"}"#,
            _ => "{}",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    BindingCalled,
    ContextCreated,
    ContextDestroyed,
    ContextsCleared,
    DocumentOpened,
    FrameNavigated,
    FrameDetached,
}

impl Event {
    pub const ALL: [Self; 7] = [
        Self::BindingCalled,
        Self::ContextCreated,
        Self::ContextDestroyed,
        Self::ContextsCleared,
        Self::DocumentOpened,
        Self::FrameNavigated,
        Self::FrameDetached,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::BindingCalled => "Runtime.bindingCalled",
            Self::ContextCreated => "Runtime.executionContextCreated",
            Self::ContextDestroyed => "Runtime.executionContextDestroyed",
            Self::ContextsCleared => "Runtime.executionContextsCleared",
            Self::DocumentOpened => "Page.documentOpened",
            Self::FrameNavigated => "Page.frameNavigated",
            Self::FrameDetached => "Page.frameDetached",
        }
    }
}

/// Storage is fixed-size; constructors check UTF-8 bytes, not scalar count.
#[derive(Clone, PartialEq, Eq)]
pub struct BoundedText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> BoundedText<N> {
    fn copy(value: &str, nonempty: bool) -> Parsed<Self> {
        if value.len() > N || (nonempty && value.is_empty()) {
            return Err(ParseError::Range);
        }
        let mut bytes = [0; N];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            bytes,
            len: value.len(),
        })
    }
    pub fn as_str(&self) -> &str {
        // Only valid UTF-8 is copied, and the initialized prefix is immutable.
        std::str::from_utf8(&self.bytes[..self.len]).expect("bounded UTF-8 invariant")
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

// Do not accidentally log identity/source strings through derived Debug.
impl<const N: usize> std::fmt::Debug for BoundedText<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundedText")
            .field("bytes", &self.len)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity(BoundedText<128>);
impl Identity {
    pub fn new(value: &str) -> Parsed<Self> {
        Ok(Self(BoundedText::copy(value, true)?))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin(BoundedText<64>);
impl Origin {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextName(BoundedText<128>);
impl ContextName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Expected native constants only. Construction never establishes native ownership.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolIdentity {
    nonce: Identity,
    document: Identity,
    selector: Selector,
}
impl ProtocolIdentity {
    pub fn new(nonce: &str, document: &str, selector: Selector) -> Parsed<Self> {
        Ok(Self {
            nonce: Identity::new(nonce)?,
            document: Identity::new(document)?,
            selector,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    Prepare,
    Mutate,
    Detach,
}
impl ControlKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Mutate => "mutate",
            Self::Detach => "detach",
        }
    }
    fn parse(value: &str) -> Parsed<Self> {
        match value {
            "prepare" => Ok(Self::Prepare),
            "mutate" => Ok(Self::Mutate),
            "detach" => Ok(Self::Detach),
            _ => Err(ParseError::Range),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeKind {
    Parsed,
    Candidate,
    ChildProbe,
    BeforeOpen,
    AfterOpen,
    Later,
    Disposed,
    DisposeWitness,
}
impl ProbeKind {
    fn parse(value: &str) -> Parsed<Self> {
        match value {
            "parsed" => Ok(Self::Parsed),
            "candidate" => Ok(Self::Candidate),
            "childProbe" => Ok(Self::ChildProbe),
            "beforeOpen" => Ok(Self::BeforeOpen),
            "afterOpen" => Ok(Self::AfterOpen),
            "later" => Ok(Self::Later),
            "disposed" => Ok(Self::Disposed),
            "disposeWitness" => Ok(Self::DisposeWitness),
            _ => Err(ParseError::Range),
        }
    }
}

/// Actual decoded fields, with n/c already compared against native expected constants.
/// Keeping this value is not a permit; native integration must join the engine context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probe {
    sequence: u8,
    kind: ProbeKind,
    flags: u8,
}
impl Probe {
    pub fn sequence(&self) -> u8 {
        self.sequence
    }
    pub fn kind(&self) -> ProbeKind {
        self.kind
    }
    pub fn flags(&self) -> u8 {
        self.flags
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ready {
    sequence: u64,
}
impl Ready {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PagePayload {
    Probe(Probe),
    Ready(Ready),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextId(u32);
impl ContextId {
    pub fn new(value: u32) -> Parsed<Self> {
        if value == 0 || value > i32::MAX as u32 {
            return Err(ParseError::Range);
        }
        Ok(Self(value))
    }
    pub fn get(self) -> u32 {
        self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingMessage {
    pub context_id: ContextId,
    pub payload: PagePayload,
}

/// Root-only sequence helper. A separately refused child is not a root sequence event.
#[derive(Default)]
pub struct ProbeSequence {
    last: u8,
}
impl ProbeSequence {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn accept_root(&mut self, probe: &Probe) -> Parsed<()> {
        if probe.kind == ProbeKind::ChildProbe || probe.sequence <= self.last {
            return Err(ParseError::Sequence);
        }
        self.last = probe.sequence;
        Ok(())
    }
}

pub fn encode_control(
    identity: &ProtocolIdentity,
    kind: ControlKind,
) -> Parsed<BoundedText<MAX_CONTROL_UTF8>> {
    #[derive(serde::Serialize)]
    struct Control<'a> {
        p: &'static str,
        n: &'a str,
        c: &'static str,
        k: &'static str,
    }
    let text = serde_json::to_string(&Control {
        p: "s4c1",
        n: identity.nonce.as_str(),
        c: identity.selector.name(),
        k: kind.name(),
    })
    .map_err(|_| ParseError::Type)?;
    BoundedText::copy(&text, true)
}

fn own_identity(fields: &Fields<'_>, identity: &ProtocolIdentity) -> Parsed<()> {
    if fields.string("p")? != "s4c1"
        || fields.string("n")? != identity.nonce.as_str()
        || fields.string("c")? != identity.selector.name()
    {
        return Err(ParseError::IdentityMismatch);
    }
    Ok(())
}

pub fn parse_control(bytes: &[u8], identity: &ProtocolIdentity) -> Parsed<ControlKind> {
    let fields = fields(bytes, MAX_CONTROL_UTF8)?;
    fields.exact(&["p", "n", "c", "k"])?;
    own_identity(&fields, identity)?;
    ControlKind::parse(&fields.string("k")?)
}

pub fn parse_probe(bytes: &[u8], identity: &ProtocolIdentity) -> Parsed<Probe> {
    let fields = fields(bytes, MAX_PROBE_UTF8)?;
    fields.exact(&["p", "n", "c", "s", "k", "f"])?;
    own_identity(&fields, identity)?;
    let sequence = fields.integer("s", 1, 8)? as u8;
    let kind = ProbeKind::parse(&fields.string("k")?)?;
    if kind == ProbeKind::ChildProbe && sequence != 1 {
        return Err(ParseError::Sequence);
    }
    Ok(Probe {
        sequence,
        kind,
        flags: fields.integer("f", 0, 31)? as u8,
    })
}

pub fn parse_ready(bytes: &[u8], identity: &ProtocolIdentity) -> Parsed<Ready> {
    let fields = fields(bytes, MAX_READY_UTF8)?;
    fields.exact(&["version", "nonce", "document", "sequence", "kind"])?;
    if fields.integer("version", 1, 1)? != 1
        || fields.string("kind")? != "ready"
        || fields.string("nonce")? != identity.nonce.as_str()
        || fields.string("document")? != identity.document.as_str()
    {
        return Err(ParseError::IdentityMismatch);
    }
    Ok(Ready {
        sequence: fields.integer("sequence", 1, 9_007_199_254_740_991)?,
    })
}

pub fn parse_binding(bytes: &[u8], identity: &ProtocolIdentity) -> Parsed<BindingMessage> {
    let fields = fields(bytes, MAX_RAW_UTF8)?;
    if fields.string("name")? != BINDING_NAME {
        return Err(ParseError::IdentityMismatch);
    }
    let context_id =
        ContextId::new(fields.integer("executionContextId", 1, i32::MAX as u64)? as u32)?;
    let payload = fields.string("payload")?;
    // Inspect only a discriminating key; both selected parsers enforce their own
    // complete exact shape. A malformed probe is never retried as ready.
    let payload_fields = self::fields(payload.as_bytes(), MAX_READY_UTF8)?;
    let payload = if payload_fields.has("p") {
        PagePayload::Probe(parse_probe(payload.as_bytes(), identity)?)
    } else {
        PagePayload::Ready(parse_ready(payload.as_bytes(), identity)?)
    };
    Ok(BindingMessage {
        context_id,
        payload,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuxData {
    pub frame_id: Option<Identity>,
    pub is_default: Option<bool>,
    pub default_type: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionContext {
    pub id: ContextId,
    pub unique_id: Option<Identity>,
    pub origin: Origin,
    pub name: ContextName,
    pub aux_data: Option<AuxData>,
}
// Refuse future field growth past the separate copied-record/storage budgets.
const _: () = {
    assert!(std::mem::size_of::<ExecutionContext>() <= 1024);
    assert!(std::mem::size_of::<Probe>() <= MAX_PROBE_UTF8);
    assert!(std::mem::size_of::<Ready>() <= MAX_READY_UTF8);
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootEligibility {
    EligibleMetadata,
    NonRoot,
    MissingOrIneligibleMetadata,
}
impl ExecutionContext {
    /// Necessary metadata only, not native navigation/session/sender authority.
    pub fn root_eligibility(&self, root: &Identity) -> RootEligibility {
        let Some(aux) = &self.aux_data else {
            return RootEligibility::MissingOrIneligibleMetadata;
        };
        if aux.frame_id.as_ref().is_some_and(|frame| frame != root) {
            return RootEligibility::NonRoot;
        }
        if self.unique_id.is_some()
            && self.origin.as_str() == OFFICIAL_ORIGIN
            && aux.frame_id.as_ref() == Some(root)
            && aux.is_default == Some(true)
            && aux.default_type == Some(true)
        {
            RootEligibility::EligibleMetadata
        } else {
            RootEligibility::MissingOrIneligibleMetadata
        }
    }
}

pub fn parse_context_created(bytes: &[u8]) -> Parsed<ExecutionContext> {
    let outer = fields(bytes, MAX_RAW_UTF8)?;
    let context = Fields::object(outer.required("context")?)?;
    let aux_data = context
        .optional("auxData")
        .map(|raw| -> Parsed<AuxData> {
            let aux = Fields::object(raw)?;
            Ok(AuxData {
                frame_id: aux.optional("frameId").map(identity_value).transpose()?,
                is_default: aux.optional("isDefault").map(bool_value).transpose()?,
                default_type: aux
                    .optional("type")
                    .map(|raw| -> Parsed<bool> {
                        let value = string_value(raw)?;
                        BoundedText::<128>::copy(&value, false)?;
                        Ok(value == "default")
                    })
                    .transpose()?,
            })
        })
        .transpose()?;
    Ok(ExecutionContext {
        id: ContextId::new(context.integer("id", 1, i32::MAX as u64)? as u32)?,
        unique_id: context
            .optional("uniqueId")
            .map(identity_value)
            .transpose()?,
        origin: Origin(BoundedText::copy(&context.string("origin")?, false)?),
        name: ContextName(BoundedText::copy(&context.string("name")?, false)?),
        aux_data,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestroyedContext {
    pub id: ContextId,
    pub unique_id: Option<Identity>,
}
pub fn parse_context_destroyed(bytes: &[u8]) -> Parsed<DestroyedContext> {
    let fields = fields(bytes, MAX_RAW_UTF8)?;
    Ok(DestroyedContext {
        id: ContextId::new(fields.integer("executionContextId", 1, i32::MAX as u64)? as u32)?,
        unique_id: fields
            .optional("executionContextUniqueId")
            .map(identity_value)
            .transpose()?,
    })
}
pub fn parse_contexts_cleared(bytes: &[u8]) -> Parsed<()> {
    fields(bytes, MAX_RAW_UTF8)?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameUrl {
    Main,
    Child,
    Blank,
    Other,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub id: Identity,
    pub parent_id: Option<Identity>,
    pub loader_id: Identity,
    pub url: FrameUrl,
    pub official_origin: bool,
    pub html_mime: bool,
}
impl Frame {
    pub fn matches_main_root(&self, root: &Identity) -> bool {
        self.id == *root
            && self.parent_id.is_none()
            && self.url == FrameUrl::Main
            && self.official_origin
            && self.html_mime
    }
}
fn frame_value(raw: &RawValue) -> Parsed<Frame> {
    let fields = Fields::object(raw)?;
    let url = fields.string("url")?;
    BoundedText::<2048>::copy(&url, false)?;
    let origin = fields.string("securityOrigin")?;
    BoundedText::<64>::copy(&origin, false)?;
    let mime = fields.string("mimeType")?;
    BoundedText::<128>::copy(&mime, false)?;
    Ok(Frame {
        id: identity_value(fields.required("id")?)?,
        parent_id: fields
            .optional("parentId")
            .map(identity_value)
            .transpose()?,
        loader_id: identity_value(fields.required("loaderId")?)?,
        url: match url.as_str() {
            MAIN_URL => FrameUrl::Main,
            CHILD_URL => FrameUrl::Child,
            "about:blank" => FrameUrl::Blank,
            _ => FrameUrl::Other,
        },
        official_origin: origin == OFFICIAL_ORIGIN,
        html_mime: mime == "text/html",
    })
}
/// Shared exact engine Frame projection for Page.documentOpened/frameNavigated.
pub fn parse_frame(bytes: &[u8]) -> Parsed<Frame> {
    let fields = fields(bytes, MAX_RAW_UTF8)?;
    frame_value(fields.required("frame")?)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetachReason {
    Remove,
    Swap,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetachedFrame {
    pub id: Identity,
    pub reason: DetachReason,
}
pub fn parse_frame_detached(bytes: &[u8]) -> Parsed<DetachedFrame> {
    let fields = fields(bytes, MAX_RAW_UTF8)?;
    Ok(DetachedFrame {
        id: identity_value(fields.required("frameId")?)?,
        reason: match fields.string("reason")?.as_str() {
            "remove" => DetachReason::Remove,
            "swap" => DetachReason::Swap,
            _ => return Err(ParseError::Range),
        },
    })
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MethodResult {
    Ack,
    InitialRoot(Identity),
}
pub fn parse_method_result(method: Method, bytes: &[u8]) -> Parsed<MethodResult> {
    let fields = fields(bytes, MAX_RAW_UTF8)?;
    if method != Method::GetFrameTree {
        fields.exact(&[])?;
        return Ok(MethodResult::Ack);
    }
    let tree = Fields::object(fields.required("frameTree")?)?;
    let frame = frame_value(tree.required("frame")?)?;
    if frame.parent_id.is_some() || frame.url != FrameUrl::Blank {
        return Err(ParseError::Shape);
    }
    if let Some(children) = tree.optional("childFrames") {
        // The owned unloaded blank root precedes any child creation. Children
        // here cannot be silently ignored as an eligible initial tree.
        let children: Vec<&RawValue> =
            serde_json::from_str(children.get()).map_err(|_| ParseError::Type)?;
        if !children.is_empty() {
            return Err(ParseError::Shape);
        }
    }
    Ok(MethodResult::InitialRoot(frame.id))
}

/// Insert-only mapping: numeric IDs are never evicted/reused within an attempt.
pub struct ContextTable {
    slots: [Option<ExecutionContext>; MAX_CONTEXTS],
}
impl Default for ContextTable {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
    }
}
impl ContextTable {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&mut self, context: ExecutionContext) -> Parsed<usize> {
        if self.get(context.id).is_some() {
            return Err(ParseError::DuplicateContext);
        }
        let slot = self
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(ParseError::Capacity)?;
        self.slots[slot] = Some(context);
        Ok(slot)
    }
    pub fn get(&self, id: ContextId) -> Option<&ExecutionContext> {
        self.slots.iter().flatten().find(|context| context.id == id)
    }
}

/// At most ten early decoded messages total, including at most one ready.
/// Popping does not reset the total or ready latch for this attempt.
pub struct StagedMessages {
    slots: [Option<BindingMessage>; MAX_BINDING_MESSAGES],
    head: usize,
    tail: usize,
    ready_seen: bool,
}
impl Default for StagedMessages {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            head: 0,
            tail: 0,
            ready_seen: false,
        }
    }
}
impl StagedMessages {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, message: BindingMessage) -> Parsed<()> {
        let ready = matches!(&message.payload, PagePayload::Ready(_));
        if self.tail == MAX_BINDING_MESSAGES || (ready && self.ready_seen) {
            return Err(ParseError::Capacity);
        }
        self.slots[self.tail] = Some(message);
        self.tail += 1;
        self.ready_seen |= ready;
        Ok(())
    }
    pub fn pop_front(&mut self) -> Option<BindingMessage> {
        if self.head == self.tail {
            return None;
        }
        let message = self.slots[self.head].take();
        self.head += 1;
        message
    }
}

/// Two slots bound simultaneous retained native raw copies. RawJson itself does
/// not validate JSON; parsing does. Native pointer scans must stop at UTF16 cap.
pub struct RawJson {
    bytes: [u8; MAX_RAW_UTF8],
    len: usize,
}
impl RawJson {
    fn from_utf8(bytes: &[u8]) -> Parsed<Self> {
        if bytes.len() > MAX_RAW_UTF8 {
            return Err(ParseError::RawBound);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| ParseError::Encoding)?;
        if text.encode_utf16().count() > MAX_RAW_UTF16 {
            return Err(ParseError::RawBound);
        }
        let mut out = Self {
            bytes: [0; MAX_RAW_UTF8],
            len: bytes.len(),
        };
        out.bytes[..bytes.len()].copy_from_slice(bytes);
        Ok(out)
    }
    fn from_utf16(units: &[u16]) -> Parsed<Self> {
        if units.len() > MAX_RAW_UTF16 {
            return Err(ParseError::RawBound);
        }
        let mut out = Self {
            bytes: [0; MAX_RAW_UTF8],
            len: 0,
        };
        for decoded in std::char::decode_utf16(units.iter().copied()) {
            let ch = decoded.map_err(|_| ParseError::Encoding)?;
            let mut encoded = [0; 4];
            let encoded = ch.encode_utf8(&mut encoded).as_bytes();
            if out.len + encoded.len() > MAX_RAW_UTF8 {
                return Err(ParseError::RawBound);
            }
            out.bytes[out.len..out.len + encoded.len()].copy_from_slice(encoded);
            out.len += encoded.len();
        }
        Ok(out)
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}
#[derive(Default)]
pub struct RawSlots {
    slots: [Option<RawJson>; 2],
}
impl RawSlots {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn copy_utf8(&mut self, bytes: &[u8]) -> Parsed<usize> {
        let index = self
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(ParseError::Capacity)?;
        self.slots[index] = Some(RawJson::from_utf8(bytes)?);
        Ok(index)
    }
    pub fn copy_utf16(&mut self, units: &[u16]) -> Parsed<usize> {
        let index = self
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(ParseError::Capacity)?;
        self.slots[index] = Some(RawJson::from_utf16(units)?);
        Ok(index)
    }
    pub fn get(&self, index: usize) -> Option<&RawJson> {
        self.slots.get(index)?.as_ref()
    }
    pub fn release(&mut self, index: usize) -> bool {
        self.slots
            .get_mut(index)
            .is_some_and(|slot| slot.take().is_some())
    }
}

fn string_value(raw: &RawValue) -> Parsed<String> {
    serde_json::from_str(raw.get()).map_err(|_| ParseError::Type)
}
fn bool_value(raw: &RawValue) -> Parsed<bool> {
    serde_json::from_str(raw.get()).map_err(|_| ParseError::Type)
}
fn identity_value(raw: &RawValue) -> Parsed<Identity> {
    Identity::new(&string_value(raw)?)
}

/// Transient borrowed projection view. Unknown values are never materialized
/// into Value or copied into an owned result. Scanner bounded/validated first.
struct Fields<'a>(Vec<(String, &'a RawValue)>);
impl<'de> Deserialize<'de> for Fields<'de> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Fields<'de>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("bounded S4 object")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut fields = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    if fields.len() >= MAX_VALUES
                        || fields.iter().any(|(existing, _)| existing == &key)
                    {
                        return Err(serde::de::Error::custom("invalid S4 fields"));
                    }
                    fields.push((key, map.next_value::<&'de RawValue>()?));
                }
                Ok(Fields(fields))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}
impl<'a> Fields<'a> {
    fn object(raw: &'a RawValue) -> Parsed<Self> {
        serde_json::from_str(raw.get()).map_err(|_| ParseError::Type)
    }
    fn optional(&self, key: &str) -> Option<&'a RawValue> {
        self.0
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, raw)| *raw)
    }
    fn required(&self, key: &str) -> Parsed<&'a RawValue> {
        self.optional(key).ok_or(ParseError::Shape)
    }
    fn has(&self, key: &str) -> bool {
        self.optional(key).is_some()
    }
    fn exact(&self, keys: &[&str]) -> Parsed<()> {
        if self.0.len() != keys.len() || keys.iter().any(|key| !self.has(key)) {
            return Err(ParseError::Shape);
        }
        Ok(())
    }
    fn string(&self, key: &str) -> Parsed<String> {
        string_value(self.required(key)?)
    }
    fn integer(&self, key: &str, min: u64, max: u64) -> Parsed<u64> {
        let raw = self.required(key)?.get().trim();
        // Exact integer token: no bool, sign, fraction, exponent or float round-trip.
        if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ParseError::Type);
        }
        let value = raw.parse::<u64>().map_err(|_| ParseError::Range)?;
        if value < min || value > max {
            return Err(ParseError::Range);
        }
        Ok(value)
    }
}
fn fields(bytes: &[u8], limit: usize) -> Parsed<Fields<'_>> {
    if bytes.len() > limit || bytes.len() > MAX_RAW_UTF8 {
        return Err(ParseError::RawBound);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ParseError::Encoding)?;
    if text.encode_utf16().count() > MAX_RAW_UTF16 {
        return Err(ParseError::RawBound);
    }
    Scanner::validate(text)?;
    serde_json::from_str(text).map_err(|_| ParseError::Shape)
}

/// Complete JSON structural validation before projection, including ignored
/// metadata. Root value has depth one; keys do not count as JSON values.
struct Scanner<'a> {
    bytes: &'a [u8],
    position: usize,
    values: usize,
}
impl<'a> Scanner<'a> {
    fn validate(text: &'a str) -> Parsed<()> {
        let mut scanner = Self {
            bytes: text.as_bytes(),
            position: 0,
            values: 0,
        };
        scanner.value(1)?;
        scanner.whitespace();
        if scanner.position != scanner.bytes.len() {
            return Err(ParseError::Structure);
        }
        Ok(())
    }
    fn whitespace(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.position += 1;
        }
    }
    fn eat(&mut self, byte: u8) -> bool {
        self.whitespace();
        if self.bytes.get(self.position) == Some(&byte) {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn require(&mut self, byte: u8) -> Parsed<()> {
        if self.eat(byte) {
            Ok(())
        } else {
            Err(ParseError::Structure)
        }
    }
    fn string(&mut self) -> Parsed<String> {
        self.whitespace();
        let start = self.position;
        if self.bytes.get(start) != Some(&b'"') {
            return Err(ParseError::Structure);
        }
        self.position += 1;
        while let Some(byte) = self.bytes.get(self.position).copied() {
            self.position += 1;
            match byte {
                b'"' => {
                    return serde_json::from_slice(&self.bytes[start..self.position])
                        .map_err(|_| ParseError::Structure);
                }
                b'\\' => {
                    if self.position == self.bytes.len() {
                        return Err(ParseError::Structure);
                    }
                    self.position += 1;
                }
                _ => {}
            }
        }
        Err(ParseError::Structure)
    }
    fn literal(&mut self, literal: &[u8]) -> Parsed<()> {
        if self.bytes.get(self.position..self.position + literal.len()) != Some(literal) {
            return Err(ParseError::Structure);
        }
        self.position += literal.len();
        Ok(())
    }
    fn digits(&mut self) -> Parsed<()> {
        let start = self.position;
        while self
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_digit)
        {
            self.position += 1;
        }
        if self.position == start {
            Err(ParseError::Structure)
        } else {
            Ok(())
        }
    }
    fn number(&mut self) -> Parsed<()> {
        if self.bytes.get(self.position) == Some(&b'-') {
            self.position += 1;
        }
        match self.bytes.get(self.position) {
            Some(b'0') => self.position += 1,
            Some(b'1'..=b'9') => self.digits()?,
            _ => return Err(ParseError::Structure),
        }
        if self.bytes.get(self.position) == Some(&b'.') {
            self.position += 1;
            self.digits()?;
        }
        if matches!(self.bytes.get(self.position), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.bytes.get(self.position), Some(b'+' | b'-')) {
                self.position += 1;
            }
            self.digits()?;
        }
        Ok(())
    }
    fn value(&mut self, depth: usize) -> Parsed<()> {
        if depth > MAX_DEPTH {
            return Err(ParseError::DepthBound);
        }
        self.values += 1;
        if self.values > MAX_VALUES {
            return Err(ParseError::ValueBound);
        }
        self.whitespace();
        match self.bytes.get(self.position).copied() {
            Some(b'{') => {
                self.position += 1;
                let mut keys = BTreeSet::new();
                if self.eat(b'}') {
                    return Ok(());
                }
                loop {
                    let key = self.string()?;
                    if !keys.insert(key) {
                        return Err(ParseError::DuplicateKey);
                    }
                    self.require(b':')?;
                    self.value(depth + 1)?;
                    if self.eat(b'}') {
                        break;
                    }
                    self.require(b',')?;
                }
            }
            Some(b'[') => {
                self.position += 1;
                if self.eat(b']') {
                    return Ok(());
                }
                loop {
                    self.value(depth + 1)?;
                    if self.eat(b']') {
                        break;
                    }
                    self.require(b',')?;
                }
            }
            Some(b'"') => {
                self.string()?;
            }
            Some(b't') => self.literal(b"true")?,
            Some(b'f') => self.literal(b"false")?,
            Some(b'n') => self.literal(b"null")?,
            Some(b'-' | b'0'..=b'9') => self.number()?,
            _ => return Err(ParseError::Structure),
        }
        Ok(())
    }
}
