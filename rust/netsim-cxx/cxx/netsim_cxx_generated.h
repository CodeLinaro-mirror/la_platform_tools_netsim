#pragma once
#include "controller/controller.h"
#include <cstdint>
#include <string>

namespace netsim {
void RunFrontendHttpServer() noexcept;

::std::int8_t DistanceToRssi(::std::int8_t tx_power, float distance) noexcept;
} // namespace netsim
