use crate::writer::CaptureWriter;
use actor_framework::ActorContext;
use async_trait::async_trait;
use netsim_model::chip::ChipId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Context for the CaptureActor, holding shared resources.
///
/// Since multiple components need access to the writers and flags,
/// they are stored in the context and protected by mutexes.
pub struct CaptureContext {
    /// Map of ChipId to CaptureWriter.
    /// Writers are created when capture is enabled for a chip.
    pub writers: HashMap<ChipId, Box<dyn CaptureWriter>>,
    /// Map of ChipId to enabled flag.
    /// Flags are created when the entity is created.
    pub flags: HashMap<ChipId, Arc<AtomicBool>>,
    /// Default capture state for new captures.
    pub default_capture_enabled: bool,
    /// Default capture directory.
    pub capture_dir: Option<PathBuf>,
}

impl Default for CaptureContext {
    fn default() -> Self {
        Self {
            writers: HashMap::new(),
            flags: HashMap::new(),
            default_capture_enabled: false,
            capture_dir: None,
        }
    }
}

#[async_trait]
impl ActorContext for CaptureContext {
    type Error = crate::error::CaptureError;
}
