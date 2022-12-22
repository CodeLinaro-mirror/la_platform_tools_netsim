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
#include <string_view>

#include "controller/chip.h"
#include "controller/device.h"
#include "model.pb.h"

namespace netsim {
namespace controller {

class Device;

class Chip {
 public:
  explicit Chip() {}

  virtual ~Chip(){};

  /**
   * Update processing for the chip. Validate and move state from the request
   * into the parent's model::Chip changing the ChipFacade as needed.
   */
  virtual void Update(const model::Chip &request);

  /**
   * Reset the state of the chip to defaults.
   */
  virtual void Reset();

  /**
   * Remove resources own by the chip and remove it from the chip emulator.
   */
  virtual void Remove();

  /**
   * Initialize the chip with a parent and index into the parents model.
   */
  void Init(std::shared_ptr<Device> parent, int index);

  /**
   * Compare keys with another chip model, return true if they are the same.
   */
  bool KeyComp(const model::Chip &);

  std::shared_ptr<Device> parent;

 protected:
  /**
   * Fetch a reference to the parent's model::Chip.
   */
  model::Chip &Model();

  /**
   * Fetch a reference to the parent's model::Device.
   */
  model::Device &DeviceModel();

  int chip_index;
};

}  // namespace controller
}  // namespace netsim
