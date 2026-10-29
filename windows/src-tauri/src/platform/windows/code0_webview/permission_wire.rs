//! Pure fix2 permission wire profiles. This module never touches native pointers,
//! threads, clocks, emitters or Protocol state and grants no action authority.
//!
//! Caller first owns strict native UTF16 capture (required NUL within 2048 units,
//! origin within 256), actual Args2/non-null empty SessionId, emitter/core/request/
//! installation/UI identity, Source queries, native HRESULTs and all postDrop
//! rechecks. It also owns the concurrent 4096-byte text inventory, including
//! capture, this request allocation/conversion and reentrant native temporaries.
//! Per-buffer caps here do not implement that inventory or attest provenance.
//! On an event decoding error the caller must still count/fail that delivery via
//! Lifecycle::Unsupported(receiver); it must not omit it. A completion error maps
//! to Malformed for its real same-ticket disposal, never an absent invocation.

use super::permission_protocol::{
    Command, CompletionValue, Context, Document, FIXED_URL, Frame, Id, Lifecycle, Receiver,
};
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WireError;
type WireResult<T> = Result<T, WireError>;

const ORIGIN: &str = "https://console.code0.ai";
const EXPRESSION: &str = r#"(async()=>{"use strict";const d=document;if(top!==self||location.href!=="https://console.code0.ai/__ai_meter_fixture__/permission-notifications-v1"||location.origin!=="https://console.code0.ai"||!isSecureContext||d.readyState!=="complete"||!d.documentElement||!d.head||!d.body||d.querySelector("script,iframe,frame,object,embed,link,img,input")!==null||typeof Notification!=="function"||typeof Notification.requestPermission!=="function"||Notification.permission!=="default"||!navigator.userActivation||navigator.userActivation.isActive!==false){return"code0-permission-precondition-v1";}const result=await Notification.requestPermission();if(d!==document||top!==self||location.href!=="https://console.code0.ai/__ai_meter_fixture__/permission-notifications-v1"){return"code0-permission-document-changed-v1";}return result==="denied"?"code0-permission-denied-v1":result==="default"?"code0-permission-default-v1":result==="granted"?"code0-permission-granted-v1":"code0-permission-invalid-v1";})()"#;
const REQUEST_CAP: usize = 2047;

/// Owned serialized text only, never evidence that a native command was issued.
/// Native caller accounts this allocation and UTF16 conversion simultaneously.
pub(super) struct CommandRequest {
    method: &'static str,
    parameters: Vec<u8>,
}
impl std::fmt::Debug for CommandRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CommandRequest(<redacted>)")
    }
}
impl CommandRequest {
    pub(super) fn method(&self) -> &'static str {
        self.method
    }
    pub(super) fn parameters(&self) -> &[u8] {
        &self.parameters
    }
}

/// Four closed profiles; only Evaluate accepts the bounded selected U. This
/// serialization check does not replace Protocol's final Source/action gate.
pub(super) fn command_request(command: Command, unique: Option<Id>) -> WireResult<CommandRequest> {
    struct Bounded(Vec<u8>);
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0.len().saturating_add(bytes.len()) > REQUEST_CAP
                || !bytes.is_ascii()
                || bytes.contains(&0)
            {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Evaluate<'a> {
        expression: &'static str,
        unique_context_id: &'a str,
        user_gesture: bool,
        await_promise: bool,
        return_by_value: bool,
        #[serde(rename = "includeCommandLineAPI")]
        include_command_line_api: bool,
        #[serde(rename = "allowUnsafeEvalBlockedByCSP")]
        allow_unsafe_eval_blocked_by_csp: bool,
    }
    let mut output = Bounded(Vec::with_capacity(REQUEST_CAP));
    match (command, unique) {
        (Command::Evaluate, Some(unique)) => {
            serde_json::to_writer(
                &mut output,
                &Evaluate {
                    expression: EXPRESSION,
                    unique_context_id: unique.as_str(),
                    user_gesture: false,
                    await_promise: true,
                    return_by_value: true,
                    include_command_line_api: false,
                    allow_unsafe_eval_blocked_by_csp: false,
                },
            )
            .map_err(|_| WireError)?;
        }
        (Command::PageEnable | Command::RuntimeEnable | Command::GetFrameTree, None) => {
            output.write_all(b"{}").map_err(|_| WireError)?;
        }
        _ => return Err(WireError),
    }
    Ok(CommandRequest {
        method: command.name(),
        parameters: output.0,
    })
}

#[derive(Clone, Copy)]
enum CompletionKind {
    EmptyAck,
    FrameTree,
    Denied,
    Precondition,
    Default,
    Granted,
    DocumentChanged,
    Invalid,
}

/// Own the parsed Frame separately; the semantic helper borrows it only at its
/// immediate capture call. No large payload enum, raw JSON or pointer is kept.
pub(super) struct DecodedCompletion {
    kind: CompletionKind,
    frame: Option<Frame>,
}
impl std::fmt::Debug for DecodedCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DecodedCompletion(<redacted>)")
    }
}
impl DecodedCompletion {
    fn unit(kind: CompletionKind) -> Self {
        Self { kind, frame: None }
    }
    pub(super) fn as_semantic(&self) -> CompletionValue<'_> {
        match self.kind {
            CompletionKind::EmptyAck => CompletionValue::EmptyAck,
            CompletionKind::FrameTree => CompletionValue::FrameTree(
                self.frame
                    .as_ref()
                    .expect("private FrameTree construction always owns its frame"),
            ),
            CompletionKind::Denied => CompletionValue::Denied,
            CompletionKind::Precondition => CompletionValue::Precondition,
            CompletionKind::Default => CompletionValue::Default,
            CompletionKind::Granted => CompletionValue::Granted,
            CompletionKind::DocumentChanged => CompletionValue::DocumentChanged,
            CompletionKind::Invalid => CompletionValue::Invalid,
        }
    }
}

