/*
 * Copyright 2022 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include "packet_hub/packet_hub.h"

#include "backend/backend_packet_hub.h"
#include "hci/hci_packet_hub.h"
#include "wifi/wifi_packet_hub.h"

namespace netsim {
namespace packet_hub {

// forward from transport to facade via packet_hub
void handle_bt_request(uint32_t facade_id,
                       packet::HCIPacket_PacketType packet_type,
                       const std::shared_ptr<std::vector<uint8_t>> &packet) {
  netsim::hci::handle_bt_request(facade_id, packet_type, packet);
}

// forward from facade to transport via packet_hub
void handle_bt_response(uint32_t facade_id,
                        packet::HCIPacket_PacketType packet_type,
                        const std::shared_ptr<std::vector<uint8_t>> &packet) {
  netsim::backend::handle_bt_response(facade_id, packet_type, packet);
}

// forward from transport to facade via packet_hub
void handle_wifi_request(uint32_t facade_id,
                         const std::shared_ptr<std::vector<uint8_t>> &packet) {
  netsim::wifi::handle_wifi_request(facade_id, packet);
}

// forward from facade to transport via packet_hub
void handle_wifi_response(uint32_t facade_id,
                          const std::shared_ptr<std::vector<uint8_t>> &packet) {
  netsim::backend::handle_wifi_response(facade_id, packet);
}

}  // namespace packet_hub
}  // namespace netsim
