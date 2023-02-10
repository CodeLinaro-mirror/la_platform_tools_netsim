#pragma once
#include <cstdint>

namespace netsim {
void RunFrontendHttpServer() noexcept;

::std::int8_t DistanceToRssi(::std::int8_t tx_power, float distance) noexcept;
} // namespace netsim