// Stack spans only: no serde Value tree, decoded String or heap token inventory.
// Root starts at depth0, matching the existing strict S3 parser's depth8 rule.
#[derive(Clone, Copy, Default)]
struct Token {
    start: usize,
    end: usize,
    next: usize,
    kind: u8,
}
struct Doc<'a> {
    raw: &'a str,
    tokens: [Token; 256],
    count: usize,
    pos: usize,
}
struct Decoded<'a> {
    chars: std::str::Chars<'a>,
}
impl Decoded<'_> {
    fn hex(&mut self) -> WireResult<u16> {
        let mut n = 0u16;
        for _ in 0..4 {
            let digit = self
                .chars
                .next()
                .and_then(|c| c.to_digit(16))
                .ok_or(WireError)?;
            n = n.checked_mul(16).ok_or(WireError)? + digit as u16;
        }
        Ok(n)
    }
    fn escaped(&mut self) -> WireResult<char> {
        Ok(match self.chars.next().ok_or(WireError)? {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => {
                let first = self.hex()?;
                let scalar = if (0xd800..=0xdbff).contains(&first) {
                    if self.chars.next() != Some('\\') || self.chars.next() != Some('u') {
                        return Err(WireError);
                    }
                    let second = self.hex()?;
                    if !(0xdc00..=0xdfff).contains(&second) {
                        return Err(WireError);
                    }
                    0x10000 + ((u32::from(first) - 0xd800) << 10) + u32::from(second) - 0xdc00
                } else {
                    u32::from(first)
                };
                char::from_u32(scalar).ok_or(WireError)?
            }
            _ => return Err(WireError),
        })
    }
}
impl Iterator for Decoded<'_> {
    type Item = WireResult<char>;
    fn next(&mut self) -> Option<Self::Item> {
        let c = self.chars.next()?;
        let value = if c == '\\' {
            self.escaped()
        } else if c < ' ' {
            Err(WireError)
        } else {
            Ok(c)
        };
        Some(value.and_then(|c| if c == '\0' { Err(WireError) } else { Ok(c) }))
    }
}
impl<'a> Doc<'a> {
    fn new(bytes: &'a [u8], cap: usize) -> WireResult<Self> {
        if bytes.len() > cap || bytes.contains(&0) {
            return Err(WireError);
        }
        let raw = std::str::from_utf8(bytes).map_err(|_| WireError)?;
        let mut doc = Self {
            raw,
            tokens: [Token::default(); 256],
            count: 0,
            pos: 0,
        };
        doc.scan(0)?;
        doc.ws();
        if doc.pos != raw.len() {
            return Err(WireError);
        }
        Ok(doc)
    }
    fn ws(&mut self) {
        while self
            .raw
            .as_bytes()
            .get(self.pos)
            .is_some_and(|b| b" \n\r\t".contains(b))
        {
            self.pos += 1;
        }
    }
    fn scan(&mut self, depth: usize) -> WireResult<usize> {
        self.ws();
        if depth > 8 || self.count == self.tokens.len() {
            return Err(WireError);
        }
        let n = self.count;
        self.count += 1;
        let start = self.pos;
        let kind = *self.raw.as_bytes().get(self.pos).ok_or(WireError)?;
        self.pos += 1;
        match kind {
            b'{' | b'[' => {
                self.ws();
                let close = if kind == b'{' { b'}' } else { b']' };
                if self.raw.as_bytes().get(self.pos) != Some(&close) {
                    loop {
                        if kind == b'{' {
                            self.ws();
                            if self.raw.as_bytes().get(self.pos) != Some(&b'"') {
                                return Err(WireError);
                            }
                            let key = self.scan(depth + 1)?;
                            // Every object checks decoded duplicate keys, before any profile
                            // interpretation; escaped aliases cannot silently overwrite a value.
                            let mut prior = n + 1;
                            while prior < key {
                                if self.decoded(prior)?.eq(self.decoded(key)?) {
                                    return Err(WireError);
                                }
                                prior = self.tokens[prior + 1].next;
                            }
                            self.ws();
                            if self.raw.as_bytes().get(self.pos) != Some(&b':') {
                                return Err(WireError);
                            }
                            self.pos += 1;
                        }
                        self.scan(depth + 1)?;
                        self.ws();
                        if self.raw.as_bytes().get(self.pos) == Some(&close) {
                            break;
                        }
                        if self.raw.as_bytes().get(self.pos) != Some(&b',') {
                            return Err(WireError);
                        }
                        self.pos += 1;
                    }
                }
                self.pos += 1;
            }
            b'"' => {
                loop {
                    let b = *self.raw.as_bytes().get(self.pos).ok_or(WireError)?;
                    self.pos += 1;
                    if b == b'"' {
                        break;
                    }
                    if b == b'\\' {
                        self.pos += 1;
                        if self.pos > self.raw.len() {
                            return Err(WireError);
                        }
                    }
                }
                let inner = self.raw.get(start + 1..self.pos - 1).ok_or(WireError)?;
                for c in (Decoded {
                    chars: inner.chars(),
                }) {
                    c?;
                }
            }
            b't' | b'f' | b'n' => {
                let literal = match kind {
                    b't' => "true",
                    b'f' => "false",
                    _ => "null",
                };
                self.pos = start + literal.len();
                if self.raw.get(start..self.pos) != Some(literal) {
                    return Err(WireError);
                }
            }
            b'0'..=b'9' => {
                while self
                    .raw
                    .as_bytes()
                    .get(self.pos)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.pos += 1;
                }
                if kind == b'0' && self.pos > start + 1 {
                    return Err(WireError);
                }
            }
            _ => return Err(WireError),
        }
        self.tokens[n] = Token {
            start,
            end: self.pos,
            next: self.count,
            kind,
        };
        Ok(n)
    }
    fn decoded(&self, n: usize) -> WireResult<Decoded<'a>> {
        let t = self.tokens[n];
        if t.kind != b'"' {
            return Err(WireError);
        }
        Ok(Decoded {
            chars: self.raw[t.start + 1..t.end - 1].chars(),
        })
    }
    fn eq(&self, n: usize, value: &str) -> bool {
        self.decoded(n)
            .is_ok_and(|decoded| decoded.eq(value.chars().map(Ok)))
    }
    fn fixed(&self, n: usize, value: &str) -> WireResult<()> {
        if self.eq(n, value) {
            Ok(())
        } else {
            Err(WireError)
        }
    }
    // Awaited fixed values require their literal wire spelling, not merely
    // equivalent decoded text. Object keys continue to compare by decoded value.
    fn literal(&self, n: usize, value: &str) -> WireResult<()> {
        let t = self.tokens[n];
        if t.kind == b'"' && &self.raw[t.start + 1..t.end - 1] == value {
            Ok(())
        } else {
            Err(WireError)
        }
    }
    fn string(&self, n: usize, max: usize) -> WireResult<()> {
        let mut len = 0;
        for c in self.decoded(n)? {
            len += c?.len_utf8();
            if len > max {
                return Err(WireError);
            }
        }
        Ok(())
    }
    fn id(&self, n: usize) -> WireResult<Id> {
        let mut bytes = [0; 128];
        let mut len = 0;
        for c in self.decoded(n)? {
            let c = c?;
            if !c.is_ascii() || len == bytes.len() {
                return Err(WireError);
            }
            bytes[len] = c as u8;
            len += 1;
        }
        Id::parse(std::str::from_utf8(&bytes[..len]).map_err(|_| WireError)?).map_err(|_| WireError)
    }
    fn cid(&self, n: usize) -> WireResult<u32> {
        let t = self.tokens[n];
        if !t.kind.is_ascii_digit() {
            return Err(WireError);
        }
        let cid = self.raw[t.start..t.end]
            .parse::<u32>()
            .map_err(|_| WireError)?;
        if (1..=2_147_483_647).contains(&cid) {
            Ok(cid)
        } else {
            Err(WireError)
        }
    }
    fn boolean(&self, n: usize, expected: bool) -> WireResult<()> {
        let t = self.tokens[n];
        let literal = if expected { "true" } else { "false" };
        if &self.raw[t.start..t.end] == literal {
            Ok(())
        } else {
            Err(WireError)
        }
    }
    fn fields<const N: usize>(
        &self,
        n: usize,
        keys: [&str; N],
        required: u32,
    ) -> WireResult<[Option<usize>; N]> {
        if self.tokens[n].kind != b'{' {
            return Err(WireError);
        }
        let mut out = [None; N];
        let mut at = n + 1;
        let mut seen = 0u32;
        while at < self.tokens[n].next {
            let index = keys
                .iter()
                .position(|key| self.eq(at, key))
                .ok_or(WireError)?;
            if seen & (1 << index) != 0 {
                return Err(WireError);
            }
            seen |= 1 << index;
            let value = at + 1;
            out[index] = Some(value);
            at = self.tokens[value].next;
        }
        if seen & required != required {
            return Err(WireError);
        }
        Ok(out)
    }
    fn empty_array(&self, n: usize) -> WireResult<()> {
        if self.tokens[n].kind == b'[' && self.tokens[n].next == n + 1 {
            Ok(())
        } else {
            Err(WireError)
        }
    }
    fn empty(&self, n: usize) -> WireResult<()> {
        self.fields(n, [], 0).map(|_| ())
    }
}

