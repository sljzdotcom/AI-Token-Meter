#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorIdentity {
    pub stable_id: String,
    pub is_primary: bool,
}

impl MonitorIdentity {
    pub fn new(stable_id: impl Into<String>, is_primary: bool) -> Self {
        Self {
            stable_id: stable_id.into(),
            is_primary,
        }
    }
}

pub fn choose_monitor<'a>(
    monitors: &'a [MonitorIdentity],
    preferred_id: Option<&str>,
) -> Option<&'a MonitorIdentity> {
    preferred_id
        .and_then(|preferred| {
            monitors
                .iter()
                .find(|monitor| monitor.stable_id == preferred)
        })
        .or_else(|| monitors.iter().find(|monitor| monitor.is_primary))
        .or_else(|| monitors.first())
}
