use std::sync::Arc;

use ai_token_meter_windows::accounts::cli_account::CliProvider;
use ai_token_meter_windows::accounts::cli_discovery::CliDiscovery;
use ai_token_meter_windows::accounts::runtime_discovery::discover_runtime_cli;
use ai_token_meter_windows::collectors::{CollectionError, candidate_for_collection};
use ai_token_meter_windows::persistence::ProviderCliSettings;
use ai_token_meter_windows::platform::windows::process::CancellationToken;

#[test]
fn pre_cancelled_runtime_discovery_stays_cancelled() {
    let cancellation = Arc::new(CancellationToken::new());
    cancellation.cancel();

    let result = discover_runtime_cli(
        CliProvider::Codex,
        &ProviderCliSettings::default(),
        cancellation,
    );

    assert!(matches!(result, CliDiscovery::Cancelled));
}

#[test]
fn collection_maps_only_confirmed_missing_to_the_not_installed_branch() {
    assert!(matches!(
        candidate_for_collection(CliDiscovery::Missing),
        Ok(None)
    ));
    assert!(matches!(
        candidate_for_collection(CliDiscovery::Unavailable),
        Err(CollectionError::Transport)
    ));
    assert!(matches!(
        candidate_for_collection(CliDiscovery::Cancelled),
        Err(CollectionError::Cancelled)
    ));
}
