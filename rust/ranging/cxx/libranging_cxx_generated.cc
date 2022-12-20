#include <cstdint>

namespace netsim {
extern "C" {
::std::int8_t netsim$cxxbridge1$distance_to_rssi(::std::int8_t tx_power, float distance) noexcept;
} // extern "C"

::std::int8_t DistanceToRssi(::std::int8_t tx_power, float distance) noexcept {
  return netsim$cxxbridge1$distance_to_rssi(tx_power, distance);
}
} // namespace netsim
