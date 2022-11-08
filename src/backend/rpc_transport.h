// Copyright (C) 2022 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#pragma once
#include <grpcpp/grpcpp.h>

#include <condition_variable>
#include <memory>
#include <mutex>
#include <queue>
#include <thread>

#include "util/blocking_queue.h"

namespace netsim {
template <typename R, typename W>
class RpcTransport {
 public:
  RpcTransport() {}

  // Start read/write
  //
  // ClientContext is used internally by Grpc and must be maintained
  // until the stream finishes.
  void Start(std::string serial,
             std::unique_ptr<grpc::ClientReaderWriter<R, W>> stream,
             std::unique_ptr<grpc::ClientContext> context) {
    mSerial = serial;
    mStream = std::move(stream);
    mContext = std::move(context);
    mReader = std::move(std::thread([&] { startReader(); }));
    mWriter = std::move(std::thread([&] { startWriter(); }));
  }

  virtual void Read(const R *msg) = 0;
  virtual void OnDone() = 0;
  virtual void OnCancel() { Finish(::grpc::Status::CANCELLED); }

  void Write(const W &msg) { mQueue.Push(msg); }

  void Finish(::grpc::Status state) {
    mQueue.Stop();
    mStatus = state;
  }

  grpc::Status Status() { return mStatus; }

  void Await() {
    mWriter.join();
    mReader.join();
  }

 private:
  void startReader() {
    R incoming;

    // InitialMetadata comes with the firt read on the stream. This will block
    // until then.
    std::cerr << "RpcTransport: WaitForInitialMetadata " << mSerial
              << std::endl;
    mStream->WaitForInitialMetadata();
    std::cerr << "RpcTransport: WaitForInitialMetadata - done " << mSerial
              << std::endl;

    while (mQueue.Active() && mStream->Read(&incoming)) {
      Read(&incoming);
    }
    OnCancel();
  }

  void startWriter() {
    W outgoing;
    while (mQueue.WaitAndPop(outgoing)) {
      if (!mStream->Write(outgoing)) break;
    }
    OnCancel();
  }

  std::unique_ptr<grpc::ClientReaderWriter<R, W>> mStream;
  std::unique_ptr<grpc::ClientContext> mContext;
  std::string mSerial;

  util::BlockingQueue<W> mQueue;
  std::thread mWriter;
  std::thread mReader;
  grpc::Status mStatus{grpc::Status::OK};
};
}  // namespace netsim
