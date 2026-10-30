// Finite S3 fixture protocol only. No general CDP execution interface.
use std::time::Instant;

pub(super) const EVENTS: [&str; 10] = [
    "Page.fileChooserOpened",
    "Runtime.executionContextCreated",
    "Runtime.executionContextDestroyed",
    "Runtime.executionContextsCleared",
    "DOM.documentUpdated",
    "Page.frameAttached",
    "Page.frameDetached",
    "Page.frameNavigated",
    "Page.documentOpened",
    "Page.navigatedWithinDocument",
];
pub(super) const METHODS: [&str; 11] = [
    "Page.enable",
    "Runtime.enable",
    "Page.setInterceptFileChooserDialog",
    "Page.getFrameTree",
    "Page.createIsolatedWorld",
    "Runtime.evaluate",
    "DOM.getDocument",
    "DOM.querySelectorAll",
    "DOM.describeNode",
    "Page.getFrameTree",
    "Runtime.evaluate",
];
const REPLY_CAPS: [usize; 11] = [64, 64, 64, 2048, 128, 256, 2048, 128, 2048, 2048, 512];
const EVENT_CAPS: [usize; 10] = [512, 1024, 256, 64, 64, 512, 512, 2048, 2048, 2048];
pub(super) const PHASE1: &str = r#"(()=>{"use strict";const d=document,h=d.documentElement,b=d.body;if(top!==self||location.protocol!=="file:"||d.readyState!=="complete"||d.doctype!==null||!h||!d.head||!b||d.childNodes.length!==1||h.children.length!==2||h.firstElementChild!==d.head||h.lastElementChild!==b||h.childNodes.length!==2||h.attributes.length!==0||d.head.attributes.length!==0||b.attributes.length!==0||d.head.childNodes.length!==0||b.childNodes.length!==0||d.querySelectorAll("input,iframe,frame").length!==0||"__code0S3"in globalThis)throw 0;const i=d.createElement("input");i.setAttribute("id","code0-s3-file");i.setAttribute("type","file");b.appendChild(i);Object.defineProperty(globalThis,"__code0S3",{value:Object.freeze({d,i})});return{v:1,kind:"prepared"};})()"#;
pub(super) const PHASE2: &str = r#"(()=>{"use strict";const s=globalThis.__code0S3,d=document,i=s&&s.i,h=d.documentElement,b=d.body;const ok=()=>s&&s.d===d&&top===self&&location.protocol==="file:"&&d.readyState==="complete"&&d.doctype===null&&h&&d.head&&b&&d.childNodes.length===1&&h.children.length===2&&h.firstElementChild===d.head&&h.lastElementChild===b&&h.childNodes.length===2&&h.attributes.length===0&&d.head.attributes.length===0&&b.attributes.length===0&&d.head.childNodes.length===0&&b.childNodes.length===1&&b.firstChild===i&&i.isConnected&&i.ownerDocument===d&&i.localName==="input"&&i.type==="file"&&i.getAttribute("id")==="code0-s3-file"&&i.getAttribute("type")==="file"&&i.attributes.length===2&&d.querySelectorAll("input,iframe,frame").length===1&&d.querySelector("input")===i;if(!ok())throw 0;return new Promise((resolve,reject)=>{let n=0,c=0,done=false;i.addEventListener("change",()=>{c++;done=true;reject(0);});i.addEventListener("cancel",e=>{n++;if(done||n!==1||c!==0||e.isTrusted!==true||e.target!==i||e.currentTarget!==i||!ok()){done=true;reject(0);return;}const f=i.files,z=i.value==="";if(f===null||f.length!==0||!z){done=true;reject(0);return;}done=true;resolve({v:1,kind:"cancel-snapshot",trusted:e.isTrusted,cancelCount:n,changeCount:c,filesCount:f.length,valueEmpty:z,sameInput:e.target===i,connected:i.isConnected,sameDocument:i.ownerDocument===d});});i.click();});})()"#;
pub(super) const WORLD: &str = "code0-s3-file-world-v1";
pub(super) const LEAF: &str = "code0-s3-chooser-empty.html";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Id {
    bytes: [u8; 128],
    len: u8,
}
impl Id {
    pub(super) fn parse(s: &str) -> Result<Self, ()> {
        let mut out = Self {
            bytes: [0; 128],
            len: 0,
        };
        if s.is_empty() || s.len() > 128 {
            return Err(());
        }
        for (n, b) in s.bytes().enumerate() {
            if !b.is_ascii_alphanumeric() && !b"_.:-".contains(&b) {
                return Err(());
            }
            out.bytes[n] = b;
        }
        out.len = s.len() as u8;
        Ok(out)
    }
    pub(super) fn as_str(&self) -> &str {
        // Every stored byte was checked ASCII on construction.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap()
    }
}
pub(super) struct Binding {
    pub(super) frame: Id,
    pub(super) backend: u32,
}
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) id: Id,
    pub(super) loader: Id,
}
#[derive(Clone, Copy)]
pub(super) struct Context {
    pub(super) id: u32,
    pub(super) unique: Id,
    pub(super) named: bool,
}
pub(super) enum Event {
    Chooser,
    Created(Context),
    Destroyed(u32, Option<Id>),
    Cleared,
    Updated,
    Attached,
    Detached(Id, bool),
    Navigated(bool),
    Opened,
    Within,
}
#[derive(Clone, Copy)]
pub(super) enum ReplyKind {
    Ack,
    Tree,
    World,
    Prepared,
    Root,
    Node,
    Input,
    Canceled,
}
#[derive(Clone, Copy)]
pub(super) struct Reply {
    pub(super) kind: ReplyKind,
    pub(super) frame: Option<Frame>,
    pub(super) scalar: u32,
}
impl Reply {
    fn unit(kind: ReplyKind) -> Self {
        Self {
            kind,
            frame: None,
            scalar: 0,
        }
    }
    fn tree(frame: Frame) -> Self {
        Self {
            kind: ReplyKind::Tree,
            frame: Some(frame),
            scalar: 0,
        }
    }
    fn number(kind: ReplyKind, scalar: u32) -> Self {
        Self {
            kind,
            frame: None,
            scalar,
        }
    }
}

