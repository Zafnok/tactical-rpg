//! [`WebStorage`]: the browser's `localStorage` via `quad_storage`,
//! namespaced under [`PREFIX`] so the game doesn't collide with other pages
//! sharing the origin.

use std::sync::PoisonError;

use trpg_ui::storage::is_valid_key;
use trpg_ui::{Storage, StorageError};

/// Every key is stored under this prefix.
const PREFIX: &str = "tactical-rpg/";

/// A [`Storage`] backed by `quad_storage::STORAGE` (`localStorage`). Cheap
/// to construct: the real state lives in `quad_storage`'s global.
#[derive(Debug, Default)]
pub struct WebStorage;

impl WebStorage {
    /// A handle to the browser's `localStorage`.
    pub fn new() -> Self {
        Self
    }

    fn prefixed(key: &str) -> String {
        format!("{PREFIX}{key}")
    }
}

impl Storage for WebStorage {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        check_key(key)?;
        let storage = quad_storage::STORAGE.lock().map_err(poisoned)?;
        Ok(storage.get(&Self::prefixed(key)))
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        check_key(key)?;
        let mut storage = quad_storage::STORAGE.lock().map_err(poisoned)?;
        storage.set(&Self::prefixed(key), value);
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        check_key(key)?;
        let mut storage = quad_storage::STORAGE.lock().map_err(poisoned)?;
        storage.remove(&Self::prefixed(key));
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        let storage = quad_storage::STORAGE.lock().map_err(poisoned)?;
        let keys = (0..storage.len())
            .filter_map(|i| storage.key(i))
            .filter_map(|key| key.strip_prefix(PREFIX).map(str::to_owned))
            .collect();
        Ok(keys)
    }
}

fn check_key(key: &str) -> Result<(), StorageError> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(StorageError::InvalidKey(key.to_owned()))
    }
}

fn poisoned<T>(_: PoisonError<T>) -> StorageError {
    StorageError::Backend("localStorage lock poisoned".to_owned())
}
