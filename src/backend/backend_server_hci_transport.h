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

#include "model/hci/hci_transport.h"  // for HciTransport
#include "packet_streamer.grpc.pb.h"
#include "packet_streamer.pb.h"

namespace netsim {
namespace hci {

using Stream =
    ::grpc::ServerReaderWriter<packet::PacketResponse, packet::PacketRequest>;

/**
 * @class BackendServerHciTransport
 *
 * Connects Rootcanal's HciTransport and BackendServer Grpc
 *
 */
class BackendServerHciTransport : public rootcanal::HciTransport {
 public:
  ~BackendServerHciTransport() {}

  /**
   * @brief Transport packets between HciTransport and Grpc. Returns when the
   * grpc connection closes or the HciTransport closes.
   */
  virtual void Transport() = 0;

  /**
   * @brief Static constructor pattern for BackendServerHciTransport class.
   *
   * Moves HCI packets between the Grpc BackendServer and the rootcanal
   * HciTransport
   *
   * @param - serial number of the owning device
   */
  static std::shared_ptr<BackendServerHciTransport> Create(std::string peer,
                                                           Stream *stream);
};

}  // namespace hci
}  // namespace netsim
