use super::code0_capture::Code0WebsiteCapture;

/// Supplied by the owned native callback, never decoded from page JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code0BridgeContext {
    pub connection_epoch: String,
    pub attempt_id: String,
    pub source_url: String,
    pub is_main_frame: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0BridgeReason {
    AuthenticationChanged,
    InvalidResponse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0BridgeRejection {
    ValidationRequired,
    InvalidContext,
    InvalidPayload,
    InvalidSequence,
    InvalidState,
    Disposed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Code0BridgeEvent {
    Control,
    Capture(Code0WebsiteCapture),
    Invalidated(Code0BridgeReason),
    Failure(Code0BridgeReason),
    Rejected(Code0BridgeRejection),
}

/// A single native-owned attempt. Any rejection revokes this instance; the
/// driver must pause/close rather than continue after a protocol violation.
pub struct Code0BridgeValidator {
    identity: Option<Identity>,
    ready: bool,
    disposed: bool,
    last_sequence: u64,
    last_request: u64,
    revision: u64,
    refresh_requests: std::collections::BTreeSet<u64>,
    pending_self: Option<PendingSelf>,
}

struct Identity {
    connection_epoch: String,
    attempt_id: String,
    nonce: String,
    document_id: String,
}

struct PendingSelf {
    request: u64,
    revision: u64,
    eligible: bool,
}

impl Code0BridgeValidator {
    pub fn new(connection_epoch: &str, attempt_id: &str, nonce: &str, document_id: &str) -> Self {
        let identity = (|| {
            Some(Identity {
                connection_epoch: canonical_uuid(connection_epoch)?,
                attempt_id: canonical_uuid(attempt_id)?,
                nonce: canonical_uuid(nonce)?,
                document_id: canonical_uuid(document_id)?,
            })
        })();
        Self {
            identity,
            ready: false,
            disposed: false,
            last_sequence: 0,
            last_request: 0,
            revision: 0,
            refresh_requests: Default::default(),
            pending_self: None,
        }
    }

    pub fn receive(&mut self, payload: &[u8], context: &Code0BridgeContext) -> Code0BridgeEvent {
        if self.disposed {
            return Code0BridgeEvent::Rejected(Code0BridgeRejection::Disposed);
        }
        match self.accept(payload, context) {
            Ok(event) => event,
            Err(reason) => {
                self.dispose();
                Code0BridgeEvent::Rejected(reason)
            }
        }
    }

    pub fn dispose(&mut self) {
        self.disposed = true;
        self.pending_self = None;
        self.refresh_requests.clear();
    }

    fn accept(
        &mut self,
        payload: &[u8],
        context: &Code0BridgeContext,
    ) -> Result<Code0BridgeEvent, Code0BridgeRejection> {
        use Code0BridgeRejection::{
            InvalidContext, InvalidPayload, InvalidSequence, InvalidState, ValidationRequired,
        };
        let identity = self.identity.as_ref().ok_or(ValidationRequired)?;
        if canonical_uuid(&context.connection_epoch).as_ref() != Some(&identity.connection_epoch)
            || canonical_uuid(&context.attempt_id).as_ref() != Some(&identity.attempt_id)
            || !context.is_main_frame
            || !official_source(&context.source_url)
        {
            return Err(InvalidContext);
        }
        if payload.len() > 2048 {
            return Err(InvalidPayload);
        }
        let fields: Fields = serde_json::from_slice(payload).map_err(|_| InvalidPayload)?;
        if fields.counter("version", true) != Some(1)
            || fields
                .string("nonce")
                .and_then(|value| canonical_uuid(&value))
                .as_ref()
                != Some(&identity.nonce)
            || fields
                .string("document")
                .and_then(|value| canonical_uuid(&value))
                .as_ref()
                != Some(&identity.document_id)
        {
            return Err(InvalidPayload);
        }
        let sequence = fields.counter("sequence", true).ok_or(InvalidPayload)?;
        if sequence <= self.last_sequence {
            return Err(InvalidSequence);
        }
        let kind = fields.string("kind").ok_or(InvalidPayload)?;
        let extra: &[&str] = match kind.as_str() {
            "ready" => &[],
            "refreshStart" | "refreshEnd" | "selfStart" => &["request", "revision"],
            "capture" => &["request", "revision", "quota", "usedQuota", "divisor"],
            "invalidate" | "failure" => &["reason"],
            _ => return Err(InvalidPayload),
        };
        if !fields.exact_keys(extra) {
            return Err(InvalidPayload);
        }
        if kind == "ready" {
            if self.ready {
                return Err(InvalidState);
            }
            self.ready = true;
            self.last_sequence = sequence;
            return Ok(Code0BridgeEvent::Control);
        }
        if !self.ready {
            return Err(InvalidState);
        }
        if kind == "invalidate" || kind == "failure" {
            let reason = fields.string("reason").ok_or(InvalidPayload)?;
            let event = match (kind.as_str(), reason.as_str()) {
                ("invalidate", "authenticationChanged") => {
                    Code0BridgeEvent::Invalidated(Code0BridgeReason::AuthenticationChanged)
                }
                ("failure", "invalidResponse") => {
                    Code0BridgeEvent::Failure(Code0BridgeReason::InvalidResponse)
                }
                _ => return Err(InvalidPayload),
            };
            self.dispose();
            return Ok(event);
        }
        let request = fields.counter("request", true).ok_or(InvalidPayload)?;
        let message_revision = fields.counter("revision", false).ok_or(InvalidPayload)?;
        match kind.as_str() {
            "refreshStart" => {
                if request <= self.last_request
                    || self.revision >= 9_007_199_254_740_991
                    || message_revision != self.revision + 1
                {
                    return Err(InvalidState);
                }
                self.revision = message_revision;
                self.last_request = request;
                self.refresh_requests.insert(request);
                self.pending_self = None;
            }
            "refreshEnd" => {
                if message_revision != self.revision || !self.refresh_requests.remove(&request) {
                    return Err(InvalidState);
                }
                self.pending_self = None;
            }
            "selfStart" => {
                if request <= self.last_request || message_revision != self.revision {
                    return Err(InvalidState);
                }
                self.last_request = request;
                self.pending_self = Some(PendingSelf {
                    request,
                    revision: self.revision,
                    eligible: self.refresh_requests.is_empty(),
                });
            }
            "capture" => {
                let pending = self.pending_self.as_ref().ok_or(InvalidState)?;
                if !pending.eligible
                    || !self.refresh_requests.is_empty()
                    || pending.request != request
                    || pending.revision != self.revision
                    || message_revision != self.revision
                {
                    return Err(InvalidState);
                }
                // RawValue tokens retain decimal evidence; no Value/float round trip.
                let filtered = format!(
                    "{{\"quota\":{},\"usedQuota\":{},\"divisor\":{}}}",
                    fields.raw("quota")?,
                    fields.raw("usedQuota")?,
                    fields.raw("divisor")?
                );
                let capture = Code0WebsiteCapture::decode_filtered_json(filtered.as_bytes())
                    .map_err(|_| InvalidPayload)?;
                self.dispose();
                return Ok(Code0BridgeEvent::Capture(capture));
            }
            _ => return Err(InvalidPayload),
        }
        self.last_sequence = sequence;
        Ok(Code0BridgeEvent::Control)
    }
}

fn canonical_uuid(value: &str) -> Option<String> {
    if value.len() != 36
        || !value.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
    {
        return None;
    }
    Some(value.to_ascii_lowercase())
}

fn official_source(source: &str) -> bool {
    if source
        .bytes()
        .any(|byte| byte <= 32 || byte == 127 || byte == b'\\')
    {
        return false;
    }
    let Some((_, tail)) = source.split_once("://") else {
        return false;
    };
    let authority = tail.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    let Ok(url) = reqwest::Url::parse(source) else {
        return false;
    };
    url.scheme() == "https"
        && url.host_str() == Some("console.code0.ai")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}

struct Fields(std::collections::BTreeMap<String, Box<serde_json::value::RawValue>>);

impl Fields {
    fn raw(&self, key: &str) -> Result<&str, Code0BridgeRejection> {
        self.0
            .get(key)
            .map(|value| value.get())
            .ok_or(Code0BridgeRejection::InvalidPayload)
    }
    fn string(&self, key: &str) -> Option<String> {
        serde_json::from_str(self.raw(key).ok()?).ok()
    }
    fn counter(&self, key: &str, positive: bool) -> Option<u64> {
        let raw = self.raw(key).ok()?.trim();
        if raw.starts_with('-') {
            return None;
        }
        let filtered = format!("{{\"quota\":{raw},\"usedQuota\":0,\"divisor\":1}}");
        let value = Code0WebsiteCapture::decode_filtered_json(filtered.as_bytes())
            .ok()?
            .balance;
        if positive && value == 0.0 {
            return None;
        }
        Some(value as u64)
    }
    fn exact_keys(&self, extra: &[&str]) -> bool {
        let base = ["version", "nonce", "document", "sequence", "kind"];
        self.0.len() == base.len() + extra.len()
            && base
                .iter()
                .chain(extra)
                .all(|key| self.0.contains_key(*key))
    }
}

impl<'de> serde::Deserialize<'de> for Fields {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldVisitor;
        impl<'de> serde::de::Visitor<'de> for FieldVisitor {
            type Value = Fields;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a fixed Code0 bridge envelope")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Fields, M::Error> {
                let allowed = [
                    "version",
                    "nonce",
                    "document",
                    "sequence",
                    "kind",
                    "request",
                    "revision",
                    "quota",
                    "usedQuota",
                    "divisor",
                    "reason",
                ];
                let mut fields = std::collections::BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !allowed.contains(&key.as_str())
                        || fields.contains_key(&key)
                        || fields.len() >= 10
                    {
                        return Err(serde::de::Error::custom("invalid Code0 bridge fields"));
                    }
                    fields.insert(key, map.next_value::<Box<serde_json::value::RawValue>>()?);
                }
                Ok(Fields(fields))
            }
        }
        deserializer.deserialize_map(FieldVisitor)
    }
}

/// The canonical expression is embedded unchanged; no network or runtime here.
pub const CODE0_BRIDGE_SOURCE: &str = include_str!("../../../../web/code0-bridge.js");
