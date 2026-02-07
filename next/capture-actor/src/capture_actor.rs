use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

use netsim_model::chip::ChipId;

use crate::writer::CaptureWriter;

/// The context for the Capture actor.
///
/// This struct holds the shared state required by the actor, including the
/// `CaptureWriter` instance for writing packet captures.
/// Since multiple components need access to the writers and flags,
/// they are stored in the context and protected by mutexes.
pub struct CaptureActor {
    /// Map of ChipId to CaptureWriter.
    /// Writers are created when capture is enabled for a chip.
    pub(crate) writers: HashMap<ChipId, Box<dyn CaptureWriter>>,
    /// Map of ChipId to enabled flag.
    /// Flags are created when the entity is created.
    pub(crate) flags: HashMap<ChipId, Arc<AtomicBool>>,
    /// Default capture state for new captures.
    pub(crate) default_capture_enabled: bool,
    /// Default capture directory.
    pub(crate) capture_dir: Option<PathBuf>,
    /// Map of active Capture Entities.
    pub(crate) entities: HashMap<ChipId, crate::service::InternalCaptureInfo>,
    /// Next available ChipId.
    pub(crate) next_id: u32,
}

impl Default for CaptureActor {
    fn default() -> Self {
        Self {
            writers: HashMap::new(),
            flags: HashMap::new(),
            default_capture_enabled: false,
            capture_dir: None,
            entities: HashMap::new(),
            next_id: 1,
        }
    }
}
