use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

use crate::collectors::refresh::RefreshCoordinator;

use super::UpdateState;

const UPDATE_EVENT: &str = "update-state-changed";

pub async fn check(app: AppHandle, state: Arc<Mutex<UpdateState>>) -> Result<UpdateState, String> {
    update(&app, &state, |current| current.begin_check())?;
    let checked = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    let result = match checked {
        Ok(available) => update(&app, &state, |current| {
            current.finish_check(available.map(|update| update.version))
        }),
        Err(_) => {
            fail(&app, &state, "Update service is temporarily unavailable");
            return Err("Update service is temporarily unavailable".to_owned());
        }
    };
    result?;
    Ok(snapshot(&state))
}

pub async fn install(
    app: AppHandle,
    state: Arc<Mutex<UpdateState>>,
    refresh_coordinator: Arc<RefreshCoordinator>,
) -> Result<(), String> {
    let expected = {
        let mut current = state
            .lock()
            .map_err(|_| "Update status is temporarily unavailable".to_owned())?;
        current
            .begin_install()
            .map_err(|_| "Check for an update before installing".to_owned())?;
        let expected = current
            .available_version
            .clone()
            .ok_or_else(|| "Check for an update before installing".to_owned())?;
        let published = current.clone();
        drop(current);
        let _ = app.emit(UPDATE_EVENT, published);
        expected
    };

    let available = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    let Some(update) = available.map_err(|_| {
        fail(&app, &state, "The update could not be verified");
        "The update could not be verified".to_owned()
    })?
    else {
        fail(&app, &state, "The selected update is no longer available");
        return Err("The selected update is no longer available".to_owned());
    };
    if update.version != expected {
        fail(&app, &state, "The available update changed; check again");
        return Err("The available update changed; check again".to_owned());
    }

    if !refresh_coordinator.suspend_and_wait(Duration::from_secs(5)) {
        refresh_coordinator.resume();
        fail(
            &app,
            &state,
            "Active usage checks did not stop; try the update again",
        );
        return Err("Active usage checks did not stop; try the update again".to_owned());
    }

    let downloaded = Arc::new(Mutex::new(0_u64));
    let progress_state = Arc::clone(&state);
    let progress_app = app.clone();
    let downloaded_for_callback = Arc::clone(&downloaded);
    let finish_state = Arc::clone(&state);
    let finish_app = app.clone();
    let result = update
        .download_and_install(
            move |chunk, total| {
                let mut downloaded = downloaded_for_callback
                    .lock()
                    .unwrap_or_else(|lock| lock.into_inner());
                *downloaded = downloaded.saturating_add(chunk as u64);
                if let Ok(mut current) = progress_state.lock() {
                    current.report_progress(*downloaded, total);
                    let _ = progress_app.emit(UPDATE_EVENT, current.clone());
                }
            },
            move || {
                if let Ok(mut current) = finish_state.lock() {
                    current.mark_installing();
                    let _ = finish_app.emit(UPDATE_EVENT, current.clone());
                }
            },
        )
        .await;
    if result.is_err() {
        refresh_coordinator.resume();
        fail(&app, &state, "The signed update could not be installed");
        return Err("The signed update could not be installed".to_owned());
    }
    Ok(())
}

pub fn snapshot(state: &Arc<Mutex<UpdateState>>) -> UpdateState {
    state
        .lock()
        .map(|state| state.clone())
        .unwrap_or_else(|_| UpdateState::new(env!("CARGO_PKG_VERSION")))
}

fn update(
    app: &AppHandle,
    state: &Arc<Mutex<UpdateState>>,
    body: impl FnOnce(&mut UpdateState) -> Result<(), super::UpdateStateError>,
) -> Result<(), String> {
    let mut current = state
        .lock()
        .map_err(|_| "Update status is temporarily unavailable".to_owned())?;
    body(&mut current).map_err(|_| "Another update operation is already running".to_owned())?;
    let published = current.clone();
    drop(current);
    let _ = app.emit(UPDATE_EVENT, published);
    Ok(())
}

fn fail(app: &AppHandle, state: &Arc<Mutex<UpdateState>>, message: &str) {
    if let Ok(mut current) = state.lock() {
        current.fail(message);
        let _ = app.emit(UPDATE_EVENT, current.clone());
    }
}
