use serde::Serialize;
use time::{Duration, OffsetDateTime};

use crate::collectors::deepseek_history::{
    AcceptOutcome, DeepSeekHistory, DeepSeekHistoryAssembler, DeepSeekHistoryChunk,
    DeepSeekHistoryError,
};
use crate::platform::windows::window_controller::DetailOwnershipToken;

const INTERACTIVE_SESSION_TIMEOUT: Duration = Duration::minutes(15);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeepSeekHistoryWindowStatus {
    #[default]
    Idle,
    Opening,
    Active,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeepSeekHistoryWindowAction {
    CreateHidden,
    ShowFocused,
    FocusExisting,
    RestoreDetail(DetailOwnershipToken),
    DestroyHistory,
    EmitStatus(DeepSeekHistoryStatusSnapshot),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekHistoryStatusSnapshot {
    pub generation: Option<u64>,
    pub status: DeepSeekHistoryWindowStatus,
}

pub trait DeepSeekHistoryWindowActionExecutor {
    type Error;

    fn execute(&mut self, action: DeepSeekHistoryWindowAction) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeepSeekHistoryWindowExecution {
    failed_actions: Vec<DeepSeekHistoryWindowAction>,
    operation_failed: bool,
}

impl DeepSeekHistoryWindowExecution {
    pub fn failed_actions(&self) -> &[DeepSeekHistoryWindowAction] {
        &self.failed_actions
    }

    pub fn succeeded(&self) -> bool {
        self.failed_actions.is_empty() && !self.operation_failed
    }

    pub fn record_operation_failure(&mut self) {
        self.operation_failed = true;
    }
}

pub fn execute_window_actions<E: DeepSeekHistoryWindowActionExecutor>(
    actions: &[DeepSeekHistoryWindowAction],
    executor: &mut E,
) -> DeepSeekHistoryWindowExecution {
    let mut execution = DeepSeekHistoryWindowExecution::default();
    for action in actions {
        if executor.execute(*action).is_err() {
            execution.failed_actions.push(*action);
        }
    }
    execution
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeepSeekHistoryGeneration(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeepSeekHistoryWindowOpen {
    pub generation: DeepSeekHistoryGeneration,
    pub status: DeepSeekHistoryStatusSnapshot,
    pub actions: Vec<DeepSeekHistoryWindowAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeepSeekHistoryTerminal {
    Completed,
    Cancelled,
    Failed,
}

impl DeepSeekHistoryTerminal {
    fn status(self) -> DeepSeekHistoryWindowStatus {
        match self {
            Self::Completed => DeepSeekHistoryWindowStatus::Completed,
            Self::Cancelled => DeepSeekHistoryWindowStatus::Cancelled,
            Self::Failed => DeepSeekHistoryWindowStatus::Failed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeepSeekHistorySessionPhase {
    Opening,
    Activating,
    Active,
    Terminating(DeepSeekHistoryTerminal),
}

struct DeepSeekHistorySession {
    generation: DeepSeekHistoryGeneration,
    nonce: String,
    assembler: DeepSeekHistoryAssembler,
    opened_at: OffsetDateTime,
    phase: DeepSeekHistorySessionPhase,
    detail_ownership: Option<DetailOwnershipToken>,
    transfer_started: bool,
    cleanup_pending: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeepSeekHistoryReadyClaim {
    generation: DeepSeekHistoryGeneration,
    actions: Vec<DeepSeekHistoryWindowAction>,
}

impl DeepSeekHistoryReadyClaim {
    pub fn actions(&self) -> &[DeepSeekHistoryWindowAction] {
        &self.actions
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeepSeekHistoryTerminalClaim {
    generation: DeepSeekHistoryGeneration,
    terminal: DeepSeekHistoryTerminal,
    actions: Vec<DeepSeekHistoryWindowAction>,
}

impl DeepSeekHistoryTerminalClaim {
    pub fn generation(&self) -> DeepSeekHistoryGeneration {
        self.generation
    }

    pub fn actions(&self) -> &[DeepSeekHistoryWindowAction] {
        &self.actions
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeepSeekHistoryReadyResolution {
    Activated(Vec<DeepSeekHistoryWindowAction>),
    Recover(DeepSeekHistoryTerminalClaim),
    Ignored,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeepSeekHistoryChunkOutcome {
    Ignored,
    Waiting {
        transfer_started: bool,
    },
    Complete {
        history: DeepSeekHistory,
        terminal: DeepSeekHistoryTerminalClaim,
    },
}

pub struct DeepSeekHistoryWindowCoordinator {
    status: DeepSeekHistoryWindowStatus,
    next_generation: u64,
    session: Option<DeepSeekHistorySession>,
}

impl Default for DeepSeekHistoryWindowCoordinator {
    fn default() -> Self {
        Self {
            status: DeepSeekHistoryWindowStatus::Idle,
            next_generation: 0,
            session: None,
        }
    }
}

impl DeepSeekHistoryWindowCoordinator {
    pub fn status(&self) -> DeepSeekHistoryWindowStatus {
        self.status
    }

    pub fn status_snapshot(&self) -> DeepSeekHistoryStatusSnapshot {
        DeepSeekHistoryStatusSnapshot {
            generation: (self.next_generation != 0).then_some(self.next_generation),
            status: self.status,
        }
    }

    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    pub fn current_generation(&self) -> Option<DeepSeekHistoryGeneration> {
        self.session.as_ref().map(|session| session.generation)
    }

    pub fn is_opening(&self, generation: DeepSeekHistoryGeneration) -> bool {
        self.session.as_ref().is_some_and(|session| {
            session.generation == generation
                && session.phase == DeepSeekHistorySessionPhase::Opening
        })
    }

    pub fn open(&mut self, nonce: &str, now: OffsetDateTime) -> DeepSeekHistoryWindowOpen {
        if let Some(session) = &self.session {
            return DeepSeekHistoryWindowOpen {
                generation: session.generation,
                status: self.status_snapshot(),
                actions: if session.phase == DeepSeekHistorySessionPhase::Active {
                    vec![DeepSeekHistoryWindowAction::FocusExisting]
                } else {
                    Vec::new()
                },
            };
        }

        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        let generation = DeepSeekHistoryGeneration(self.next_generation);
        self.session = Some(DeepSeekHistorySession {
            generation,
            nonce: nonce.to_owned(),
            assembler: DeepSeekHistoryAssembler::new(nonce),
            opened_at: now,
            phase: DeepSeekHistorySessionPhase::Opening,
            detail_ownership: None,
            transfer_started: false,
            cleanup_pending: false,
        });
        self.status = DeepSeekHistoryWindowStatus::Opening;
        let status = self.status_snapshot();
        DeepSeekHistoryWindowOpen {
            generation,
            status,
            actions: vec![
                DeepSeekHistoryWindowAction::CreateHidden,
                DeepSeekHistoryWindowAction::EmitStatus(status),
            ],
        }
    }

    pub fn navigation_finished(
        &mut self,
        _generation: DeepSeekHistoryGeneration,
    ) -> Vec<DeepSeekHistoryWindowAction> {
        Vec::new()
    }

    pub fn claim_ready(
        &mut self,
        generation: DeepSeekHistoryGeneration,
        nonce: &str,
    ) -> Option<DeepSeekHistoryReadyClaim> {
        let session = self.session.as_mut()?;
        if session.generation != generation
            || session.nonce != nonce
            || session.phase != DeepSeekHistorySessionPhase::Opening
        {
            return None;
        }
        session.phase = DeepSeekHistorySessionPhase::Activating;
        Some(DeepSeekHistoryReadyClaim {
            generation,
            actions: vec![DeepSeekHistoryWindowAction::ShowFocused],
        })
    }

    pub fn attach_detail_ownership(
        &mut self,
        generation: DeepSeekHistoryGeneration,
        ownership: DetailOwnershipToken,
    ) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        if session.generation != generation
            || session.phase != DeepSeekHistorySessionPhase::Activating
            || session.detail_ownership.is_some()
        {
            return false;
        }
        session.detail_ownership = Some(ownership);
        true
    }

    pub fn finish_ready(
        &mut self,
        claim: DeepSeekHistoryReadyClaim,
        execution: &DeepSeekHistoryWindowExecution,
    ) -> DeepSeekHistoryReadyResolution {
        let Some(session) = self.session.as_mut() else {
            return DeepSeekHistoryReadyResolution::Ignored;
        };
        if session.generation != claim.generation
            || session.phase != DeepSeekHistorySessionPhase::Activating
        {
            return DeepSeekHistoryReadyResolution::Ignored;
        }
        if execution.succeeded() {
            session.phase = DeepSeekHistorySessionPhase::Active;
            self.status = DeepSeekHistoryWindowStatus::Active;
            DeepSeekHistoryReadyResolution::Activated(vec![
                DeepSeekHistoryWindowAction::EmitStatus(status_snapshot(
                    claim.generation,
                    DeepSeekHistoryWindowStatus::Active,
                )),
            ])
        } else {
            session.phase =
                DeepSeekHistorySessionPhase::Terminating(DeepSeekHistoryTerminal::Failed);
            DeepSeekHistoryReadyResolution::Recover(terminal_claim(
                claim.generation,
                DeepSeekHistoryTerminal::Failed,
                session.detail_ownership,
            ))
        }
    }

    pub fn accept_chunk(
        &mut self,
        generation: DeepSeekHistoryGeneration,
        chunk: DeepSeekHistoryChunk,
        now: OffsetDateTime,
    ) -> Result<DeepSeekHistoryChunkOutcome, DeepSeekHistoryError> {
        let Some(session) = self.session.as_mut() else {
            return Ok(DeepSeekHistoryChunkOutcome::Ignored);
        };
        if session.generation != generation
            || matches!(session.phase, DeepSeekHistorySessionPhase::Terminating(_))
        {
            return Ok(DeepSeekHistoryChunkOutcome::Ignored);
        }
        match session.assembler.accept(chunk, now)? {
            AcceptOutcome::Waiting => {
                let transfer_started = !session.transfer_started;
                session.transfer_started = true;
                Ok(DeepSeekHistoryChunkOutcome::Waiting { transfer_started })
            }
            AcceptOutcome::Complete(history) => {
                session.phase =
                    DeepSeekHistorySessionPhase::Terminating(DeepSeekHistoryTerminal::Completed);
                Ok(DeepSeekHistoryChunkOutcome::Complete {
                    history,
                    terminal: terminal_claim(
                        generation,
                        DeepSeekHistoryTerminal::Completed,
                        session.detail_ownership,
                    ),
                })
            }
        }
    }

    pub fn claim_timeout(
        &mut self,
        generation: DeepSeekHistoryGeneration,
    ) -> Option<DeepSeekHistoryTerminalClaim> {
        if !self.is_opening(generation) {
            return None;
        }
        self.claim_terminal(generation, DeepSeekHistoryTerminal::Failed)
    }

    pub fn claim_session_timeout(
        &mut self,
        generation: DeepSeekHistoryGeneration,
        now: OffsetDateTime,
    ) -> Option<DeepSeekHistoryTerminalClaim> {
        let session = self.session.as_ref()?;
        if session.generation != generation
            || matches!(session.phase, DeepSeekHistorySessionPhase::Terminating(_))
            || now < session.opened_at
            || now - session.opened_at < INTERACTIVE_SESSION_TIMEOUT
        {
            return None;
        }
        self.claim_terminal(generation, DeepSeekHistoryTerminal::Failed)
    }

    pub fn is_transfer_in_progress(&self, generation: DeepSeekHistoryGeneration) -> bool {
        self.session.as_ref().is_some_and(|session| {
            session.generation == generation
                && session.transfer_started
                && !matches!(session.phase, DeepSeekHistorySessionPhase::Terminating(_))
        })
    }

    pub fn claim_transfer_timeout(
        &mut self,
        generation: DeepSeekHistoryGeneration,
    ) -> Option<DeepSeekHistoryTerminalClaim> {
        if !self.is_transfer_in_progress(generation) {
            return None;
        }
        self.claim_terminal(generation, DeepSeekHistoryTerminal::Failed)
    }

    pub fn claim_closed(
        &mut self,
        generation: DeepSeekHistoryGeneration,
    ) -> Option<DeepSeekHistoryTerminalClaim> {
        self.claim_terminal(generation, DeepSeekHistoryTerminal::Cancelled)
    }

    pub fn claim_failed(
        &mut self,
        generation: DeepSeekHistoryGeneration,
    ) -> Option<DeepSeekHistoryTerminalClaim> {
        self.claim_terminal(generation, DeepSeekHistoryTerminal::Failed)
    }

    pub fn cleanup_generation(&self) -> Option<DeepSeekHistoryGeneration> {
        self.session
            .as_ref()
            .and_then(|session| session.cleanup_pending.then_some(session.generation))
    }

    pub fn reconcile_cleanup(&mut self, generation: DeepSeekHistoryGeneration) -> bool {
        if self.cleanup_generation() != Some(generation) {
            return false;
        }
        self.session = None;
        true
    }

    pub fn reconcile_destroyed(&mut self, generation: DeepSeekHistoryGeneration) -> bool {
        self.reconcile_cleanup(generation)
    }

    pub fn finish_cleanup_after_window_removed(
        &mut self,
        generation: DeepSeekHistoryGeneration,
    ) -> bool {
        let Some(session) = self.session.as_ref() else {
            return true;
        };
        if session.generation != generation || !session.cleanup_pending {
            return false;
        }
        self.session = None;
        true
    }

    fn claim_terminal(
        &mut self,
        generation: DeepSeekHistoryGeneration,
        terminal: DeepSeekHistoryTerminal,
    ) -> Option<DeepSeekHistoryTerminalClaim> {
        let session = self.session.as_mut()?;
        if session.generation != generation
            || matches!(session.phase, DeepSeekHistorySessionPhase::Terminating(_))
        {
            return None;
        }
        session.phase = DeepSeekHistorySessionPhase::Terminating(terminal);
        Some(terminal_claim(
            generation,
            terminal,
            session.detail_ownership,
        ))
    }

    pub fn finish_terminal(
        &mut self,
        claim: DeepSeekHistoryTerminalClaim,
        execution: &DeepSeekHistoryWindowExecution,
    ) -> Vec<DeepSeekHistoryWindowAction> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        if session.generation != claim.generation
            || session.phase != DeepSeekHistorySessionPhase::Terminating(claim.terminal)
        {
            return Vec::new();
        }
        let status = if execution.succeeded() {
            claim.terminal.status()
        } else {
            DeepSeekHistoryWindowStatus::Failed
        };
        self.status = status;
        let destroy_failed = execution
            .failed_actions
            .contains(&DeepSeekHistoryWindowAction::DestroyHistory);
        if destroy_failed {
            if let Some(session) = self.session.as_mut() {
                session.cleanup_pending = true;
            }
        } else {
            self.session = None;
        }
        vec![DeepSeekHistoryWindowAction::EmitStatus(status_snapshot(
            claim.generation,
            status,
        ))]
    }
}

fn status_snapshot(
    generation: DeepSeekHistoryGeneration,
    status: DeepSeekHistoryWindowStatus,
) -> DeepSeekHistoryStatusSnapshot {
    DeepSeekHistoryStatusSnapshot {
        generation: Some(generation.0),
        status,
    }
}

fn terminal_claim(
    generation: DeepSeekHistoryGeneration,
    terminal: DeepSeekHistoryTerminal,
    detail_ownership: Option<DetailOwnershipToken>,
) -> DeepSeekHistoryTerminalClaim {
    let mut actions = Vec::with_capacity(2);
    if terminal != DeepSeekHistoryTerminal::Cancelled {
        actions.push(DeepSeekHistoryWindowAction::DestroyHistory);
    }
    if let Some(ownership) = detail_ownership {
        actions.push(DeepSeekHistoryWindowAction::RestoreDetail(ownership));
    }
    DeepSeekHistoryTerminalClaim {
        generation,
        terminal,
        actions,
    }
}