// One immutable absolute attempt end. Stop entry and logical revocation differ.
pub(super) struct Epoch {
    pub(super) end: Instant,
    pub(super) armed: bool,
    pub(super) phase2: bool,
    pub(super) invalid: u16,
    pub(super) bootstrap: u8,
    pub(super) target: Option<Instant>,
    pub(super) effect: Option<Instant>,
    pub(super) stop: Option<Instant>,
}
impl Epoch {
    pub(super) fn new(end: Instant) -> Self {
        Self {
            end,
            armed: false,
            phase2: false,
            invalid: 0,
            bootstrap: 0,
            target: None,
            effect: None,
            stop: None,
        }
    }
    pub(super) fn invalidate(&mut self, index: usize) {
        self.invalid |= 1 << index.min(10);
    }
    pub(super) fn bootstrap_delivery(&mut self) -> bool {
        self.bootstrap = self.bootstrap.saturating_add(1);
        if self.bootstrap > 16 {
            self.invalidate(10);
        }
        self.invalid == 0
    }
    pub(super) fn arm(&mut self) {
        if self.armed {
            self.invalidate(10);
        } else {
            self.armed = true;
        }
    }
    // Owner revocation is an independent observation, not a Stop timestamp.
    // Only the already matched chooser may account for its own closing flags.
    pub(super) fn observe_revocation(&mut self, revoked: bool, chooser_caused: bool) {
        if revoked && (!chooser_caused || self.target.is_none()) {
            self.invalidate(10);
        }
    }
    pub(super) fn timely(&self, now: Instant) -> bool {
        now < self.end && self.invalid == 0
    }
    pub(super) fn effect_at(&mut self, now: Instant) -> bool {
        if !self.armed
            || !self.phase2
            || !self.timely(now)
            || self.stop.is_some()
            || self.effect.is_some()
        {
            self.invalidate(10);
            return false;
        }
        self.effect = Some(now);
        true
    }
    pub(super) fn target_at(&mut self, now: Instant) -> bool {
        if !self.armed
            || !self.phase2
            || !self.timely(now)
            || self.stop.is_some()
            || self.target.is_some()
        {
            self.invalidate(0);
            return false;
        }
        self.target = Some(now);
        true
    }
    pub(super) fn stop_at(&mut self, now: Instant) {
        if self.stop.is_none() {
            self.stop = Some(now);
        }
    }
    pub(super) fn positive(&self) -> bool {
        self.invalid == 0
            && self
                .target
                .zip(self.effect)
                .zip(self.stop)
                .is_some_and(|((t, e), s)| t < s && e < s && t < self.end && e < self.end)
    }
}
pub(super) fn text_fits(params: usize, response: usize, pinned: usize) -> bool {
    params <= 2048
        && response <= 2048
        && params
            .checked_add(response)
            .and_then(|n| n.checked_add(pinned))
            .is_some_and(|n| n <= 4096)
}

// Conservative encoded URL upper bound, checked before the URL owns text.
pub(super) fn uri_construction_fits(path_utf8_bytes: usize) -> bool {
    path_utf8_bytes
        .checked_mul(3)
        .and_then(|n| n.checked_add(16))
        .is_some_and(|n| n <= 2048)
}

// Caller reserves 2048 text bytes before serialization, then retains its actual
// byte charge. Borrowed IDs/expressions are never cloned into a Value tree.
pub(super) fn params(
    index: usize,
    frame: Option<Id>,
    unique: Option<Id>,
    node: u32,
) -> Result<(Vec<u16>, usize), ()> {
    struct Wide(Vec<u16>);
    impl std::io::Write for Wide {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0.len() + bytes.len() > 2048 || !bytes.is_ascii() {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
            self.0.extend(bytes.iter().map(|b| u16::from(*b)));
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Eval<'a> {
        expression: &'a str,
        unique_context_id: &'a str,
        user_gesture: bool,
        #[serde(rename = "includeCommandLineAPI")]
        include_command_line_api: bool,
        #[serde(rename = "allowUnsafeEvalBlockedByCSP")]
        allow_unsafe_eval_blocked_by_csp: bool,
        return_by_value: bool,
        await_promise: bool,
        generate_preview: bool,
    }
    #[derive(serde::Serialize)]
    struct World<'a> {
        #[serde(rename = "frameId")]
        frame: &'a str,
        #[serde(rename = "worldName")]
        name: &'a str,
        #[serde(rename = "grantUniveralAccess")]
        access: bool,
    }
    #[derive(serde::Serialize)]
    struct Query {
        #[serde(rename = "nodeId")]
        node: u32,
        selector: &'static str,
    }
    #[derive(serde::Serialize)]
    struct Describe {
        #[serde(rename = "nodeId")]
        node: u32,
        depth: u8,
        pierce: bool,
    }
    if matches!(index, 7 | 8) && !(1..=2147483647).contains(&node) {
        return Err(());
    }
    let mut out = Wide(Vec::with_capacity(2049));
    use std::io::Write;
    match index {
        0 | 1 | 3 | 9 => out.write_all(b"{}"),
        2 => out.write_all(b"{\"enabled\":true,\"cancel\":true}"),
        4 => serde_json::to_writer(
            &mut out,
            &World {
                frame: frame.as_ref().ok_or(())?.as_str(),
                name: WORLD,
                access: false,
            },
        )
        .map_err(std::io::Error::other),
        5 | 10 => serde_json::to_writer(
            &mut out,
            &Eval {
                expression: if index == 5 { PHASE1 } else { PHASE2 },
                unique_context_id: unique.as_ref().ok_or(())?.as_str(),
                user_gesture: index == 10,
                include_command_line_api: false,
                allow_unsafe_eval_blocked_by_csp: false,
                return_by_value: true,
                await_promise: index == 10,
                generate_preview: false,
            },
        )
        .map_err(std::io::Error::other),
        6 => out.write_all(b"{\"depth\":1,\"pierce\":false}"),
        7 => serde_json::to_writer(
            &mut out,
            &Query {
                node,
                selector: "input",
            },
        )
        .map_err(std::io::Error::other),
        8 => serde_json::to_writer(
            &mut out,
            &Describe {
                node,
                depth: 1,
                pierce: false,
            },
        )
        .map_err(std::io::Error::other),
        _ => return Err(()),
    }
    .map_err(|_| ())?;
    let bytes = out.0.len();
    out.0.push(0);
    Ok((out.0, bytes))
}

