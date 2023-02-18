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
#include "frontend/frontend_client.h"

#include <google/protobuf/util/json_util.h>
#include <stdlib.h>

#include <chrono>
#include <iomanip>
#include <iostream>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>

#include "../rust/frontend-client-cxx/cxx/frontend_client_cxx_generated.h"
#include "frontend.grpc.pb.h"
#include "frontend.pb.h"
#include "google/protobuf/empty.pb.h"
#include "grpcpp/create_channel.h"
#include "grpcpp/security/credentials.h"
#include "grpcpp/support/status_code_enum.h"
#include "model.pb.h"
#include "util/ini_file.h"
#include "util/os_utils.h"
#include "util/string_utils.h"

namespace netsim {
namespace frontend {
namespace {
const std::chrono::duration kConnectionDeadline = std::chrono::seconds(1);

std::unique_ptr<frontend::FrontendService::Stub> NewFrontendStub() {
  auto port = netsim::osutils::GetServerAddress();
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

// A synchronous client for the netsim frontend service.
class FrontendClientImpl : public FrontendClient {
 public:
  FrontendClientImpl(std::unique_ptr<frontend::FrontendService::Stub> stub)
      : stub_(std::move(stub)) {}

  std::unique_ptr<ClientResult> make_result(
      const grpc::Status &status,
      const google::protobuf::Message &message) const {
    std::vector<unsigned char> message_vec(message.ByteSizeLong());
    message.SerializeToArray(message_vec.data(), message_vec.size());
    if (!status.ok()) {
      return std::make_unique<ClientResult>(false, status.error_message(),
                                            message_vec);
    }
    return std::make_unique<ClientResult>(true, "", message_vec);
  }

  // Gets the version of the network simulator service.
  std::unique_ptr<ClientResult> GetVersion() const override {
    frontend::VersionResponse response;
    grpc::ClientContext context_;
    auto status = stub_->GetVersion(&context_, {}, &response);
    return make_result(status, response);
  }

  // Gets the list of device information
  std::unique_ptr<ClientResult> GetDevices() const override {
    frontend::GetDevicesResponse response;
    grpc::ClientContext context_;
    auto status = stub_->GetDevices(&context_, {}, &response);
    return make_result(status, response);
  }

  std::unique_ptr<ClientResult> Reset() const override {
    grpc::ClientContext context_;
    google::protobuf::Empty response;
    auto status = stub_->Reset(&context_, {}, &response);
    return make_result(status, response);
  }

  // Patchs the information of the device
  std::unique_ptr<ClientResult> PatchDevice(
      rust::Vec<::rust::u8> const &request_byte_vec) const override {
    ::google::protobuf::Empty response;
    grpc::ClientContext context_;
    frontend::PatchDeviceRequest request;
    request.ParsePartialFromArray(request_byte_vec.data(),
                                  request_byte_vec.size());
    auto status = stub_->PatchDevice(&context_, request, &response);
    return make_result(status, response);
  }

  // Helper function to redirect Grpc
  std::unique_ptr<ClientResult> SendGrpc(
      frontend::GrpcMethod const &grpc_method,
      rust::Vec<::rust::u8> const &request_byte_vec) const override {
    switch (grpc_method) {
      case frontend::GrpcMethod::GetVersion:
        return GetVersion();
      case frontend::GrpcMethod::PatchDevice:
        return PatchDevice(request_byte_vec);
      case frontend::GrpcMethod::GetDevices:
        return GetDevices();
      case frontend::GrpcMethod::Reset:
        return Reset();
      default:
        return make_result(::grpc::Status(::grpc::StatusCode::UNKNOWN,
                                          "Unknown GrpcMethod found."),
                           ::google::protobuf::Empty());
    }
  }

 private:
  std::unique_ptr<frontend::FrontendService::Stub> stub_;

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

std::unique_ptr<FrontendClient> NewFrontendClient() {
  auto stub = NewFrontendStub();
  return (stub == nullptr
              ? nullptr
              : std::make_unique<FrontendClientImpl>(std::move(stub)));
}

}  // namespace frontend
}  // namespace netsim
