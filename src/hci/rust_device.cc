// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#include "hci/rust_device.h"

#include <cstdint>

#include "netsim-daemon/src/ffi.rs.h"
#include "packets/link_layer_packets.h"
#include "phy.h"
#include "rust/cxx.h"

namespace netsim::hci::facade {
void RustDevice::Tick() { ::netsim::hci::facade::Tick(*callbacks_); }

void RustDevice::ReceiveLinkLayerPacket(
    ::model::packets::LinkLayerPacketView packet, rootcanal::Phy::Type type,
    int8_t rssi) {
  auto packet_vec = packet.bytes().bytes();
  auto slice = rust::Slice<const uint8_t>(packet_vec.data(), packet_vec.size());

  ::netsim::hci::facade::ReceiveLinkLayerPacket(
      *callbacks_, packet.GetSourceAddress().ToString(),
      packet.GetDestinationAddress().ToString(),
      static_cast<int8_t>(packet.GetType()), slice);
}

void RustBluetoothChip::SendLinkLayerLePacket(
    const rust::Slice<const uint8_t> packet, int8_t tx_power) const {
  std::vector<uint8_t> buffer(packet.begin(), packet.end());
  rust_device->SendLinkLayerPacket(buffer, rootcanal::Phy::Type::LOW_ENERGY,
                                   tx_power);
}
}  // namespace netsim::hci::facade
