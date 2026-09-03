use std::env;
use std::fs;
use std::path::Path;

use ai_token_meter_windows::updater::signature::verify_tauri_signature;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 3 {
        return Err("usage: verify_update_signature ARCHIVE SIGNATURE TAURI_CONFIG".into());
    }
    let archive = fs::read(Path::new(&arguments[0]))?;
    let signature = fs::read_to_string(Path::new(&arguments[1]))?;
    let config: serde_json::Value = serde_json::from_slice(&fs::read(Path::new(&arguments[2]))?)?;
    let public_key = config
        .pointer("/plugins/updater/pubkey")
        .and_then(serde_json::Value::as_str)
        .ok_or("Tauri updater public key is missing")?;
    verify_tauri_signature(&archive, &signature, public_key)
        .map_err(|error| format!("Windows updater signature verification failed: {error:?}"))?;
    println!("Windows updater signature verified with the embedded public key.");
    Ok(())
}
