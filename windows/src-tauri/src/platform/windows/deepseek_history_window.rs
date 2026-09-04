use serde::Serialize;

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
pub enum DeepSeekHistoryWindowEvent {
    OpenRequested,
    PageLoadFinished,
    LoadTimedOut,
    WindowClosed,
    Completed,
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

#[derive(Debug, Default)]
pub struct DeepSeekHistoryWindowMachine {
    status: DeepSeekHistoryWindowStatus,
    session_active: bool,
}

impl DeepSeekHistoryWindowMachine {
    pub fn status(&self) -> DeepSeekHistoryWindowStatus {
        self.status
    }

    pub fn has_session(&self) -> bool {
        self.session_active
    }

    pub fn transition(
        &mut self,
        event: DeepSeekHistoryWindowEvent,
    ) -> Vec<DeepSeekHistoryWindowAction> {
        use DeepSeekHistoryWindowAction as Action;
        use DeepSeekHistoryWindowEvent as Event;
        use DeepSeekHistoryWindowStatus as Status;

        match event {
            Event::OpenRequested if self.session_active => vec![Action::FocusExisting],
            Event::OpenRequested => {
                self.status = Status::Opening;
                self.session_active = true;
                vec![Action::CreateHidden, Action::EmitStatus(Status::Opening)]
            }
            Event::PageLoadFinished if self.status == Status::Opening && self.session_active => {
                self.status = Status::Active;
                vec![Action::ShowFocused, Action::EmitStatus(Status::Active)]
            }
            Event::LoadTimedOut if self.status == Status::Opening && self.session_active => {
                self.status = Status::Failed;
                self.session_active = false;
                vec![
                    Action::DestroyHistory,
                    Action::RestoreDetail,
                    Action::EmitStatus(Status::Failed),
                ]
            }
            Event::WindowClosed if self.session_active => {
                self.status = Status::Cancelled;
                self.session_active = false;
                vec![Action::RestoreDetail, Action::EmitStatus(Status::Cancelled)]
            }
            Event::Completed if self.session_active => {
                self.status = Status::Completed;
                self.session_active = false;
                vec![
                    Action::DestroyHistory,
                    Action::RestoreDetail,
                    Action::EmitStatus(Status::Completed),
                ]
            }
            Event::Failed if self.session_active => {
                self.status = Status::Failed;
                self.session_active = false;
                vec![
                    Action::DestroyHistory,
                    Action::RestoreDetail,
                    Action::EmitStatus(Status::Failed),
                ]
            }
            _ => Vec::new(),
        }
    }
}
