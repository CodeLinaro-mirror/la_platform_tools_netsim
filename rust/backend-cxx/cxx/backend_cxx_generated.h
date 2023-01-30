#pragma once
#include "backend/backend_server.h"
#include <memory>
#include <string>

namespace netsim {
  using PacketStreamClient = ::netsim::PacketStreamClient;
}

namespace netsim {
void StreamPacketHandler(::std::unique_ptr<::netsim::PacketStreamClient> packet_stream_client) noexcept;
} // namespace netsim
