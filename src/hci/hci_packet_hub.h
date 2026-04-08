/*
 * Copyright 2022 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

// Use gRPC HCI PacketType definitions so we don't expose Rootcanal's version
// outside of the Bluetooth Facade.
#include <cstdint>
#include <memory>
#include <vector>

#include "netsim/hci_packet.pb.h"
#include "rust/cxx.h"

namespace netsim {
namespace hci {

/* Handle packet requests for the Bluetooth Facade which may come over
   different transports including gRPC. */

void handle_bt_request(uint32_t rootcanal_id,
                       packet::HCIPacket_PacketType packet_type,
                       const std::shared_ptr<std::vector<uint8_t>> &packet);

void HandleBtRequestCxx(uint32_t rootcanal_id, uint8_t packet_type,
                        const rust::Vec<uint8_t> &packet);

}  // namespace hci
}  // namespace netsim
