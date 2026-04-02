// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// //! # Observability & Tracing
// //!
// //! This module provides the tracing infrastructure for the entire actor
// system. //!
// //! ## Overview
// //!
// //! The [`setup_tracing`] function initializes structured logging with the
// `tracing` crate, //! providing hierarchical spans that show the complete
// request flow through the system. //!
// //! ## Configuration
// //!
// //! The framework uses a compact format that hides the crate/module prefix
// (`with_target(false)`). //! This keeps log lines short while still providing
// rich structured data. //!
// //! - **Structured logging** with `tracing` crate
// //! - **Hierarchical spans** for request tracing
// //! - **Configurable log levels** via `RUST_LOG` environment variable
// //! - **Compact format** optimized for development
// //!
// //! ## What Gets Traced
// //!
// //! - **Actor Lifecycle**: Startup, shutdown, and final state
// //! - **Resource Operations**: Create, Get, Update, Delete, and custom
// Actions //! - **Request Flow**: Hierarchical spans showing the complete
// request path //! - **Errors**: Detailed error context with entity IDs and
// failure reasons //!
// //! ## Usage Examples
// //!
// //! ```bash
// //! # Compact logs (default)
// //! RUST_LOG=info cargo run
// //!
// //! # Show full payloads with debug logs
// //! RUST_LOG=debug cargo run
// //!
// //! # Very verbose tracing
// //! RUST_LOG=trace cargo run
// //!
// //! # Filter to specific modules
// //! RUST_LOG=actor_recipe::framework=debug cargo run
// //! ```
// //!
// //! ## Debug Flag for Full Payload
// //!
// //! When you run with `RUST_LOG=debug`, functions log full payloads **once**
// at the start: //!
// //! ```rust
// //! # use tracing::debug;
// //! # #[derive(Debug)]
// //! # struct Order { id: String }
// //! # let order = Order { id: "123".to_string() };
// //! debug!(?order, "create_order called");
// //! ```
// //!
// //! The `?` syntax is a `tracing` macro feature that records the variable
// using its //! `Debug` representation as a structured field.
// //!
// //! Running with `RUST_LOG=debug` will show:
// //!
// //! ```text
// //! DEBUG create_order called order={...}
// //! INFO order_processing:create_order: Processing create_order request
// (Client Side) //! ```
// //!
// //! All subsequent logs remain concise, showing only the workflow hierarchy.
// //!
// //! ## Workflow Trace Example
// //!
// //! The tracing output shows the complete order creation workflow with
// hierarchical spans. //!
// //! **With `RUST_LOG=info`** (compact):
// //!
// //! ```text
// //! INFO Sending create_order to actor
// //! INFO Created user_id="user_1" size=1
// //! INFO Created product_id="product_1" size=1
// //! INFO Action ok product_id="product_1"
// //! INFO Created order_id="order_1" size=1
// //! ```
// //!
// //! **With `RUST_LOG=debug`** (detailed):
// //!
// //! ```text
// //! DEBUG create_order called order=Order { id: "", user_id: "user_1",
// product_id: "product_1", quantity: 3, total: 75.0 } //! INFO Sending
// create_order to actor //! DEBUG Get user_id="user_1"
// //! INFO Created user_id="user_1" size=1
// //! DEBUG Get product_id="product_1"
// //! INFO Created product_id="product_1" size=1
// //! DEBUG Action product_id="product_1" action=ReserveStock(3)
// //! INFO Action ok product_id="product_1"
// //! DEBUG Create params=OrderCreate { user_id: "user_1", product_id:
// "product_1", quantity: 3, total: 75.0 } //! INFO Created order_id="order_1"
// size=1 //! ```
// //!
// //! **Key Observations**:
// //! 1. **User Validation** → `Get user_id="user_1"` → User found in actor
// //! 2. **Product Validation** → `Get product_id="product_1"` → Product found
// //! 3. **Stock Reservation** → `Action...ReserveStock(3)` → Stock reserved
// (happens in `Order::on_create`) //! 4. **Order Creation** → `Create
// params=OrderCreate{...}` → Order created //!
// //! Each step is traced with structured fields that can be filtered and
// analyzed in //! production logging systems.
// //!
// //! ## Output Formats
// //!
// //! The compact format shows span hierarchy inline:
// //! - `INFO user_creation: Creating test user` - top-level span
// //! - `INFO order_processing:create_order: Processing request` - nested spans
// //!
// //! Use `debug` level to see full object details at function entry points.
// pub fn setup_tracing() {
//     tracing_subscriber::fmt()
//         .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
//         .with_target(false) // Don't show module paths - we use entity_type
// instead         .compact() // Compact format shows spans inline (e.g.,
// "order_processing:create_order")         .init();
// }
