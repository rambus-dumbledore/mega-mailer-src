use std::sync::atomic::{Ordering, AtomicBool};
use std::sync::Arc;
use crate::types::*;

pub fn set_ctrlc_handler(r: Arc<AtomicBool>) -> Result<()> {
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .map_err(|e| {
        Error::InternalError(InternalError::RuntimeError(format!("Error setting signal handler: {}", e)))
    })?;
    Ok(())
}