// No Value tree or decoded String allocations. Tokens are bounded stack spans;
// decoded comparisons/IDs use a streaming iterator over the borrowed raw buffer.
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
impl Iterator for Decoded<'_> {
    type Item = Result<char, ()>;
    fn next(&mut self) -> Option<Self::Item> {
        let c = self.chars.next()?;
        let v = if c != '\\' {
            if c < ' ' {
                return Some(Err(()));
            }
            c
        } else {
            match self.chars.next() {
                Some('"') => '"',
                Some('\\') => '\\',
                Some('/') => '/',
                Some('b') => '\u{8}',
                Some('f') => '\u{c}',
                Some('n') => '\n',
                Some('r') => '\r',
                Some('t') => '\t',
                Some('u') => {
                    let mut hex = || {
                        let mut n = 0u16;
                        for _ in 0..4 {
                            n = n.checked_mul(16).ok_or(())?
                                + self.chars.next().and_then(|c| c.to_digit(16)).ok_or(())? as u16;
                        }
                        Ok::<_, ()>(n)
                    };
                    let first = match hex() {
                        Ok(n) => n,
                        Err(()) => return Some(Err(())),
                    };
                    if (0xd800..=0xdbff).contains(&first) {
                        if self.chars.next() != Some('\\') || self.chars.next() != Some('u') {
                            return Some(Err(()));
                        }
                        let mut second = 0u32;
                        for _ in 0..4 {
                            let Some(n) = self.chars.next().and_then(|c| c.to_digit(16)) else {
                                return Some(Err(()));
                            };
                            second = second * 16 + n;
                        }
                        if !(0xdc00..=0xdfff).contains(&second) {
                            return Some(Err(()));
                        }
                        match char::from_u32(
                            0x10000 + ((u32::from(first) - 0xd800) << 10) + second - 0xdc00,
                        ) {
                            Some(c) => c,
                            None => return Some(Err(())),
                        }
                    } else {
                        match char::from_u32(u32::from(first)) {
                            Some(c) => c,
                            None => return Some(Err(())),
                        }
                    }
                }
                _ => return Some(Err(())),
            }
        };
        Some(if v == '\0' { Err(()) } else { Ok(v) })
    }
}
impl<'a> Doc<'a> {
    fn new(raw: &'a str, cap: usize) -> Result<Self, ()> {
        if raw.len() > cap || raw.as_bytes().contains(&0) {
            return Err(());
        }
        let mut d = Self {
            raw,
            tokens: [Token::default(); 256],
            count: 0,
            pos: 0,
        };
        d.scan(0)?;
        d.ws();
        if d.pos != raw.len() {
            return Err(());
        }
        Ok(d)
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
    fn scan(&mut self, depth: usize) -> Result<usize, ()> {
        self.ws();
        if depth > 8 || self.count == self.tokens.len() {
            return Err(());
        }
        let n = self.count;
        self.count += 1;
        let start = self.pos;
        let kind = *self.raw.as_bytes().get(self.pos).ok_or(())?;
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
                                return Err(());
                            }
                            self.scan(depth + 1)?;
                            self.ws();
                            if self.raw.as_bytes().get(self.pos) != Some(&b':') {
                                return Err(());
                            }
                            self.pos += 1;
                        }
                        self.scan(depth + 1)?;
                        self.ws();
                        if self.raw.as_bytes().get(self.pos) == Some(&close) {
                            break;
                        }
                        if self.raw.as_bytes().get(self.pos) != Some(&b',') {
                            return Err(());
                        }
                        self.pos += 1;
                    }
                }
                self.pos += 1;
            }
            b'"' => {
                loop {
                    let b = *self.raw.as_bytes().get(self.pos).ok_or(())?;
                    self.pos += 1;
                    if b == b'"' {
                        break;
                    }
                    if b == b'\\' {
                        self.pos += 1;
                        if self.pos > self.raw.len() {
                            return Err(());
                        }
                    }
                }
                let inner = self.raw.get(start + 1..self.pos - 1).ok_or(())?;
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
                    return Err(());
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
                    return Err(());
                }
            }
            _ => return Err(()),
        }
        self.tokens[n] = Token {
            start,
            end: self.pos,
            next: self.count,
            kind,
        };
        Ok(n)
    }
    fn decoded(&self, n: usize) -> Result<Decoded<'a>, ()> {
        let t = self.tokens[n];
        if t.kind != b'"' {
            return Err(());
        }
        Ok(Decoded {
            chars: self.raw[t.start + 1..t.end - 1].chars(),
        })
    }
    fn eq(&self, n: usize, value: &str) -> bool {
        self.decoded(n).is_ok_and(|v| v.eq(value.chars().map(Ok)))
    }
    fn string(&self, n: usize, max: usize) -> Result<(), ()> {
        let mut len = 0;
        for c in self.decoded(n)? {
            len += c?.len_utf8();
            if len > max {
                return Err(());
            }
        }
        Ok(())
    }
    fn id(&self, n: usize) -> Result<Id, ()> {
        let mut b = [0u8; 128];
        let mut len = 0;
        for c in self.decoded(n)? {
            let c = c?;
            if !c.is_ascii() || len == 128 {
                return Err(());
            }
            b[len] = c as u8;
            len += 1;
        }
        Id::parse(std::str::from_utf8(&b[..len]).map_err(|_| ())?)
    }
    fn integer(&self, n: usize, min: u32, max: u32) -> Result<u32, ()> {
        let t = self.tokens[n];
        if !t.kind.is_ascii_digit() {
            return Err(());
        }
        let v = self.raw[t.start..t.end].parse::<u32>().map_err(|_| ())?;
        if v < min || v > max {
            return Err(());
        }
        Ok(v)
    }
    fn boolean(&self, n: usize) -> Result<bool, ()> {
        let t = self.tokens[n];
        match &self.raw[t.start..t.end] {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(()),
        }
    }
    fn fixed(&self, n: usize, s: &str) -> Result<(), ()> {
        if self.eq(n, s) { Ok(()) } else { Err(()) }
    }
    fn fields<const N: usize>(
        &self,
        n: usize,
        keys: [&str; N],
        required: u32,
    ) -> Result<[Option<usize>; N], ()> {
        if self.tokens[n].kind != b'{' {
            return Err(());
        }
        let mut out = [None; N];
        let mut at = n + 1;
        let mut seen = 0u32;
        while at < self.tokens[n].next {
            let index = keys.iter().position(|k| self.eq(at, k)).ok_or(())?;
            if seen & (1 << index) != 0 {
                return Err(());
            }
            seen |= 1 << index;
            let value = at + 1;
            out[index] = Some(value);
            at = self.tokens[value].next;
        }
        if seen & required != required {
            return Err(());
        }
        Ok(out)
    }
    fn array(&self, n: usize, max: usize) -> Result<([usize; 16], usize), ()> {
        if self.tokens[n].kind != b'[' {
            return Err(());
        }
        let mut a = [0; 16];
        let mut count = 0;
        let mut at = n + 1;
        while at < self.tokens[n].next {
            if count >= max || count == 16 {
                return Err(());
            }
            a[count] = at;
            count += 1;
            at = self.tokens[at].next;
        }
        Ok((a, count))
    }
    fn empty_array(&self, n: usize) -> Result<(), ()> {
        self.array(n, 0).map(|_| ())
    }
    fn empty(&self, n: usize) -> Result<(), ()> {
        self.fields(n, [], 0).map(|_| ())
    }
}