fn decode_frame(doc: &Doc<'_>, n: usize) -> WireResult<(Document, Frame)> {
    let f = doc.fields(
        n,
        [
            "id",
            "loaderId",
            "url",
            "domainAndRegistry",
            "securityOrigin",
            "mimeType",
            "secureContextType",
            "crossOriginIsolatedContextType",
            "gatedAPIFeatures",
            "name",
            "urlFragment",
            "unreachableUrl",
            "securityOriginDetails",
            "adFrameStatus",
        ],
        511,
    )?;
    let required = |i: usize| f[i].ok_or(WireError);
    let document = if doc.eq(required(2)?, FIXED_URL) {
        Document::Selected
    } else if doc.eq(required(2)?, "about:blank") {
        Document::Blank
    } else {
        return Err(WireError);
    };
    let selected = document == Document::Selected;
    doc.fixed(required(3)?, if selected { "code0.ai" } else { "" })?;
    doc.fixed(required(4)?, if selected { ORIGIN } else { "://" })?;
    doc.fixed(required(5)?, "text/html")?;
    doc.fixed(
        required(6)?,
        if selected { "Secure" } else { "InsecureScheme" },
    )?;
    doc.fixed(required(7)?, "NotIsolated")?;
    doc.empty_array(required(8)?)?;
    for value in f[9..12].iter().flatten() {
        doc.fixed(*value, "")?;
    }
    if let Some(n) = f[12] {
        let fields = doc.fields(n, ["isLocalhost"], 1)?;
        doc.boolean(fields[0].ok_or(WireError)?, false)?;
    }
    if let Some(n) = f[13] {
        let fields = doc.fields(n, ["adFrameType", "explanations"], 1)?;
        doc.fixed(fields[0].ok_or(WireError)?, "none")?;
        if let Some(n) = fields[1] {
            doc.empty_array(n)?;
        }
    }
    Ok((
        document,
        Frame {
            id: doc.id(required(0)?)?,
            loader: doc.id(required(1)?)?,
        },
    ))
}
fn decode_context(doc: &Doc<'_>, n: usize) -> WireResult<(Document, Context)> {
    let fields = doc.fields(n, ["id", "origin", "name", "uniqueId", "auxData"], 31)?;
    let required = |i: usize| fields[i].ok_or(WireError);
    let document = if doc.eq(required(1)?, ORIGIN) {
        Document::Selected
    } else if doc.eq(required(1)?, "://") {
        Document::Blank
    } else {
        return Err(WireError);
    };
    doc.fixed(required(2)?, "")?;
    let aux = doc.fields(required(4)?, ["isDefault", "frameId", "type"], 7)?;
    doc.boolean(aux[0].ok_or(WireError)?, true)?;
    doc.fixed(aux[2].ok_or(WireError)?, "default")?;
    let context = Context::new(
        doc.id(aux[1].ok_or(WireError)?)?,
        doc.cid(required(0)?)?,
        doc.id(required(3)?)?,
    )
    .map_err(|_| WireError)?;
    Ok((document, context))
}

