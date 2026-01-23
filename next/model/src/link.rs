// Copyright 2025 The Android Open Source Project

use crate::chip::{ChipId, ChipKind};
use serde::{Deserialize, Serialize};

/// A unique identifier for a link, represented as a u32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkId(pub u32);

impl std::fmt::Display for LinkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for LinkId {
    fn from(id: u32) -> Self {
        LinkId(id)
    }
}

impl From<LinkId> for u32 {
    fn from(id: LinkId) -> Self {
        id.0
    }
}

impl Default for LinkId {
    fn default() -> Self {
        LinkId(0)
    }
}

/// Internal representation of a Link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub id: LinkId,
    pub sender: ChipId,
    pub receiver: ChipId,
    pub kind: ChipKind,
    pub rssi: i8,
}

/// Update for a Link.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkUpdate {
    pub rssi: Option<i8>,
}

impl Default for Link {
    fn default() -> Self {
        Self {
            id: LinkId::default(),
            sender: ChipId(0),
            receiver: ChipId(0),
            kind: ChipKind::UNSPECIFIED,
            rssi: 0,
        }
    }
}
