use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Uwb {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UwbUpdate {
    #[serde(flatten)]
    pub radio: crate::chip::RadioUpdate,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct UwbCreate {
    // Future UWB specific properties.
}