/// Decode only bytes actually captured by the future native callback. Shape
/// acceptance cannot attest Args2/session/emitter identity or lifecycle phase.
pub(super) fn decode_lifecycle(receiver: Receiver, bytes: &[u8]) -> WireResult<Lifecycle> {
    let cap = match receiver {
        Receiver::ContextCreated => 1024,
        Receiver::ContextDestroyed | Receiver::StartedLoading | Receiver::StoppedLoading => 256,
        Receiver::ContextsCleared => 64,
        Receiver::FrameAttached | Receiver::FrameDetached => 512,
        Receiver::FrameNavigated | Receiver::DocumentOpened | Receiver::WithinDocument => 2048,
    };
    let doc = Doc::new(bytes, cap)?;
    Ok(match receiver {
        Receiver::ContextCreated => {
            let f = doc.fields(0, ["context"], 1)?;
            let (document, context) = decode_context(&doc, f[0].ok_or(WireError)?)?;
            Lifecycle::ContextCreated(document, context)
        }
        Receiver::ContextDestroyed => {
            let f = doc.fields(0, ["executionContextId", "executionContextUniqueId"], 3)?;
            Lifecycle::ContextDestroyed {
                numeric: doc.cid(f[0].ok_or(WireError)?)?,
                unique: doc.id(f[1].ok_or(WireError)?)?,
            }
        }
        Receiver::ContextsCleared => {
            doc.empty(0)?;
            Lifecycle::ContextsCleared
        }
        Receiver::FrameNavigated => {
            let f = doc.fields(0, ["frame", "type"], 3)?;
            doc.fixed(f[1].ok_or(WireError)?, "Navigation")?;
            let (document, frame) = decode_frame(&doc, f[0].ok_or(WireError)?)?;
            Lifecycle::FrameNavigated(document, frame)
        }
        Receiver::StartedLoading | Receiver::StoppedLoading => {
            let f = doc.fields(0, ["frameId"], 1)?;
            let frame = doc.id(f[0].ok_or(WireError)?)?;
            if receiver == Receiver::StartedLoading {
                Lifecycle::StartedLoading(frame)
            } else {
                Lifecycle::StoppedLoading(frame)
            }
        }
        Receiver::FrameAttached => {
            let f = doc.fields(0, ["frameId", "parentFrameId"], 3)?;
            if doc.id(f[0].ok_or(WireError)?)? == doc.id(f[1].ok_or(WireError)?)? {
                return Err(WireError);
            }
            Lifecycle::Unsupported(receiver)
        }
        Receiver::FrameDetached => {
            let f = doc.fields(0, ["frameId", "reason"], 3)?;
            doc.id(f[0].ok_or(WireError)?)?;
            let reason = f[1].ok_or(WireError)?;
            if !doc.eq(reason, "remove") && !doc.eq(reason, "swap") {
                return Err(WireError);
            }
            Lifecycle::Unsupported(receiver)
        }
        Receiver::DocumentOpened => {
            let f = doc.fields(0, ["frame"], 1)?;
            decode_frame(&doc, f[0].ok_or(WireError)?)?;
            Lifecycle::Unsupported(receiver)
        }
        Receiver::WithinDocument => {
            let f = doc.fields(0, ["frameId", "url", "navigationType"], 7)?;
            doc.id(f[0].ok_or(WireError)?)?;
            doc.string(f[1].ok_or(WireError)?, 1024)?;
            let kind = f[2].ok_or(WireError)?;
            if !["fragment", "historyApi", "other"]
                .iter()
                .any(|s| doc.eq(kind, s))
            {
                return Err(WireError);
            }
            Lifecycle::Unsupported(receiver)
        }
    })
}

