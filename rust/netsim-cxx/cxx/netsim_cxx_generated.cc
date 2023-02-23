#include "controller/controller.h"
#include <cstdint>
#include <string>

namespace netsim {
extern "C" {
void netsim$cxxbridge1$run_frontend_http_server() noexcept;

::std::int8_t netsim$cxxbridge1$distance_to_rssi(::std::int8_t tx_power, float distance) noexcept;
} // extern "C"

namespace scene_controller {
extern "C" {
::std::uint32_t netsim$scene_controller$cxxbridge1$get_devices(::std::string const &request, ::std::string &response, ::std::string &error_message) noexcept {
  ::std::uint32_t (*get_devices$)(::std::string const &, ::std::string &, ::std::string &) = ::netsim::scene_controller::GetDevices;
  return get_devices$(request, response, error_message);
}

::std::uint32_t netsim$scene_controller$cxxbridge1$update_device(::std::string const &request, ::std::string &response, ::std::string &error_message) noexcept {
  ::std::uint32_t (*update_device$)(::std::string const &, ::std::string &, ::std::string &) = ::netsim::scene_controller::UpdateDevice;
  return update_device$(request, response, error_message);
}
} // extern "C"
} // namespace scene_controller

void RunFrontendHttpServer() noexcept {
  netsim$cxxbridge1$run_frontend_http_server();
}

::std::int8_t DistanceToRssi(::std::int8_t tx_power, float distance) noexcept {
  return netsim$cxxbridge1$distance_to_rssi(tx_power, distance);
}
} // namespace netsim
