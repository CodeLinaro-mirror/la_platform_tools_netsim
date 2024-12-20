message(STATUS "Adding rust ${CMAKE_CURRENT_LIST_DIR}/Cargo.toml")

# Delete any old Cargo.lock files
file(REMOVE ${CMAKE_CURRENT_LIST_DIR}/Cargo.lock)

# This will automatically register all the tests as well.
corrosion_import_crate(MANIFEST_PATH Cargo.toml FLAGS --offline --verbose)

# Set corrosion env vars
corrosion_set_env_vars(netsim-common CARGO_HOME=${Rust_CARGO_HOME})
corrosion_set_env_vars(http-proxy CARGO_HOME=${Rust_CARGO_HOME})
corrosion_set_env_vars(libslirp-rs CARGO_HOME=${Rust_CARGO_HOME})
corrosion_set_env_vars(capture CARGO_HOME=${Rust_CARGO_HOME})

function(set_prebuilt_packets_env_vars target)
  corrosion_set_env_vars(
    ${target}
    LINK_LAYER_PACKETS_PREBUILT=${RootCanalGeneratedPackets_rs}
    NETLINK_PACKETS_PREBUILT=${NetlinkPackets_rs}
    MAC80211_HWSIM_PACKETS_PREBUILT=${Mac80211HwsimPackets_rs}
    IEEE80211_PACKETS_PREBUILT=${Ieee80211Packets_rs}
    LLC_PACKETS_PREBUILT=${LlcPackets_rs}
    CARGO_HOME=${Rust_CARGO_HOME})
endfunction()

set_prebuilt_packets_env_vars(hostapd-rs)
set_prebuilt_packets_env_vars(netsim-packets)

corrosion_set_env_vars(
  netsim-daemon
  CARGO_HOME=${Rust_CARGO_HOME}
  GRPCIO_SYS_GRPC_INCLUDE_PATH="${CMAKE_CURRENT_SOURCE_DIR}/../../../external/grpc/include"
)

corrosion_set_env_vars(
  netsim-cli
  CARGO_HOME=${Rust_CARGO_HOME}
  GRPCIO_SYS_GRPC_INCLUDE_PATH="${CMAKE_CURRENT_SOURCE_DIR}/../../../external/grpc/include"
)
corrosion_set_env_vars(
  netsim-proto
  CARGO_HOME=${Rust_CARGO_HOME}
  GRPCIO_SYS_GRPC_INCLUDE_PATH="${CMAKE_CURRENT_SOURCE_DIR}/../../../external/grpc/include"
)

add_custom_target(
  netsim_rust_packets
  DEPENDS pdl_gen-RootCanalGeneratedPackets_rs pdl_gen-NetlinkPackets_rs
          pdl_gen-LlcPackets_rs pdl_gen-Ieee80211Packets_rs
          pdl_gen-Mac80211HwsimPackets_rs)

# Make sure we have the rust packets generated before we build them.
add_dependencies(cargo-build_netsim-packets netsim_rust_packets)
add_dependencies(cargo-build_hostapd-rs cargo-build_netsim-packets)
add_dependencies(cargo-build_netsim-daemon cargo-build_netsim-packets)

# cxx crates
if(WIN32)
  # 1. Set CMAKE_CXX_FLAGS, as it is not set in Rust crates imported by
  #    corrosion.
  # 2. Set RUST_CXX_NO_EXCEPTIONS preprocesor definition for cxx crate to disable
  #    compilation error message of "cannot use 'throw' with exceptions".
  # 3. MSVC uses environment variable "CL" to prepend arguments to the
  #    command-line arguments. However, we verified MSVC also supports CXXFLAGS,
  #    which is used by cc crate.
  # 4. This isn't needed in mac and Linux because corrosion doesn't pass
  #    -fno-exception flag.
  set(CXXFLAGS "${CMAKE_CXX_FLAGS} /DRUST_CXX_NO_EXCEPTIONS")
  corrosion_set_env_vars(netsim-daemon CXXFLAGS=${CXXFLAGS})
endif()

if(LINUX)
  # b/260271048
  set(RUSTFLAGS "-Clink-args=-Wl,-rpath,${CMAKE_LIBRARY_OUTPUT_DIRECTORY}")
  corrosion_add_target_rustflags(netsim-cli ${RUSTFLAGS})
  corrosion_add_target_rustflags(netsim-daemon ${RUSTFLAGS})
endif()

add_subdirectory(proto)

# To re-generate Rust protobuf files, follow these steps:
#
# 1. Read the file comments in scripts/proto_update.sh
# 2. Execute `scripts/proto_update.sh`

android_license(TARGET "netsim-daemon" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "netsim-proto" LIBNAME None SPDX None LICENSE None
                LOCAL None)

add_library(netsim-cli-rust-lib INTERFACE)

target_link_libraries(netsim-cli-rust-lib INTERFACE netsim-proto netsim-cli)

android_license(TARGET netsim-cli-rust-lib LIBNAME None SPDX None LICENSE None
                LOCAL None)
