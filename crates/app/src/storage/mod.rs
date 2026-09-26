//! Platform [`Storage`] backends (ticket 0207, ADR-0009): files in the OS
//! data directory on native, `localStorage` on web.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use native::FileStorage;
#[cfg(target_arch = "wasm32")]
pub use web::WebStorage;

use macroquad::prelude::{error, info};
use trpg_ui::{Storage, StorageError};

/// The key the debug-only [`smoke_check`] writes and removes.
const SMOKE_TEST_KEY: &str = "smoke_test";

/// The storage backend for this platform. Native falls back to an in-memory
/// store (nothing persists) if the OS has no usable data directory (the
/// relevant environment variable is unset, which only happens on
/// unsupported or misconfigured systems); it logs an error when that
/// happens.
pub fn platform() -> Box<dyn Storage> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match FileStorage::new() {
            Ok(storage) => Box::new(storage),
            Err(e) => {
                error!("storage: {e}; saves and settings will not persist");
                Box::new(trpg_ui::MemoryStorage::new())
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(WebStorage::new())
    }
}

/// Debug-only startup check: writes and reads back a throwaway key and logs
/// the result, then removes it.
#[cfg(debug_assertions)]
pub fn smoke_check(storage: &mut dyn Storage) {
    let result = run_smoke_check(storage);
    match result {
        Ok(()) => info!("storage: smoke check passed"),
        Err(e) => error!("storage: smoke check failed: {e}"),
    }
}

#[cfg(debug_assertions)]
fn run_smoke_check(storage: &mut dyn Storage) -> Result<(), StorageError> {
    storage.write(SMOKE_TEST_KEY, "ok")?;
    let read_back = storage.read(SMOKE_TEST_KEY)?;
    storage.delete(SMOKE_TEST_KEY)?;
    if read_back.as_deref() == Some("ok") {
        Ok(())
    } else {
        Err(StorageError::Backend(format!(
            "wrote \"ok\", read back {read_back:?}"
        )))
    }
}
