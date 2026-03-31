// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Print options for protobuf JSON mapping

use protobuf_json_mapping::PrintOptions;

/// A commonly used protobuf JSON print options for devices_handler,
/// captures_handler, and links_handler
pub const JSON_PRINT_OPTION: PrintOptions = PrintOptions {
    enum_values_int: false,
    proto_field_name: false,
    always_output_default_values: true,
    _future_options: (),
};
