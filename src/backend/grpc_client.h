/*
 * Copyright 2022 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

// C++ Client implementation for PacketStreamer gRPC.
//
// This is used by netsimd for forwarding packet to another netsimd.

#pragma once

#include "rust/cxx.h"

namespace netsim {
namespace backend {
namespace client {

uint32_t StreamPackets(const rust::String &server);

using ReadCallback = rust::Fn<void(
    uint32_t, const rust::Slice<::std::uint8_t const> proto_bytes)>;

bool ReadPacketResponseLoop(uint32_t stream_id, ReadCallback read_fn);

bool WritePacketRequest(uint32_t stream_id,
                        const rust::Slice<::std::uint8_t const> proto_bytes);

}  // namespace client
}  // namespace backend
}  // namespace netsim
