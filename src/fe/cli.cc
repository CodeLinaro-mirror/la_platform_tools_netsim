// Copyright 2022 The Android Open Source Project
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

// Frontend command line interface.
#include "fe/cli.h"

#include <stdlib.h>

#include <chrono>
#include <iomanip>
#include <iostream>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>

#include "frontend.grpc.pb.h"
#include "grpcpp/create_channel.h"
#include "grpcpp/security/credentials.h"
#include "grpcpp/support/status_code_enum.h"
#include "util/ini_file.h"
#include "util/os_utils.h"
#include "util/string_utils.h"

namespace netsim {
namespace {
const std::chrono::duration kConnectionDeadline = std::chrono::seconds(1);

std::optional<std::string> GetServerAddress() {
  auto filepath = osutils::GetDiscoveryDirectory().append("netsim.ini");
  IniFile iniFile(filepath);
  iniFile.Read();
  return iniFile.Get("grpc.port");
}

void Usage(const char *msg) { std::cerr << "Usage: " << msg << std::endl; }

constexpr char kUsage[] = "Usage: [version|devices|radio|move|capture|reset]";
using Args = std::vector<std::string_view>;

// A synchronous client for the netsim frontend service.
class FrontendClient {
 public:
  FrontendClient(std::unique_ptr<frontend::FrontendService::Stub> stub)
      : stub_(std::move(stub)) {}

  // Gets the version of the network simulator service.
  // version
  void GetVersion(const Args &args) {
    if (args.size() != 1) return Usage("version");

    frontend::VersionResponse response;
    auto status = stub_->GetVersion(&context_, {}, &response);
    if (CheckStatus(status, "GetVersion"))
      std::cout << "received: " << response.version() << std::endl;
  }

  // move <device_serial> <x> <y> <z>
  void SetPosition(const Args &args) {
    if (args.size() != 5) return Usage("move <device_serial> <x> <y> <z>");
    float x = std::atof(stringutils::AsString(args.at(2)).c_str()),
          y = std::atof(stringutils::AsString(args.at(3)).c_str()),
          z = std::atof(stringutils::AsString(args.at(4)).c_str());
    auto device_serial = std::string(args.at(1));
    frontend::UpdateDeviceRequest request;
    request.mutable_device()->set_device_serial(device_serial);
    request.mutable_device()->mutable_position()->set_x(x);
    request.mutable_device()->mutable_position()->set_y(y);
    request.mutable_device()->mutable_position()->set_z(z);
    google::protobuf::Empty response;
    auto status = stub_->UpdateDevice(&context_, request, &response);
    if (CheckStatus(status, "SetPosition"))
      std::cout << "move " << device_serial << " " << x << " " << y << " " << z
                << std::endl;
  }

  // set-visibility <device_serial> <on|off>
  void SetVisibility(const Args &args) {
    if (args.size() != 3)
      return Usage("set-visibility <device_serial> <on|off>");

    auto device_serial = std::string(args.at(1));
    auto visible = args.at(2) == "on";
    frontend::UpdateDeviceRequest request;
    request.mutable_device()->set_device_serial(device_serial);
    request.mutable_device()->set_visible(visible);
    auto status = stub_->UpdateDevice(&context_, request, nullptr);
    if (CheckStatus(status, "SetVisibility"))
      std::cout << "set-visibility " << device_serial << " " << visible;
  }

  std::string stateToString(const model::Chip::Radio &radio) {
    return radio.state() == model::State::ON ? "up" : "down";
  }

  // devices
  void GetDevices(const Args &args) {
    if (args.size() != 1) return Usage("devices");

    frontend::GetDevicesResponse response;
    auto status = stub_->GetDevices(&context_, {}, &response);
    if (CheckStatus(status, "GetDevices")) {
      std::cout << "List of devices attached\n";
      for (const auto &device : response.devices()) {
        std::stringstream stream;
        stream << std::fixed << std::setprecision(2) << "position("
               << device.position().x() << "," << device.position().y() << ","
               << device.position().z() << ")";
        const std::string position = stream.str();
        std::cout << device.device_serial() << "\t";

        for (const auto &chip : device.chips()) {
          switch (chip.chip_case()) {
            case model::Chip::ChipCase::kBt:
              std::cout << "ble:" << stateToString(chip.bt().low_energy())
                        << "\t"
                        << "classic:" << stateToString(chip.bt().classic());
              break;
            default:
              std::cout << "unknown:down";
              break;
          }
        }
        std::cout << position << std::endl;
      }
    }
  }

