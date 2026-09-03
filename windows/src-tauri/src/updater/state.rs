use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdatePhase {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Installing,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub phase: UpdatePhase,
    pub current_version: String,
    pub available_version: Option<String>,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
}

impl UpdateState {
    pub fn new(current_version: impl Into<String>) -> Self {
        Self {
            phase: UpdatePhase::Idle,
            current_version: current_version.into(),
            available_version: None,
            progress_percent: None,
            message: None,
        }
    }

    pub fn begin_check(&mut self) -> Result<(), UpdateStateError> {
        if matches!(
            self.phase,
            UpdatePhase::Checking | UpdatePhase::Downloading | UpdatePhase::Installing
        ) {
            return Err(UpdateStateError::OperationInProgress);
        }
        self.phase = UpdatePhase::Checking;
        self.available_version = None;
        self.progress_percent = None;
        self.message = None;
        Ok(())
    }

    pub fn finish_check(
        &mut self,
        available_version: Option<String>,
    ) -> Result<(), UpdateStateError> {
        if self.phase != UpdatePhase::Checking {
            return Err(UpdateStateError::InvalidTransition);
        }
        if let Some(version) = available_version {
            if !valid_semver(&version) {
                self.fail("The update service returned an invalid version");
                return Err(UpdateStateError::InvalidVersion);
            }
            self.phase = UpdatePhase::Available;
            self.available_version = Some(version);
        } else {
            self.phase = UpdatePhase::UpToDate;
            self.available_version = None;
        }
        Ok(())
    }

    pub fn can_install(&self) -> bool {
        self.phase == UpdatePhase::Available && self.available_version.is_some()
    }

    pub fn begin_install(&mut self) -> Result<(), UpdateStateError> {
        if !self.can_install() {
            return Err(UpdateStateError::InvalidTransition);
        }
        self.phase = UpdatePhase::Downloading;
        self.progress_percent = None;
        self.message = None;
        Ok(())
    }

    pub fn report_progress(&mut self, downloaded: u64, total: Option<u64>) {
        self.progress_percent = total.filter(|total| *total > 0).map(|total| {
            downloaded
                .saturating_mul(100)
                .saturating_div(total)
                .min(100) as u8
        });
    }

    pub fn mark_installing(&mut self) {
        if self.phase == UpdatePhase::Downloading {
            self.phase = UpdatePhase::Installing;
            self.progress_percent = Some(100);
        }
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.phase = UpdatePhase::Failed;
        self.available_version = None;
        self.progress_percent = None;
        self.message = Some(message.into());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateStateError {
    OperationInProgress,
    InvalidTransition,
    InvalidVersion,
}

fn valid_semver(value: &str) -> bool {
    let value = value.strip_prefix('v').unwrap_or(value);
    let (core, suffix) = value
        .split_once('-')
        .map_or((value, None), |(core, suffix)| (core, Some(suffix)));
    let mut numbers = core.split('.');
    let valid_core = (0..3).all(|_| {
        numbers.next().is_some_and(|part| {
            !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
        })
    }) && numbers.next().is_none();
    let valid_suffix = suffix.is_none_or(|suffix| {
        !suffix.is_empty()
            && suffix.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '-')
            })
    });
    valid_core && valid_suffix
}
