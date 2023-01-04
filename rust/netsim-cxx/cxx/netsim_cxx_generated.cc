#include <cstdint>

namespace netsim {
extern "C" {
void netsim$cxxbridge1$run_frontend_http_server() noexcept;

::std::int8_t netsim$cxxbridge1$distance_to_rssi(::std::int8_t tx_power, float distance) noexcept;
} // extern "C"

void RunFrontendHttpServer() noexcept {
  netsim$cxxbridge1$run_frontend_http_server();
}

::std::int8_t DistanceToRssi(::std::int8_t tx_power, float distance) noexcept {
  return netsim$cxxbridge1$distance_to_rssi(tx_power, distance);
}
} // namespace netsim
