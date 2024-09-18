// Copyright 2023 The Android Open Source Project
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

#include "wifi/wifi_facade.h"

#include <memory>

#include "netsim-daemon/src/ffi.rs.h"
#include "netsim/config.pb.h"
#include "rust/cxx.h"
#include "util/log.h"
#include "util/string_utils.h"
#ifdef NETSIM_ANDROID_EMULATOR
#include "android-qemu2-glue/emulation/VirtioWifiForwarder.h"
#include "android-qemu2-glue/emulation/WifiService.h"
#include "android-qemu2-glue/netsim/libslirp_driver.h"
#endif

namespace netsim::wifi {
namespace {

#ifdef NETSIM_ANDROID_EMULATOR
std::shared_ptr<android::qemu2::WifiService> wifi_service;
#endif
;

}  // namespace

namespace facade {

size_t HandleWifiCallback(const uint8_t *buf, size_t size) {
  //  Broadcast the response to all WiFi chips.
  std::vector<uint8_t> packet(buf, buf + size);
  rust::Slice<const uint8_t> packet_slice(packet.data(), packet.size());
  wifi::facade::HandleWiFiResponse(packet_slice);
  return size;
}

void Start(const rust::Slice<::std::uint8_t const> proto_bytes) {
#ifdef NETSIM_ANDROID_EMULATOR
  // Initialize hostapd and slirp inside WiFi Service.
  config::WiFi config;
  config.ParseFromArray(proto_bytes.data(), proto_bytes.size());

  android::qemu2::HostapdOptions hostapd = {
      .disabled = config.hostapd_options().disabled(),
      .ssid = config.hostapd_options().ssid(),
      .passwd = config.hostapd_options().passwd()};

  auto host_dns = stringutils::Split(config.slirp_options().host_dns(), ",");
  android::qemu2::SlirpOptions slirpOpts = {
      .disabled = config.slirp_options().disabled(),
      .ipv4 = (config.slirp_options().has_ipv4() ? config.slirp_options().ipv4()
                                                 : true),
      .restricted = config.slirp_options().restricted(),
      .vnet = config.slirp_options().vnet(),
      .vhost = config.slirp_options().vhost(),
      .vmask = config.slirp_options().vmask(),
      .ipv6 = (config.slirp_options().has_ipv6() ? config.slirp_options().ipv6()
                                                 : true),
      .vprefix6 = config.slirp_options().vprefix6(),
      .vprefixLen = (uint8_t)config.slirp_options().vprefixlen(),
      .vhost6 = config.slirp_options().vhost6(),
      .vhostname = config.slirp_options().vhostname(),
      .tftpath = config.slirp_options().tftpath(),
      .bootfile = config.slirp_options().bootfile(),
      .dhcpstart = config.slirp_options().dhcpstart(),
      .dns = config.slirp_options().dns(),
      .dns6 = config.slirp_options().dns6(),
      .host_dns = host_dns,
  };
  if (!config.slirp_options().host_dns().empty()) {
    BtsLogInfo("Host DNS server: %s",
               config.slirp_options().host_dns().c_str());
  }
  auto builder = android::qemu2::WifiService::Builder()
                     .withHostapd(hostapd)
                     .withSlirp(slirpOpts)
                     .withOnReceiveCallback(HandleWifiCallback)
                     .withVerboseLogging(true);
  wifi_service = builder.build();
  if (!wifi_service->init()) {
    BtsLogWarn("Failed to initialize wifi service");
  }
#endif
}
void Stop() {
#ifdef NETSIM_ANDROID_EMULATOR
  wifi_service->stop();
#endif
}

}  // namespace facade

void libslirp_main_loop_wait() {
#ifdef NETSIM_ANDROID_EMULATOR
  // main_loop_wait is a non-blocking call where fds maintained by the
  // WiFi service (slirp) are polled and serviced for I/O. When any fd
  // become ready for I/O, slirp_pollfds_poll() will be invoked to read
  // from the open sockets therefore incoming packets are serviced.
  android::qemu2::libslirp_main_loop_wait(true);
#endif
}

void HandleWifiRequestCxx(const rust::Vec<uint8_t> &packet) {
#ifdef NETSIM_ANDROID_EMULATOR
  // Send the packet to the WiFi service.
  struct iovec iov[1];
  iov[0].iov_base = (void *)packet.data();
  iov[0].iov_len = packet.size();
  wifi_service->send(android::base::IOVector(iov, iov + 1));
#endif
}

void HostapdSendCxx(const rust::Vec<uint8_t> &packet) {
#ifdef NETSIM_ANDROID_EMULATOR
  // Send the packet to Hostapd.
  struct iovec iov[1];
  iov[0].iov_base = (void *)packet.data();
  iov[0].iov_len = packet.size();
  wifi_service->hostapd_send(android::base::IOVector(iov, iov + 1));
#endif
}

void LibslirpSendCxx(const rust::Vec<uint8_t> &packet) {
#ifdef NETSIM_ANDROID_EMULATOR
  // Send the packet to libslirp.
  struct iovec iov[1];
  iov[0].iov_base = (void *)packet.data();
  iov[0].iov_len = packet.size();
  wifi_service->libslirp_send(android::base::IOVector(iov, iov + 1));
#endif
}

bool IsEapolCxx(const rust::Slice<uint8_t const> packet) {
#ifdef NETSIM_ANDROID_EMULATOR
  struct iovec iov[1];
  iov[0].iov_base = (void *)packet.data();
  iov[0].iov_len = packet.size();
  return std::dynamic_pointer_cast<android::qemu2::VirtioWifiForwarder>(
             wifi_service)
      ->is_eapol(android::base::IOVector(iov, iov + 1));
#else
  return 0;
#endif
}

}  // namespace netsim::wifi
