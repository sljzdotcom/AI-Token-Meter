//! Bounded permission protocol state only; no native calls, JSON parser or integration.
//! Tests were defined before implementation and are UNRUN. Native callers must supply
//! real, strictly decoded facts at the actual boundaries documented by each method.

pub const FIXED_URL: &str =
    "https://console.code0.ai/__ai_meter_fixture__/permission-notifications-v1";
const ATTEMPT_MS: u64 = 45_000;
const CLOSE_MS: u64 = 10_000;
const COVERAGE_COUNT: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub request: u64,
    pub generation: u64,
    pub installation: u64,
    pub core: u64,
}
impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Identity(<opaque>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Boundary {
    pub identity: Identity,
    /// Milliseconds on one monotonic clock, measured at this actual boundary.
    pub ms: u64,
}

/// No heap allocation or raw/native identifier in Debug output.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Id {
    bytes: [u8; 128],
    len: u8,
}
impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Id(<redacted>)")
    }
}
impl Id {
    pub fn parse(value: &str) -> Result<Self, Problem> {
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
        {
            return Err(Problem::Shape);
        }
        let mut bytes = [0; 128];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            bytes,
            len: value.len() as u8,
        })
    }
    /// Only the bounded request serializer may use this; never log it.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)])
            .expect("Id's private constructor accepts ASCII only")
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Context {
    pub frame: Id,
    numeric: u32,
    unique: Id,
}
impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Context(<redacted>)")
    }
}
impl Context {
    pub fn new(frame: Id, numeric: u32, unique: Id) -> Result<Self, Problem> {
        if !(1..=2_147_483_647).contains(&numeric) {
            return Err(Problem::Shape);
        }
        Ok(Self {
            frame,
            numeric,
            unique,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub id: Id,
    pub loader: Id,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Document {
    Blank,
    Selected,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Receiver {
    ContextCreated,
    ContextDestroyed,
    ContextsCleared,
    FrameAttached,
    FrameDetached,
    FrameNavigated,
    DocumentOpened,
    WithinDocument,
    StartedLoading,
    StoppedLoading,
}
impl Receiver {
    pub const ALL: [Self; 10] = [
        Self::ContextCreated,
        Self::ContextDestroyed,
        Self::ContextsCleared,
        Self::FrameAttached,
        Self::FrameDetached,
        Self::FrameNavigated,
        Self::DocumentOpened,
        Self::WithinDocument,
        Self::StartedLoading,
        Self::StoppedLoading,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::ContextCreated => "Runtime.executionContextCreated",
            Self::ContextDestroyed => "Runtime.executionContextDestroyed",
            Self::ContextsCleared => "Runtime.executionContextsCleared",
            Self::FrameAttached => "Page.frameAttached",
            Self::FrameDetached => "Page.frameDetached",
            Self::FrameNavigated => "Page.frameNavigated",
            Self::DocumentOpened => "Page.documentOpened",
            Self::WithinDocument => "Page.navigatedWithinDocument",
            Self::StartedLoading => "Page.frameStartedLoading",
            Self::StoppedLoading => "Page.frameStoppedLoading",
        }
    }
}
/// Input only after the caller strictly validates the complete fix2 JSON profile,
/// Args2 empty SessionId, UI affinity, emitter and copy/token/depth budgets.
/// A profile/decoding failure must be delivered as Unsupported, never omitted.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    ContextCreated(Document, Context),
    ContextDestroyed { numeric: u32, unique: Id },
    ContextsCleared,
    FrameNavigated(Document, Frame),
    StartedLoading(Id),
    StoppedLoading(Id),
    Unsupported(Receiver),
}
impl std::fmt::Debug for Lifecycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Lifecycle(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageKey {
    S2Hook(u8),
    Receiver(Receiver),
    NavigationCompleted,
    ResourceRequested,
    AllSourceFilter,
    CoreMessage,
}
impl CoverageKey {
    fn index(self) -> Result<usize, Problem> {
        match self {
            Self::S2Hook(index) if index < 10 => Ok(usize::from(index)),
            Self::S2Hook(_) => Err(Problem::Shape),
            Self::Receiver(receiver) => Ok(10 + receiver as usize),
            Self::NavigationCompleted => Ok(20),
            Self::ResourceRequested => Ok(21),
            Self::AllSourceFilter => Ok(22),
            Self::CoreMessage => Ok(23),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NativeToken {
    Event(i64),
    Filter,
}
impl std::fmt::Debug for NativeToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Event(_) => "Event(<redacted>)",
            Self::Filter => "Filter",
        })
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct Registration {
    token: Option<NativeToken>,
    removal_sequence: Option<u64>,
    removal_returned: bool,
    removed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemovalTicket {
    identity: Identity,
    index: usize,
    sequence: u64,
    pub token: NativeToken,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counter {
    pub value: u16,
    pub overflowed: bool,
}
impl Counter {
    fn increment(&mut self) {
        if let Some(value) = self.value.checked_add(1) {
            self.value = value;
        } else {
            self.overflowed = true;
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Registering,
    Enabling,
    Navigating,
    Armed,
    Issued,
    Closing,
    Cutoff,
    Released,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlankBranch {
    ZeroObserved,
    Observed,
    /// Actual bound initial context+Stop, with no initial Start observation.
    /// This never supplies a selected loading fact or actual context retirement.
    ObservedInitialStopOnly,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    NotReady,
    Shape,
    Identity,
    Affinity,
    Clock,
    Deadline,
    CloseDeadline,
    Coverage,
    Phase,
    Replay,
    Root,
    Context,
    Lifecycle,
    Budget,
    Native,
    Result,
    Released,
    Overflow,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutoffReason {
    RequiredRemovalAttempt,
    CoverageLost,
    ControlsDisposal,
    StopEntry,
    ControllerCloseEntry,
    AffinityLost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cutoff {
    pub reason: CutoffReason,
    pub sequence: Option<u64>,
    pub ms: Option<u64>,
}
#[derive(Clone, Copy, Debug)]
struct Mark {
    sequence: u64,
    epoch: u64,
    positive: bool,
}
#[derive(Clone, Copy, Debug, Default)]
struct DocumentRecord {
    root: Option<Id>,
    frame: Option<Frame>,
    context: Option<Context>,
    started: bool,
    stopped: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    PageEnable,
    RuntimeEnable,
    GetFrameTree,
    Evaluate,
}
impl Command {
    pub fn name(self) -> &'static str {
        match self {
            Self::PageEnable => "Page.enable",
            Self::RuntimeEnable => "Runtime.enable",
            Self::GetFrameTree => "Page.getFrameTree",
            Self::Evaluate => "Runtime.evaluate",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandTicket {
    identity: Identity,
    command: Command,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackTicket {
    command: CommandTicket,
    entry_sequence: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub ticket: CommandTicket,
    pub unique_context: Id,
}
/// These semantic tags are NOT a JSON parser or native callback evidence. Denied
/// is legal only after the exact bounded closed reply/token-spelling parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionValue<'a> {
    EmptyAck,
    FrameTree(&'a Frame),
    Denied,
    Precondition,
    Default,
    Granted,
    DocumentChanged,
    Invalid,
    Malformed,
}
#[derive(Clone, Copy, Debug, Default)]
struct Operation {
    issued: bool,
    returned: Option<bool>,
    handler_live: bool,
    entry: Option<Mark>,
    capture: Option<Mark>,
    semantic_valid: bool,
    qualified: Option<bool>,
    consumed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NativeCheck {
    KindNotifications,
    NonUser,
    Origin,
    BeforeDefault,
    DenyAttempted,
    DenyReturned,
    DenySucceeded,
    AfterDeny,
}
impl NativeCheck {
    pub const ALL: [Self; 8] = [
        Self::KindNotifications,
        Self::NonUser,
        Self::Origin,
        Self::BeforeDefault,
        Self::DenyAttempted,
        Self::DenyReturned,
        Self::DenySucceeded,
        Self::AfterDeny,
    ];
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeTicket {
    identity: Identity,
    sequence: u64,
}
#[derive(Clone, Copy, Debug)]
struct NativeRecord {
    entry: Mark,
    checks: u8,
    all_positive: bool,
    refused: bool,
    qualified: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub phase: Phase,
    pub blank_branch: Option<BlankBranch>,
    pub failure: Option<Problem>,
    pub epoch: u64,
    pub action_revoked: bool,
    pub first_close_ms: Option<u64>,
    pub cutoff: Option<Cutoff>,
    pub deliveries: Counter,
    pub bootstrap_deliveries: Counter,
    pub disposal_deliveries: Counter,
    pub obligations: u8,
    pub command_handlers: u8,
    pub owned_registrations: u8,
    /// None is unobserved; false is an actually finished, ineligible candidate.
    pub native_qualified: Option<bool>,
    pub effect_qualified: Option<bool>,
}

/// One immutable native request/install/core incarnation, never reset or cloned.
/// This protocol owns bounded state, not COM references or Session events. Its
/// booleans mean supplied boundary facts, never claims of actual native execution.
/// Caller owns monotonic-clock origin and globally unique identity allocation.
/// Keep caller's real Active/handler/text inventories independent of this ledger.
pub struct Protocol {
    identity: Identity,
    deadline_ms: u64,
    last_ms: u64,
    sequence: u64,
    epoch: u64,
    phase: Phase,
    failure: Option<Problem>,
    revoked: bool,
    first_close_ms: Option<u64>,
    cutoff: Option<Cutoff>,
    physical_entry: bool,
    registrations: [Registration; COVERAGE_COUNT],
    settings: bool,
    blank: DocumentRecord,
    selected: DocumentRecord,
    branch: Option<BlankBranch>,
    blank_destroyed: bool,
    cleared: bool,
    navigation_return: Option<bool>,
    navigation_start: Option<u64>,
    navigation_complete: Option<u64>,
    main_response: Option<bool>,
    source_matched: bool,
    operations: [Operation; 4],
    active: Option<Command>,
    native: Option<NativeRecord>,
    deliveries: Counter,
    bootstrap_deliveries: Counter,
    disposal_deliveries: Counter,
}
impl Protocol {
    pub fn new(identity: Identity, attempt_started_ms: u64) -> Result<Self, Problem> {
        let deadline_ms = attempt_started_ms
            .checked_add(ATTEMPT_MS)
            .ok_or(Problem::Overflow)?;
        Ok(Self {
            identity,
            deadline_ms,
            last_ms: attempt_started_ms,
            sequence: 0,
            epoch: 0,
            phase: Phase::Registering,
            failure: None,
            revoked: false,
            first_close_ms: None,
            cutoff: None,
            physical_entry: false,
            registrations: [Registration::default(); COVERAGE_COUNT],
            settings: false,
            blank: DocumentRecord::default(),
            selected: DocumentRecord::default(),
            branch: None,
            blank_destroyed: false,
            cleared: false,
            navigation_return: None,
            navigation_start: None,
            navigation_complete: None,
            main_response: None,
            source_matched: false,
            operations: [Operation::default(); 4],
            active: None,
            native: None,
            deliveries: Counter::default(),
            bootstrap_deliveries: Counter::default(),
            disposal_deliveries: Counter::default(),
        })
    }
    fn revoke(&mut self, ms: u64) {
        self.revoked = true;
        if self.first_close_ms.is_none() {
            self.first_close_ms = Some(ms);
        }
        if self.phase != Phase::Released && self.cutoff.is_none() {
            self.phase = Phase::Closing;
        }
    }
    fn fail(&mut self, problem: Problem, ms: u64) {
        if self.failure.is_none() {
            self.failure = Some(problem);
        }
        self.epoch = 1;
        self.revoke(ms);
    }
    fn reject<T>(&mut self, problem: Problem, ms: u64) -> Result<T, Problem> {
        self.fail(problem, ms);
        Err(problem)
    }
    fn coverage_intact(&self) -> bool {
        self.registrations
            .iter()
            .all(|r| r.token.is_some() && r.removal_sequence.is_none())
    }
    fn close_timely(&self, ms: u64) -> bool {
        self.first_close_ms.is_none_or(|start| {
            ms.checked_sub(start)
                .is_some_and(|elapsed| elapsed < CLOSE_MS)
        })
    }
    fn mark(&mut self, boundary: Boundary) -> Result<Mark, Problem> {
        self.boundary_mark(boundary, true)
    }
    fn boundary_mark(
        &mut self,
        boundary: Boundary,
        positive_deadline: bool,
    ) -> Result<Mark, Problem> {
        if boundary.identity != self.identity {
            return self.reject(Problem::Identity, boundary.ms);
        }
        if self.phase == Phase::Released {
            return self.reject(Problem::Released, boundary.ms);
        }
        if boundary.ms < self.last_ms {
            return self.reject(Problem::Clock, self.last_ms);
        }
        self.last_ms = boundary.ms;
        self.sequence = match self.sequence.checked_add(1) {
            Some(sequence) => sequence,
            None => return self.reject(Problem::Overflow, boundary.ms),
        };
        if positive_deadline && boundary.ms >= self.deadline_ms {
            self.fail(Problem::Deadline, boundary.ms);
        }
        if !self.close_timely(boundary.ms) {
            self.fail(Problem::CloseDeadline, boundary.ms);
        }
        Ok(Mark {
            sequence: self.sequence,
            epoch: self.epoch,
            positive: boundary.ms < self.deadline_ms
                && self.close_timely(boundary.ms)
                && self.failure.is_none()
                && self.cutoff.is_none()
                && !self.physical_entry
                && self.coverage_intact()
                && self.settings,
        })
    }
    fn action_live(&self, mark: Mark) -> bool {
        mark.positive && !self.revoked && mark.epoch == self.epoch
    }
    fn disposal_gate(&mut self, boundary: Boundary) -> Result<Mark, Problem> {
        let mark = self.boundary_mark(boundary, false)?;
        if !self.close_timely(boundary.ms) {
            return Err(Problem::CloseDeadline);
        }
        Ok(mark)
    }
    /// Call after a real successful registration and all relevant native Drops.
    /// A failed/uncertain add must instead latch CoverageLost; never feed a token
    /// inferred from an expected inventory. Actual retained handlers remain owned
    /// by the caller even if this setup cannot become ready.
    pub fn registration_added(
        &mut self,
        key: CoverageKey,
        token: NativeToken,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.mark(boundary)?;
        let index = match key.index() {
            Ok(index) => index,
            Err(e) => return self.reject(e, boundary.ms),
        };
        if self.phase != Phase::Registering || self.revoked {
            return self.reject(Problem::Phase, boundary.ms);
        }
        if (key == CoverageKey::AllSourceFilter) != (token == NativeToken::Filter) {
            return self.reject(Problem::Shape, boundary.ms);
        }
        if self.registrations[index].token.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.registrations[index].token = Some(token);
        Ok(())
    }
    pub fn settings_acknowledged(&mut self, boundary: Boundary) -> Result<(), Problem> {
        self.mark(boundary)?;
        if self.phase != Phase::Registering || self.revoked {
            return self.reject(Problem::Phase, boundary.ms);
        }
        if self.settings {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.settings = true;
        Ok(())
    }
    fn latch_cutoff(&mut self, reason: CutoffReason, mark: Mark, ms: u64) -> Cutoff {
        let value = *self.cutoff.get_or_insert(Cutoff {
            reason,
            sequence: Some(mark.sequence),
            ms: Some(ms),
        });
        self.epoch = 1;
        self.revoke(ms);
        if self.phase != Phase::Released {
            self.phase = Phase::Cutoff;
        }
        value
    }
    /// Invoke before any removal reservation, action-local native clone, call or
    /// potentially reentrant Drop. It never waits for a completion or clears it.
    pub fn cutoff(&mut self, reason: CutoffReason, boundary: Boundary) -> Cutoff {
        if reason == CutoffReason::AffinityLost {
            return self.affinity_lost();
        }
        let mark = match self.boundary_mark(boundary, false) {
            Ok(mark) => mark,
            Err(_) => {
                // Loss without a qualified UI boundary cannot invent an order/time.
                return *self.cutoff.get_or_insert(Cutoff {
                    reason: CutoffReason::CoverageLost,
                    sequence: None,
                    ms: None,
                });
            }
        };
        if reason == CutoffReason::CoverageLost {
            self.fail(Problem::Coverage, boundary.ms);
        }
        if matches!(
            reason,
            CutoffReason::StopEntry
                | CutoffReason::ControllerCloseEntry
                | CutoffReason::ControlsDisposal
        ) {
            self.physical_entry = true;
        }
        self.latch_cutoff(reason, mark, self.last_ms)
    }
    /// UI-side recording of an already atomically published affinity loss. The
    /// native caller must publish its thread-safe failure/cutoff before ANY UI
    /// access. This method is not a cross-thread COM/RefCell synchronization API.
    /// There is deliberately no invented UI sequence/time for that loss.
    pub fn affinity_lost(&mut self) -> Cutoff {
        if self.failure.is_none() {
            self.failure = Some(Problem::Affinity);
        }
        self.epoch = 1;
        self.revoked = true;
        let value = *self.cutoff.get_or_insert(Cutoff {
            reason: CutoffReason::AffinityLost,
            sequence: None,
            ms: None,
        });
        if self.phase != Phase::Released {
            self.phase = Phase::Cutoff;
        }
        value
    }
    pub fn removal_entered(
        &mut self,
        key: CoverageKey,
        boundary: Boundary,
    ) -> Result<RemovalTicket, Problem> {
        let mark = self.boundary_mark(boundary, false)?;
        self.latch_cutoff(CutoffReason::RequiredRemovalAttempt, mark, boundary.ms);
        let index = match key.index() {
            Ok(index) => index,
            Err(e) => return self.reject(e, boundary.ms),
        };
        let r = self.registrations[index];
        let Some(token) = r.token else {
            return self.reject(Problem::Coverage, boundary.ms);
        };
        if r.removal_sequence.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.registrations[index].removal_sequence = Some(mark.sequence);
        Ok(RemovalTicket {
            identity: self.identity,
            index,
            sequence: mark.sequence,
            token,
        })
    }
    pub fn removal_returned(
        &mut self,
        ticket: RemovalTicket,
        success: bool,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.boundary_mark(boundary, false)?;
        if ticket.identity != self.identity || ticket.index >= COVERAGE_COUNT {
            return self.reject(Problem::Identity, boundary.ms);
        }
        let r = self.registrations[ticket.index];
        if r.token != Some(ticket.token)
            || r.removal_sequence != Some(ticket.sequence)
            || r.removal_returned
        {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.registrations[ticket.index].removal_returned = true;
        self.registrations[ticket.index].removed = success;
        if !success {
            self.fail(Problem::Coverage, boundary.ms);
        }
        Ok(())
    }
    fn join_root(record: &mut DocumentRecord, root: Id) -> Result<(), Problem> {
        if record.root.is_some_and(|previous| previous != root) {
            return Err(Problem::Root);
        }
        record.root = Some(root);
        Ok(())
    }
    fn blank_fact(&mut self, event: Lifecycle) -> Result<(), Problem> {
        if self.branch.is_some() {
            return Err(Problem::Lifecycle);
        }
        match event {
            Lifecycle::ContextCreated(Document::Blank, context) => {
                if !self.operations[Command::RuntimeEnable as usize].issued {
                    return Err(Problem::Phase);
                }
                if self.blank.context.is_some() {
                    return Err(Problem::Replay);
                }
                Self::join_root(&mut self.blank, context.frame)?;
                self.blank.context = Some(context);
            }
            Lifecycle::FrameNavigated(Document::Blank, frame) => {
                if self.blank.frame.is_some() {
                    return Err(Problem::Replay);
                }
                Self::join_root(&mut self.blank, frame.id)?;
                self.blank.frame = Some(frame);
            }
            Lifecycle::StartedLoading(root) => {
                if self.blank.started {
                    return Err(Problem::Replay);
                }
                Self::join_root(&mut self.blank, root)?;
                self.blank.started = true;
            }
            Lifecycle::StoppedLoading(root) => {
                if self.blank.stopped {
                    return Err(Problem::Replay);
                }
                Self::join_root(&mut self.blank, root)?;
                self.blank.stopped = true;
            }
            _ => return Err(Problem::Lifecycle),
        }
        Ok(())
    }
    fn selected_fact(&mut self, event: Lifecycle) -> Result<(), Problem> {
        match event {
            Lifecycle::ContextCreated(Document::Selected, context) => {
                if self.selected.context.is_some() {
                    return Err(Problem::Replay);
                }
                if self
                    .blank
                    .context
                    .is_some_and(|blank| blank.unique == context.unique)
                {
                    return Err(Problem::Context);
                }
                Self::join_root(&mut self.selected, context.frame)?;
                self.selected.context = Some(context);
            }
            Lifecycle::FrameNavigated(Document::Selected, frame) => {
                if self.selected.frame.is_some() {
                    return Err(Problem::Replay);
                }
                if self
                    .blank
                    .frame
                    .is_some_and(|blank| blank.loader == frame.loader)
                {
                    return Err(Problem::Root);
                }
                Self::join_root(&mut self.selected, frame.id)?;
                self.selected.frame = Some(frame);
            }
            Lifecycle::StartedLoading(root) => {
                if self.selected.started {
                    return Err(Problem::Replay);
                }
                Self::join_root(&mut self.selected, root)?;
                self.selected.started = true;
            }
            Lifecycle::StoppedLoading(root) => {
                if self.selected.stopped {
                    return Err(Problem::Replay);
                }
                Self::join_root(&mut self.selected, root)?;
                self.selected.stopped = true;
            }
            Lifecycle::ContextsCleared => {
                if self.cleared || self.selected.context.is_some() {
                    return Err(Problem::Lifecycle);
                }
                self.cleared = true;
            }
            Lifecycle::ContextDestroyed { numeric, unique } => {
                if self.blank_destroyed
                    || !self
                        .blank
                        .context
                        .is_some_and(|blank| blank.numeric == numeric && blank.unique == unique)
                {
                    return Err(Problem::Context);
                }
                self.blank_destroyed = true;
            }
            _ => return Err(Problem::Lifecycle),
        }
        if matches!(
            self.branch,
            Some(BlankBranch::Observed | BlankBranch::ObservedInitialStopOnly)
        ) && self.selected.root.is_some()
            && self.selected.root != self.blank.root
        {
            return Err(Problem::Root);
        }
        Ok(())
    }
    /// Count each actual delivery before validity. No raw JSON is retained.
    /// Loading means post-reservation selected-F observations ONLY; it does not
    /// attest loader provenance when an unobserved late blank shares that F.
    pub fn lifecycle(&mut self, event: Lifecycle, boundary: Boundary) -> Result<(), Problem> {
        self.deliveries.increment();
        let bootstrap = matches!(
            self.phase,
            Phase::Registering | Phase::Enabling | Phase::Navigating
        );
        if bootstrap {
            self.bootstrap_deliveries.increment();
        } else {
            self.disposal_deliveries.increment();
        }
        let mark = self.boundary_mark(boundary, self.cutoff.is_none())?;
        if self.deliveries.overflowed
            || self.bootstrap_deliveries.overflowed
            || self.disposal_deliveries.overflowed
            || self.bootstrap_deliveries.value > 16
        {
            return self.reject(Problem::Budget, boundary.ms);
        }
        if self.cutoff.is_some() {
            return if matches!(event, Lifecycle::Unsupported(_)) {
                self.reject(Problem::Lifecycle, boundary.ms)
            } else {
                // Strictly decoded delivery only: no adoption, retirement or epoch repair.
                Ok(())
            };
        }
        if !mark.positive {
            return self.reject(Problem::Lifecycle, boundary.ms);
        }
        let result = match self.phase {
            Phase::Enabling => self.blank_fact(event),
            Phase::Navigating => self.selected_fact(event),
            _ => Err(Problem::Lifecycle),
        };
        if let Err(problem) = result {
            return self.reject(problem, boundary.ms);
        }
        Ok(())
    }
    pub fn navigation_reserved(&mut self, boundary: Boundary) -> Result<(), Problem> {
        let mark = self.mark(boundary)?;
        if !self.action_live(mark)
            || self.phase != Phase::Enabling
            || !self.acknowledged(Command::RuntimeEnable)
            || self.active.is_some()
        {
            return Err(Problem::NotReady);
        }
        if self.branch.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        if self.blank.root.is_some() {
            if self.blank.context.is_none() {
                return Err(Problem::NotReady);
            }
            self.branch = Some(match (self.blank.started, self.blank.stopped) {
                (true, false) => return Err(Problem::NotReady),
                (false, true) => BlankBranch::ObservedInitialStopOnly,
                (false, false) | (true, true) => BlankBranch::Observed,
            });
        } else {
            self.branch = Some(BlankBranch::ZeroObserved);
        }
        self.phase = Phase::Navigating;
        Ok(())
    }
    fn navigation_gate(&mut self, boundary: Boundary) -> Result<(), Problem> {
        let mark = self.mark(boundary)?;
        if !self.action_live(mark) || self.phase != Phase::Navigating {
            return self.reject(Problem::Phase, boundary.ms);
        }
        Ok(())
    }
    pub fn navigation_started(
        &mut self,
        id: u64,
        url: &str,
        redirect: bool,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.navigation_gate(boundary)?;
        if self.navigation_start.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        if url != FIXED_URL || redirect {
            return self.reject(Problem::Native, boundary.ms);
        }
        self.navigation_start = Some(id);
        Ok(())
    }
    pub fn navigation_completed(
        &mut self,
        id: u64,
        success: bool,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.navigation_gate(boundary)?;
        if self.navigation_complete.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        if self.navigation_start != Some(id) || !success {
            return self.reject(Problem::Native, boundary.ms);
        }
        self.navigation_complete = Some(id);
        Ok(())
    }
    pub fn navigation_returned(
        &mut self,
        success: bool,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.navigation_gate(boundary)?;
        if self.navigation_return.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.navigation_return = Some(success);
        if !success {
            return self.reject(Problem::Native, boundary.ms);
        }
        Ok(())
    }
    pub fn main_response_installed(
        &mut self,
        success: bool,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.navigation_gate(boundary)?;
        if self.main_response.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.main_response = Some(success);
        if !success {
            return self.reject(Problem::Native, boundary.ms);
        }
        Ok(())
    }
    fn non_tree_ready(&self) -> bool {
        let blank_closed = match self.branch {
            Some(BlankBranch::ZeroObserved) => self.blank.root.is_none(),
            Some(BlankBranch::Observed) => {
                self.blank.context.is_some()
                    && (self.blank_destroyed || self.cleared)
                    && self.blank.started == self.blank.stopped
            }
            Some(BlankBranch::ObservedInitialStopOnly) => {
                self.blank.context.is_some()
                    && (self.blank_destroyed || self.cleared)
                    && !self.blank.started
                    && self.blank.stopped
            }
            None => false,
        };
        blank_closed
            && self.selected.context.is_some()
            && self.selected.frame.is_some()
            && self.selected.started
            && self.selected.stopped
            && self.navigation_return == Some(true)
            && self.navigation_start.is_some()
            && self.navigation_complete == self.navigation_start
            && self.main_response == Some(true)
    }
    fn acknowledged(&self, command: Command) -> bool {
        let op = self.operations[command as usize];
        op.returned == Some(true) && op.qualified == Some(true) && op.consumed
    }
    pub fn source_after_navigation(
        &mut self,
        source: &str,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.navigation_gate(boundary)?;
        if self.source_matched {
            return self.reject(Problem::Replay, boundary.ms);
        }
        if !self.non_tree_ready() || !self.acknowledged(Command::GetFrameTree) {
            return Err(Problem::NotReady);
        }
        if source != FIXED_URL {
            return self.reject(Problem::Native, boundary.ms);
        }
        self.source_matched = true;
        Ok(())
    }
    pub fn arm(&mut self, boundary: Boundary) -> Result<(), Problem> {
        let mark = self.mark(boundary)?;
        if !self.action_live(mark)
            || self.phase != Phase::Navigating
            || !self.non_tree_ready()
            || !self.source_matched
            || !self.acknowledged(Command::GetFrameTree)
            || self.active.is_some()
        {
            return Err(Problem::NotReady);
        }
        self.phase = Phase::Armed;
        Ok(())
    }
    fn enter_command(
        &mut self,
        command: Command,
        mark: Mark,
        ms: u64,
    ) -> Result<CommandTicket, Problem> {
        if self.operations[command as usize].issued {
            return self.reject(Problem::Replay, ms);
        }
        if !self.action_live(mark) || self.active.is_some() {
            return Err(Problem::NotReady);
        }
        let ready = match command {
            Command::PageEnable => self.phase == Phase::Registering,
            Command::RuntimeEnable => {
                self.phase == Phase::Enabling && self.acknowledged(Command::PageEnable)
            }
            Command::GetFrameTree => {
                self.phase == Phase::Navigating
                    && self.non_tree_ready()
                    && self.acknowledged(Command::RuntimeEnable)
            }
            Command::Evaluate => {
                self.phase == Phase::Armed
                    && self.non_tree_ready()
                    && self.source_matched
                    && self.acknowledged(Command::GetFrameTree)
            }
        };
        if !ready {
            return Err(Problem::NotReady);
        }
        self.operations[command as usize] = Operation {
            issued: true,
            handler_live: true,
            ..Operation::default()
        };
        self.active = Some(command);
        if command == Command::PageEnable {
            self.phase = Phase::Enabling;
        }
        if command == Command::Evaluate {
            self.phase = Phase::Issued;
        }
        Ok(CommandTicket {
            identity: self.identity,
            command,
        })
    }
    /// Reserve immediately at actual call entry, before any synchronous callback.
    /// No intervening native call/Drop is permitted before the intended method.
    /// A failed submission consumes this sole ticket and cannot be retried.
    pub fn command_entered(
        &mut self,
        command: Command,
        boundary: Boundary,
    ) -> Result<CommandTicket, Problem> {
        let mark = self.mark(boundary)?;
        if command == Command::Evaluate {
            return self.reject(Problem::Phase, boundary.ms);
        }
        self.enter_command(command, mark, boundary.ms)
    }
    /// Copy-only preparation hint. This neither reserves an operation nor
    /// attests readiness, current identity, Source, coverage or any deadline.
    /// Native callers must still use evaluation_entered at actual call entry
    /// and compare its returned U with the already prepared request's U.
    pub fn selected_unique_context(&self) -> Option<Id> {
        self.selected.context.map(|context| context.unique)
    }
    /// Source is from the final actual same-core getter and postDrop recheck.
    /// This returns only U: there is no numeric contextId fallback.
    pub fn evaluation_entered(
        &mut self,
        fresh_source: &str,
        boundary: Boundary,
    ) -> Result<Evaluation, Problem> {
        let mark = self.mark(boundary)?;
        if fresh_source != FIXED_URL {
            return self.reject(Problem::Native, boundary.ms);
        }
        let ticket = self.enter_command(Command::Evaluate, mark, boundary.ms)?;
        let unique_context = self.selected.context.ok_or(Problem::NotReady)?.unique;
        Ok(Evaluation {
            ticket,
            unique_context,
        })
    }
    fn operation(&mut self, ticket: CommandTicket, ms: u64) -> Result<Operation, Problem> {
        if ticket.identity != self.identity {
            return self.reject(Problem::Identity, ms);
        }
        let operation = self.operations[ticket.command as usize];
        if !operation.issued {
            return self.reject(Problem::Replay, ms);
        }
        Ok(operation)
    }
    pub fn command_returned(
        &mut self,
        ticket: CommandTicket,
        success: bool,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.mark(boundary)?;
        let op = self.operation(ticket, boundary.ms)?;
        if op.returned.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.operations[ticket.command as usize].returned = Some(success);
        if !success {
            self.fail(Problem::Native, boundary.ms);
        }
        Ok(())
    }
    /// Genuine invocation, even late/error/unknown, may enter only the original
    /// same-ticket disposal gate. An absent callback never comes through here.
    pub fn completion_entered(
        &mut self,
        ticket: CommandTicket,
        boundary: Boundary,
    ) -> Result<CallbackTicket, Problem> {
        let mut mark = self.disposal_gate(boundary)?;
        let op = self.operation(ticket, boundary.ms)?;
        if op.entry.is_some() || op.consumed || !op.handler_live {
            return self.reject(Problem::Replay, boundary.ms);
        }
        if ticket.command == Command::Evaluate
            && self.native.and_then(|n| n.qualified) != Some(true)
        {
            mark.positive = false;
            self.fail(Problem::Result, boundary.ms);
        }
        self.operations[ticket.command as usize].entry = Some(mark);
        Ok(CallbackTicket {
            command: ticket,
            entry_sequence: mark.sequence,
        })
    }
    fn callback_operation(
        &mut self,
        ticket: CallbackTicket,
        ms: u64,
    ) -> Result<Operation, Problem> {
        let op = self.operation(ticket.command, ms)?;
        if op
            .entry
            .is_none_or(|entry| entry.sequence != ticket.entry_sequence)
            || op.consumed
        {
            return self.reject(Problem::Replay, ms);
        }
        Ok(op)
    }
    pub fn completion_captured(
        &mut self,
        ticket: CallbackTicket,
        hresult_success: bool,
        value: CompletionValue<'_>,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        let mark = self.disposal_gate(boundary)?;
        let op = self.callback_operation(ticket, boundary.ms)?;
        if op.capture.is_some() || op.qualified.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        let semantic_valid = hresult_success
            && match (ticket.command.command, value) {
                (Command::PageEnable | Command::RuntimeEnable, CompletionValue::EmptyAck) => true,
                (Command::GetFrameTree, CompletionValue::FrameTree(frame)) => {
                    self.selected.frame == Some(*frame)
                }
                (Command::Evaluate, CompletionValue::Denied) => true,
                _ => false,
            };
        let operation = &mut self.operations[ticket.command.command as usize];
        operation.capture = Some(mark);
        operation.semantic_valid = semantic_valid;
        if !semantic_valid {
            self.fail(Problem::Result, boundary.ms);
        }
        Ok(())
    }
    /// Call after strict parsing and ALL relevant native/error/args/interface
    /// Drops. Full pre-cutoff qualification is frozen here; consumption cannot
    /// later upgrade it or demand continuity through the teardown blind interval.
    pub fn completion_qualified(
        &mut self,
        ticket: CallbackTicket,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        let mark = self.disposal_gate(boundary)?;
        let op = self.callback_operation(ticket, boundary.ms)?;
        if op.qualified.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        let Some(capture) = op.capture else {
            return self.reject(Problem::Phase, boundary.ms);
        };
        let entry = op.entry.ok_or(Problem::Phase)?;
        let qualified = op.semantic_valid
            && entry.positive
            && capture.positive
            && mark.positive
            && entry.epoch == capture.epoch
            && capture.epoch == mark.epoch;
        self.operations[ticket.command.command as usize].qualified = Some(qualified);
        if !qualified {
            self.fail(
                if boundary.ms >= self.deadline_ms {
                    Problem::Deadline
                } else {
                    Problem::Result
                },
                boundary.ms,
            );
        }
        Ok(())
    }
    pub fn completion_consumed(
        &mut self,
        ticket: CommandTicket,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.disposal_gate(boundary)?;
        let op = self.operation(ticket, boundary.ms)?;
        if op.qualified.is_none() || op.consumed {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.operations[ticket.command as usize].consumed = true;
        if self.active == Some(ticket.command) {
            self.active = None;
        }
        Ok(())
    }
    /// Only actual handler Drop calls this. It cannot imply invocation, cancel
    /// an outstanding obligation, create a receipt, or wait before Stop/Close.
    pub fn command_handler_dropped(
        &mut self,
        ticket: CommandTicket,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        if self.phase == Phase::Released {
            if boundary.identity != self.identity {
                return Err(Problem::Identity);
            }
            // An invalid early Released claim must not erase a later actual Drop.
            self.fail(Problem::Released, self.last_ms);
        } else {
            self.boundary_mark(boundary, false)?;
        }
        let op = self.operation(ticket, boundary.ms)?;
        if !op.handler_live {
            return self.reject(Problem::Replay, boundary.ms);
        }
        self.operations[ticket.command as usize].handler_live = false;
        Ok(())
    }
    /// Observation eligibility ONLY. Err must NEVER suppress the native caller's
    /// once-per-delivered-usable-args SetState(DENY) and subsequent State readback.
    /// Duplicate/unmatched callbacks still require their own restrictive action.
    pub fn native_entered(&mut self, boundary: Boundary) -> Result<NativeTicket, Problem> {
        let mark = self.mark(boundary)?;
        if self.native.is_some() {
            return self.reject(Problem::Replay, boundary.ms);
        }
        if !self.action_live(mark)
            || self.phase != Phase::Issued
            || self.active != Some(Command::Evaluate)
        {
            return self.reject(Problem::Phase, boundary.ms);
        }
        self.native = Some(NativeRecord {
            entry: mark,
            checks: 0,
            all_positive: true,
            refused: false,
            qualified: None,
        });
        Ok(NativeTicket {
            identity: self.identity,
            sequence: mark.sequence,
        })
    }
    fn native_record(&mut self, ticket: NativeTicket, ms: u64) -> Result<NativeRecord, Problem> {
        if ticket.identity != self.identity {
            return self.reject(Problem::Identity, ms);
        }
        match self.native {
            Some(native)
                if native.entry.sequence == ticket.sequence && native.qualified.is_none() =>
            {
                Ok(native)
            }
            _ => self.reject(Problem::Replay, ms),
        }
    }
    /// Import only an earlier actual native close after permanent revocation.
    /// This changes no event/ticket/current mark and can only shorten close10.
    pub fn adopt_native_close_origin(&mut self, ms: u64) {
        if !self.revoked {
            self.fail(Problem::Clock, self.last_ms);
            return;
        }
        if let Some(old) = self.first_close_ms {
            self.first_close_ms = Some(old.min(ms));
        }
    }
    /// A missing native clock supplies no invented boundary. Keep only the last
    /// genuine semantic time and permanently reject; caller retains unknown/null.
    pub fn native_clock_failed(&mut self) {
        self.fail(Problem::Clock, self.last_ms);
    }
    /// Adapter failure is a real negative boundary, not a lifecycle delivery,
    /// native action, ticket, completion or permission grant.
    pub fn native_adapter_failed(&mut self, boundary: Boundary) {
        if self.boundary_mark(boundary, false).is_ok() {
            self.fail(Problem::Native, boundary.ms);
        }
    }
    pub fn native_checked(
        &mut self,
        ticket: NativeTicket,
        check: NativeCheck,
        success: bool,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        let mark = self.mark(boundary)?;
        let mut native = self.native_record(ticket, boundary.ms)?;
        let bit = 1_u8 << check as u8;
        if native.checks & bit != 0 {
            return self.reject(Problem::Replay, boundary.ms);
        }
        if native.checks.count_ones() != u32::from(check as u8) {
            return self.reject(Problem::Phase, boundary.ms);
        }
        native.checks |= bit;
        native.all_positive &= success && mark.positive && mark.epoch == native.entry.epoch;
        self.native = Some(native);
        if !success {
            self.fail(Problem::Native, boundary.ms);
        }
        Ok(())
    }
    /// Immediate actual refusal boundary; no effect wait. Repeated failures or
    /// receipts cannot move the first-close clock. Session PolicyRejected is a
    /// separate real caller observation and is never synthesized here.
    pub fn native_refused(
        &mut self,
        ticket: NativeTicket,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        self.mark(boundary)?;
        let mut native = self.native_record(ticket, boundary.ms)?;
        if native.refused || native.checks & (1 << NativeCheck::DenyAttempted as u8) == 0 {
            return self.reject(Problem::Replay, boundary.ms);
        }
        native.refused = true;
        self.native = Some(native);
        self.revoke(boundary.ms);
        Ok(())
    }
    pub fn native_qualified(
        &mut self,
        ticket: NativeTicket,
        boundary: Boundary,
    ) -> Result<(), Problem> {
        let mark = self.mark(boundary)?;
        let mut native = self.native_record(ticket, boundary.ms)?;
        native.qualified = Some(
            native.entry.positive
                && native.all_positive
                && native.checks == u8::MAX
                && native.refused
                && mark.positive
                && mark.epoch == native.entry.epoch,
        );
        self.native = Some(native);
        if native.qualified == Some(false) {
            self.fail(Problem::Native, boundary.ms);
        }
        Ok(())
    }
    /// Actual caller-supplied Released observation, not a cleanup implementation.
    /// Unconsumed callbacks/retained tokens remain visible; nothing is zeroed.
    pub fn released(&mut self, boundary: Boundary) {
        if self.boundary_mark(boundary, false).is_err() {
            return;
        }
        let status = self.status();
        if status.obligations != 0
            || status.command_handlers != 0
            || status.owned_registrations != 0
        {
            self.fail(Problem::Coverage, self.last_ms);
        }
        self.cutoff(CutoffReason::ControlsDisposal, boundary);
        self.phase = Phase::Released;
    }
    pub fn status(&self) -> Status {
        Status {
            phase: self.phase,
            blank_branch: self.branch,
            failure: self.failure,
            epoch: self.epoch,
            action_revoked: self.revoked,
            first_close_ms: self.first_close_ms,
            cutoff: self.cutoff,
            deliveries: self.deliveries,
            bootstrap_deliveries: self.bootstrap_deliveries,
            disposal_deliveries: self.disposal_deliveries,
            obligations: self
                .operations
                .iter()
                .filter(|op| op.issued && !op.consumed)
                .count() as u8,
            command_handlers: self.operations.iter().filter(|op| op.handler_live).count() as u8,
            owned_registrations: self
                .registrations
                .iter()
                .filter(|r| r.token.is_some() && !r.removed)
                .count() as u8,
            native_qualified: self.native.and_then(|native| native.qualified),
            effect_qualified: self.operations[Command::Evaluate as usize].qualified,
        }
    }
}

pub fn permission_origin_matches(origin: &str) -> bool {
    matches!(
        origin,
        "https://console.code0.ai"
            | "https://console.code0.ai/"
            | "https://console.code0.ai:443"
            | "https://console.code0.ai:443/"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> Identity {
        Identity {
            request: 11,
            generation: 22,
            installation: 33,
            core: 44,
        }
    }
    fn at(ms: u64) -> Boundary {
        Boundary {
            identity: identity(),
            ms,
        }
    }
    fn id(value: &str) -> Id {
        Id::parse(value).unwrap()
    }
    fn frame(loader: &str) -> Frame {
        Frame {
            id: id("root"),
            loader: id(loader),
        }
    }
    fn context(unique: &str) -> Context {
        Context::new(id("root"), 7, id(unique)).unwrap()
    }
    fn installed() -> Protocol {
        let mut p = Protocol::new(identity(), 0).unwrap();
        for index in 0..10 {
            p.registration_added(CoverageKey::S2Hook(index), NativeToken::Event(0), at(1))
                .unwrap();
        }
        for receiver in Receiver::ALL {
            p.registration_added(
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
            p.registration_added(key, NativeToken::Event(0), at(1))
                .unwrap();
        }
        p.registration_added(CoverageKey::AllSourceFilter, NativeToken::Filter, at(1))
            .unwrap();
        p.settings_acknowledged(at(1)).unwrap();
        p
    }
    fn complete(p: &mut Protocol, ticket: CommandTicket, value: CompletionValue<'_>, ms: u64) {
        p.command_returned(ticket, true, at(ms)).unwrap();
        let callback = p.completion_entered(ticket, at(ms)).unwrap();
        p.completion_captured(callback, true, value, at(ms))
            .unwrap();
        p.completion_qualified(callback, at(ms)).unwrap();
        p.completion_consumed(ticket, at(ms)).unwrap();
        p.command_handler_dropped(ticket, at(ms)).unwrap();
    }
    fn enabling() -> Protocol {
        let mut p = installed();
        let page = p.command_entered(Command::PageEnable, at(2)).unwrap();
        complete(&mut p, page, CompletionValue::EmptyAck, 2);
        let runtime = p.command_entered(Command::RuntimeEnable, at(3)).unwrap();
        complete(&mut p, runtime, CompletionValue::EmptyAck, 3);
        p
    }
    fn assembled(blank: bool, order: &[u8]) -> Protocol {
        let mut p = enabling();
        if blank {
            p.lifecycle(
                Lifecycle::FrameNavigated(Document::Blank, frame("blank-loader")),
                at(4),
            )
            .unwrap();
            p.lifecycle(
                Lifecycle::ContextCreated(Document::Blank, context("blank-context")),
                at(4),
            )
            .unwrap();
        }
        p.navigation_reserved(at(5)).unwrap();
        if blank {
            p.lifecycle(Lifecycle::ContextsCleared, at(6)).unwrap();
        }
        for index in order {
            let event = match index {
                0 => Lifecycle::FrameNavigated(Document::Selected, frame("selected-loader")),
                1 => Lifecycle::ContextCreated(Document::Selected, context("selected-context")),
                2 => Lifecycle::StartedLoading(id("root")),
                3 => Lifecycle::StoppedLoading(id("root")),
                _ => unreachable!(),
            };
            p.lifecycle(event, at(7)).unwrap();
        }
        p.navigation_started(0, FIXED_URL, false, at(8)).unwrap();
        p.navigation_completed(0, true, at(9)).unwrap();
        p.navigation_returned(true, at(9)).unwrap();
        p.main_response_installed(true, at(9)).unwrap();
        let tree = p.command_entered(Command::GetFrameTree, at(10)).unwrap();
        complete(
            &mut p,
            tree,
            CompletionValue::FrameTree(&frame("selected-loader")),
            10,
        );
        p.source_after_navigation(FIXED_URL, at(11)).unwrap();
        p
    }
    fn issued() -> (Protocol, CommandTicket) {
        let mut p = assembled(false, &[0, 1, 2, 3]);
        p.arm(at(12)).unwrap();
        let evaluation = p.evaluation_entered(FIXED_URL, at(13)).unwrap();
        assert_eq!(evaluation.unique_context, id("selected-context"));
        p.command_returned(evaluation.ticket, true, at(13)).unwrap();
        (p, evaluation.ticket)
    }
    fn native(p: &mut Protocol, ms: u64) {
        let ticket = p.native_entered(at(ms)).unwrap();
        for check in NativeCheck::ALL {
            p.native_checked(ticket, check, true, at(ms)).unwrap();
            if check == NativeCheck::DenyAttempted {
                p.native_refused(ticket, at(ms)).unwrap();
            }
        }
        p.native_qualified(ticket, at(ms)).unwrap();
    }

    // Mutation: accepting empty/oversized/non-ASCII context identities or CID zero.
    #[test]
    fn identifiers_and_context_ids_are_bounded() {
        for bad in ["", "a/b", "a b", "é"] {
            assert!(Id::parse(bad).is_err());
        }
        assert!(Id::parse(&"a".repeat(128)).is_ok());
        assert!(Id::parse(&"a".repeat(129)).is_err());
        assert!(Context::new(id("f"), 0, id("u")).is_err());
        assert!(Context::new(id("f"), 2_147_483_648, id("u")).is_err());
    }
    // Mutation: treating token zero as absent or allowing action before all coverage.
    #[test]
    fn coverage_requires_every_real_registration_and_zero_token_is_owned() {
        let mut empty = Protocol::new(identity(), 0).unwrap();
        assert_eq!(
            empty.command_entered(Command::PageEnable, at(1)),
            Err(Problem::NotReady)
        );
        let mut p = installed();
        let removal = p.removal_entered(CoverageKey::S2Hook(0), at(2)).unwrap();
        assert_eq!(removal.token, NativeToken::Event(0));
        assert_eq!(p.status().owned_registrations, 24);
        p.removal_returned(removal, false, at(2)).unwrap();
        assert_eq!(p.status().owned_registrations, 24);
        assert!(p.removal_entered(CoverageKey::S2Hook(0), at(3)).is_err());
    }
    // Mutation: allowing a second in-flight command or issuing Runtime before ACK.
    #[test]
    fn commands_are_ordered_once_only_and_require_consumed_ack() {
        let mut p = installed();
        let first = p.command_entered(Command::PageEnable, at(2)).unwrap();
        assert_eq!(
            p.command_entered(Command::RuntimeEnable, at(2)),
            Err(Problem::NotReady)
        );
        complete(&mut p, first, CompletionValue::EmptyAck, 3);
        assert!(p.command_entered(Command::PageEnable, at(4)).is_err());
        assert_eq!(p.status().obligations, 0);
    }
    // Mutation: allowing lifecycle delivery before actual enable entry.
    #[test]
    fn registration_phase_has_no_bootstrap_exception() {
        let mut p = installed();
        assert!(
            p.lifecycle(Lifecycle::StartedLoading(id("root")), at(2))
                .is_err()
        );
        assert_eq!(p.status().bootstrap_deliveries.value, 1);
        assert!(p.status().action_revoked);
    }
    // Mutation: adopting selected facts before navigation reservation.
    #[test]
    fn selected_context_before_reservation_is_rejected() {
        let mut p = enabling();
        assert!(
            p.lifecycle(
                Lifecycle::ContextCreated(Document::Selected, context("s")),
                at(4)
            )
            .is_err()
        );
    }
    // Mutation: assigning positive authority from one provisional root fact.
    #[test]
    fn main_context_frame_and_loads_join_in_every_relative_order() {
        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        if a == b || a == c || a == d || b == c || b == d || c == d {
                            continue;
                        }
                        for blank in [false, true] {
                            let mut p = assembled(blank, &[a, b, c, d]);
                            p.arm(at(12)).unwrap();
                            assert_eq!(p.status().phase, Phase::Armed);
                        }
                    }
                }
            }
        }
    }
    // Migration of the sole conflicting legacy test: orphan Stop remains blocked;
    // context+Stop now selects the explicit third branch, and start-only still waits.
    #[test]
    fn initial_orphan_stop_and_start_only_require_their_missing_facts() {
        let mut p = enabling();
        p.lifecycle(Lifecycle::StoppedLoading(id("root")), at(4))
            .unwrap();
        assert_eq!(p.navigation_reserved(at(5)), Err(Problem::NotReady));
        p.lifecycle(
            Lifecycle::ContextCreated(Document::Blank, context("b")),
            at(6),
        )
        .unwrap();
        p.navigation_reserved(at(6)).unwrap();
        assert_eq!(
            p.status().blank_branch,
            Some(BlankBranch::ObservedInitialStopOnly)
        );
        assert!(!p.blank.started);
        assert!(p.blank.stopped);

        let mut p = enabling();
        p.lifecycle(
            Lifecycle::ContextCreated(Document::Blank, context("b")),
            at(4),
        )
        .unwrap();
        p.lifecycle(Lifecycle::StartedLoading(id("root")), at(4))
            .unwrap();
        assert_eq!(p.navigation_reserved(at(5)), Err(Problem::NotReady));
        assert_eq!(p.status().blank_branch, None);
        p.lifecycle(Lifecycle::StoppedLoading(id("root")), at(6))
            .unwrap();
        p.navigation_reserved(at(7)).unwrap();
        assert_eq!(p.status().blank_branch, Some(BlankBranch::Observed));
        assert!(p.blank.started);
        assert!(p.blank.stopped);
    }
    // Mutation: allowing late recognizable blank adoption or reusing pre-reservation load end.
    #[test]
    fn zero_blank_branch_is_sealed_and_main_load_pair_is_separate() {
        let mut p = enabling();
        p.navigation_reserved(at(4)).unwrap();
        assert!(
            p.lifecycle(
                Lifecycle::ContextCreated(Document::Blank, context("late")),
                at(5)
            )
            .is_err()
        );
        let mut p = enabling();
        p.lifecycle(
            Lifecycle::ContextCreated(Document::Blank, context("b")),
            at(4),
        )
        .unwrap();
        p.lifecycle(Lifecycle::StartedLoading(id("root")), at(4))
            .unwrap();
        p.lifecycle(Lifecycle::StoppedLoading(id("root")), at(4))
            .unwrap();
        p.navigation_reserved(at(5)).unwrap();
        assert!(!p.non_tree_ready());
    }
    // Mutation: clearing a provisional selected context or matching only numeric CID.
    #[test]
    fn selected_context_clear_and_wrong_unique_id_destroy_fail() {
        let mut p = enabling();
        p.lifecycle(
            Lifecycle::ContextCreated(Document::Blank, context("b")),
            at(4),
        )
        .unwrap();
        p.navigation_reserved(at(5)).unwrap();
        assert!(
            p.lifecycle(
                Lifecycle::ContextDestroyed {
                    numeric: 7,
                    unique: id("other")
                },
                at(6)
            )
            .is_err()
        );
        let mut p = enabling();
        p.navigation_reserved(at(5)).unwrap();
        p.lifecycle(
            Lifecycle::ContextCreated(Document::Selected, context("s")),
            at(6),
        )
        .unwrap();
        assert!(p.lifecycle(Lifecycle::ContextsCleared, at(6)).is_err());
    }
    // Mutation: treating the optional redundant tombstone destroy as a post-arm exception.
    #[test]
    fn tombstone_destroy_is_once_only_and_never_grandfathered_after_arm() {
        let mut p = assembled(true, &[1, 0, 3, 2]);
        p.lifecycle(
            Lifecycle::ContextDestroyed {
                numeric: 7,
                unique: id("blank-context"),
            },
            at(11),
        )
        .unwrap();
        p.arm(at(12)).unwrap();
        assert!(
            p.lifecycle(
                Lifecycle::ContextDestroyed {
                    numeric: 7,
                    unique: id("blank-context")
                },
                at(13)
            )
            .is_err()
        );
        assert_eq!(p.status().epoch, 1);
    }
    // Mutation: adopting a changed root or reusing the blank loader/unique context.
    #[test]
    fn observed_blank_identity_is_not_reassigned() {
        for bad in [
            Lifecycle::FrameNavigated(
                Document::Selected,
                Frame {
                    id: id("other"),
                    loader: id("new"),
                },
            ),
            Lifecycle::FrameNavigated(Document::Selected, frame("blank-loader")),
            Lifecycle::ContextCreated(Document::Selected, context("blank-context")),
        ] {
            let mut p = enabling();
            p.lifecycle(
                Lifecycle::FrameNavigated(Document::Blank, frame("blank-loader")),
                at(4),
            )
            .unwrap();
            p.lifecycle(
                Lifecycle::ContextCreated(Document::Blank, context("blank-context")),
                at(4),
            )
            .unwrap();
            p.navigation_reserved(at(5)).unwrap();
            assert!(p.lifecycle(bad, at(6)).is_err());
        }
    }
    // Mutation: reopening assembly after arm, tolerating forbidden events, or uncounting invalid delivery.
    #[test]
    fn every_receiver_invalidates_after_arm_and_delivery_is_counted() {
        for receiver in Receiver::ALL {
            let mut p = assembled(false, &[0, 1, 2, 3]);
            p.arm(at(12)).unwrap();
            assert!(
                p.lifecycle(Lifecycle::Unsupported(receiver), at(13))
                    .is_err()
            );
            assert_eq!(p.status().deliveries.value, 5);
            assert_eq!(p.status().epoch, 1);
            assert!(p.arm(at(14)).is_err());
        }
    }
    // Mutation: saturating silently or wrapping delivery accounting.
    #[test]
    fn counters_saturate_with_an_explicit_overflow_bit() {
        let mut counter = Counter {
            value: u16::MAX,
            overflowed: false,
        };
        counter.increment();
        assert_eq!(counter.value, u16::MAX);
        assert!(counter.overflowed);
    }
    // Mutation: extending original45 at arm or treating equality as timely.
    #[test]
    fn original_deadline_equality_is_late_and_first_close_never_restarts() {
        let (mut p, command) = issued();
        native(&mut p, 44_999);
        assert_eq!(p.status().first_close_ms, Some(44_999));
        p.cutoff(CutoffReason::RequiredRemovalAttempt, at(45_000));
        assert_eq!(p.status().first_close_ms, Some(44_999));
        assert_eq!(p.status().failure, None);
        let callback = p.completion_entered(command, at(45_000)).unwrap();
        p.completion_captured(callback, true, CompletionValue::Denied, at(45_000))
            .unwrap();
        p.completion_qualified(callback, at(45_000)).unwrap();
        assert_eq!(p.status().effect_qualified, Some(false));
        assert_eq!(p.status().failure, Some(Problem::Deadline));
    }
    // Mutation: allowing an early denied completion to be matched retroactively.
    #[test]
    fn effect_invocation_before_native_action_never_qualifies() {
        let (mut p, command) = issued();
        let callback = p.completion_entered(command, at(14)).unwrap();
        p.completion_captured(callback, true, CompletionValue::Denied, at(14))
            .unwrap();
        p.completion_qualified(callback, at(14)).unwrap();
        assert_eq!(p.status().effect_qualified, Some(false));
    }
    // Mutation: using action-live as a receipt gate or allowing receipt to reactivate action.
    #[test]
    fn genuine_denied_completion_can_qualify_while_closing_without_reauthorization() {
        let (mut p, command) = issued();
        native(&mut p, 14);
        assert!(p.status().action_revoked);
        let callback = p.completion_entered(command, at(15)).unwrap();
        p.completion_captured(callback, true, CompletionValue::Denied, at(15))
            .unwrap();
        p.completion_qualified(callback, at(15)).unwrap();
        assert_eq!(p.status().native_qualified, Some(true));
        assert_eq!(p.status().effect_qualified, Some(true));
        assert!(p.evaluation_entered(FIXED_URL, at(15)).is_err());
    }
    // Mutation: qualifying entry-only evidence across removal/capture/Drop reentry.
    #[test]
    fn cutoff_crossing_capture_or_postdrop_rejects_effect_but_disposes_obligation() {
        for cutoff_before_capture in [false, true] {
            let (mut p, command) = issued();
            native(&mut p, 14);
            let callback = p.completion_entered(command, at(15)).unwrap();
            if cutoff_before_capture {
                p.cutoff(CutoffReason::RequiredRemovalAttempt, at(15));
            }
            p.completion_captured(callback, true, CompletionValue::Denied, at(15))
                .unwrap();
            if !cutoff_before_capture {
                p.cutoff(CutoffReason::RequiredRemovalAttempt, at(15));
            }
            p.completion_qualified(callback, at(15)).unwrap();
            p.completion_consumed(command, at(16)).unwrap();
            assert_eq!(p.status().effect_qualified, Some(false));
            assert_eq!(p.status().obligations, 0);
            assert_eq!(p.status().command_handlers, 1);
            p.command_handler_dropped(command, at(16)).unwrap();
            assert_eq!(p.status().command_handlers, 0);
        }
    }
    // Mutation: requalifying a frozen receipt at consumption after cleanup closes coverage.
    #[test]
    fn fully_qualified_historical_receipt_survives_later_cutoff_only_as_history() {
        let (mut p, command) = issued();
        native(&mut p, 14);
        let callback = p.completion_entered(command, at(15)).unwrap();
        p.completion_captured(callback, true, CompletionValue::Denied, at(15))
            .unwrap();
        p.completion_qualified(callback, at(15)).unwrap();
        let first = p.cutoff(CutoffReason::RequiredRemovalAttempt, at(16));
        assert_eq!(p.cutoff(CutoffReason::StopEntry, at(17)), first);
        p.completion_consumed(command, at(18)).unwrap();
        assert_eq!(p.status().effect_qualified, Some(true));
        assert!(p.status().cutoff.is_some());
        assert_eq!(p.status().obligations, 0);
    }
    // Mutation: admitting a nested callback before the first removal's cutoff is visible.
    #[test]
    fn removal_reservation_latches_cutoff_before_exposing_native_token() {
        let (mut p, command) = issued();
        native(&mut p, 14);
        let removal = p
            .removal_entered(CoverageKey::Receiver(Receiver::DocumentOpened), at(15))
            .unwrap();
        assert_eq!(
            p.status().cutoff.unwrap().reason,
            CutoffReason::RequiredRemovalAttempt
        );
        let callback = p.completion_entered(command, at(15)).unwrap();
        p.completion_captured(callback, true, CompletionValue::Denied, at(15))
            .unwrap();
        p.completion_qualified(callback, at(15)).unwrap();
        assert_eq!(p.status().effect_qualified, Some(false));
        p.removal_returned(removal, true, at(16)).unwrap();
        assert_eq!(p.status().owned_registrations, 23);
    }
    // Mutation: accepting denied JSON despite failed HRESULT or another semantic token.
    #[test]
    fn hresult_and_closed_denied_value_are_independent_requirements() {
        for (success, value) in [
            (false, CompletionValue::Denied),
            (true, CompletionValue::Default),
            (true, CompletionValue::Granted),
            (true, CompletionValue::Precondition),
            (true, CompletionValue::DocumentChanged),
            (true, CompletionValue::Invalid),
            (true, CompletionValue::Malformed),
        ] {
            let (mut p, command) = issued();
            native(&mut p, 14);
            let callback = p.completion_entered(command, at(15)).unwrap();
            p.completion_captured(callback, success, value, at(15))
                .unwrap();
            p.completion_qualified(callback, at(15)).unwrap();
            assert_eq!(p.status().effect_qualified, Some(false));
        }
    }
    // Mutation: handler Drop masquerades as a missing invocation/disposition.
    #[test]
    fn handler_drop_without_invocation_preserves_obligation_and_no_receipt() {
        let (mut p, command) = issued();
        p.command_handler_dropped(command, at(14)).unwrap();
        assert_eq!(p.status().command_handlers, 0);
        assert_eq!(p.status().obligations, 1);
        assert_eq!(p.status().effect_qualified, None);
        assert!(p.completion_consumed(command, at(15)).is_err());
    }
    // Mutation: positive-original45 predicate also blocks truthful late disposal.
    #[test]
    fn original_late_error_completion_drains_within_original_first_close_budget() {
        let (mut p, command) = issued();
        native(&mut p, 44_999);
        p.cutoff(CutoffReason::StopEntry, at(45_000));
        let callback = p.completion_entered(command, at(45_001)).unwrap();
        p.completion_captured(callback, false, CompletionValue::Malformed, at(45_002))
            .unwrap();
        p.completion_qualified(callback, at(45_003)).unwrap();
        p.completion_consumed(command, at(45_004)).unwrap();
        assert_eq!(p.status().obligations, 0);
        assert_eq!(p.status().effect_qualified, Some(false));
        assert_eq!(p.status().first_close_ms, Some(44_999));
    }
    // Mutation: stale identity, Released or close10 equality can settle old ownership.
    #[test]
    fn stale_released_and_close_deadline_deliveries_cannot_drain() {
        for mode in 0..3 {
            let (mut p, command) = issued();
            native(&mut p, 14);
            let mut boundary = at(15);
            if mode == 0 {
                boundary.identity.generation += 1;
            }
            if mode == 1 {
                p.released(at(15));
            }
            if mode == 2 {
                boundary.ms = 10_014;
            }
            assert!(p.completion_entered(command, boundary).is_err());
            assert_eq!(p.status().obligations, 1);
        }
    }
    // Mutation: fabricating a UI sequence number on affinity loss or clearing cutoff.
    #[test]
    fn affinity_loss_has_no_invented_ui_boundary_and_is_permanent() {
        let mut p = installed();
        let first = p.affinity_lost();
        assert_eq!(first.sequence, None);
        assert_eq!(first.ms, None);
        assert_eq!(p.cutoff(CutoffReason::CoverageLost, at(2)), first);
        assert_eq!(p.status().failure, Some(Problem::Affinity));
        assert!(p.command_entered(Command::PageEnable, at(3)).is_err());
    }
    // Mutation: an unrecognized permission origin is accepted through broad URL parsing.
    #[test]
    fn origin_allowlist_and_full_document_source_have_separate_meanings() {
        for origin in [
            "https://console.code0.ai",
            "https://console.code0.ai/",
            "https://console.code0.ai:443",
            "https://console.code0.ai:443/",
        ] {
            assert!(permission_origin_matches(origin));
        }
        for origin in [
            FIXED_URL,
            "https://CONSOLE.code0.ai",
            "https://console.code0.ai?x",
            "https://console.code0.ai:444",
            "https://x@console.code0.ai",
        ] {
            assert!(!permission_origin_matches(origin));
        }
    }

    // Mutation: valid teardown delivery adopts/invalidates state or rechecks an old receipt's45.
    #[test]
    fn historical_receipt_consumption_after_original_deadline_is_not_a_new_positive() {
        let (mut p, command) = issued();
        native(&mut p, 44_990);
        let callback = p.completion_entered(command, at(44_991)).unwrap();
        p.completion_captured(callback, true, CompletionValue::Denied, at(44_992))
            .unwrap();
        p.completion_qualified(callback, at(44_993)).unwrap();
        p.cutoff(CutoffReason::RequiredRemovalAttempt, at(44_994));
        p.lifecycle(Lifecycle::ContextsCleared, at(45_001)).unwrap();
        p.completion_consumed(command, at(45_002)).unwrap();
        assert_eq!(p.status().effect_qualified, Some(true));
        assert_eq!(p.status().failure, None);
        assert_eq!(p.status().disposal_deliveries.value, 1);
        assert_eq!(p.status().obligations, 0);
    }
    // Mutation: recording readback before DENY or finishing partial facts as a checked action.
    #[test]
    fn native_checked_action_requires_real_order_and_all_independent_facts() {
        let (mut p, _) = issued();
        let ticket = p.native_entered(at(14)).unwrap();
        assert!(
            p.native_checked(ticket, NativeCheck::AfterDeny, true, at(14))
                .is_err()
        );
        assert_eq!(p.status().native_qualified, None);
        let (mut p, _) = issued();
        let ticket = p.native_entered(at(14)).unwrap();
        p.native_qualified(ticket, at(14)).unwrap();
        assert_eq!(p.status().native_qualified, Some(false));
        assert!(p.status().action_revoked);
    }

    // Mutation: coverage loss becomes a normal successful teardown cutoff.
    #[test]
    fn unexpected_coverage_loss_is_sticky_and_never_repairs() {
        let (mut p, command) = issued();
        native(&mut p, 14);
        p.cutoff(CutoffReason::CoverageLost, at(15));
        assert_eq!(p.status().failure, Some(Problem::Coverage));
        let callback = p.completion_entered(command, at(16)).unwrap();
        p.completion_captured(callback, true, CompletionValue::Denied, at(16))
            .unwrap();
        p.completion_qualified(callback, at(16)).unwrap();
        p.completion_consumed(command, at(16)).unwrap();
        assert_eq!(p.status().effect_qualified, Some(false));
        assert_eq!(p.status().obligations, 0);
    }
    // Mutation: only cutoff is checked and an actual physical entry can slip through.
    #[test]
    fn physical_entry_before_qualification_defeats_new_positive_evidence() {
        for reason in [
            CutoffReason::StopEntry,
            CutoffReason::ControllerCloseEntry,
            CutoffReason::ControlsDisposal,
        ] {
            let (mut p, command) = issued();
            native(&mut p, 14);
            let callback = p.completion_entered(command, at(15)).unwrap();
            p.completion_captured(callback, true, CompletionValue::Denied, at(15))
                .unwrap();
            p.cutoff(reason, at(15));
            p.completion_qualified(callback, at(15)).unwrap();
            assert_eq!(p.status().effect_qualified, Some(false));
        }
    }
    // Mutation: a peek reserves Evaluate or alters the protocol ledger.
    #[test]
    fn selected_unique_context_peek_is_copy_only_and_reserves_nothing() {
        let mut p = assembled(false, &[0, 1, 2, 3]);
        let before = p.status();
        for _ in 0..3 {
            assert_eq!(p.selected_unique_context(), Some(id("selected-context")));
            assert_eq!(p.status(), before);
        }
        // Having a context is deliberately weaker than Armed.
        assert!(p.evaluation_entered(FIXED_URL, at(12)).is_err());
        assert_eq!(p.status().obligations, 0);
        p.arm(at(13)).unwrap();
        let prepared = p.selected_unique_context().unwrap();
        let evaluation = p.evaluation_entered(FIXED_URL, at(14)).unwrap();
        assert_eq!(evaluation.unique_context, prepared);
        assert_eq!(p.status().obligations, 1);
        assert!(p.evaluation_entered(FIXED_URL, at(15)).is_err());
    }

    // Mutation: a previously successful peek bypasses fresh Source/time/cutoff.
    #[test]
    fn selected_unique_context_peek_never_authorizes_after_source_time_or_cutoff_loss() {
        let empty = Protocol::new(identity(), 0).unwrap();
        assert_eq!(empty.selected_unique_context(), None);
        for variant in 0..3 {
            let mut p = assembled(false, &[0, 1, 2, 3]);
            p.arm(at(12)).unwrap();
            let prepared = p.selected_unique_context().unwrap();
            let result = match variant {
                0 => p.evaluation_entered("about:blank", at(13)),
                1 => p.evaluation_entered(FIXED_URL, at(45_000)),
                _ => {
                    p.cutoff(CutoffReason::StopEntry, at(13));
                    p.evaluation_entered(FIXED_URL, at(14))
                }
            };
            assert!(result.is_err());
            assert_eq!(p.status().obligations, 0);
            assert_eq!(p.selected_unique_context(), Some(prepared));
            assert!(p.status().action_revoked);
        }
    }

    // Missing behavior: a real observed initial stop-only prefix cannot reserve
    // the once-only navigation, despite a bound context and completed ACKs.
    mod initial_stop_only_red_tests {
        use super::*;

        fn assert_stop_only_reserves(context_first: bool, with_blank_frame: bool) {
            let mut p = enabling();
            if with_blank_frame {
                p.lifecycle(
                    Lifecycle::FrameNavigated(Document::Blank, frame("blank-loader")),
                    at(4),
                )
                .unwrap();
            }
            let context = Lifecycle::ContextCreated(Document::Blank, context("blank-context"));
            let stopped = Lifecycle::StoppedLoading(id("root"));
            let events = if context_first {
                [context, stopped]
            } else {
                [stopped, context]
            };
            for event in events {
                p.lifecycle(event, at(4)).unwrap();
            }
            assert!(p.blank.context.is_some());
            assert!(!p.blank.started);
            assert!(p.blank.stopped);
            assert_eq!(p.blank.root, Some(id("root")));
            assert_eq!(p.status().phase, Phase::Enabling);
            assert_eq!(p.status().blank_branch, None);
            assert_eq!(p.status().failure, None);
            assert_eq!(
                p.navigation_reserved(at(5)),
                Ok(()),
                "IB-R observed initial stop-only prefix must reserve without fabricating start"
            );
            assert_eq!(p.status().phase, Phase::Navigating);
            assert!(p.status().blank_branch.is_some());
            assert!(!p.blank.started);
            assert!(p.blank.stopped);
            assert!(p.selected.context.is_none());
            assert!(p.selected.frame.is_none());
            assert!(!p.selected.started);
            assert!(!p.selected.stopped);
            assert_eq!(
                p.command_entered(Command::GetFrameTree, at(6)),
                Err(Problem::NotReady)
            );
            assert_eq!(p.arm(at(6)), Err(Problem::NotReady));
        }

        #[test]
        fn context_then_stop_without_blank_frame_reserves_navigation() {
            assert_stop_only_reserves(true, false);
        }

        #[test]
        fn stop_then_context_without_blank_frame_reserves_navigation() {
            assert_stop_only_reserves(false, false);
        }

        #[test]
        fn context_then_stop_with_blank_frame_reserves_navigation() {
            assert_stop_only_reserves(true, true);
        }

        #[test]
        fn stop_then_context_with_blank_frame_reserves_navigation() {
            assert_stop_only_reserves(false, true);
        }
    }
    // Controlled Protocol inputs below are not native COM/event evidence.
    // Mutation targets are named per test; no readiness/receipt is assigned directly.
    mod initial_stop_only_green_tests {
        use super::*;

        fn prefix(context_first: bool, with_frame: bool) -> Protocol {
            let mut p = enabling();
            if with_frame {
                p.lifecycle(
                    Lifecycle::FrameNavigated(Document::Blank, frame("blank-loader")),
                    at(4),
                )
                .unwrap();
            }
            let context = Lifecycle::ContextCreated(Document::Blank, context("blank-context"));
            let stop = Lifecycle::StoppedLoading(id("root"));
            for event in if context_first {
                [context, stop]
            } else {
                [stop, context]
            } {
                p.lifecycle(event, at(4)).unwrap();
            }
            p
        }
        fn reserved(context_first: bool, with_frame: bool) -> Protocol {
            let mut p = prefix(context_first, with_frame);
            p.navigation_reserved(at(5)).unwrap();
            assert_eq!(
                p.status().blank_branch,
                Some(BlankBranch::ObservedInitialStopOnly)
            );
            assert!(!p.blank.started);
            assert!(p.blank.stopped);
            assert_eq!(p.blank.frame.is_some(), with_frame);
            assert!(!p.blank_destroyed);
            assert!(!p.cleared);
            assert!(p.selected.root.is_none());
            assert!(!p.selected.started);
            assert!(!p.selected.stopped);
            p
        }
        fn selected_event(index: u8) -> Lifecycle {
            match index {
                0 => Lifecycle::FrameNavigated(Document::Selected, frame("selected-loader")),
                1 => Lifecycle::ContextCreated(Document::Selected, context("selected-context")),
                2 => Lifecycle::StartedLoading(id("root")),
                3 => Lifecycle::StoppedLoading(id("root")),
                _ => unreachable!(),
            }
        }
        fn destroy_blank(p: &mut Protocol, ms: u64) {
            p.lifecycle(
                Lifecycle::ContextDestroyed {
                    numeric: 7,
                    unique: id("blank-context"),
                },
                at(ms),
            )
            .unwrap();
        }
        fn native_navigation(p: &mut Protocol) {
            p.navigation_started(0, FIXED_URL, false, at(8)).unwrap();
            p.navigation_completed(0, true, at(9)).unwrap();
            p.navigation_returned(true, at(9)).unwrap();
            p.main_response_installed(true, at(9)).unwrap();
        }
        fn ready_without_tree(retired: bool) -> Protocol {
            let mut p = reserved(true, true);
            if retired {
                destroy_blank(&mut p, 6);
            }
            for index in 0..4 {
                p.lifecycle(selected_event(index), at(7)).unwrap();
            }
            native_navigation(&mut p);
            p
        }
        fn tree_and_source(p: &mut Protocol, ms: u64) {
            let tree = p.command_entered(Command::GetFrameTree, at(ms)).unwrap();
            complete(
                p,
                tree,
                CompletionValue::FrameTree(&frame("selected-loader")),
                ms,
            );
            p.source_after_navigation(FIXED_URL, at(ms + 1)).unwrap();
        }
        fn issued_stop_only() -> (Protocol, CommandTicket) {
            let mut p = ready_without_tree(true);
            tree_and_source(&mut p, 10);
            p.arm(at(12)).unwrap();
            let evaluation = p.evaluation_entered(FIXED_URL, at(13)).unwrap();
            assert_eq!(evaluation.unique_context, id("selected-context"));
            p.command_returned(evaluation.ticket, true, at(13)).unwrap();
            (p, evaluation.ticket)
        }
        fn effect(p: &mut Protocol, command: CommandTicket, ms: u64) {
            let callback = p.completion_entered(command, at(ms)).unwrap();
            p.completion_captured(callback, true, CompletionValue::Denied, at(ms))
                .unwrap();
            p.completion_qualified(callback, at(ms)).unwrap();
            p.completion_consumed(command, at(ms)).unwrap();
            p.command_handler_dropped(command, at(ms)).unwrap();
        }
        fn all_removals_and_release(p: &mut Protocol, ms: u64) {
            let mut keys = Vec::new();
            for index in 0..10 {
                keys.push(CoverageKey::S2Hook(index));
            }
            for receiver in Receiver::ALL {
                keys.push(CoverageKey::Receiver(receiver));
            }
            keys.extend([
                CoverageKey::NavigationCompleted,
                CoverageKey::ResourceRequested,
                CoverageKey::AllSourceFilter,
                CoverageKey::CoreMessage,
            ]);
            for key in keys {
                let ticket = p.removal_entered(key, at(ms)).unwrap();
                p.removal_returned(ticket, true, at(ms)).unwrap();
            }
            p.cutoff(CutoffReason::StopEntry, at(ms));
            p.cutoff(CutoffReason::ControllerCloseEntry, at(ms));
            p.released(at(ms));
        }

        // Mutation: third branch invents a start/retirement or lets initial Stop fill selected Stop.
        #[test]
        fn all_prefixes_and_24_selected_orders_reach_checked_effect_and_protocol_release() {
            let mut permutations = 0;
            for a in 0..4 {
                for b in 0..4 {
                    for c in 0..4 {
                        for d in 0..4 {
                            if a == b || a == c || a == d || b == c || b == d || c == d {
                                continue;
                            }
                            permutations += 1;
                            for context_first in [false, true] {
                                for with_frame in [false, true] {
                                    for clear in [false, true] {
                                        let mut p = reserved(context_first, with_frame);
                                        if clear {
                                            p.lifecycle(Lifecycle::ContextsCleared, at(6)).unwrap();
                                        }
                                        for index in [a, b, c, d] {
                                            p.lifecycle(selected_event(index), at(7)).unwrap();
                                        }
                                        if !clear {
                                            destroy_blank(&mut p, 7);
                                        }
                                        assert_eq!(p.blank_destroyed, !clear);
                                        assert_eq!(p.cleared, clear);
                                        native_navigation(&mut p);
                                        tree_and_source(&mut p, 10);
                                        p.arm(at(12)).unwrap();
                                        let eval = p.evaluation_entered(FIXED_URL, at(13)).unwrap();
                                        assert_eq!(eval.unique_context, id("selected-context"));
                                        p.command_returned(eval.ticket, true, at(13)).unwrap();
                                        native(&mut p, 14);
                                        assert!(p.status().action_revoked);
                                        assert_eq!(p.status().first_close_ms, Some(14));
                                        effect(&mut p, eval.ticket, 15);
                                        all_removals_and_release(&mut p, 16);
                                        let status = p.status();
                                        assert_eq!(status.phase, Phase::Released);
                                        assert_eq!(status.failure, None);
                                        assert_eq!(status.native_qualified, Some(true));
                                        assert_eq!(status.effect_qualified, Some(true));
                                        assert_eq!(status.obligations, 0);
                                        assert_eq!(status.command_handlers, 0);
                                        assert_eq!(status.owned_registrations, 0);
                                        assert!(!p.blank.started);
                                        assert!(p.blank.stopped);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            assert_eq!(permutations, 24);
        }

        // Mutation: broaden old Observed/Z or ignore an actual pre-reservation Start.
        #[test]
        fn old_empty_and_observed_none_or_full_pair_classifications_stay_exact() {
            let mut zero = enabling();
            zero.navigation_reserved(at(4)).unwrap();
            assert_eq!(zero.status().blank_branch, Some(BlankBranch::ZeroObserved));
            for with_frame in [false, true] {
                for pair in [None, Some(true), Some(false)] {
                    let mut p = enabling();
                    if with_frame {
                        p.lifecycle(
                            Lifecycle::FrameNavigated(Document::Blank, frame("b")),
                            at(4),
                        )
                        .unwrap();
                    }
                    p.lifecycle(
                        Lifecycle::ContextCreated(Document::Blank, context("b")),
                        at(4),
                    )
                    .unwrap();
                    if let Some(start_first) = pair {
                        let start = Lifecycle::StartedLoading(id("root"));
                        let stop = Lifecycle::StoppedLoading(id("root"));
                        for event in if start_first {
                            [start, stop]
                        } else {
                            [stop, start]
                        } {
                            p.lifecycle(event, at(4)).unwrap();
                        }
                    }
                    p.navigation_reserved(at(5)).unwrap();
                    assert_eq!(p.status().blank_branch, Some(BlankBranch::Observed));
                    assert_eq!(p.blank.started, pair.is_some());
                    assert_eq!(p.blank.stopped, pair.is_some());
                }
            }
        }

        // Mutation: optional frame or orphan Stop authorizes without a qualified BContext.
        #[test]
        fn orphan_stop_or_frame_never_selects_zero_or_stop_only() {
            for with_frame in [false, true] {
                let mut p = enabling();
                if with_frame {
                    p.lifecycle(
                        Lifecycle::FrameNavigated(Document::Blank, frame("b")),
                        at(4),
                    )
                    .unwrap();
                    assert_eq!(p.navigation_reserved(at(4)), Err(Problem::NotReady));
                }
                p.lifecycle(Lifecycle::StoppedLoading(id("root")), at(4))
                    .unwrap();
                assert_eq!(p.navigation_reserved(at(5)), Err(Problem::NotReady));
                assert_eq!(p.status().blank_branch, None);
                assert_eq!(p.status().failure, None);
            }
        }

        // Mutation: duplicate/mismatched lifecycle inputs are hidden to obtain the new prefix.
        #[test]
        fn initial_duplicates_wrong_root_and_unsupported_events_are_sticky() {
            for (bad, problem) in [
                (Lifecycle::StoppedLoading(id("root")), Problem::Replay),
                (
                    Lifecycle::ContextCreated(Document::Blank, context("other-context")),
                    Problem::Replay,
                ),
                (
                    Lifecycle::FrameNavigated(Document::Blank, frame("other-loader")),
                    Problem::Replay,
                ),
                (Lifecycle::StartedLoading(id("other-root")), Problem::Root),
            ] {
                let mut p = prefix(true, true);
                assert_eq!(p.lifecycle(bad, at(5)), Err(problem));
                assert_eq!(p.status().failure, Some(problem));
                assert_eq!(p.navigation_reserved(at(6)), Err(Problem::NotReady));
                assert_eq!(p.status().blank_branch, None);
            }
            for receiver in Receiver::ALL {
                let mut p = prefix(false, false);
                assert_eq!(
                    p.lifecycle(Lifecycle::Unsupported(receiver), at(5)),
                    Err(Problem::Lifecycle)
                );
                assert_eq!(p.navigation_reserved(at(6)), Err(Problem::NotReady));
            }
            for wrong in [
                Lifecycle::StoppedLoading(id("other-root")),
                Lifecycle::FrameNavigated(
                    Document::Blank,
                    Frame {
                        id: id("other-root"),
                        loader: id("b"),
                    },
                ),
            ] {
                let mut p = enabling();
                p.lifecycle(
                    Lifecycle::ContextCreated(Document::Blank, context("b")),
                    at(4),
                )
                .unwrap();
                assert_eq!(p.lifecycle(wrong, at(5)), Err(Problem::Root));
                assert_eq!(p.navigation_reserved(at(6)), Err(Problem::NotReady));
            }
        }

        // Mutation: selecting this branch bypasses actual Runtime ACK qualification/consumption.
        #[test]
        fn unconsumed_or_failed_enable_ack_cannot_reserve() {
            for fail in [false, true] {
                let mut p = installed();
                let page = p.command_entered(Command::PageEnable, at(2)).unwrap();
                complete(&mut p, page, CompletionValue::EmptyAck, 2);
                let runtime = p.command_entered(Command::RuntimeEnable, at(3)).unwrap();
                p.lifecycle(
                    Lifecycle::ContextCreated(Document::Blank, context("b")),
                    at(4),
                )
                .unwrap();
                p.lifecycle(Lifecycle::StoppedLoading(id("root")), at(4))
                    .unwrap();
                assert_eq!(p.navigation_reserved(at(4)), Err(Problem::NotReady));
                p.command_returned(runtime, true, at(4)).unwrap();
                let callback = p.completion_entered(runtime, at(4)).unwrap();
                p.completion_captured(callback, !fail, CompletionValue::EmptyAck, at(4))
                    .unwrap();
                assert_eq!(p.navigation_reserved(at(4)), Err(Problem::NotReady));
                p.completion_qualified(callback, at(4)).unwrap();
                assert_eq!(p.navigation_reserved(at(4)), Err(Problem::NotReady));
                p.completion_consumed(runtime, at(4)).unwrap();
                p.command_handler_dropped(runtime, at(4)).unwrap();
                assert_eq!(p.status().blank_branch, None);
                if fail {
                    assert_eq!(p.status().failure, Some(Problem::Result));
                    assert_eq!(p.navigation_reserved(at(5)), Err(Problem::NotReady));
                } else {
                    p.navigation_reserved(at(5)).unwrap();
                    assert_eq!(
                        p.status().blank_branch,
                        Some(BlankBranch::ObservedInitialStopOnly)
                    );
                }
            }
        }

        // Mutation: the third-branch admission short-circuits identity/coverage/cutoff/45 gates.
        #[test]
        fn reservation_keeps_current_identity_coverage_cutoff_and_original45() {
            for field in 0..4 {
                let mut p = prefix(true, false);
                let mut boundary = at(5);
                match field {
                    0 => boundary.identity.request += 1,
                    1 => boundary.identity.generation += 1,
                    2 => boundary.identity.installation += 1,
                    _ => boundary.identity.core += 1,
                }
                assert_eq!(p.navigation_reserved(boundary), Err(Problem::Identity));
                assert_eq!(p.status().blank_branch, None);
            }
            for reason in [
                CutoffReason::CoverageLost,
                CutoffReason::RequiredRemovalAttempt,
                CutoffReason::StopEntry,
                CutoffReason::ControllerCloseEntry,
                CutoffReason::ControlsDisposal,
            ] {
                let mut p = prefix(false, true);
                p.cutoff(reason, at(5));
                assert_eq!(p.navigation_reserved(at(5)), Err(Problem::NotReady));
                assert_eq!(p.status().blank_branch, None);
            }
            let mut p = prefix(true, false);
            p.affinity_lost();
            assert_eq!(p.navigation_reserved(at(5)), Err(Problem::NotReady));
            assert_eq!(p.status().failure, Some(Problem::Affinity));
            let mut p = prefix(true, false);
            assert_eq!(p.navigation_reserved(at(45_000)), Err(Problem::NotReady));
            assert_eq!(p.status().failure, Some(Problem::Deadline));
            assert_eq!(p.status().blank_branch, None);
        }

        // Mutation: initial Stop/branch is credited as one selected load event.
        #[test]
        fn missing_any_selected_fact_blocks_tree_source_arm_and_evaluate() {
            for missing in 0..4 {
                let mut p = reserved(true, true);
                destroy_blank(&mut p, 6);
                for index in 0..4 {
                    if index != missing {
                        p.lifecycle(selected_event(index), at(7)).unwrap();
                    }
                }
                native_navigation(&mut p);
                assert_eq!(
                    p.command_entered(Command::GetFrameTree, at(10)),
                    Err(Problem::NotReady)
                );
                assert_eq!(
                    p.source_after_navigation(FIXED_URL, at(10)),
                    Err(Problem::NotReady)
                );
                assert_eq!(p.arm(at(10)), Err(Problem::NotReady));
                assert_eq!(
                    p.evaluation_entered(FIXED_URL, at(10)),
                    Err(Problem::NotReady)
                );
                assert!(!p.operations[Command::GetFrameTree as usize].issued);
                assert!(!p.operations[Command::Evaluate as usize].issued);
                p.lifecycle(selected_event(missing), at(11)).unwrap();
                tree_and_source(&mut p, 12);
                p.arm(at(14)).unwrap();
                assert!(p.evaluation_entered(FIXED_URL, at(15)).is_ok());
            }
        }

        // Mutation: observed Stop is treated as actual blank-context retirement.
        #[test]
        fn missing_retirement_blocks_until_actual_whole_pair_destroy() {
            let mut p = ready_without_tree(false);
            assert_eq!(
                p.command_entered(Command::GetFrameTree, at(10)),
                Err(Problem::NotReady)
            );
            assert_eq!(p.arm(at(10)), Err(Problem::NotReady));
            assert!(!p.blank_destroyed);
            assert!(!p.cleared);
            destroy_blank(&mut p, 11);
            tree_and_source(&mut p, 12);
            p.arm(at(14)).unwrap();
        }

        // Mutation: third branch loses BF/BL/BU/tombstone and clear-order protections.
        #[test]
        fn selected_identity_retirement_and_late_blank_rules_are_unchanged() {
            for (event, problem) in [
                (
                    Lifecycle::FrameNavigated(
                        Document::Selected,
                        Frame {
                            id: id("other"),
                            loader: id("s"),
                        },
                    ),
                    Problem::Root,
                ),
                (
                    Lifecycle::ContextCreated(
                        Document::Selected,
                        Context::new(id("other"), 7, id("s")).unwrap(),
                    ),
                    Problem::Root,
                ),
                (
                    Lifecycle::FrameNavigated(Document::Selected, frame("blank-loader")),
                    Problem::Root,
                ),
                (Lifecycle::StartedLoading(id("other")), Problem::Root),
                (Lifecycle::StoppedLoading(id("other")), Problem::Root),
                (
                    Lifecycle::ContextCreated(Document::Selected, context("blank-context")),
                    Problem::Context,
                ),
                (
                    Lifecycle::ContextDestroyed {
                        numeric: 8,
                        unique: id("blank-context"),
                    },
                    Problem::Context,
                ),
                (
                    Lifecycle::ContextDestroyed {
                        numeric: 7,
                        unique: id("wrong"),
                    },
                    Problem::Context,
                ),
                (
                    Lifecycle::ContextDestroyed {
                        numeric: 7,
                        unique: id("selected-context"),
                    },
                    Problem::Context,
                ),
                (
                    Lifecycle::FrameNavigated(Document::Blank, frame("late")),
                    Problem::Lifecycle,
                ),
                (
                    Lifecycle::ContextCreated(Document::Blank, context("late")),
                    Problem::Lifecycle,
                ),
            ] {
                let mut p = reserved(false, true);
                assert_eq!(p.lifecycle(event, at(6)), Err(problem));
                assert!(p.status().action_revoked);
                assert_eq!(p.arm(at(7)), Err(Problem::NotReady));
            }
            let mut p = reserved(true, false);
            p.lifecycle(selected_event(1), at(6)).unwrap();
            assert_eq!(
                p.lifecycle(Lifecycle::ContextsCleared, at(7)),
                Err(Problem::Lifecycle)
            );
            let mut p = reserved(true, false);
            destroy_blank(&mut p, 6);
            assert_eq!(
                p.lifecycle(
                    Lifecycle::ContextDestroyed {
                        numeric: 7,
                        unique: id("blank-context")
                    },
                    at(7)
                ),
                Err(Problem::Context)
            );
        }

        // Mutation: stop-only skips actual retirement or tombstone/clear ordering differs from Observed.
        #[test]
        fn real_redundant_retirement_signals_keep_tombstone_rules() {
            for clear_first in [false, true] {
                let mut p = reserved(true, true);
                if clear_first {
                    p.lifecycle(Lifecycle::ContextsCleared, at(6)).unwrap();
                    destroy_blank(&mut p, 6);
                } else {
                    destroy_blank(&mut p, 6);
                    p.lifecycle(Lifecycle::ContextsCleared, at(6)).unwrap();
                }
                for index in 0..4 {
                    p.lifecycle(selected_event(index), at(7)).unwrap();
                }
                native_navigation(&mut p);
                tree_and_source(&mut p, 10);
                p.arm(at(12)).unwrap();
                assert_eq!(
                    p.lifecycle(
                        Lifecycle::ContextDestroyed {
                            numeric: 7,
                            unique: id("blank-context")
                        },
                        at(13)
                    ),
                    Err(Problem::Lifecycle)
                );
            }
            let mut p = reserved(true, false);
            p.lifecycle(Lifecycle::ContextsCleared, at(6)).unwrap();
            assert_eq!(
                p.lifecycle(Lifecycle::ContextsCleared, at(7)),
                Err(Problem::Lifecycle)
            );
        }

        // Mutation: missing/failed native navigation or response is replaced by CDP facts.
        #[test]
        fn every_native_navigation_and_response_input_remains_mandatory() {
            for missing in 0..4 {
                let mut p = reserved(true, false);
                destroy_blank(&mut p, 6);
                for index in 0..4 {
                    p.lifecycle(selected_event(index), at(7)).unwrap();
                }
                if missing != 0 {
                    p.navigation_started(0, FIXED_URL, false, at(8)).unwrap();
                }
                if missing != 0 && missing != 1 {
                    p.navigation_completed(0, true, at(9)).unwrap();
                }
                if missing != 2 {
                    p.navigation_returned(true, at(9)).unwrap();
                }
                if missing != 3 {
                    p.main_response_installed(true, at(9)).unwrap();
                }
                assert_eq!(
                    p.command_entered(Command::GetFrameTree, at(10)),
                    Err(Problem::NotReady)
                );
                assert_eq!(p.arm(at(10)), Err(Problem::NotReady));
                match missing {
                    0 => {
                        p.navigation_started(0, FIXED_URL, false, at(11)).unwrap();
                        p.navigation_completed(0, true, at(11)).unwrap();
                    }
                    1 => p.navigation_completed(0, true, at(11)).unwrap(),
                    2 => p.navigation_returned(true, at(11)).unwrap(),
                    _ => p.main_response_installed(true, at(11)).unwrap(),
                }
                tree_and_source(&mut p, 12);
                p.arm(at(14)).unwrap();
            }
            for bad in 0..6 {
                let mut p = reserved(true, false);
                let result = match bad {
                    0 => p.navigation_started(0, "about:blank", false, at(6)),
                    1 => p.navigation_started(0, FIXED_URL, true, at(6)),
                    2 | 3 => {
                        p.navigation_started(0, FIXED_URL, false, at(6)).unwrap();
                        p.navigation_completed(if bad == 2 { 1 } else { 0 }, bad == 2, at(7))
                    }
                    4 => p.navigation_returned(false, at(6)),
                    _ => p.main_response_installed(false, at(6)),
                };
                assert_eq!(result, Err(Problem::Native));
                assert_eq!(p.arm(at(8)), Err(Problem::NotReady));
            }
        }

        // Mutation: tree ACK or fresh full Source becomes optional or adopts another loader.
        #[test]
        fn tree_and_both_source_boundaries_cannot_be_substituted() {
            let mut p = ready_without_tree(true);
            assert_eq!(
                p.source_after_navigation(FIXED_URL, at(10)),
                Err(Problem::NotReady)
            );
            assert_eq!(p.arm(at(10)), Err(Problem::NotReady));
            let tree = p.command_entered(Command::GetFrameTree, at(11)).unwrap();
            complete(
                &mut p,
                tree,
                CompletionValue::FrameTree(&frame("wrong-loader")),
                11,
            );
            assert_eq!(p.status().failure, Some(Problem::Result));
            assert_eq!(p.arm(at(12)), Err(Problem::NotReady));
            let mut p = ready_without_tree(true);
            let tree = p.command_entered(Command::GetFrameTree, at(10)).unwrap();
            complete(
                &mut p,
                tree,
                CompletionValue::FrameTree(&frame("selected-loader")),
                10,
            );
            assert_eq!(p.arm(at(11)), Err(Problem::NotReady));
            assert_eq!(
                p.source_after_navigation("https://console.code0.ai/", at(11)),
                Err(Problem::Native)
            );
            let mut p = ready_without_tree(true);
            tree_and_source(&mut p, 10);
            p.arm(at(12)).unwrap();
            assert_eq!(
                p.evaluation_entered("about:blank", at(13)),
                Err(Problem::Native)
            );
            assert!(!p.operations[Command::Evaluate as usize].issued);
        }

        // Mutation: reserved third branch can be reused or reopens lifecycle after arm.
        #[test]
        fn reservation_and_postarm_lifecycle_are_once_only() {
            let mut p = reserved(true, true);
            assert_eq!(p.navigation_reserved(at(6)), Err(Problem::NotReady));
            assert_eq!(
                p.status().blank_branch,
                Some(BlankBranch::ObservedInitialStopOnly)
            );
            for index in 0..4 {
                let mut p = ready_without_tree(true);
                tree_and_source(&mut p, 10);
                p.arm(at(12)).unwrap();
                assert_eq!(
                    p.lifecycle(selected_event(index), at(13)),
                    Err(Problem::Lifecycle)
                );
                assert_eq!(p.status().failure, Some(Problem::Lifecycle));
                assert!(p.evaluation_entered(FIXED_URL, at(14)).is_err());
            }
        }

        // Mutation: a new branch bypasses denied-result/native-action/cutoff conjunctions.
        #[test]
        fn effect_needs_native_action_successful_closed_result_and_pre_cutoff_qualification() {
            let (mut p, command) = issued_stop_only();
            effect(&mut p, command, 14);
            assert_eq!(p.status().effect_qualified, Some(false));
            for (success, value) in [
                (false, CompletionValue::Denied),
                (true, CompletionValue::Default),
                (true, CompletionValue::Granted),
                (true, CompletionValue::Malformed),
            ] {
                let (mut p, command) = issued_stop_only();
                native(&mut p, 14);
                let callback = p.completion_entered(command, at(15)).unwrap();
                p.completion_captured(callback, success, value, at(15))
                    .unwrap();
                p.completion_qualified(callback, at(15)).unwrap();
                p.completion_consumed(command, at(16)).unwrap();
                assert_eq!(p.status().effect_qualified, Some(false));
                assert_eq!(p.status().obligations, 0);
            }
            for before_capture in [false, true] {
                let (mut p, command) = issued_stop_only();
                native(&mut p, 14);
                let callback = p.completion_entered(command, at(15)).unwrap();
                if before_capture {
                    p.cutoff(CutoffReason::RequiredRemovalAttempt, at(15));
                }
                p.completion_captured(callback, true, CompletionValue::Denied, at(15))
                    .unwrap();
                if !before_capture {
                    p.cutoff(CutoffReason::RequiredRemovalAttempt, at(15));
                }
                p.completion_qualified(callback, at(15)).unwrap();
                p.completion_consumed(command, at(16)).unwrap();
                assert_eq!(p.status().effect_qualified, Some(false));
                assert_eq!(p.status().obligations, 0);
            }
        }

        // Mutation: native DENY or real handler Drop is credited as an absent effect invocation.
        #[test]
        fn missing_effect_never_drains_and_native_failure_never_qualifies() {
            let (mut p, command) = issued_stop_only();
            native(&mut p, 14);
            assert_eq!(p.status().native_qualified, Some(true));
            assert_eq!(p.status().effect_qualified, None);
            p.command_handler_dropped(command, at(15)).unwrap();
            assert_eq!(p.status().obligations, 1);
            assert_eq!(p.status().effect_qualified, None);
            let (mut p, _) = issued_stop_only();
            let ticket = p.native_entered(at(14)).unwrap();
            for check in NativeCheck::ALL {
                p.native_checked(ticket, check, check != NativeCheck::AfterDeny, at(14))
                    .unwrap();
                if check == NativeCheck::DenyAttempted {
                    p.native_refused(ticket, at(14)).unwrap();
                }
            }
            p.native_qualified(ticket, at(14)).unwrap();
            assert_eq!(p.status().native_qualified, Some(false));
            assert!(p.status().action_revoked);
        }

        // Mutation: new-branch receipt extends45/close10 or permits physical postStop evidence.
        #[test]
        fn original_and_close_equality_and_physical_entry_keep_their_negative_meanings() {
            let (mut p, command) = issued_stop_only();
            native(&mut p, 44_999);
            effect(&mut p, command, 45_000);
            assert_eq!(p.status().effect_qualified, Some(false));
            assert_eq!(p.status().failure, Some(Problem::Deadline));
            assert_eq!(p.status().first_close_ms, Some(44_999));
            let (mut p, command) = issued_stop_only();
            native(&mut p, 14);
            assert_eq!(
                p.completion_entered(command, at(10_014)),
                Err(Problem::CloseDeadline)
            );
            assert_eq!(p.status().obligations, 1);
            assert_eq!(p.status().first_close_ms, Some(14));
            for reason in [
                CutoffReason::StopEntry,
                CutoffReason::ControllerCloseEntry,
                CutoffReason::ControlsDisposal,
            ] {
                let (mut p, command) = issued_stop_only();
                native(&mut p, 14);
                p.cutoff(reason, at(15));
                effect(&mut p, command, 16);
                assert_eq!(p.status().effect_qualified, Some(false));
                assert_eq!(p.status().obligations, 0);
                assert!(p.status().action_revoked);
            }
        }
    }
}

#[cfg(test)]
mod native_failure_fix2_tests {
    use super::*;

    // Mutation: a local native/counter failure leaves authority live or restarts close10.
    #[test]
    fn adapter_failure_revokes_without_inventing_delivery_ticket_or_restarting_close() {
        let identity = Identity {
            request: 1,
            generation: 2,
            installation: 3,
            core: 4,
        };
        let mut protocol = Protocol::new(identity, 0).unwrap();
        protocol.native_adapter_failed(Boundary { identity, ms: 1 });
        let first = protocol.status();
        assert!(first.action_revoked);
        assert_eq!(first.first_close_ms, Some(1));
        assert_eq!(first.failure, Some(Problem::Native));
        assert_eq!(first.deliveries.value, 0);
        assert_eq!(first.obligations, 0);
        protocol.native_adapter_failed(Boundary { identity, ms: 999 });
        assert_eq!(protocol.status().first_close_ms, Some(1));
        protocol.adopt_native_close_origin(0);
        assert_eq!(protocol.status().first_close_ms, Some(0));
        protocol.adopt_native_close_origin(500);
        assert_eq!(protocol.status().first_close_ms, Some(0));
        assert!(
            protocol
                .native_entered(Boundary { identity, ms: 1000 })
                .is_err()
        );
        assert_eq!(protocol.status().obligations, 0);
    }
}

#[cfg(test)]
mod native_clock_fix2_tests {
    use super::*;

    // Mutation: unavailable native clock invents a new observed time or grants a ticket.
    #[test]
    fn unavailable_native_clock_is_negative_and_retains_last_known_semantic_time() {
        let identity = Identity {
            request: 1,
            generation: 2,
            installation: 3,
            core: 4,
        };
        let mut protocol = Protocol::new(identity, 0).unwrap();
        protocol.native_clock_failed();
        assert_eq!(protocol.status().failure, Some(Problem::Clock));
        assert_eq!(protocol.status().first_close_ms, Some(0));
        assert!(protocol.status().action_revoked);
        assert_eq!(protocol.status().deliveries.value, 0);
        assert_eq!(protocol.status().obligations, 0);
    }
}
