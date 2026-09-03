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

#[cfg(test)]
mod tests {
    use super::{MonitorIdentity, choose_monitor};

    fn monitors() -> Vec<MonitorIdentity> {
        vec![
            MonitorIdentity::new("secondary-display", false),
            MonitorIdentity::new("primary-display", true),
        ]
    }

    #[test]
    fn saved_physical_monitor_wins_even_when_another_monitor_is_primary() {
        let monitors = monitors();

        let selected = choose_monitor(&monitors, Some("secondary-display"));

        assert_eq!(
            selected.map(|monitor| monitor.stable_id.as_str()),
            Some("secondary-display")
        );
    }

    #[test]
    fn missing_saved_monitor_temporarily_falls_back_to_primary() {
        let monitors = monitors();

        let selected = choose_monitor(&monitors, Some("disconnected-display"));

        assert_eq!(
            selected.map(|monitor| monitor.stable_id.as_str()),
            Some("primary-display")
        );
    }

    #[test]
    fn first_monitor_is_used_only_when_no_primary_is_reported() {
        let monitors = vec![
            MonitorIdentity::new("first-display", false),
            MonitorIdentity::new("second-display", false),
        ];

        let selected = choose_monitor(&monitors, Some("disconnected-display"));

        assert_eq!(
            selected.map(|monitor| monitor.stable_id.as_str()),
            Some("first-display")
        );
    }

    #[test]
    fn empty_monitor_list_leaves_the_existing_window_untouched() {
        assert_eq!(choose_monitor(&[], Some("saved-display")), None);
    }
}
