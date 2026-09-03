use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorIdentity {
    pub stable_id: String,
    pub legacy_id: Option<String>,
    pub is_primary: bool,
}

impl MonitorIdentity {
    pub fn new(stable_id: impl Into<String>, is_primary: bool) -> Self {
        Self {
            stable_id: stable_id.into(),
            legacy_id: None,
            is_primary,
        }
    }

    pub fn with_legacy_id(
        stable_id: impl Into<String>,
        legacy_id: impl Into<String>,
        is_primary: bool,
    ) -> Self {
        Self {
            stable_id: stable_id.into(),
            legacy_id: Some(legacy_id.into()),
            is_primary,
        }
    }

    fn matches(&self, identifier: &str) -> bool {
        self.stable_id == identifier || self.legacy_id.as_deref() == Some(identifier)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorResolution<'a> {
    pub selected: &'a MonitorIdentity,
    pub migrated_identifier: Option<String>,
}

pub fn resolve_monitor<'a>(
    monitors: &'a [MonitorIdentity],
    preferred_id: Option<&str>,
) -> Option<MonitorResolution<'a>> {
    if let Some(preferred_id) = preferred_id
        && let Some(selected) = monitors
            .iter()
            .find(|monitor| monitor.matches(preferred_id))
    {
        let migrated_identifier =
            (selected.stable_id != preferred_id).then(|| selected.stable_id.clone());
        return Some(MonitorResolution {
            selected,
            migrated_identifier,
        });
    }
    let selected = monitors
        .iter()
        .find(|monitor| monitor.is_primary)
        .or_else(|| monitors.first())?;
    Some(MonitorResolution {
        selected,
        migrated_identifier: None,
    })
}

pub fn choose_monitor<'a>(
    monitors: &'a [MonitorIdentity],
    preferred_id: Option<&str>,
) -> Option<&'a MonitorIdentity> {
    resolve_monitor(monitors, preferred_id).map(|resolution| resolution.selected)
}

pub fn stable_physical_identifier(device_path: &str) -> Option<String> {
    let normalized = device_path.trim_matches('\0').trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let digest = Sha256::digest(normalized.as_bytes());
    Some(format!("device:{digest:x}"))
}

pub fn stable_runtime_identifier(runtime_name: &str) -> Option<String> {
    let normalized = runtime_name.trim_matches('\0').trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let digest = Sha256::digest(normalized.as_bytes());
    Some(format!("runtime:{digest:x}"))
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonitorTopology {
    pub stable_id: String,
    pub is_primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl MonitorTopology {
    pub fn new(
        stable_id: impl Into<String>,
        is_primary: bool,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            stable_id: stable_id.into(),
            is_primary,
            x,
            y,
            width,
            height,
        }
    }
}

pub struct MonitorTopologyTracker {
    previous: Vec<MonitorTopology>,
}

impl MonitorTopologyTracker {
    pub fn new(mut initial: Vec<MonitorTopology>) -> Self {
        initial.sort();
        Self { previous: initial }
    }

    pub fn update(&mut self, mut current: Vec<MonitorTopology>) -> bool {
        current.sort();
        if self.previous == current {
            return false;
        }
        self.previous = current;
        true
    }
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

    #[test]
    fn legacy_runtime_name_selects_and_migrates_to_the_physical_identifier() {
        let monitors = vec![MonitorIdentity::with_legacy_id(
            "device:stable-hash",
            "\\\\.\\DISPLAY2",
            true,
        )];

        let resolution = super::resolve_monitor(&monitors, Some("\\\\.\\DISPLAY2"))
            .expect("legacy monitor should resolve");

        assert_eq!(resolution.selected.stable_id, "device:stable-hash");
        assert_eq!(
            resolution.migrated_identifier.as_deref(),
            Some("device:stable-hash")
        );
    }

    #[test]
    fn physical_identifier_is_normalized_hashed_and_never_exposes_the_device_path() {
        let raw = r"\\?\DISPLAY#DEL40A9#5&1234&0&UID4357#{identifier}";

        let first = super::stable_physical_identifier(raw).expect("nonempty device path");
        let second = super::stable_physical_identifier(&raw.to_uppercase())
            .expect("same normalized device path");

        assert_eq!(first, second);
        assert!(first.starts_with("device:"));
        assert!(!first.contains("DEL40A9"));
        assert!(!first.contains("UID4357"));
    }

    #[test]
    fn disconnect_and_reconnect_each_trigger_runtime_repositioning() {
        let dual = vec![
            super::MonitorTopology::new("device:primary", true, 0, 0, 1920, 1080),
            super::MonitorTopology::new("device:target", false, 1920, 0, 2560, 1440),
        ];
        let single = vec![super::MonitorTopology::new(
            "device:primary",
            true,
            0,
            0,
            1920,
            1080,
        )];

        let mut tracker = super::MonitorTopologyTracker::new(dual.clone());

        assert!(tracker.update(single));
        assert!(tracker.update(dual.clone()));
        assert!(!tracker.update(dual));
    }
}
