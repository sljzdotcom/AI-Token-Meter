use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StripPreferences {
    pub schema_version: u64,
    pub density: String,
    pub fold_delay: u64,
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
            fold_delay: Option<u64>,
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
        if let Some(density) = stored.density {
            value.density = density;
        }
        if let Some(delay) = stored.fold_delay {
            value.fold_delay = delay;
        }
        if let Some(order) = stored.ordered_providers {
            value.ordered_providers = order;
        }
        if let Some(hidden) = stored.hidden_providers {
            value.hidden_providers = hidden;
        }
        value.hidden_until = stored.hidden_until;
        if stored.schema_version.unwrap_or(0) < 2 && !already_has_gemini {
            value.hidden_providers.push("gemini".into());
        }
        value.normalize();
        Ok(value)
    }
}

impl Default for StripPreferences {
    fn default() -> Self {
        Self {
            schema_version: 2,
            density: "compact".into(),
            fold_delay: 0,
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
        if self.density != "comfortable" {
            self.density = "compact".into();
        }
        if ![0, 5, 15].contains(&self.fold_delay) {
            self.fold_delay = 0;
        }
        let known = ["claude", "codex", "deepseek", "gemini"];
        self.schema_version = 2;
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
            return (12.0, 96.0);
        }
        let count = self.visible_providers().len().clamp(1, 4);
        if self.density == "comfortable" {
            (108.0, 212.0 + (count - 1) as f64 * 72.0)
        } else {
            (65.0, 170.0 + (count - 1) as f64 * 58.0)
        }
    }
    pub fn hidden(&self, now: i64) -> bool {
        self.hidden_until.is_some_and(|until| until > now)
    }
}

#[derive(Default)]
pub struct FoldState {
    deadline: Option<f64>,
    previous_delay: u64,
    pub folded: bool,
}
impl FoldState {
    pub fn update(&mut self, now: f64, delay: u64, locked: bool) -> bool {
        if locked || delay == 0 || delay != self.previous_delay {
            self.folded = false;
            self.deadline = None;
        }
        self.previous_delay = delay;
        if !locked && delay > 0 {
            let deadline = self.deadline.get_or_insert(now + delay as f64);
            self.folded = now >= *deadline;
        }
        self.folded
    }
}
