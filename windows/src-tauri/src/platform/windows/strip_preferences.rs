use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StripPreferences {
    pub schema_version: u64,
    pub density: String,
    pub automatically_collapses: bool,
    pub reveal_delay_milliseconds: u64,
    pub collapse_delay_milliseconds: u64,
    pub ordered_providers: Vec<String>,
    pub hidden_providers: Vec<String>,
    pub hidden_until: Option<i64>,
}

impl<'de> Deserialize<'de> for StripPreferences {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize, Default)]
        #[serde(default, rename_all = "camelCase")]
        struct Stored {
            schema_version: Option<u64>,
            density: Option<String>,
            automatically_collapses: Option<bool>,
            fold_delay: Option<u64>,
            reveal_delay_milliseconds: Option<u64>,
            collapse_delay_milliseconds: Option<u64>,
            ordered_providers: Option<Vec<String>>,
            hidden_providers: Option<Vec<String>>,
            hidden_until: Option<i64>,
        }
        let stored = Stored::deserialize(deserializer)?;
        let already_has_gemini = stored
            .ordered_providers
            .iter()
            .chain(stored.hidden_providers.iter())
            .flatten()
            .any(|id| id == "gemini");
        let mut value = Self::default();
        let version = stored.schema_version.unwrap_or(0);
        if let Some(density) = stored.density {
            value.density = density;
        }
        if version >= 4
            && let Some(automatically_collapses) = stored.automatically_collapses
        {
            value.automatically_collapses = automatically_collapses;
        }
        if version >= 3 {
            if let Some(delay) = stored.reveal_delay_milliseconds {
                value.reveal_delay_milliseconds = delay;
            }
            if let Some(delay) = stored.collapse_delay_milliseconds {
                value.collapse_delay_milliseconds = delay;
            }
        } else if let Some(delay) = stored.fold_delay {
            value.collapse_delay_milliseconds = if delay == 0 {
                800
            } else {
                delay.saturating_mul(1_000).min(5_000)
            };
        }
        if let Some(order) = stored.ordered_providers {
            value.ordered_providers = order;
        }
        if let Some(hidden) = stored.hidden_providers {
            value.hidden_providers = hidden;
        }
        value.hidden_until = stored.hidden_until;
        if version < 2 && !already_has_gemini {
            value.hidden_providers.push("gemini".into());
        }
        value.normalize();
        Ok(value)
    }
}

impl Default for StripPreferences {
    fn default() -> Self {
        Self {
            schema_version: 4,
            density: "compact".into(),
            automatically_collapses: true,
            reveal_delay_milliseconds: 150,
            collapse_delay_milliseconds: 800,
            ordered_providers: vec![
                "claude".into(),
                "codex".into(),
                "deepseek".into(),
                "gemini".into(),
            ],
            hidden_providers: vec![],
            hidden_until: None,
        }
    }
}

impl StripPreferences {
    pub fn normalize(&mut self) {
        if !["comfortable", "compact", "mini"].contains(&self.density.as_str()) {
            self.density = "compact".into();
        }
        if self.reveal_delay_milliseconds > 2_000 {
            self.reveal_delay_milliseconds = 150;
        }
        if self.collapse_delay_milliseconds > 5_000 {
            self.collapse_delay_milliseconds = 800;
        }
        let known = ["claude", "codex", "deepseek", "gemini"];
        self.schema_version = 4;
        let mut order = Vec::new();
        for id in self
            .ordered_providers
            .iter()
            .map(String::as_str)
            .chain(known)
        {
            if known.contains(&id) && !order.contains(&id.to_owned()) {
                order.push(id.to_owned());
            }
        }
        self.ordered_providers = order;
        self.hidden_providers
            .retain(|id| known.contains(&id.as_str()));
        let mut seen = std::collections::HashSet::new();
        self.hidden_providers.retain(|id| seen.insert(id.clone()));
        if self.visible_providers().is_empty() {
            self.hidden_providers
                .retain(|id| id != &self.ordered_providers[0]);
        }
    }
    pub fn visible_providers(&self) -> Vec<String> {
        self.ordered_providers
            .iter()
            .filter(|id| !self.hidden_providers.contains(id))
            .cloned()
            .collect()
    }
    pub fn logical_size(&self, folded: bool) -> (f64, f64) {
        if folded {
            return (20.0, 96.0);
        }
        let count = self.visible_providers().len().clamp(1, 4);
        match self.density.as_str() {
            "comfortable" => (108.0, 212.0 + (count - 1) as f64 * 72.0),
            "mini" => (65.0, 170.0 + (count - 1) as f64 * 58.0),
            _ => (78.0, 170.0 + (count - 1) as f64 * 58.0),
        }
    }
    pub fn hidden(&self, now: i64) -> bool {
        self.hidden_until.is_some_and(|until| until > now)
    }
}

