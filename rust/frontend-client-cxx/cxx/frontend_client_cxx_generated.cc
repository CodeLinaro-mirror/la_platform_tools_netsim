#include "frontend/frontend_client.h"
#include <cstddef>
#include <memory>
#include <new>
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
  namespace frontend {
    using ClientResult = ::netsim::frontend::ClientResult;
    using FrontendClient = ::netsim::frontend::FrontendClient;
  }
}

namespace netsim {
namespace frontend {
extern "C" {
::netsim::frontend::FrontendClient *netsim$frontend$cxxbridge1$new_frontend_client() noexcept {
  ::std::unique_ptr<::netsim::frontend::FrontendClient> (*new_frontend_client$)() = ::netsim::frontend::NewFrontendClient;
  return new_frontend_client$().release();
}

::netsim::frontend::ClientResult *netsim$frontend$cxxbridge1$FrontendClient$get_version(const ::netsim::frontend::FrontendClient &self) noexcept {
  ::std::unique_ptr<::netsim::frontend::ClientResult> (::netsim::frontend::FrontendClient::*get_version$)() const = &::netsim::frontend::FrontendClient::GetVersion;
  return (self.*get_version$)().release();
}

::netsim::frontend::ClientResult *netsim$frontend$cxxbridge1$FrontendClient$get_devices(const ::netsim::frontend::FrontendClient &self) noexcept {
  ::std::unique_ptr<::netsim::frontend::ClientResult> (::netsim::frontend::FrontendClient::*get_devices$)() const = &::netsim::frontend::FrontendClient::GetDevices;
  return (self.*get_devices$)().release();
}
} // extern "C"
} // namespace frontend
} // namespace netsim

extern "C" {
static_assert(::rust::detail::is_complete<::netsim::frontend::FrontendClient>::value, "definition of FrontendClient is required");
static_assert(sizeof(::std::unique_ptr<::netsim::frontend::FrontendClient>) == sizeof(void *), "");
static_assert(alignof(::std::unique_ptr<::netsim::frontend::FrontendClient>) == alignof(void *), "");
void cxxbridge1$unique_ptr$netsim$frontend$FrontendClient$null(::std::unique_ptr<::netsim::frontend::FrontendClient> *ptr) noexcept {
  ::new (ptr) ::std::unique_ptr<::netsim::frontend::FrontendClient>();
}
void cxxbridge1$unique_ptr$netsim$frontend$FrontendClient$raw(::std::unique_ptr<::netsim::frontend::FrontendClient> *ptr, ::netsim::frontend::FrontendClient *raw) noexcept {
  ::new (ptr) ::std::unique_ptr<::netsim::frontend::FrontendClient>(raw);
}
const ::netsim::frontend::FrontendClient *cxxbridge1$unique_ptr$netsim$frontend$FrontendClient$get(const ::std::unique_ptr<::netsim::frontend::FrontendClient>& ptr) noexcept {
  return ptr.get();
}
::netsim::frontend::FrontendClient *cxxbridge1$unique_ptr$netsim$frontend$FrontendClient$release(::std::unique_ptr<::netsim::frontend::FrontendClient>& ptr) noexcept {
  return ptr.release();
}
void cxxbridge1$unique_ptr$netsim$frontend$FrontendClient$drop(::std::unique_ptr<::netsim::frontend::FrontendClient> *ptr) noexcept {
  ::rust::deleter_if<::rust::detail::is_complete<::netsim::frontend::FrontendClient>::value>{}(ptr);
}

static_assert(::rust::detail::is_complete<::netsim::frontend::ClientResult>::value, "definition of ClientResult is required");
static_assert(sizeof(::std::unique_ptr<::netsim::frontend::ClientResult>) == sizeof(void *), "");
static_assert(alignof(::std::unique_ptr<::netsim::frontend::ClientResult>) == alignof(void *), "");
void cxxbridge1$unique_ptr$netsim$frontend$ClientResult$null(::std::unique_ptr<::netsim::frontend::ClientResult> *ptr) noexcept {
  ::new (ptr) ::std::unique_ptr<::netsim::frontend::ClientResult>();
}
void cxxbridge1$unique_ptr$netsim$frontend$ClientResult$raw(::std::unique_ptr<::netsim::frontend::ClientResult> *ptr, ::netsim::frontend::ClientResult *raw) noexcept {
  ::new (ptr) ::std::unique_ptr<::netsim::frontend::ClientResult>(raw);
}
const ::netsim::frontend::ClientResult *cxxbridge1$unique_ptr$netsim$frontend$ClientResult$get(const ::std::unique_ptr<::netsim::frontend::ClientResult>& ptr) noexcept {
  return ptr.get();
}
::netsim::frontend::ClientResult *cxxbridge1$unique_ptr$netsim$frontend$ClientResult$release(::std::unique_ptr<::netsim::frontend::ClientResult>& ptr) noexcept {
  return ptr.release();
}
void cxxbridge1$unique_ptr$netsim$frontend$ClientResult$drop(::std::unique_ptr<::netsim::frontend::ClientResult> *ptr) noexcept {
  ::rust::deleter_if<::rust::detail::is_complete<::netsim::frontend::ClientResult>::value>{}(ptr);
}
} // extern "C"
