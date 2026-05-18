use anyhow::{Context, Result};
use keyring_core::Entry;

#[cfg(target_os = "macos")]
use apple_native_keyring_store::Store as PlatformStore;
#[cfg(target_os = "linux")]
use dbus_secret_service_keyring_store::Store as PlatformStore;
#[cfg(target_os = "windows")]
use windows_native_keyring_store::Store as PlatformStore;

use crate::APP_NAME;

#[derive(Debug, Clone)]
pub enum SecretService {
    TypingCom,
}

impl SecretService {
    fn as_str(&self) -> &'static str {
        match self {
            Self::TypingCom => "typing.com",
        }
    }
}

pub fn init_keyring() {
    let store = PlatformStore::new().expect("failed to initialize secret store");
    keyring_core::set_default_store(store);
}

fn entry(service: &SecretService, username: &str) -> Result<Entry> {
    Entry::new(&format!("{APP_NAME}:{}", service.as_str()), username)
        .context("failed to create keyring entry")
}

pub fn save_password(service: SecretService, username: &str, password: &str) -> Result<()> {
    entry(&service, username)?
        .set_password(password)
        .with_context(|| format!("failed to save {} password", service.as_str()))
}

pub fn load_password(service: SecretService, username: &str) -> Result<String> {
    entry(&service, username)?
        .get_password()
        .with_context(|| format!("{} password not found in keyring", service.as_str()))
}

#[cfg(test)]
mod tests {
    use keyring_core::{Entry, mock};

    #[test]
    fn test_credential_roundtrip() {
        keyring_core::set_default_store(mock::Store::new().unwrap());
        let entry = Entry::new("test-service", "test-user").unwrap();
        entry.set_password("secret").unwrap();
        assert_eq!(entry.get_password().unwrap(), "secret");
        keyring_core::unset_default_store();
    }
}
