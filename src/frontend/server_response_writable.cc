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

#include "server_response_writable.h"

#include <fstream>
#include <iostream>
#include <string>

#include "../../rust/netsim-cxx/cxx/netsim_cxx_generated.h"

namespace netsim {
namespace frontend {
/// The C++ implementation of the CxxResponder interface. This is used by the
/// gRPC server to invoke the Rust pcap handler and process a responses.
class CxxServerResponseWritable : public CxxServerResponseWriter {
 public:
  CxxServerResponseWritable(std::ofstream *outfile) : outfile(outfile){};

 private:
  void put_error(unsigned int error_code,
                 const std::string &response) const override {
    std::cout << "cxx result: " << error_code << " " << response.c_str()
              << std::endl;
  }
  void put_ok_with_length(const std::string &mime_type,
                          unsigned int length) const override {
    *outfile << "HTTP/1.1 200 OK" << std::endl
             << "Content-Type: " << mime_type << std::endl
             << "Content-Length: " << length << std::endl
             << std::endl;
  }
  void put_chunk(rust::Slice<const uint8_t> chunk) const override {
    outfile->write((char *)chunk.data(), chunk.size());
  }
  void put_ok(const std::string &mime_type,
              const std::string &body) const override {
    std::cout << "cxx result: " << mime_type.c_str() << " " << body.c_str()
              << std::endl;
  }
  std::ofstream *outfile;
};

void StartMockGrpcServer() {
  std::ofstream outfile("/tmp/test.txt");
  CxxServerResponseWritable responder(&outfile);
  HandlePcapCxx(responder, "GET", "0", "HandlePcapCxx");
}
}  // namespace frontend
}  // namespace netsim