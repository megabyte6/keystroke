use keyring_core::Entry;

#[cfg(target_os = "macos")]
use apple_native_keyring_store::Store as PlatformStore;
#[cfg(target_os = "linux")]
use dbus_secret_service_keyring_store::Store as PlatformStore;
#[cfg(target_os = "windows")]
use windows_native_keyring_store::Store as PlatformStore;

use crate::APP_NAME;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create keyring entry")]
    EntryCreate(#[source] keyring_core::Error),
    #[error("failed to save {service} password, for '{username}'")]
    SavePassword {
        service: String,
        username: String,
        #[source]
        source: keyring_core::Error,
    },
    #[error("{service} password for '{username}' not found in keyring")]
    LoadPassword {
        service: String,
        username: String,
        #[source]
        source: keyring_core::Error,
    },
}

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
    Entry::new(&format!("{APP_NAME}:{}", service.as_str()), username).map_err(Error::EntryCreate)
}

pub fn save_password(service: SecretService, username: &str, password: &str) -> Result<()> {
    entry(&service, username)?
        .set_password(password)
        .map_err(|source| Error::SavePassword {
            service: service.as_str().to_string(),
            username: username.to_string(),
            source,
        })
}

pub fn load_password(service: SecretService, username: &str) -> Result<String> {
    entry(&service, username)?
        .get_password()
        .map_err(|source| Error::LoadPassword {
            service: service.as_str().to_string(),
            username: username.to_string(),
            source,
        })
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
