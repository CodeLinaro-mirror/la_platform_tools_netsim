#include "controller/controller.h"
#include <array>
#include <cstddef>
#include <cstdint>
#include <new>
#include <string>
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

namespace detail {
template <typename T, typename = void *>
struct operator_new {
  void *operator()(::std::size_t sz) { return ::operator new(sz); }
};

template <typename T>
struct operator_new<T, decltype(T::operator new(sizeof(T)))> {
  void *operator()(::std::size_t sz) { return T::operator new(sz); }
};
} // namespace detail

template <typename T>
union MaybeUninit {
  T value;
  void *operator new(::std::size_t sz) { return detail::operator_new<T>{}(sz); }
  MaybeUninit() {}
  ~MaybeUninit() {}
};
} // namespace cxxbridge1
} // namespace rust

namespace netsim {
extern "C" {
void netsim$cxxbridge1$run_frontend_http_server() noexcept;

::std::int8_t netsim$cxxbridge1$distance_to_rssi(::std::int8_t tx_power, float distance) noexcept;

void netsim$cxxbridge1$get_version(::rust::String *return$) noexcept;
} // extern "C"

namespace scene_controller {
extern "C" {
::std::uint32_t netsim$scene_controller$cxxbridge1$get_devices(::std::string const &request, ::std::string &response, ::std::string &error_message) noexcept {
  ::std::uint32_t (*get_devices$)(::std::string const &, ::std::string &, ::std::string &) = ::netsim::scene_controller::GetDevices;
  return get_devices$(request, response, error_message);
}

::std::uint32_t netsim$scene_controller$cxxbridge1$patch_device(::std::string const &request, ::std::string &response, ::std::string &error_message) noexcept {
  ::std::uint32_t (*patch_device$)(::std::string const &, ::std::string &, ::std::string &) = ::netsim::scene_controller::PatchDevice;
  return patch_device$(request, response, error_message);
}
} // extern "C"
} // namespace scene_controller

void RunFrontendHttpServer() noexcept {
  netsim$cxxbridge1$run_frontend_http_server();
}

::std::int8_t DistanceToRssi(::std::int8_t tx_power, float distance) noexcept {
  return netsim$cxxbridge1$distance_to_rssi(tx_power, distance);
}

::rust::String GetVersion() noexcept {
  ::rust::MaybeUninit<::rust::String> return$;
  netsim$cxxbridge1$get_version(&return$.value);
  return ::std::move(return$.value);
}
} // namespace netsim
