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
#include <memory>
#include <string>

#include "model.pb.h"
#include "model/hci/hci_transport.h"  // for HciTransport

namespace netsim {
namespace hci {

/** Manages the bluetooth chip emulation provided by the root canal library.
 *
 * Owns the TestModel, setup, and manages the packet flow into and out of
 * rootcanal.
 */
// TODO: rename to HciChipEmulator
class ChipEmulator {
 public:
  virtual ~ChipEmulator(){};

  // Deleted copy constructor for singleton class
  ChipEmulator(const ChipEmulator &) = delete;
  // Deleted copy assignment for singleton class
  ChipEmulator &operator=(const ChipEmulator &) = delete;

  // Retrieve the singleton
  static ChipEmulator &Get();

  // Enable/disable the enabled radios for a device
  virtual void SetDeviceRadio(const std::string &, netsim::model::PhyKind,
                              netsim::model::PhyState) = 0;

  // Starts the bluetooth chip emulator.
  virtual void Start() = 0;

  // Closes the bluetooth chip emulator.
  virtual void Close() = 0;

  virtual int8_t GetRssi(int sender_id, int receiver_id, int8_t tx_power) = 0;

  // Turn packet catpure on/off
  virtual void SetPacketCapture(const std::string &, bool) = 0;

  virtual void AddHciConnection(
      const std::string &,
      std::shared_ptr<rootcanal::HciTransport> transport) = 0;

 protected:
  ChipEmulator() {}
};

}  // namespace hci
}  // namespace netsim