  // radio <ble|bt> up|down <device>
  void SetRadio(const Args &args) {
    std::unordered_set<std::string> radio_le = {"ble", "low_energy", "le"};
    std::unordered_set<std::string> radio_bt = {"bt", "classic"};
    std::unordered_set<std::string> up_status = {"up", "on", "enabled", "true"};

    if (args.size() != 4)
      return Usage("arg count - radio <ble|classic> <up|down> <device_serial>");
    auto radio_str = std::string(args.at(1));
    bool is_le = radio_le.count(radio_str);
    bool is_bt = radio_bt.count(radio_str);
    if (!(is_le || is_bt)) {
      return Usage(
          "unknown radio - radio <ble|classic> <up|down> <device_serial>");
    }
    auto radio_state = up_status.count(std::string(args.at(2)))
                           ? model::State::ON
                           : model::State::OFF;

    auto device_serial = std::string(args.at(3));
    frontend::UpdateDeviceRequest request;
    google::protobuf::Empty response;
    request.mutable_device()->set_device_serial(device_serial);
    auto bt = request.mutable_device()->add_chips()->mutable_bt();
    if (is_le) {
      bt->mutable_low_energy()->set_state(radio_state);
    } else {
      bt->mutable_classic()->set_state(radio_state);
    }
    auto status = stub_->UpdateDevice(&context_, request, &response);
    if (CheckStatus(status, "SetRadio")) {
      std::cout << "radio " << args.at(1) << " is " << args.at(2) << " for "
                << args.at(3) << std::endl;
    }
  }

  // capture <true/false> [<device_serial>]
  void SetPacketCapture(const Args &args) {
    if (args.size() != 2 && args.size() != 3)
      return Usage("capture <true/false> [<device_serial>]");

    auto capture = args.at(1) == "on";
    auto device_serial = (args.size() == 3) ? std::string(args.at(2)) : "";

    frontend::SetPacketCaptureRequest request;
    request.set_capture(capture);
    if (!device_serial.empty()) request.set_device_serial(device_serial);
    google::protobuf::Empty response;
    auto status = stub_->SetPacketCapture(&context_, request, &response);
    if (CheckStatus(status, "SetPacketCapture")) {
      std::cout << "turn " << (capture ? "on" : "off") << " packet capture for "
                << (device_serial.empty() ? "all devices" : device_serial)
                << std::endl;
    }
  }

  // Reset all devices.
  // reset
  void Reset(const Args &args) {
    if (args.size() != 1) return Usage("reset");
    google::protobuf::Empty response;
    auto status = stub_->Reset(&context_, {}, &response);
    if (CheckStatus(status, "Reset"))
      std::cout << "Reset all devices" << std::endl;
  }

 private:
  std::unique_ptr<frontend::FrontendService::Stub> stub_;
  grpc::ClientContext context_;

  static bool CheckStatus(const grpc::Status &status,
                          const std::string &message) {
    if (status.ok()) return true;
    if (status.error_code() == grpc::StatusCode::UNAVAILABLE)
      std::cerr << "error: netsim frontend service is unavailable, "
                   "please restart."
                << std::endl;
    else
      std::cerr << "error: request to service failed (" << status.error_code()
                << ") - " << status.error_message() << std::endl;
    return false;
  }
};
}  // namespace

std::unique_ptr<frontend::FrontendService::Stub> NewFrontendStub() {
  auto port = GetServerAddress();
  if (!port.has_value()) {
    return {};
  }
  auto server = "localhost:" + port.value();
  std::shared_ptr<grpc::Channel> channel =
      grpc::CreateChannel(server, grpc::InsecureChannelCredentials());

  auto deadline = std::chrono::system_clock::now() + kConnectionDeadline;
  if (!channel->WaitForConnected(deadline)) {
    return nullptr;
  }

  return frontend::FrontendService::NewStub(channel);
}

int SendCommand(std::unique_ptr<frontend::FrontendService::Stub> stub,
                const Args &args) {
  FrontendClient frontend(std::move(stub));
  if (args.empty()) {
    std::cout << kUsage << std::endl;
    return 1;
  }

  auto cmd = args.front();

  if (cmd == "version")
    frontend.GetVersion(args);
  else if (cmd == "radio")
    frontend.SetRadio(args);
  else if (cmd == "move")
    frontend.SetPosition(args);
  else if (cmd == "set-visibility")
    frontend.SetVisibility(args);
  else if (cmd == "devices")
    frontend.GetDevices(args);
  else if (cmd == "capture")
    frontend.SetPacketCapture(args);
  else if (cmd == "reset")
    frontend.Reset(args);
  else if (cmd == "positions" || cmd == "visibility" ||
           cmd == "set-link-loss" || cmd == "set-range" || cmd == "net-cat") {
    std::cout << "Not implement yet" << std::endl;
  } else {
    std::cout << "Unknown: " << args[0] << std::endl;
    std::cout << kUsage << std::endl;
  }
  return 0;
}

}  // namespace netsim
