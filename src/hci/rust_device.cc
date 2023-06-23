// Copyright 2023 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#include "hci/rust_device.h"

#include <cstdint>

#include "netsim-cxx/src/lib.rs.h"
#include "packets/link_layer_packets.h"
#include "phy.h"
#include "rust/cxx.h"

namespace netsim::hci::facade {
void RustDevice::Tick() { ::netsim::hci::facade::Tick(*callbacks_); }

void RustDevice::ReceiveLinkLayerPacket(
    ::model::packets::LinkLayerPacketView packet, rootcanal::Phy::Type type,
    int8_t rssi) {
  auto packet_vec = std::vector<uint8_t>(packet.begin(), packet.end());
  auto slice = rust::Slice<const uint8_t>(packet_vec.data(), packet_vec.size());

  ::netsim::hci::facade::ReceiveLinkLayerPacket(
      *callbacks_, packet.GetSourceAddress().ToString(),
      packet.GetDestinationAddress().ToString(),
      static_cast<int8_t>(packet.GetType()), slice);
}

void RustBluetoothChip::SendLinkLayerPacket(
    const rust::Slice<const uint8_t> packet, uint8_t type,
    int8_t tx_power) const {
  std::vector<uint8_t> buffer(packet.begin(), packet.end());
  rust_device->SendLinkLayerPacket(buffer, rootcanal::Phy::Type(type),
                                   tx_power);
}

rust::Vec<uint8_t> GenerateAdvertisingPacket(
    const rust::String &address, const rust::Slice<const uint8_t> packet) {
  std::vector<uint8_t> buffer(packet.begin(), packet.end());
  rootcanal::Address rootcanal_address;
  rootcanal::Address::FromString(std::string(address), rootcanal_address);
  auto builder = ::model::packets::LeLegacyAdvertisingPduBuilder::Create(
      rootcanal_address, rootcanal::Address::kEmpty,
      ::model::packets::AddressType::PUBLIC,
      ::model::packets::AddressType::PUBLIC,
      ::model::packets::LegacyAdvertisingType::ADV_NONCONN_IND, buffer);
  auto cxx_packet = builder->SerializeToBytes();
  auto packet_rust = rust::Vec<uint8_t>();
  std::copy(cxx_packet.begin(), cxx_packet.end(),
            std::back_inserter(packet_rust));
  return packet_rust;
}

rust::Vec<uint8_t> GenerateScanResponsePacket(
    const rust::String &source_address, const rust::String &destination_address,
    const rust::Slice<const uint8_t> packet) {
  std::vector<uint8_t> packet_cxx(packet.begin(), packet.end());
  rootcanal::Address rootcanal_source_address;
  rootcanal::Address rootcanal_destination_address;
  rootcanal::Address::FromString(std::string(source_address),
                                 rootcanal_source_address);
  rootcanal::Address::FromString(std::string(destination_address),
                                 rootcanal_destination_address);

  auto builder = ::model::packets::LeScanResponseBuilder::Create(
      rootcanal_destination_address, rootcanal_source_address,
      ::model::packets::AddressType::PUBLIC, packet_cxx);

  auto cxx_packet = builder->SerializeToBytes();
  auto packet_rust = rust::Vec<uint8_t>();
  std::copy(cxx_packet.begin(), cxx_packet.end(),
            std::back_inserter(packet_rust));
  return packet_rust;
}

}  // namespace netsim::hci::facade