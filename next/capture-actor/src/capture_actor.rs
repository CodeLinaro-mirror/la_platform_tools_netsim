use std::{collections::HashMap, path::PathBuf};

use netsim_model::ChipId;

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
    /// Default capture state for new captures.
    pub(crate) default_capture_enabled: bool,
    /// Default capture directory.
    pub(crate) capture_dir: Option<PathBuf>,
    /// Map of active Capture Entities.
    pub(crate) entities: HashMap<ChipId, crate::service::InternalCaptureInfo>,
}

impl CaptureActor {
    pub fn new(default_capture_enabled: bool) -> Self {
        Self {
            writers: HashMap::new(),
            default_capture_enabled,
            capture_dir: None,
            entities: HashMap::new(),
        }
    }
}
