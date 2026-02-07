use serde::{Deserialize, Serialize};

/// Parameters for creating a Cellular chip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellCreate {
    // Future Cellular specific properties.
}

/// Cellular technology specific chip information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// A string representing the current state of the cellular modem.
    pub state: String,
}
