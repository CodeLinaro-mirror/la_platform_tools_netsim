#!/bin/bash
cargo build
export TARGET=target/cxxbridge/frontend-client-cxx/src
cp $TARGET/lib.rs.h cxx/frontend_client_cxx_generated.h
cp $TARGET/lib.rs.cc cxx/frontend_client_cxx_generated.cc
sh ../../scripts/format_code.sh