fn frame(d: &Doc<'_>, n: usize, uri: &str) -> Result<Frame, ()> {
    let f = d.fields(
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
    let id = d.id(f[0].ok_or(())?)?;
    let loader = d.id(f[1].ok_or(())?)?;
    d.fixed(f[2].ok_or(())?, uri)?;
    d.fixed(f[3].ok_or(())?, "")?;
    d.string(f[4].ok_or(())?, 256)?;
    d.fixed(f[5].ok_or(())?, "text/html")?;
    if ![
        "Secure",
        "SecureLocalhost",
        "InsecureScheme",
        "InsecureAncestor",
    ]
    .iter()
    .any(|s| d.eq(f[6].unwrap(), s))
    {
        return Err(());
    }
    if !["Isolated", "NotIsolated", "NotIsolatedFeatureDisabled"]
        .iter()
        .any(|s| d.eq(f[7].unwrap(), s))
    {
        return Err(());
    }
    let (a, count) = d.array(f[8].unwrap(), 4)?;
    let mut mask = 0;
    for at in a.into_iter().take(count) {
        let i = [
            "SharedArrayBuffers",
            "SharedArrayBuffersTransferAllowed",
            "PerformanceMeasureMemory",
            "PerformanceProfile",
        ]
        .iter()
        .position(|s| d.eq(at, s))
        .ok_or(())?;
        if mask & (1 << i) != 0 {
            return Err(());
        }
        mask |= 1 << i;
    }
    for at in f[9..12].iter().flatten() {
        d.fixed(*at, "")?;
    }
    if let Some(at) = f[12] {
        let x = d.fields(at, ["isLocalhost"], 1)?;
        if d.boolean(x[0].unwrap())? {
            return Err(());
        }
    }
    if let Some(at) = f[13] {
        let x = d.fields(at, ["adFrameType", "explanations"], 1)?;
        d.fixed(x[0].unwrap(), "none")?;
        if let Some(a) = x[1] {
            d.empty_array(a)?;
        }
    }
    Ok(Frame { id, loader })
}

// Position-specific Node profile, at most two shallow objects per reply.
fn node(
    d: &Doc<'_>,
    n: usize,
    position: u8,
    uri: &str,
    owner: Option<Id>,
    expected: u32,
) -> Result<(u32, u32), ()> {
    let f = d.fields(
        n,
        [
            "nodeId",
            "backendNodeId",
            "nodeType",
            "nodeName",
            "localName",
            "nodeValue",
            "documentURL",
            "baseURL",
            "compatibilityMode",
            "xmlVersion",
            "isSVG",
            "isScrollable",
            "affectedByStartingStyles",
            "adoptedStyleSheets",
            "parentId",
            "childNodeCount",
            "children",
            "attributes",
            "frameId",
            "shadowRoots",
            "shadowRootType",
        ],
        63,
    )?;
    let id = d.integer(f[0].unwrap(), if position == 3 { 0 } else { 1 }, 2147483647)?;
    let backend = d.integer(f[1].unwrap(), 1, 2147483647)?;
    let (kind, name, local) = match position {
        0 => (9, "#document", ""),
        1 => (1, "HTML", "html"),
        2 => (1, "INPUT", "input"),
        3 => (11, "#document-fragment", ""),
        _ => return Err(()),
    };
    d.integer(f[2].unwrap(), kind, kind)?;
    d.fixed(f[3].unwrap(), name)?;
    d.fixed(f[4].unwrap(), local)?;
    d.fixed(f[5].unwrap(), "")?;
    let allowed = match position {
        0 => [6, 7, 8, 9, 10, 11, 12, 13, 15, 16].as_slice(),
        1 => [10, 11, 12, 14, 15, 16, 17, 18].as_slice(),
        2 => [10, 11, 12, 14, 15, 16, 17, 19].as_slice(),
        _ => [10, 11, 12, 13, 14, 15, 16, 20].as_slice(),
    };
    if f.iter()
        .enumerate()
        .any(|(i, v)| i >= 6 && v.is_some() && !allowed.contains(&i))
    {
        return Err(());
    }
    if let Some(a) = f[10]
        && d.boolean(a)?
    {
        return Err(());
    }
    for a in [f[11], f[12]].into_iter().flatten() {
        d.boolean(a)?;
    }
    if let Some(a) = f[13] {
        d.empty_array(a)?;
    }
    if let Some(a) = f[14] {
        d.integer(
            a,
            if position == 1 { expected } else { 1 },
            if position == 1 { expected } else { 2147483647 },
        )?;
    }
    if let Some(a) = f[15] {
        let (min, max) = match position {
            0 => (1, 1),
            1 => (2, 2),
            2 => (0, 0),
            _ => (0, 16),
        };
        d.integer(a, min, max)?;
    }
    if position == 0 {
        d.fixed(f[6].ok_or(())?, uri)?;
        if let Some(a) = f[7] {
            d.fixed(a, uri)?;
        }
        if let Some(a) = f[8]
            && !["QuirksMode", "LimitedQuirksMode", "NoQuirksMode"]
                .iter()
                .any(|s| d.eq(a, s))
        {
            return Err(());
        }
        if let Some(a) = f[9] {
            d.fixed(a, "")?;
        }
        let (a, c) = d.array(f[16].ok_or(())?, 1)?;
        if c != 1 {
            return Err(());
        }
        node(d, a[0], 1, uri, owner, id)?;
    } else {
        if let Some(a) = f[16] {
            d.empty_array(a)?;
        }
        if position == 1 {
            if let Some(a) = f[17] {
                d.empty_array(a)?;
            }
            if let Some(a) = f[18] {
                d.fixed(a, owner.ok_or(())?.as_str())?;
            }
        }
        if position == 2 {
            if id != expected {
                return Err(());
            }
            let (a, c) = d.array(f[17].ok_or(())?, 4)?;
            if c != 4 {
                return Err(());
            }
            let valid = (d.eq(a[0], "id")
                && d.eq(a[1], "code0-s3-file")
                && d.eq(a[2], "type")
                && d.eq(a[3], "file"))
                || (d.eq(a[2], "id")
                    && d.eq(a[3], "code0-s3-file")
                    && d.eq(a[0], "type")
                    && d.eq(a[1], "file"));
            if !valid {
                return Err(());
            }
            if let Some(a) = f[19] {
                let (a, c) = d.array(a, 1)?;
                if c == 1 {
                    node(d, a[0], 3, uri, owner, 0)?;
                }
            }
        }
        if position == 3 {
            d.fixed(f[20].ok_or(())?, "user-agent")?;
        }
    }
    if f.iter().flatten().count() > 17 {
        return Err(());
    }
    Ok((id, backend))
}

pub(super) fn parse_reply(
    index: usize,
    raw: &str,
    uri: &str,
    owner: Option<Id>,
    expected: u32,
) -> Result<Reply, ()> {
    let d = Doc::new(raw, *REPLY_CAPS.get(index).ok_or(())?)?;
    match index {
        0..=2 => {
            d.empty(0)?;
            Ok(Reply::unit(ReplyKind::Ack))
        }
        3 | 9 => {
            let f = d.fields(0, ["frameTree"], 1)?;
            let t = d.fields(f[0].unwrap(), ["frame", "childFrames"], 1)?;
            if let Some(a) = t[1] {
                d.empty_array(a)?;
            }
            Ok(Reply::tree(frame(&d, t[0].unwrap(), uri)?))
        }
        4 => {
            let f = d.fields(0, ["executionContextId"], 1)?;
            Ok(Reply::number(
                ReplyKind::World,
                d.integer(f[0].unwrap(), 1, 2147483647)?,
            ))
        }
        5 | 10 => {
            let f = d.fields(0, ["result"], 1)?;
            let r = d.fields(
                f[0].unwrap(),
                ["type", "value", "className", "description"],
                3,
            )?;
            d.fixed(r[0].unwrap(), "object")?;
            for a in r[2..].iter().flatten() {
                d.fixed(*a, "Object")?;
            }
            if index == 5 {
                let v = d.fields(r[1].unwrap(), ["v", "kind"], 3)?;
                d.integer(v[0].unwrap(), 1, 1)?;
                d.fixed(v[1].unwrap(), "prepared")?;
                Ok(Reply::unit(ReplyKind::Prepared))
            } else {
                let v = d.fields(
                    r[1].unwrap(),
                    [
                        "v",
                        "kind",
                        "trusted",
                        "cancelCount",
                        "changeCount",
                        "filesCount",
                        "valueEmpty",
                        "sameInput",
                        "connected",
                        "sameDocument",
                    ],
                    1023,
                )?;
                d.integer(v[0].unwrap(), 1, 1)?;
                d.fixed(v[1].unwrap(), "cancel-snapshot")?;
                d.integer(v[3].unwrap(), 1, 1)?;
                d.integer(v[4].unwrap(), 0, 0)?;
                d.integer(v[5].unwrap(), 0, 0)?;
                for i in [2, 6, 7, 8, 9] {
                    if !d.boolean(v[i].unwrap())? {
                        return Err(());
                    }
                }
                Ok(Reply::unit(ReplyKind::Canceled))
            }
        }
        6 => {
            let f = d.fields(0, ["root"], 1)?;
            Ok(Reply::number(
                ReplyKind::Root,
                node(&d, f[0].unwrap(), 0, uri, owner, 0)?.0,
            ))
        }
        7 => {
            let f = d.fields(0, ["nodeIds"], 1)?;
            let (a, c) = d.array(f[0].unwrap(), 1)?;
            if c != 1 {
                return Err(());
            }
            Ok(Reply::number(
                ReplyKind::Node,
                d.integer(a[0], 1, 2147483647)?,
            ))
        }
        8 => {
            let f = d.fields(0, ["node"], 1)?;
            Ok(Reply::number(
                ReplyKind::Input,
                node(&d, f[0].unwrap(), 2, uri, owner, expected)?.1,
            ))
        }
        _ => Err(()),
    }
}

pub(super) fn parse_event(
    index: usize,
    raw: &str,
    uri: &str,
    binding: Option<&Binding>,
) -> Result<Event, ()> {
    let d = Doc::new(raw, *EVENT_CAPS.get(index).ok_or(())?)?;
    match index {
        0 => {
            let f = d.fields(0, ["frameId", "mode", "backendNodeId"], 7)?;
            let b = binding.ok_or(())?;
            d.fixed(f[0].unwrap(), b.frame.as_str())?;
            d.fixed(f[1].unwrap(), "selectSingle")?;
            d.integer(f[2].unwrap(), b.backend, b.backend)?;
            Ok(Event::Chooser)
        }
        1 => {
            let f = d.fields(0, ["context"], 1)?;
            let c = d.fields(
                f[0].unwrap(),
                ["id", "origin", "name", "uniqueId", "auxData"],
                15,
            )?;
            let id = d.integer(c[0].unwrap(), 1, 2147483647)?;
            d.string(c[1].unwrap(), 256)?;
            let named = d.eq(c[2].unwrap(), WORLD);
            if !named && !d.eq(c[2].unwrap(), "") {
                return Err(());
            }
            let unique = d.id(c[3].unwrap())?;
            if let Some(a) = c[4] {
                let a = d.fields(a, ["isDefault", "frameId", "type"], 3)?;
                if d.boolean(a[0].unwrap())? == named {
                    return Err(());
                }
                let frame = d.id(a[1].unwrap())?;
                if named && binding.is_some_and(|b| b.frame != frame) {
                    return Err(());
                }
                if let Some(t) = a[2] {
                    d.fixed(t, if named { "isolated" } else { "default" })?;
                }
            }
            Ok(Event::Created(Context { id, unique, named }))
        }
        2 => {
            let f = d.fields(0, ["executionContextId", "executionContextUniqueId"], 1)?;
            Ok(Event::Destroyed(
                d.integer(f[0].unwrap(), 1, 2147483647)?,
                f[1].map(|a| d.id(a)).transpose()?,
            ))
        }
        3 => {
            d.empty(0)?;
            Ok(Event::Cleared)
        }
        4 => {
            d.empty(0)?;
            Ok(Event::Updated)
        }
        5 => {
            let f = d.fields(0, ["frameId", "parentFrameId"], 3)?;
            d.id(f[0].unwrap())?;
            d.id(f[1].unwrap())?;
            Ok(Event::Attached)
        }
        6 => {
            let f = d.fields(0, ["frameId", "reason"], 3)?;
            let id = d.id(f[0].unwrap())?;
            let remove = d.eq(f[1].unwrap(), "remove");
            if !remove && !d.eq(f[1].unwrap(), "swap") {
                return Err(());
            }
            Ok(Event::Detached(id, remove))
        }
        7 => {
            let f = d.fields(0, ["frame", "type"], 3)?;
            let regular = d.eq(f[1].unwrap(), "Navigation");
            if !regular && !d.eq(f[1].unwrap(), "BackForwardCacheRestore") {
                return Err(());
            }
            frame(&d, f[0].unwrap(), uri)?;
            Ok(Event::Navigated(regular))
        }
        8 => {
            let f = d.fields(0, ["frame"], 1)?;
            frame(&d, f[0].unwrap(), uri)?;
            Ok(Event::Opened)
        }
        9 => {
            let f = d.fields(0, ["frameId", "url", "navigationType"], 7)?;
            d.id(f[0].unwrap())?;
            d.string(f[1].unwrap(), 1024)?;
            if !["fragment", "historyApi", "other"]
                .iter()
                .any(|s| d.eq(f[2].unwrap(), s))
            {
                return Err(());
            }
            Ok(Event::Within)
        }
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Breakage caught: decoded duplicate keys overwrite an accepted scalar.
    #[test]
    fn chooser_rejects_duplicate_decoded_keys_and_unbound_targets() {
        let good = r#"{"frameId":"F","mode":"selectSingle","backendNodeId":7}"#;
        let expected = Binding {
            frame: Id::parse("F").unwrap(),
            backend: 7,
        };
        assert!(parse_event(0, good, "file:///C:/owned.html", Some(&expected)).is_ok());
        for bad in [
            r#"{"frameId":"F","frame\u0049d":"F","mode":"selectSingle","backendNodeId":7}"#,
            r#"{"frameId":"F","mode":"selectSingle","backendNodeId":7.0}"#,
            r#"{"frameId":"F","mode":"selectSingle","backendNodeId":7e0}"#,
            r#"{"frameId":"F","mode":"selectSingle","backendNodeId":true}"#,
            r#"{"frameId":"F","mode":"selectSingle","backendNodeId":8}"#,
            r#"{"frameId":"F","mode":"selectSingle","backendNodeId":7,"extra":null}"#,
        ] {
            assert!(parse_event(0, bad, "file:///C:/owned.html", Some(&expected)).is_err());
        }
        assert!(parse_event(0, good, "file:///C:/owned.html", None).is_err());
    }

    // Breakage caught: a late or exception-shaped effect becomes success.
    #[test]
    fn cancel_snapshot_requires_exact_remote_profile_and_before_stop_capture() {
        let good = r#"{"result":{"type":"object","value":{"v":1,"kind":"cancel-snapshot","trusted":true,"cancelCount":1,"changeCount":0,"filesCount":0,"valueEmpty":true,"sameInput":true,"connected":true,"sameDocument":true}}}"#;
        assert!(matches!(
            parse_reply(10, good, "file:///C:/owned.html", None, 0),
            Ok(Reply {
                kind: ReplyKind::Canceled,
                frame: None,
                scalar: 0
            })
        ));
        for bad in [
            good.replace("true", "false"),
            good.replace(r#""cancelCount":1"#, r#""cancelCount":2"#),
            good.replace(r#""result":"#, r#""exceptionDetails":null,"result":"#),
            good.replace(
                r#""type":"object""#,
                r#""type":"object","objectId":"handle""#,
            ),
        ] {
            assert!(parse_reply(10, &bad, "file:///C:/owned.html", None, 0).is_err());
        }
        let start = std::time::Instant::now();
        let mut e = Epoch::new(start + std::time::Duration::from_secs(45));
        e.arm();
        e.phase2 = true;
        assert!(e.effect_at(start));
        e.stop_at(start + std::time::Duration::from_millis(1));
        assert!(!e.effect_at(start + std::time::Duration::from_millis(2)));
        assert!(!e.positive());
    }

    // Breakage caught: subsequent snapshots revive an invalidated epoch.
    #[test]
    fn observed_document_invalidation_is_sticky_and_bootstrap_is_bounded() {
        let start = std::time::Instant::now();
        let mut e = Epoch::new(start + std::time::Duration::from_secs(45));
        for _ in 0..16 {
            assert!(e.bootstrap_delivery());
        }
        assert!(!e.bootstrap_delivery());
        e.arm();
        e.phase2 = true;
        assert!(!e.effect_at(start));
        assert!(!e.positive());
        let mut e = Epoch::new(start + std::time::Duration::from_secs(45));
        e.arm();
        e.invalidate(4);
        e.phase2 = true;
        assert!(!e.effect_at(start));
        assert!(!e.target_at(start));
        e.arm();
        assert!(!e.positive());
    }

    // Breakage caught: UTF escapes normalize invalid IDs or a copy exceeds its cap.
    #[test]
    fn fixed_parser_validates_unicode_numbers_and_closed_empty_ack() {
        for bad in [
            "{\"x\":0}",
            "{ }null",
            "null",
            "[]",
            "{\"x\":\"\\u0000\"}",
            "{\"x\":\"\\ud800\"}",
        ] {
            assert!(parse_reply(0, bad, "file:///C:/owned.html", None, 0).is_err());
        }
        assert!(matches!(
            parse_reply(0, " {} ", "file:///C:/owned.html", None, 0),
            Ok(Reply {
                kind: ReplyKind::Ack,
                frame: None,
                scalar: 0
            })
        ));
        assert!(parse_reply(0, &" ".repeat(65), "file:///C:/owned.html", None, 0).is_err());
        assert!(Id::parse("").is_err());
        assert!(Id::parse("bad/id").is_err());
        assert!(Id::parse(&"a".repeat(129)).is_err());
        assert!(text_fits(2048, 512, 384));
        assert!(!text_fits(2048, 2048, 384));
    }

    #[test]
    fn original_deadline_equality_target_orders_and_duplicate_target_are_fatal() {
        let now = Instant::now();
        let tick = std::time::Duration::from_millis(1);
        for effect_first in [false, true] {
            let mut e = Epoch::new(now + tick * 45_000);
            e.arm();
            e.phase2 = true;
            if effect_first {
                assert!(e.effect_at(now));
                assert!(e.target_at(now + tick));
            } else {
                assert!(e.target_at(now));
                assert!(e.effect_at(now + tick));
            }
            e.stop_at(now + tick * 2);
            assert!(e.positive());
            assert!(!e.target_at(now + tick * 3));
            assert!(!e.positive());
        }
        let mut late = Epoch::new(now);
        late.arm();
        late.phase2 = true;
        assert!(!late.effect_at(now));
        assert!(!late.target_at(now));
        let mut missing = Epoch::new(now + tick);
        missing.arm();
        missing.phase2 = true;
        assert!(missing.effect_at(now));
        missing.stop_at(now + tick);
        assert!(!missing.positive());
    }

    #[test]
    fn fixed_parameter_profiles_preserve_all_eleven_methods_and_maximum_ids() {
        let id = Id::parse(&"a".repeat(128)).unwrap();
        for index in 0..11 {
            let (wide, charge) = params(index, Some(id), Some(id), 7).unwrap();
            assert_eq!(wide.last(), Some(&0));
            assert_eq!(wide.len(), charge + 1);
            assert!(charge <= 2048);
            let raw = String::from_utf16(&wide[..charge]).unwrap();
            if index == 5 || index == 10 {
                let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
                assert_eq!(value["uniqueContextId"], id.as_str());
                assert_eq!(
                    value["expression"],
                    if index == 5 { PHASE1 } else { PHASE2 }
                );
                assert_eq!(value["userGesture"], index == 10);
                assert_eq!(value["awaitPromise"], index == 10);
                assert_eq!(value["allowUnsafeEvalBlockedByCSP"], false);
                assert!(value.get("contextId").is_none());
                assert_eq!(charge, if index == 5 { 1085 } else { 1710 });
            }
        }
        assert!(params(4, None, None, 0).is_err());
        assert!(params(10, None, None, 0).is_err());
        assert!(params(7, None, None, 0).is_err());
        assert!(params(8, None, None, 2147483648).is_err());
        assert!(params(11, None, None, 0).is_err());
    }

    #[test]
    fn closed_frame_and_shallow_nodes_accept_only_position_specific_metadata() {
        let uri = "file:///C:/owned.html";
        let frame = r#"{"frameTree":{"frame":{"id":"F","loaderId":"L","url":"file:///C:/owned.html","domainAndRegistry":"","securityOrigin":"file://","mimeType":"text/html","secureContextType":"Secure","crossOriginIsolatedContextType":"NotIsolated","gatedAPIFeatures":[],"securityOriginDetails":{"isLocalhost":false},"adFrameStatus":{"adFrameType":"none","explanations":[]}},"childFrames":[]}}"#;
        assert!(matches!(
            parse_reply(3, frame, uri, None, 0),
            Ok(Reply {
                kind: ReplyKind::Tree,
                frame: Some(_),
                scalar: 0
            })
        ));
        assert!(parse_reply(9, frame, "file:///C:/other.html", None, 0).is_err());
        let root = r##"{"root":{"nodeId":1,"backendNodeId":2,"nodeType":9,"nodeName":"#document","localName":"","nodeValue":"","documentURL":"file:///C:/owned.html","xmlVersion":"","children":[{"nodeId":3,"backendNodeId":4,"nodeType":1,"nodeName":"HTML","localName":"html","nodeValue":"","parentId":1,"frameId":"F"}]}}"##;
        let owner = Some(Id::parse("F").unwrap());
        assert!(matches!(
            parse_reply(6, root, uri, owner, 0),
            Ok(Reply {
                kind: ReplyKind::Root,
                frame: None,
                scalar: 1
            })
        ));
        assert!(
            parse_reply(
                6,
                &root.replace(r#""frameId":"F""#, r#""frameId":"G""#),
                uri,
                owner,
                0
            )
            .is_err()
        );
        assert!(
            parse_reply(
                6,
                &root.replace(r#""xmlVersion":"""#, r#""xmlVersion":"1.0""#),
                uri,
                owner,
                0
            )
            .is_err()
        );
        let input = r##"{"node":{"nodeId":7,"backendNodeId":8,"nodeType":1,"nodeName":"INPUT","localName":"input","nodeValue":"","attributes":["type","file","id","code0-s3-file"],"shadowRoots":[{"nodeId":0,"backendNodeId":9,"nodeType":11,"nodeName":"#document-fragment","localName":"","nodeValue":"","shadowRootType":"user-agent","childNodeCount":16}]}}"##;
        assert!(matches!(
            parse_reply(8, input, uri, owner, 7),
            Ok(Reply {
                kind: ReplyKind::Input,
                frame: None,
                scalar: 8
            })
        ));
        for bad in [
            input.replace(r#""nodeId":7"#, r#""nodeId":0"#),
            input.replace(r#""nodeId":7"#, r#""nodeId":7,"frameId":"F""#),
            input.replace("user-agent", "open"),
            input.replace(r#""childNodeCount":16"#, r#""childNodeCount":17"#),
        ] {
            assert!(parse_reply(8, &bad, uri, owner, 7).is_err());
        }
    }

    #[test]
    fn optional_remote_labels_context_aux_and_encoding_are_closed() {
        let good = r#"{"result":{"type":"object","value":{"v":1,"kind":"prepared"},"className":"Object","description":"Object"}}"#;
        assert!(matches!(
            parse_reply(5, good, "", None, 0),
            Ok(Reply {
                kind: ReplyKind::Prepared,
                frame: None,
                scalar: 0
            })
        ));
        for bad in [
            good.replace("Object", "Array"),
            good.replace(r#""className":"Object""#, r#""className":null"#),
            good.replace(
                r#""type":"object""#,
                r#""type":"object","ty\u0070e":"object""#,
            ),
        ] {
            assert!(parse_reply(5, &bad, "", None, 0).is_err());
        }
        let binding = Binding {
            frame: Id::parse("F").unwrap(),
            backend: 7,
        };
        let context = r#"{"context":{"id":3,"origin":"","name":"code0-s3-file-world-v1","uniqueId":"U","auxData":{"isDefault":false,"frameId":"F","type":"isolated"}}}"#;
        assert!(matches!(
            parse_event(1, context, "", Some(&binding)),
            Ok(Event::Created(_))
        ));
        for bad in [
            context.replace(r#""frameId":"F""#, r#""frameId":"G""#),
            context.replace("isolated\"", "worker\""),
            context.replace(r#""id":3"#, r#""id":2147483648"#),
            context.replace(r#""uniqueId":"U""#, r#""uniqueId":"\ud800""#),
        ] {
            assert!(parse_event(1, &bad, "", Some(&binding)).is_err());
        }
        assert!(text_fits(2048, 2048, 0));
        assert!(!text_fits(2049, 0, 0));
        assert!(!text_fits(0, 0, usize::MAX));
    }

    #[test]
    fn uri_construction_checks_encoded_upper_bound_before_allocation() {
        assert!(uri_construction_fits(0));
        assert!(uri_construction_fits(677));
        assert!(!uri_construction_fits(678));
        assert!(!uri_construction_fits(usize::MAX));
    }

    #[test]
    fn unrelated_revocation_before_stop_cannot_be_repaired_by_late_target() {
        let now = Instant::now();
        let tick = std::time::Duration::from_millis(1);
        for effect_first in [false, true] {
            let mut epoch = Epoch::new(now + tick * 45_000);
            epoch.arm();
            epoch.phase2 = true;
            if effect_first {
                assert!(epoch.effect_at(now));
            }
            assert!(epoch.stop.is_none());
            // An actual non-chooser refusal has revoked the owner, before Stop.
            epoch.observe_revocation(true, false);
            epoch.observe_revocation(false, false);
            assert!(!epoch.target_at(now + tick));
            if !effect_first {
                assert!(!epoch.effect_at(now + tick));
            }
            epoch.stop_at(now + tick * 2);
            assert!(!epoch.positive());
        }
        let mut invented_cause = Epoch::new(now + tick);
        invented_cause.arm();
        invented_cause.phase2 = true;
        invented_cause.observe_revocation(true, true);
        assert!(!invented_cause.target_at(now));
    }

    #[test]
    fn only_matching_chooser_closing_preserves_both_pre_stop_orders() {
        let now = Instant::now();
        let tick = std::time::Duration::from_millis(1);
        for effect_first in [false, true] {
            let mut epoch = Epoch::new(now + tick * 45_000);
            epoch.arm();
            epoch.phase2 = true;
            epoch.observe_revocation(false, false);
            if effect_first {
                assert!(epoch.effect_at(now));
            }
            assert!(epoch.target_at(now + tick));
            epoch.observe_revocation(true, true);
            if !effect_first {
                assert!(epoch.effect_at(now + tick * 2));
            }
            assert!(epoch.stop.is_none());
            epoch.stop_at(now + tick * 3);
            assert!(epoch.positive());
            // A later actual unrelated refusal still poisons prior positives.
            epoch.observe_revocation(true, false);
            epoch.observe_revocation(true, true);
            assert!(!epoch.positive());
        }
        let mut revoked_after_target = Epoch::new(now + tick * 45_000);
        revoked_after_target.arm();
        revoked_after_target.phase2 = true;
        assert!(revoked_after_target.target_at(now));
        revoked_after_target.observe_revocation(true, true);
        assert!(revoked_after_target.effect_at(now + tick));
        assert!(revoked_after_target.stop.is_none());
        revoked_after_target.observe_revocation(true, false);
        revoked_after_target.stop_at(now + tick * 2);
        assert!(!revoked_after_target.positive());
    }

    #[test]
    fn inline_reply_storage_preserves_frame_and_numeric_payloads() {
        let uri = "file:///C:/owned.html";
        let tree = r#"{"frameTree":{"frame":{"id":"F","loaderId":"L","url":"file:///C:/owned.html","domainAndRegistry":"","securityOrigin":"file://","mimeType":"text/html","secureContextType":"Secure","crossOriginIsolatedContextType":"NotIsolated","gatedAPIFeatures":[]}}}"#;
        match parse_reply(3, tree, uri, None, 0).unwrap() {
            Reply {
                kind: ReplyKind::Tree,
                frame: Some(Frame { id, loader }),
                scalar: 0,
            } => {
                assert_eq!(id.as_str(), "F");
                assert_eq!(loader.as_str(), "L");
            }
            _ => panic!("tree reply did not retain its required inline frame"),
        }
        assert!(parse_reply(3, &tree.replace(r#""id":"F","#, ""), uri, None, 0).is_err());
        assert!(matches!(
            parse_reply(4, r#"{"executionContextId":7}"#, uri, None, 0),
            Ok(Reply {
                kind: ReplyKind::World,
                frame: None,
                scalar: 7
            })
        ));
        for bad in [
            r#"{"executionContextId":0}"#,
            r#"{"executionContextId":2147483648}"#,
            r#"{"executionContextId":1e0}"#,
        ] {
            assert!(parse_reply(4, bad, uri, None, 0).is_err());
        }
    }
}
