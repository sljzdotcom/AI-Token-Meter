use serde::Serialize;
use time::OffsetDateTime;

use crate::collectors::deepseek_history::{
    AcceptOutcome, DeepSeekHistory, DeepSeekHistoryAssembler, DeepSeekHistoryChunk,
    DeepSeekHistoryError,
};

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
    RestoreDetail,
    DestroyHistory,
    EmitStatus(DeepSeekHistoryWindowStatus),
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
    phase: DeepSeekHistorySessionPhase,
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
    Waiting,
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
                actions: vec![DeepSeekHistoryWindowAction::FocusExisting],
            };
        }

        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        let generation = DeepSeekHistoryGeneration(self.next_generation);
        self.session = Some(DeepSeekHistorySession {
            generation,
            nonce: nonce.to_owned(),
            assembler: DeepSeekHistoryAssembler::new(nonce, now),
            phase: DeepSeekHistorySessionPhase::Opening,
        });
        self.status = DeepSeekHistoryWindowStatus::Opening;
        DeepSeekHistoryWindowOpen {
            generation,
            actions: vec![
                DeepSeekHistoryWindowAction::CreateHidden,
                DeepSeekHistoryWindowAction::EmitStatus(DeepSeekHistoryWindowStatus::Opening),
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
                DeepSeekHistoryWindowAction::EmitStatus(DeepSeekHistoryWindowStatus::Active),
            ])
        } else {
            session.phase =
                DeepSeekHistorySessionPhase::Terminating(DeepSeekHistoryTerminal::Failed);
            DeepSeekHistoryReadyResolution::Recover(terminal_claim(
                claim.generation,
                DeepSeekHistoryTerminal::Failed,
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
            AcceptOutcome::Waiting => Ok(DeepSeekHistoryChunkOutcome::Waiting),
            AcceptOutcome::Complete(history) => {
                session.phase =
                    DeepSeekHistorySessionPhase::Terminating(DeepSeekHistoryTerminal::Completed);
                Ok(DeepSeekHistoryChunkOutcome::Complete {
                    history,
                    terminal: terminal_claim(generation, DeepSeekHistoryTerminal::Completed),
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
        Some(terminal_claim(generation, terminal))
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
        self.session = None;
        self.status = status;
        vec![DeepSeekHistoryWindowAction::EmitStatus(status)]
    }
}

fn terminal_claim(
    generation: DeepSeekHistoryGeneration,
    terminal: DeepSeekHistoryTerminal,
) -> DeepSeekHistoryTerminalClaim {
    let actions = match terminal {
        DeepSeekHistoryTerminal::Cancelled => vec![DeepSeekHistoryWindowAction::RestoreDetail],
        DeepSeekHistoryTerminal::Completed | DeepSeekHistoryTerminal::Failed => vec![
            DeepSeekHistoryWindowAction::DestroyHistory,
            DeepSeekHistoryWindowAction::RestoreDetail,
        ],
    };
    DeepSeekHistoryTerminalClaim {
        generation,
        terminal,
        actions,
    }
}