/// JSON semantics only. Pass the actual successful/failed HRESULT separately to
/// Protocol::completion_captured even for an otherwise correctly decoded value.
/// GetFrameTree F/L is not adopted here: Protocol joins it to the selected frame.
pub(super) fn decode_completion(command: Command, bytes: &[u8]) -> WireResult<DecodedCompletion> {
    let cap = match command {
        Command::PageEnable | Command::RuntimeEnable => 64,
        Command::GetFrameTree => 2048,
        Command::Evaluate => 512,
    };
    let doc = Doc::new(bytes, cap)?;
    match command {
        Command::PageEnable | Command::RuntimeEnable => {
            doc.empty(0)?;
            Ok(DecodedCompletion::unit(CompletionKind::EmptyAck))
        }
        Command::GetFrameTree => {
            let f = doc.fields(0, ["frameTree"], 1)?;
            let tree = doc.fields(f[0].ok_or(WireError)?, ["frame", "childFrames"], 1)?;
            if let Some(children) = tree[1] {
                doc.empty_array(children)?;
            }
            let (document, frame) = decode_frame(&doc, tree[0].ok_or(WireError)?)?;
            if document != Document::Selected {
                return Err(WireError);
            }
            Ok(DecodedCompletion {
                kind: CompletionKind::FrameTree,
                frame: Some(frame),
            })
        }
        Command::Evaluate => {
            let f = doc.fields(0, ["result"], 1)?;
            let result = doc.fields(f[0].ok_or(WireError)?, ["type", "value", "description"], 3)?;
            doc.literal(result[0].ok_or(WireError)?, "string")?;
            let value = result[1].ok_or(WireError)?;
            let (literal, kind) = [
                ("code0-permission-denied-v1", CompletionKind::Denied),
                (
                    "code0-permission-precondition-v1",
                    CompletionKind::Precondition,
                ),
                ("code0-permission-default-v1", CompletionKind::Default),
                ("code0-permission-granted-v1", CompletionKind::Granted),
                (
                    "code0-permission-document-changed-v1",
                    CompletionKind::DocumentChanged,
                ),
                ("code0-permission-invalid-v1", CompletionKind::Invalid),
            ]
            .into_iter()
            .find(|(literal, _)| doc.literal(value, literal).is_ok())
            .ok_or(WireError)?;
            if let Some(description) = result[2] {
                doc.literal(description, literal)?;
            }
            Ok(DecodedCompletion::unit(kind))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::super::permission_protocol::{
        Boundary, CoverageKey, Identity, NativeToken, Problem, Protocol,
    };
    use super::*;

    const SELECTED_FRAME: &str = r#"{"id":"root","loaderId":"selected-loader","url":"https://console.code0.ai/__ai_meter_fixture__/permission-notifications-v1","domainAndRegistry":"code0.ai","securityOrigin":"https://console.code0.ai","mimeType":"text/html","secureContextType":"Secure","crossOriginIsolatedContextType":"NotIsolated","gatedAPIFeatures":[]}"#;
    const BLANK_FRAME: &str = r#"{"id":"root","loaderId":"blank-loader","url":"about:blank","domainAndRegistry":"","securityOrigin":"://","mimeType":"text/html","secureContextType":"InsecureScheme","crossOriginIsolatedContextType":"NotIsolated","gatedAPIFeatures":[]}"#;
    const SELECTED_CONTEXT: &str = r#"{"context":{"id":7,"origin":"https://console.code0.ai","name":"","uniqueId":"selected-context","auxData":{"isDefault":true,"frameId":"root","type":"default"}}}"#;
    const BLANK_CONTEXT: &str = r#"{"context":{"id":7,"origin":"://","name":"","uniqueId":"blank-context","auxData":{"isDefault":true,"frameId":"root","type":"default"}}}"#;
    const DENIED: &[u8] = br#"{"result":{"type":"string","value":"code0-permission-denied-v1"}}"#;

    fn id(s: &str) -> Id {
        Id::parse(s).unwrap()
    }
    fn frame() -> Frame {
        Frame {
            id: id("root"),
            loader: id("selected-loader"),
        }
    }
    fn navigated(frame: &str) -> String {
        format!(r#"{{"frame":{frame},"type":"Navigation"}}"#)
    }
    fn tree(frame: &str) -> String {
        format!(r#"{{"frameTree":{{"frame":{frame}}}}}"#)
    }
    fn json(s: &str) -> serde_json::Value {
        serde_json::from_str(s).unwrap()
    }
    fn event(receiver: Receiver, value: &serde_json::Value) -> Result<Lifecycle, WireError> {
        decode_lifecycle(receiver, &serde_json::to_vec(value).unwrap())
    }

    // Mutation: wrong method, nonempty enable/tree params or extra caller-selected input.
    #[test]
    fn only_four_fixed_methods_and_their_closed_parameter_shapes_are_serialized() {
        for (command, method) in [
            (Command::PageEnable, "Page.enable"),
            (Command::RuntimeEnable, "Runtime.enable"),
            (Command::GetFrameTree, "Page.getFrameTree"),
        ] {
            let request = command_request(command, None).unwrap();
            assert_eq!(request.method(), method);
            assert_eq!(request.parameters(), b"{}");
            assert!(command_request(command, Some(id("u"))).is_err());
        }
        assert!(command_request(Command::Evaluate, None).is_err());
    }

    // Mutation: enabling gesture, using numeric contextId, changing the frozen program or flags.
    #[test]
    fn evaluate_serializes_only_the_frozen_expression_unique_context_and_exact_flags() {
        let request = command_request(Command::Evaluate, Some(id("u_.:-09"))).unwrap();
        assert_eq!(request.method(), "Runtime.evaluate");
        let value: serde_json::Value = serde_json::from_slice(request.parameters()).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "expression": "(async()=>{\"use strict\";const d=document;if(top!==self||location.href!==\"https://console.code0.ai/__ai_meter_fixture__/permission-notifications-v1\"||location.origin!==\"https://console.code0.ai\"||!isSecureContext||d.readyState!==\"complete\"||!d.documentElement||!d.head||!d.body||d.querySelector(\"script,iframe,frame,object,embed,link,img,input\")!==null||typeof Notification!==\"function\"||typeof Notification.requestPermission!==\"function\"||Notification.permission!==\"default\"||!navigator.userActivation||navigator.userActivation.isActive!==false){return\"code0-permission-precondition-v1\";}const result=await Notification.requestPermission();if(d!==document||top!==self||location.href!==\"https://console.code0.ai/__ai_meter_fixture__/permission-notifications-v1\"){return\"code0-permission-document-changed-v1\";}return result===\"denied\"?\"code0-permission-denied-v1\":result===\"default\"?\"code0-permission-default-v1\":result===\"granted\"?\"code0-permission-granted-v1\":\"code0-permission-invalid-v1\";})()",
                "uniqueContextId": "u_.:-09", "userGesture": false, "awaitPromise": true,
                "returnByValue": true, "includeCommandLineAPI": false,
                "allowUnsafeEvalBlockedByCSP": false
            })
        );
        assert!(request.parameters().is_ascii());
        assert!(!request.parameters().contains(&0));
        assert!(request.parameters().len() < 2048);
        let longest = command_request(Command::Evaluate, Some(id(&"u".repeat(128)))).unwrap();
        assert!(longest.parameters().len() < 2048);
    }

    // Mutation: nonempty/array/null ACK is treated as enable completion.
    #[test]
    fn enable_replies_require_an_exact_empty_object() {
        for command in [Command::PageEnable, Command::RuntimeEnable] {
            assert_eq!(
                decode_completion(command, b" \n{}\t")
                    .unwrap()
                    .as_semantic(),
                CompletionValue::EmptyAck
            );
            for bad in [b"[]".as_slice(), b"null", b"{\"result\":{}}", b"{\"x\":0}"] {
                assert!(decode_completion(command, bad).is_err());
            }
        }
    }

    // Mutation: HRESULT success is inferred from good wire text.
    #[test]
    fn decoded_good_ack_does_not_replace_the_independent_actual_hresult() {
        let identity = Identity {
            request: 1,
            generation: 2,
            installation: 3,
            core: 4,
        };
        let at = |ms| Boundary { identity, ms };
        for success in [false, true] {
            let mut protocol = Protocol::new(identity, 0).unwrap();
            for hook in 0..10 {
                protocol
                    .registration_added(CoverageKey::S2Hook(hook), NativeToken::Event(0), at(1))
                    .unwrap();
            }
            for receiver in Receiver::ALL {
                protocol
                    .registration_added(
                        CoverageKey::Receiver(receiver),
                        NativeToken::Event(0),
                        at(1),
                    )
                    .unwrap();
            }
            for key in [
                CoverageKey::NavigationCompleted,
                CoverageKey::ResourceRequested,
                CoverageKey::CoreMessage,
            ] {
                protocol
                    .registration_added(key, NativeToken::Event(0), at(1))
                    .unwrap();
            }
            protocol
                .registration_added(CoverageKey::AllSourceFilter, NativeToken::Filter, at(1))
                .unwrap();
            protocol.settings_acknowledged(at(1)).unwrap();
            let ticket = protocol
                .command_entered(Command::PageEnable, at(2))
                .unwrap();
            protocol.command_returned(ticket, true, at(2)).unwrap();
            let callback = protocol.completion_entered(ticket, at(3)).unwrap();
            let decoded = decode_completion(Command::PageEnable, b"{}").unwrap();
            protocol
                .completion_captured(callback, success, decoded.as_semantic(), at(3))
                .unwrap();
            protocol.completion_qualified(callback, at(3)).unwrap();
            assert_eq!(
                protocol.status().failure,
                if success { None } else { Some(Problem::Result) }
            );
        }
    }

    // Mutation: accepting Promise/object/default/granted as a denied effect.
    #[test]
    fn evaluated_fixed_failure_tokens_are_distinct_from_denied() {
        for (token, want) in [
            ("denied", CompletionValue::Denied),
            ("default", CompletionValue::Default),
            ("granted", CompletionValue::Granted),
            ("precondition", CompletionValue::Precondition),
            ("document-changed", CompletionValue::DocumentChanged),
            ("invalid", CompletionValue::Invalid),
        ] {
            let input = format!(
                r#"{{"result":{{"value":"code0-permission-{token}-v1","type":"string"}}}}"#
            );
            assert_eq!(
                decode_completion(Command::Evaluate, input.as_bytes())
                    .unwrap()
                    .as_semantic(),
                want
            );
        }
        for bad in [
            r#"{"result":{"type":"object","value":{}}}"#,
            r#"{"result":{"type":"string","value":"denied"}}"#,
            r#"{"result":{"type":"string","value":null}}"#,
        ] {
            assert!(decode_completion(Command::Evaluate, bad.as_bytes()).is_err());
        }
    }

    // Mutation: ignoring result extras, exceptionDetails, wrong description or unknown token.
    #[test]
    fn awaited_envelope_has_only_result_type_value_and_matching_optional_description() {
        let valid = br#"{"result":{"type":"string","value":"code0-permission-denied-v1","description":"code0-permission-denied-v1"}}"#;
        assert_eq!(
            decode_completion(Command::Evaluate, valid)
                .unwrap()
                .as_semantic(),
            CompletionValue::Denied
        );
        let base: serde_json::Value = serde_json::from_slice(DENIED).unwrap();
        for key in [
            "objectId",
            "subtype",
            "unserializableValue",
            "preview",
            "className",
            "extra",
        ] {
            let mut value = base.clone();
            value["result"][key] = serde_json::json!("unexpected");
            assert!(
                decode_completion(Command::Evaluate, &serde_json::to_vec(&value).unwrap()).is_err()
            );
        }
        for key in ["error", "exceptionDetails", "other"] {
            let mut value = base.clone();
            value[key] = serde_json::json!({});
            assert!(
                decode_completion(Command::Evaluate, &serde_json::to_vec(&value).unwrap()).is_err()
            );
        }
        assert!(decode_completion(Command::Evaluate, br#"{"result":{"type":"string","value":"code0-permission-denied-v1","description":"code0-permission-default-v1"}}"#).is_err());
    }

    // Mutation: decoded equality permits escaped fixed result spellings.
    #[test]
    fn fixed_awaited_token_spellings_are_literal_while_event_strings_may_escape() {
        for bad in [
            r#"{"result":{"type":"string","value":"code0-permission-\u0064enied-v1"}}"#,
            r#"{"result":{"type":"string","value":"code0-permission-denied-v1","description":"code0-permission-\u0064enied-v1"}}"#,
        ] {
            assert!(decode_completion(Command::Evaluate, bad.as_bytes()).is_err());
        }
        assert_eq!(
            decode_lifecycle(Receiver::StartedLoading, br#"{"frame\u0049d":"ro\u006ft"}"#).unwrap(),
            Lifecycle::StartedLoading(id("root"))
        );
    }

    // Mutation: last duplicate decoded key silently wins, at root or nested depth.
    #[test]
    fn duplicate_decoded_keys_are_rejected_recursively() {
        for bad in [
            r#"{"frameId":"root","frame\u0049d":"other"}"#,
            r#"{"context":{"id":7,"origin":"://","name":"","uniqueId":"b","auxData":{"isDefault":true,"frameId":"root","type":"default","t\u0079pe":"default"}}}"#,
        ] {
            let receiver = if bad.contains("context") {
                Receiver::ContextCreated
            } else {
                Receiver::StartedLoading
            };
            assert!(decode_lifecycle(receiver, bad.as_bytes()).is_err());
        }
        for bad in [
            r#"{"result":{"type":"string","value":"code0-permission-denied-v1","v\u0061lue":"code0-permission-denied-v1"}}"#,
            r#"{"result":{"type":"string","value":"code0-permission-denied-v1"},"res\u0075lt":{}}"#,
        ] {
            assert!(decode_completion(Command::Evaluate, bad.as_bytes()).is_err());
        }
    }

    // Mutation: omitting auxData/type/uniqueId or accepting an isolated/worker/default-false context.
    #[test]
    fn selected_and_blank_contexts_require_the_complete_closed_default_world_profile() {
        assert_eq!(
            decode_lifecycle(Receiver::ContextCreated, SELECTED_CONTEXT.as_bytes()).unwrap(),
            Lifecycle::ContextCreated(
                Document::Selected,
                Context::new(id("root"), 7, id("selected-context")).unwrap()
            )
        );
        assert_eq!(
            decode_lifecycle(Receiver::ContextCreated, BLANK_CONTEXT.as_bytes()).unwrap(),
            Lifecycle::ContextCreated(
                Document::Blank,
                Context::new(id("root"), 7, id("blank-context")).unwrap()
            )
        );
        for key in ["id", "origin", "name", "uniqueId", "auxData"] {
            let mut value = json(SELECTED_CONTEXT);
            value["context"].as_object_mut().unwrap().remove(key);
            assert!(event(Receiver::ContextCreated, &value).is_err());
        }
        for key in ["isDefault", "frameId", "type"] {
            let mut value = json(SELECTED_CONTEXT);
            value["context"]["auxData"]
                .as_object_mut()
                .unwrap()
                .remove(key);
            assert!(event(Receiver::ContextCreated, &value).is_err());
        }
        for (key, value) in [
            ("type", serde_json::json!("isolated")),
            ("isDefault", serde_json::json!(false)),
            ("extra", serde_json::json!(true)),
        ] {
            let mut input = json(SELECTED_CONTEXT);
            input["context"]["auxData"][key] = value;
            assert!(event(Receiver::ContextCreated, &input).is_err());
        }
    }

    // Mutation: accepting Bool/float/CID zero, overflow or numeric-only context identity.
    #[test]
    fn context_numeric_ids_are_real_integers_and_never_an_identity_fallback() {
        for numeric in [
            "0",
            "-1",
            "true",
            "1.0",
            "1e0",
            "2147483648",
            "null",
            "\"7\"",
        ] {
            let value = SELECTED_CONTEXT.replacen("\"id\":7", &format!("\"id\":{numeric}"), 1);
            assert!(decode_lifecycle(Receiver::ContextCreated, value.as_bytes()).is_err());
        }
        for numeric in ["1", "2147483647"] {
            let value = SELECTED_CONTEXT.replacen("\"id\":7", &format!("\"id\":{numeric}"), 1);
            assert!(decode_lifecycle(Receiver::ContextCreated, value.as_bytes()).is_ok());
        }
    }

    // Mutation: accepting empty, overlong or non-ASCII identifiers.
    #[test]
    fn every_decoded_id_obeys_ascii_character_and_128_byte_limits() {
        for bad in ["", "a/b", "a b", "é", "a?b", &"a".repeat(129)] {
            let value = serde_json::json!({"frameId": bad});
            assert!(event(Receiver::StartedLoading, &value).is_err());
        }
        let value = serde_json::json!({"frameId": "A_.:-09".repeat(16)});
        assert!(event(Receiver::StartedLoading, &value).is_ok());
        let value = serde_json::json!({"frameId": "a".repeat(128)});
        assert!(event(Receiver::StoppedLoading, &value).is_ok());
    }

    // Mutation: selected/blank frame is accepted from URL or origin alone, or required keys default.
    #[test]
    fn frame_profiles_require_all_fixed_fields_and_separate_selected_from_blank() {
        assert_eq!(
            decode_lifecycle(
                Receiver::FrameNavigated,
                navigated(SELECTED_FRAME).as_bytes()
            )
            .unwrap(),
            Lifecycle::FrameNavigated(Document::Selected, frame())
        );
        assert!(matches!(
            decode_lifecycle(Receiver::FrameNavigated, navigated(BLANK_FRAME).as_bytes()).unwrap(),
            Lifecycle::FrameNavigated(Document::Blank, _)
        ));
        for key in [
            "id",
            "loaderId",
            "url",
            "domainAndRegistry",
            "securityOrigin",
            "mimeType",
            "secureContextType",
            "crossOriginIsolatedContextType",
            "gatedAPIFeatures",
        ] {
            let mut input = json(&navigated(SELECTED_FRAME));
            input["frame"].as_object_mut().unwrap().remove(key);
            assert!(event(Receiver::FrameNavigated, &input).is_err());
        }
        for (key, value) in [
            ("url", "https://console.code0.ai/"),
            ("securityOrigin", "://"),
            ("mimeType", "application/json"),
            ("secureContextType", "InsecureScheme"),
            ("crossOriginIsolatedContextType", "Isolated"),
        ] {
            let mut input = json(&navigated(SELECTED_FRAME));
            input["frame"][key] = serde_json::json!(value);
            assert!(event(Receiver::FrameNavigated, &input).is_err());
        }
    }

    // Mutation: permitting unlisted properties, nonempty optional values or ad statuses.
    #[test]
    fn only_known_empty_frame_options_are_accepted_recursively() {
        let mut input = json(&navigated(SELECTED_FRAME));
        for key in ["name", "urlFragment", "unreachableUrl"] {
            input["frame"][key] = serde_json::json!("");
        }
        input["frame"]["securityOriginDetails"] = serde_json::json!({"isLocalhost":false});
        input["frame"]["adFrameStatus"] =
            serde_json::json!({"adFrameType":"none","explanations":[]});
        assert!(event(Receiver::FrameNavigated, &input).is_ok());
        for key in ["name", "urlFragment", "unreachableUrl", "parentId", "extra"] {
            let mut bad = input.clone();
            bad["frame"][key] = serde_json::json!("x");
            assert!(event(Receiver::FrameNavigated, &bad).is_err());
        }
        for (key, value) in [
            (
                "securityOriginDetails",
                serde_json::json!({"isLocalhost":true}),
            ),
            (
                "securityOriginDetails",
                serde_json::json!({"isLocalhost":false,"extra":false}),
            ),
            ("adFrameStatus", serde_json::json!({"adFrameType":"root"})),
            (
                "adFrameStatus",
                serde_json::json!({"adFrameType":"none","explanations":["x"]}),
            ),
        ] {
            let mut bad = input.clone();
            bad["frame"][key] = value;
            assert!(event(Receiver::FrameNavigated, &bad).is_err());
        }
    }

    // Mutation: root tree adopts blank/different URL, children, parent or unknown top shape.
    #[test]
    fn frame_tree_is_a_selected_root_with_no_children_or_extra_envelopes() {
        let owned =
            decode_completion(Command::GetFrameTree, tree(SELECTED_FRAME).as_bytes()).unwrap();
        assert_eq!(owned.as_semantic(), CompletionValue::FrameTree(&frame()));
        let mut value = json(&tree(SELECTED_FRAME));
        value["frameTree"]["childFrames"] = serde_json::json!([]);
        assert!(
            decode_completion(Command::GetFrameTree, &serde_json::to_vec(&value).unwrap()).is_ok()
        );
        value["frameTree"]["childFrames"] = serde_json::json!([{"frame":json(SELECTED_FRAME)}]);
        assert!(
            decode_completion(Command::GetFrameTree, &serde_json::to_vec(&value).unwrap()).is_err()
        );
        assert!(decode_completion(Command::GetFrameTree, tree(BLANK_FRAME).as_bytes()).is_err());
        let mut value = json(&tree(SELECTED_FRAME));
        value["other"] = serde_json::json!({});
        assert!(
            decode_completion(Command::GetFrameTree, &serde_json::to_vec(&value).unwrap()).is_err()
        );
    }

    // Mutation: destroying by CID only or dropping the uniqueId part during parsing.
    #[test]
    fn destruction_preserves_the_exact_whole_pair() {
        let raw = br#"{"executionContextId":7,"executionContextUniqueId":"blank-context"}"#;
        assert_eq!(
            decode_lifecycle(Receiver::ContextDestroyed, raw).unwrap(),
            Lifecycle::ContextDestroyed {
                numeric: 7,
                unique: id("blank-context")
            }
        );
        for raw in [
            r#"{"executionContextId":7}"#,
            r#"{"executionContextUniqueId":"b"}"#,
            r#"{"executionContextId":true,"executionContextUniqueId":"b"}"#,
            r#"{"executionContextId":7,"executionContextUniqueId":"b","extra":0}"#,
        ] {
            assert!(decode_lifecycle(Receiver::ContextDestroyed, raw.as_bytes()).is_err());
        }
    }

    // Mutation: a valid clear/load envelope defaults omitted fields or accepts extra keys.
    #[test]
    fn clear_and_each_loading_receiver_use_only_their_exact_profiles() {
        assert_eq!(
            decode_lifecycle(Receiver::ContextsCleared, b"{}").unwrap(),
            Lifecycle::ContextsCleared
        );
        assert!(decode_lifecycle(Receiver::ContextsCleared, b"{\"x\":0}").is_err());
        for receiver in [Receiver::StartedLoading, Receiver::StoppedLoading] {
            assert!(decode_lifecycle(receiver, b"{}").is_err());
            assert!(
                decode_lifecycle(receiver, br#"{"frameId":"root","loaderId":"invented"}"#).is_err()
            );
        }
        assert_eq!(
            decode_lifecycle(Receiver::StartedLoading, br#"{"frameId":"root"}"#).unwrap(),
            Lifecycle::StartedLoading(id("root"))
        );
        assert_eq!(
            decode_lifecycle(Receiver::StoppedLoading, br#"{"frameId":"root"}"#).unwrap(),
            Lifecycle::StoppedLoading(id("root"))
        );
    }

    // Mutation: forbidden event profiles become an accepted document transition or lose shape checks.
    #[test]
    fn all_four_forbidden_receiver_shapes_are_validated_then_tagged_unsupported() {
        for (receiver, raw) in [
            (Receiver::FrameAttached, r#"{"frameId":"child","parentFrameId":"root"}"#.to_owned()),
            (Receiver::FrameDetached, r#"{"frameId":"root","reason":"swap"}"#.to_owned()),
            (Receiver::DocumentOpened, format!(r#"{{"frame":{SELECTED_FRAME}}}"#)),
            (Receiver::WithinDocument, r#"{"frameId":"root","url":"https://console.code0.ai/#x","navigationType":"fragment"}"#.to_owned()),
        ] { assert_eq!(decode_lifecycle(receiver, raw.as_bytes()).unwrap(), Lifecycle::Unsupported(receiver)); }
        for (receiver, raw) in [
            (
                Receiver::FrameAttached,
                r#"{"frameId":"root","parentFrameId":"root"}"#,
            ),
            (
                Receiver::FrameAttached,
                r#"{"frameId":"child","parentFrameId":"root","stack":{}}"#,
            ),
            (
                Receiver::FrameDetached,
                r#"{"frameId":"root","reason":"other"}"#,
            ),
            (
                Receiver::WithinDocument,
                r#"{"frameId":"root","url":"x","navigationType":"reload"}"#,
            ),
        ] {
            assert!(decode_lifecycle(receiver, raw.as_bytes()).is_err());
        }
    }

    // Mutation: relaxing navigation enum, origin spellings or missing type.
    #[test]
    fn navigation_type_and_context_origin_have_no_broader_fallback_grammar() {
        for kind in ["BackForwardCacheRestore", "other", "navigation"] {
            let raw = navigated(SELECTED_FRAME).replace("Navigation", kind);
            assert!(decode_lifecycle(Receiver::FrameNavigated, raw.as_bytes()).is_err());
        }
        for origin in [
            "file://",
            "https://console.code0.ai/",
            "https://console.code0.ai:443",
            "https://CONSOLE.code0.ai",
            "about:blank",
        ] {
            let raw = SELECTED_CONTEXT.replace("https://console.code0.ai", origin);
            assert!(decode_lifecycle(Receiver::ContextCreated, raw.as_bytes()).is_err());
        }
    }

    // Mutation: common parser permits invalid UTF8, JSON trailing values, NUL or unpaired surrogates.
    #[test]
    fn invalid_encoding_trailing_json_and_decoded_nul_are_rejected() {
        for raw in [
            b"\xff".as_slice(),
            b"{}{}",
            b"{}\0",
            br#"{"frameId":"a\u0000b"}"#,
            br#"{"frameId":"\ud800"}"#,
            br#"{"frameId":"\udc00"}"#,
            br#"{"frameId":"\ud800\u0061"}"#,
            br#"{"frameId":"a\q"}"#,
        ] {
            assert!(decode_lifecycle(Receiver::StartedLoading, raw).is_err());
        }
        let raw = br#"{"frameId":"root","url":"\ud83d\ude00","navigationType":"other"}"#;
        assert_eq!(
            decode_lifecycle(Receiver::WithinDocument, raw).unwrap(),
            Lifecycle::Unsupported(Receiver::WithinDocument)
        );
    }

    // Mutation: missing per-receiver/reply cap or counting Unicode chars instead of UTF8 bytes.
    #[test]
    fn per_payload_caps_and_decoded_url_byte_limit_are_exact() {
        for (receiver, cap) in [
            (Receiver::ContextsCleared, 64),
            (Receiver::ContextDestroyed, 256),
            (Receiver::FrameAttached, 512),
            (Receiver::FrameDetached, 512),
            (Receiver::ContextCreated, 1024),
            (Receiver::FrameNavigated, 2048),
            (Receiver::DocumentOpened, 2048),
            (Receiver::WithinDocument, 2048),
            (Receiver::StartedLoading, 256),
            (Receiver::StoppedLoading, 256),
        ] {
            let mut raw = match receiver {
                Receiver::ContextsCleared => "{}".to_owned(),
                Receiver::ContextDestroyed => {
                    r#"{"executionContextId":7,"executionContextUniqueId":"b"}"#.to_owned()
                }
                Receiver::FrameAttached => r#"{"frameId":"c","parentFrameId":"r"}"#.to_owned(),
                Receiver::FrameDetached => r#"{"frameId":"r","reason":"remove"}"#.to_owned(),
                Receiver::ContextCreated => SELECTED_CONTEXT.to_owned(),
                Receiver::FrameNavigated => navigated(SELECTED_FRAME),
                Receiver::DocumentOpened => format!(r#"{{"frame":{SELECTED_FRAME}}}"#),
                Receiver::WithinDocument => {
                    r#"{"frameId":"r","url":"x","navigationType":"other"}"#.to_owned()
                }
                Receiver::StartedLoading | Receiver::StoppedLoading => {
                    r#"{"frameId":"r"}"#.to_owned()
                }
            };
            raw.push_str(&" ".repeat(cap - raw.len()));
            assert!(decode_lifecycle(receiver, raw.as_bytes()).is_ok());
            raw.push(' ');
            assert!(decode_lifecycle(receiver, raw.as_bytes()).is_err());
        }
        for (command, cap, mut raw) in [
            (Command::PageEnable, 64, "{}".to_owned()),
            (Command::RuntimeEnable, 64, "{}".to_owned()),
            (Command::GetFrameTree, 2048, tree(SELECTED_FRAME)),
            (
                Command::Evaluate,
                512,
                std::str::from_utf8(DENIED).unwrap().to_owned(),
            ),
        ] {
            raw.push_str(&" ".repeat(cap - raw.len()));
            assert!(decode_completion(command, raw.as_bytes()).is_ok());
            raw.push(' ');
            assert!(decode_completion(command, raw.as_bytes()).is_err());
        }
        for (url, pass) in [("é".repeat(512), true), ("é".repeat(513), false)] {
            let value = serde_json::json!({"frameId":"r","url":url,"navigationType":"historyApi"});
            assert_eq!(event(Receiver::WithinDocument, &value).is_ok(), pass);
        }
    }

    // Mutation: token/depth limits are checked after unbounded allocation or are off by one.
    #[test]
    fn stack_token_count_and_depth_are_bounded_without_a_value_tree() {
        let at_limit = format!("[{}]", vec!["0"; 255].join(","));
        assert!(Doc::new(at_limit.as_bytes(), 2048).is_ok());
        let over = format!("[{}]", vec!["0"; 256].join(","));
        assert!(Doc::new(over.as_bytes(), 2048).is_err());
        let valid = format!("{}0{}", "[".repeat(8), "]".repeat(8));
        assert!(Doc::new(valid.as_bytes(), 2048).is_ok());
        let invalid = format!("{}0{}", "[".repeat(9), "]".repeat(9));
        assert!(Doc::new(invalid.as_bytes(), 2048).is_err());
    }
}
