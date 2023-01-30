#!/bin/bash

cargo build
export TARGET=target/cxxbridge/backend-cxx/src/
cp $TARGET/lib.rs.h cxx/backend_cxx_generated.h
cp $TARGET/lib.rs.cc cxx/backend_cxx_generated.cc
sh ../../scripts/format_code.sh
