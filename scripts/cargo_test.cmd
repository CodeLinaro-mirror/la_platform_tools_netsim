@echo off
setlocal

:: Get the directory of the script
set REPO=%~dp0\..\..\..

:: Get the Rust version, package, and objs path from arguments
set RUST_PKG=%1
set OUT_PATH=%2
set RUST_VERSION=%3
set CLANG_VERSION=%4
set OBJS_PATH=%OUT_PATH%

:: Set environment variables
set PATH=%PATH%;%OUT_PATH%\lib64
set PATH=%PATH%;%REPO%\prebuilts\gcc\linux-x86\host\x86_64-w64-mingw32-4.8\x86_64-w64-mingw32\lib;%REPO%\prebuilts\gcc\linux-x86\host\x86_64-w64-mingw32-4.8\x86_64-w64-mingw32\bin
set CC_x86_64-pc-windows-gnu=%REPO%/prebuilts/clang/host/windows-x86/%CLANG_VERSION%/bin/clang-cl.exe
set HOST_CC=%REPO%/prebuilts/clang/host/windows-x86/%CLANG_VERSION%/bin/clang-cl.exe
set CXX_x86_64-pc-windows-gnu=%REPO%/prebuilts/clang/host/windows-x86/%CLANG_VERSION%/bin/clang-cl.exe
set HOST_CXX=%REPO%/prebuilts/clang/host/windows-x86/%CLANG_VERSION%/bin/clang-cl.exe
set AR_x86_64-pc-windows-gnu=%REPO%/prebuilts/clang/host/windows-x86/%CLANG_VERSION%/bin/llvm-ar
set CORROSION_BUILD_DIR=%OUT_PATH%/rust
set CARGO_BUILD_RUSTC=%REPO%/prebuilts/rust/windows-x86/%RUST_VERSION%/bin/rustc
set RUSTC=%REPO%/prebuilts/rust/windows-x86/%RUST_VERSION%/bin/rustc
set CARGO_HOME=%OUT_PATH%\rust\.cargo
set RUSTFLAGS=-Cdefault-linker-libraries=yes

:: Paths to pdl generated packets files
set PDL_PATH=%OUT_PATH%\pdl\pdl_gen
set MAC80211_HWSIM_PACKETS_PREBUILT=%PDL_PATH%\mac80211_hwsim_packets.rs
set IEEE80211_PACKETS_PREBUILT=%PDL_PATH%\ieee80211_packets.rs
set LLC_PACKETS_PREBUILT=%PDL_PATH%\llc_packets.rs
set NETLINK_PACKETS_PREBUILT=%PDL_PATH%\netlink_packets.rs

:: Run the cargo command
%REPO%\prebuilts\rust\windows-x86\%RUST_VERSION%\bin\cargo.exe test -vv --target=x86_64-pc-windows-gnu --config target.x86_64-pc-windows-gnu.linker='%OUT_PATH%\toolchain\ld-emu.cmd' --package %RUST_PKG% --manifest-path %REPO%\tools\netsim\rust\Cargo.toml --release -- --nocapture