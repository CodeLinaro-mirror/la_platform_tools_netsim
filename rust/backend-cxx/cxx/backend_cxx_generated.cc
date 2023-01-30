#include "backend/backend_server.h"
#include <cstddef>
#include <memory>
#include <new>
#include <string>
#include <type_traits>
#include <utility>

namespace rust {
inline namespace cxxbridge1 {
// #include "rust/cxx.h"

#ifndef CXXBRIDGE1_IS_COMPLETE
#define CXXBRIDGE1_IS_COMPLETE
namespace detail {
namespace {
template <typename T, typename = std::size_t>
struct is_complete : std::false_type {};
template <typename T>
struct is_complete<T, decltype(sizeof(T))> : std::true_type {};
} // namespace
} // namespace detail
#endif // CXXBRIDGE1_IS_COMPLETE

namespace {
template <bool> struct deleter_if {
  template <typename T> void operator()(T *) {}
};

template <> struct deleter_if<true> {
  template <typename T> void operator()(T *ptr) { ptr->~T(); }
};
} // namespace
} // namespace cxxbridge1
} // namespace rust

namespace netsim {
  using PacketStreamClient = ::netsim::PacketStreamClient;
}

namespace netsim {
extern "C" {
void netsim$cxxbridge1$stream_packets_handler(::netsim::PacketStreamClient *packet_stream_client) noexcept;

::std::string *netsim$cxxbridge1$PacketStreamClient$read(::netsim::PacketStreamClient const &self) noexcept {
  ::std::unique_ptr<::std::string> (::netsim::PacketStreamClient::*read$)() const = &::netsim::PacketStreamClient::Read;
  return (self.*read$)().release();
}

void netsim$cxxbridge1$PacketStreamClient$write(::netsim::PacketStreamClient const &self, ::std::string const &response) noexcept {
  void (::netsim::PacketStreamClient::*write$)(::std::string const &) const = &::netsim::PacketStreamClient::Write;
  (self.*write$)(response);
}
} // extern "C"

void StreamPacketHandler(::std::unique_ptr<::netsim::PacketStreamClient> packet_stream_client) noexcept {
  netsim$cxxbridge1$stream_packets_handler(packet_stream_client.release());
}
} // namespace netsim

extern "C" {
static_assert(::rust::detail::is_complete<::netsim::PacketStreamClient>::value, "definition of PacketStreamClient is required");
static_assert(sizeof(::std::unique_ptr<::netsim::PacketStreamClient>) == sizeof(void *), "");
static_assert(alignof(::std::unique_ptr<::netsim::PacketStreamClient>) == alignof(void *), "");
void cxxbridge1$unique_ptr$netsim$PacketStreamClient$null(::std::unique_ptr<::netsim::PacketStreamClient> *ptr) noexcept {
  ::new (ptr) ::std::unique_ptr<::netsim::PacketStreamClient>();
}
void cxxbridge1$unique_ptr$netsim$PacketStreamClient$raw(::std::unique_ptr<::netsim::PacketStreamClient> *ptr, ::netsim::PacketStreamClient *raw) noexcept {
  ::new (ptr) ::std::unique_ptr<::netsim::PacketStreamClient>(raw);
}
::netsim::PacketStreamClient const *cxxbridge1$unique_ptr$netsim$PacketStreamClient$get(::std::unique_ptr<::netsim::PacketStreamClient> const &ptr) noexcept {
  return ptr.get();
}
::netsim::PacketStreamClient *cxxbridge1$unique_ptr$netsim$PacketStreamClient$release(::std::unique_ptr<::netsim::PacketStreamClient> &ptr) noexcept {
  return ptr.release();
}
void cxxbridge1$unique_ptr$netsim$PacketStreamClient$drop(::std::unique_ptr<::netsim::PacketStreamClient> *ptr) noexcept {
  ::rust::deleter_if<::rust::detail::is_complete<::netsim::PacketStreamClient>::value>{}(ptr);
}
} // extern "C"
