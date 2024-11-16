@echo off
setlocal

:: Get the directory of the script
set REPO=%~dp0\..\..\..

:: Get the Rust version, package, and objs path from arguments
set RUST_VERSION=%1
set RUST_PKG=%2
set OUT_PATH=%3

:: Set environment variables
set PATH=%PATH%;%OUT_PATH%\lib64
set CARGO_HOME=%OUT_PATH%\rust\.cargo
:: Paths to pdl generated packets files
set PDL_PATH=%OUT_PATH%\pdl\pdl_gen
set MAC80211_HWSIM_PACKETS_PREBUILT=%PDL_PATH%\mac80211_hwsim_packets.rs
set IEEE80211_PACKETS_PREBUILT=%PDL_PATH%\ieee80211_packets.rs
set LLC_PACKETS_PREBUILT=%PDL_PATH%\llc_packets.rs
set NETLINK_PACKETS_PREBUILT=%PDL_PATH%\netlink_packets.rs

:: Build the package
cmake --build %OUT_PATH% %RUST_PKG%

:: Run the cargo command
cargo.exe test -vv --package %RUST_PKG% --manifest-path %REPO%\tools\netsim\rust\Cargo.toml