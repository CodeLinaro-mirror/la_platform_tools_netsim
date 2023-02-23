#include "frontend/frontend_client.h"
#include <array>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <new>
#include <string>
#include <type_traits>
#include <utility>

namespace rust {
inline namespace cxxbridge1 {
// #include "rust/cxx.h"

struct unsafe_bitcopy_t;

#ifndef CXXBRIDGE1_RUST_STRING
#define CXXBRIDGE1_RUST_STRING
class String final {
public:
  String() noexcept;
  String(const String &) noexcept;
  String(String &&) noexcept;
  ~String() noexcept;

  String(const std::string &);
  String(const char *);
  String(const char *, std::size_t);
  String(const char16_t *);
  String(const char16_t *, std::size_t);

  static String lossy(const std::string &) noexcept;
  static String lossy(const char *) noexcept;
  static String lossy(const char *, std::size_t) noexcept;
  static String lossy(const char16_t *) noexcept;
  static String lossy(const char16_t *, std::size_t) noexcept;

  String &operator=(const String &) &noexcept;
  String &operator=(String &&) &noexcept;

  explicit operator std::string() const;

  const char *data() const noexcept;
  std::size_t size() const noexcept;
  std::size_t length() const noexcept;
  bool empty() const noexcept;

  const char *c_str() noexcept;

  std::size_t capacity() const noexcept;
  void reserve(size_t new_cap) noexcept;

  using iterator = char *;
  iterator begin() noexcept;
  iterator end() noexcept;

  using const_iterator = const char *;
  const_iterator begin() const noexcept;
  const_iterator end() const noexcept;
  const_iterator cbegin() const noexcept;
  const_iterator cend() const noexcept;

  bool operator==(const String &) const noexcept;
  bool operator!=(const String &) const noexcept;
  bool operator<(const String &) const noexcept;
  bool operator<=(const String &) const noexcept;
  bool operator>(const String &) const noexcept;
  bool operator>=(const String &) const noexcept;

  void swap(String &) noexcept;

  String(unsafe_bitcopy_t, const String &) noexcept;

private:
  struct lossy_t;
  String(lossy_t, const char *, std::size_t) noexcept;
  String(lossy_t, const char16_t *, std::size_t) noexcept;
  friend void swap(String &lhs, String &rhs) noexcept { lhs.swap(rhs); }

  std::array<std::uintptr_t, 3> repr;
};
#endif // CXXBRIDGE1_RUST_STRING

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
    using FrontendClient = ::netsim::frontend::FrontendClient;
    using ClientResult = ::netsim::frontend::ClientResult;
  }
}

namespace netsim {
namespace frontend {
extern "C" {
::netsim::frontend::FrontendClient *netsim$frontend$cxxbridge1$new_frontend_client() noexcept {
  ::std::unique_ptr<::netsim::frontend::FrontendClient> (*new_frontend_client$)() = ::netsim::frontend::NewFrontendClient;
  return new_frontend_client$().release();
}

::netsim::frontend::ClientResult *netsim$frontend$cxxbridge1$FrontendClient$get_version(::netsim::frontend::FrontendClient const &self) noexcept {
  ::std::unique_ptr<::netsim::frontend::ClientResult> (::netsim::frontend::FrontendClient::*get_version$)() const = &::netsim::frontend::FrontendClient::GetVersion;
  return (self.*get_version$)().release();
}

::netsim::frontend::ClientResult *netsim$frontend$cxxbridge1$FrontendClient$get_devices(::netsim::frontend::FrontendClient const &self) noexcept {
  ::std::unique_ptr<::netsim::frontend::ClientResult> (::netsim::frontend::FrontendClient::*get_devices$)() const = &::netsim::frontend::FrontendClient::GetDevices;
  return (self.*get_devices$)().release();
}

bool netsim$frontend$cxxbridge1$ClientResult$is_ok(::netsim::frontend::ClientResult const &self) noexcept {
  bool (::netsim::frontend::ClientResult::*is_ok$)() const = &::netsim::frontend::ClientResult::IsOk;
  return (self.*is_ok$)();
}

void netsim$frontend$cxxbridge1$ClientResult$err(::netsim::frontend::ClientResult const &self, ::rust::String *return$) noexcept {
  ::rust::String (::netsim::frontend::ClientResult::*err$)() const = &::netsim::frontend::ClientResult::Err;
  new (return$) ::rust::String((self.*err$)());
}

void netsim$frontend$cxxbridge1$ClientResult$byte_str(::netsim::frontend::ClientResult const &self, ::rust::String *return$) noexcept {
  ::rust::String (::netsim::frontend::ClientResult::*byte_str$)() const = &::netsim::frontend::ClientResult::ByteStr;
  new (return$) ::rust::String((self.*byte_str$)());
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
::netsim::frontend::FrontendClient const *cxxbridge1$unique_ptr$netsim$frontend$FrontendClient$get(::std::unique_ptr<::netsim::frontend::FrontendClient> const &ptr) noexcept {
  return ptr.get();
}
::netsim::frontend::FrontendClient *cxxbridge1$unique_ptr$netsim$frontend$FrontendClient$release(::std::unique_ptr<::netsim::frontend::FrontendClient> &ptr) noexcept {
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
::netsim::frontend::ClientResult const *cxxbridge1$unique_ptr$netsim$frontend$ClientResult$get(::std::unique_ptr<::netsim::frontend::ClientResult> const &ptr) noexcept {
  return ptr.get();
}
::netsim::frontend::ClientResult *cxxbridge1$unique_ptr$netsim$frontend$ClientResult$release(::std::unique_ptr<::netsim::frontend::ClientResult> &ptr) noexcept {
  return ptr.release();
}
void cxxbridge1$unique_ptr$netsim$frontend$ClientResult$drop(::std::unique_ptr<::netsim::frontend::ClientResult> *ptr) noexcept {
  ::rust::deleter_if<::rust::detail::is_complete<::netsim::frontend::ClientResult>::value>{}(ptr);
}
} // extern "C"
