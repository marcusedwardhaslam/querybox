use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use super::profile::ConnectionProfile;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Keyring error: {0}")]
    Keyring(String),
}

fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("querybox")
}

fn profiles_path() -> PathBuf {
    config_dir().join("connections.json")
}

pub fn load_profiles() -> Result<Vec<ConnectionProfile>, StorageError> {
    let path = profiles_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = fs::read_to_string(&path)?;
    let profiles: Vec<ConnectionProfile> = serde_json::from_str(&data)?;
    Ok(profiles)
}

pub fn save_profiles(profiles: &[ConnectionProfile]) -> Result<(), StorageError> {
    let path = profiles_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(profiles)?;
    fs::write(&path, data)?;
    Ok(())
}

pub fn store_password(profile: &ConnectionProfile, password: &str) -> Result<(), StorageError> {
    let entry = keyring::Entry::new("querybox", &profile.keyring_key())
        .map_err(|e| StorageError::Keyring(e.to_string()))?;
    entry
        .set_password(password)
        .map_err(|e| StorageError::Keyring(e.to_string()))?;
    Ok(())
}

pub fn get_password(profile: &ConnectionProfile) -> Result<Option<String>, StorageError> {
    let entry = keyring::Entry::new("querybox", &profile.keyring_key())
        .map_err(|e| StorageError::Keyring(e.to_string()))?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(StorageError::Keyring(e.to_string())),
    }
}
