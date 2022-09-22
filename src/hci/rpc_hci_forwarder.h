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

#pragma once
#include <grpcpp/grpcpp.h>

#include "emulated_bluetooth_vhci.pb.h"  // for HCIPacket
#include "hci/hci_chip_emulator.h"
#include "hci/rpc_transport.h"
#include "model/hci/hci_transport.h"

namespace netsim {
namespace hci {

using HCIPacket = android::emulation::bluetooth::HCIPacket;

// A grpc client-side bi-directional stream of HCIPacket
using HCIStream = grpc::ClientReaderWriter<HCIPacket, HCIPacket>;

/**
 * @class RpcHciForwarder
 *
 * Connects HCIPacket between ChipEmulator (Rootcanal) and Grpc
 *
 */
class RpcHciForwarder : public RpcTransport<HCIPacket, HCIPacket>,
                        public rootcanal::HciTransport {
 public:
  ~RpcHciForwarder() {}

  /**
   * @brief Static constructor pattern for RpcHciForwarder class.
   *
   * RpcHciForwarder moves HCI packets between the Grpc HCITransport and the
   * rootcanal HciTransport
   *
   * @param - serial number of the owning device
   */
  static std::shared_ptr<RpcHciForwarder> Create(std::string);
};

}  // namespace hci
}  // namespace netsim
