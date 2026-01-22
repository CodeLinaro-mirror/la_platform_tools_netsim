// Copyright 2025 The Android Open Source Project

//! # Link API
//!
//! This crate defines the public API for the Link Actor in Netsim.
//! It provides types and messages for managing links between simulated chips.
//!
//! ## Design
//!
//! The Link API is designed to be used by other actors (e.g., Device Actor, Frontend)
//! to create, modify, and query links. It uses the `actor-framework`'s resource
//! pattern, where `Link` is the resource and `LinkAction` defines global operations.
//!
//! Note that `LinkAction` only contains domain-specific actions (e.g., notifying chip addition).
//! Standard CRUD operations (Create, Read, Update, Delete) are provided by the `actor-framework`
//! and are available to all actors.
//!
//! ## Usage
//!
//! ```rust
//! use link_api::{LinkCreate, LinkAction};
//! // Create a link
//! let create_params = LinkCreate { ... };
//! // Send an action
//! let action = LinkAction::NotifyChipAdded(chip_id, network_kind);
//! ```

pub mod action;
pub mod create;
pub use action::LinkAction;
pub use create::LinkCreate;
pub use netsim_model::chip::{ChipId, ChipKind};
pub use netsim_model::link::{Link, LinkId, LinkUpdate};

#[cfg_attr(feature = "testing", mockall::automock)]
#[async_trait::async_trait]
pub trait LinkClient: Send + Sync {
    async fn list(&self) -> Result<Vec<Link>, String>;
    async fn create(&self, params: LinkCreate) -> Result<LinkId, String>;
    async fn update(&self, id: LinkId, patch: LinkUpdate) -> Result<(), String>;
    async fn delete(&self, id: LinkId) -> Result<(), String>;
    async fn action(&self, id: Option<LinkId>, action: LinkAction) -> Result<(), String>;
    async fn notify_chip_added(&self, chip_id: ChipId, kind: ChipKind) -> Result<(), String>;
    async fn notify_chip_removed(&self, chip_id: ChipId) -> Result<(), String>;
}
