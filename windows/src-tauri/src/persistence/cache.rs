use std::path::{Path, PathBuf};

use crate::domain::{ProviderId, UsageSnapshot};

use super::atomic_json::{AtomicJsonStore, PersistenceResult};

pub struct SnapshotCache {
    directory: PathBuf,
}

impl SnapshotCache {
    pub fn new(directory: impl AsRef<Path>) -> Self {
        Self {
            directory: directory.as_ref().to_owned(),
        }
    }

    pub fn save(&self, snapshot: &UsageSnapshot) -> PersistenceResult<()> {
        AtomicJsonStore::write(&self.path(snapshot.provider_id), snapshot)
    }

    pub fn load(&self, provider_id: ProviderId) -> PersistenceResult<Option<UsageSnapshot>> {
        AtomicJsonStore::read(&self.path(provider_id))
    }

    fn path(&self, provider_id: ProviderId) -> PathBuf {
        self.directory
            .join(format!("{}.json", provider_id.as_str()))
    }
}