#[derive(Default)]
pub struct FoldState {
    reveal_deadline: Option<f64>,
    collapse_deadline: Option<f64>,
    pub folded: bool,
}
impl FoldState {
    pub fn update(
        &mut self,
        now: f64,
        reveal_delay_milliseconds: u64,
        collapse_delay_milliseconds: u64,
        hovering: bool,
        locked_open: bool,
        automatically_collapses: bool,
    ) -> bool {
        if locked_open || !automatically_collapses {
            self.folded = false;
            self.reveal_deadline = None;
            self.collapse_deadline = None;
            return false;
        }
        if self.folded {
            self.collapse_deadline = None;
            if !hovering {
                self.reveal_deadline = None;
                return true;
            }
            let deadline = self
                .reveal_deadline
                .get_or_insert(now + reveal_delay_milliseconds as f64 / 1_000.0);
            if now >= *deadline {
                self.folded = false;
                self.reveal_deadline = None;
            }
            return self.folded;
        }
        self.reveal_deadline = None;
        if hovering {
            self.collapse_deadline = None;
            return false;
        }
        let deadline = self
            .collapse_deadline
            .get_or_insert(now + collapse_delay_milliseconds as f64 / 1_000.0);
        if now >= *deadline {
            self.folded = true;
            self.collapse_deadline = None;
        }
        self.folded
    }
}

#[cfg(test)]
mod tests {
    use super::{FoldState, StripPreferences};

    #[test]
    fn schema_four_has_independent_bounded_delays_and_new_dimensions() {
        let defaults = StripPreferences::default();
        assert_eq!(defaults.schema_version, 4);
        assert!(defaults.automatically_collapses);
        assert_eq!(defaults.reveal_delay_milliseconds, 150);
        assert_eq!(defaults.collapse_delay_milliseconds, 800);
        assert_eq!(defaults.logical_size(false), (78.0, 344.0));
        assert_eq!(defaults.logical_size(true), (20.0, 96.0));

        let legacy: StripPreferences = serde_json::from_str(
            r#"{"schemaVersion":2,"foldDelay":5,"orderedProviders":["claude"]}"#,
        )
        .unwrap();
        assert_eq!(legacy.reveal_delay_milliseconds, 150);
        assert_eq!(legacy.collapse_delay_milliseconds, 5_000);
        assert!(legacy.automatically_collapses);

        let schema_three_with_unknown_new_field: StripPreferences = serde_json::from_str(
            r#"{"schemaVersion":3,"automaticallyCollapses":false,"orderedProviders":["claude"]}"#,
        )
        .unwrap();
        assert!(schema_three_with_unknown_new_field.automatically_collapses);

        let mut mini = defaults.clone();
        mini.density = "mini".into();
        assert_eq!(mini.logical_size(false), (65.0, 344.0));
    }

    #[test]
    fn disabling_automatic_collapse_immediately_expands_and_clears_deadlines() {
        let mut state = FoldState::default();
        assert!(!state.update(0.0, 150, 800, false, false, true));
        assert!(state.update(0.8, 150, 800, false, false, true));
        assert!(!state.update(0.81, 150, 800, false, false, false));
        assert!(!state.update(30.0, 150, 800, false, false, false));
    }

    #[test]
    fn fold_state_cancels_stale_reveal_and_collapse_deadlines() {
        let mut state = FoldState::default();
        assert!(!state.update(0.0, 150, 800, false, false, true));
        assert!(!state.update(0.79, 150, 800, true, false, true));
        assert!(!state.update(0.80, 150, 800, false, false, true));
        assert!(!state.update(1.59, 150, 800, false, false, true));
        assert!(state.update(1.60, 150, 800, false, false, true));
        assert!(state.update(1.61, 150, 800, true, false, true));
        assert!(state.update(1.70, 150, 800, false, false, true));
        assert!(state.update(2.00, 150, 800, true, false, true));
        assert!(!state.update(2.15, 150, 800, true, false, true));
        assert!(!state.update(30.0, 150, 800, false, true, true));
    }
}
