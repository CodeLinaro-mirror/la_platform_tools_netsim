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

/// Transport of virtual controller messages (hci, uci, netlink) from
/// desktop emulator (emu) to simulator.

#pragma once
#include <string>

namespace netsim {

/// RpcHalTransport is the transport for devices that use the grpc connection
/// mechanism.
///
class RpcHalTransport {
 public:
  // Pure virtual destructor for derived classes
  virtual ~RpcHalTransport() = 0;

  // Creates and returns a RpcHalTransport.
  static std::unique_ptr<RpcHalTransport> Create();

  /// Creates a new client connected to an emulator packet forwarding service.
  /// Attaches virtual controllers and starts processing packets.
  ///
  /// \param port The localhost port of the endpoint to connect to.
  /// \param serial Serial string to uniquely identify the emulator device.
  //
  virtual bool connect(int port, std::string serial) = 0;

  // Discover new Grpc endpoints
  //
  // Returns true if new endpoints were discovered.
  virtual bool discover() = 0;
};

}  // namespace netsim
