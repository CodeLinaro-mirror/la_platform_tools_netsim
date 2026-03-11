load("@grpc-java//:java_grpc_library.bzl", "java_grpc_library")
load("@protobuf//bazel:cc_proto_library.bzl", "cc_proto_library")
load("@protobuf//bazel:java_proto_library.bzl", "java_proto_library")
load("@protobuf//bazel:proto_library.bzl", "proto_library")
load("@rules_cc//cc:defs.bzl", "cc_library")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "grpcio-compiler-lib",
    srcs = glob(
        [
            "grpcio-compiler/src/*.rs",
            "grpcio-compiler/src/**/*.rs",
        ],
        exclude = ["grpcio-compiler/src/bin/**"],
    ),
    crate_features = ["protobuf-codec"],
    crate_name = "grpcio_compiler",
    edition = "2018",
    deps = [
        "@anyhow",
        "@protobuf-codegen//:protobuf_codegen",
        "@protobuf-rust//:protobuf-rust",
    ],
)

rust_binary(
    name = "grpc_rust_plugin",
    srcs = ["grpcio-compiler/src/bin/grpc_rust_plugin.rs"],
    edition = "2018",
    deps = [
        ":grpcio-compiler-lib",
        "@protobuf-rust//:protobuf-rust",
    ],
)

rust_binary(
    name = "build_proto",
    srcs = ["build_proto.rs"],
    edition = "2021",
)

# Common
# Model
# Config
# Frontend
# HCI Packet
# Packet Streamer
# Startup
# Stats
# (Granular targets removed as they are unused)

# Combined target for C++ compatibility (legacy support)
proto_library(
    name = "netsim_proto_library",
    srcs = glob(["netsim/*.proto"]),
    strip_import_prefix = "/proto",
    deps = [
        "@protobuf//:empty_proto",
        "@protobuf//:timestamp_proto",
        "@rootcanal//:rootcanal-configuration-proto",
    ],
)

cc_proto_library(
    name = "netsim_cc_proto",
    deps = [":netsim_proto_library"],
)

java_proto_library(
    name = "netsim_java_proto",
    deps = [":netsim_proto_library"],
)

java_grpc_library(
    name = "netsim_java_grpc",
    srcs = [":netsim_proto_library"],
    deps = [":netsim_java_proto"],
)

filegroup(
    name = "netsim_proto_srcs",
    srcs = glob(["netsim/*.proto"]),
)

# GRPC Generation
genrule(
    name = "netsim_grpc_gen",
    srcs = [
        ":netsim_proto_srcs",
        "netsim/common.proto",
        "@rootcanal//:proto/rootcanal/configuration.proto",
        "@protobuf//:well_known_type_protos",
    ],
    outs = [
        "netsim/frontend_grpc.rs",
        "netsim/packet_streamer_grpc.rs",
        "netsim/access_point_grpc.rs",
    ],
    cmd = """
        NETSIM_PROTO_ROOT=$$(dirname $$(dirname $(location netsim/common.proto)))
        ROOTCANAL_PROTO_ROOT=$$(dirname $$(dirname $(location @rootcanal//:proto/rootcanal/configuration.proto)))
        PROTOBUF_ROOT=external/protobuf+/src

        $(location @protobuf//:protoc) \\
            --plugin=protoc-gen-grpc=$(location :grpc_rust_plugin) \\
            --grpc_out=$(@D) \\
            --proto_path=$$NETSIM_PROTO_ROOT \\
            --proto_path=$$ROOTCANAL_PROTO_ROOT \\
            --proto_path=$$PROTOBUF_ROOT \\
            $(locations :netsim_proto_srcs)
    """,
    tools = [
        ":grpc_rust_plugin",
        "@protobuf//:protoc",
    ],
)

# Proto Generation (Messages + Lib)
genrule(
    name = "netsim_proto_gen",
    srcs = [
        ":netsim_proto_srcs",
        "netsim/common.proto",
        "@rootcanal//:proto/rootcanal/configuration.proto",
        ":netsim_grpc_gen",
        "@protobuf//:well_known_type_protos",
    ],
    outs = [
        "netsim/common.rs",
        "netsim/config.rs",
        "netsim/frontend.rs",
        "netsim/hci_packet.rs",
        "netsim/model.rs",
        "netsim/packet_streamer.rs",
        "netsim/startup.rs",
        "netsim/stats.rs",
        "netsim/access_point.rs",
        "rootcanal/configuration.rs",
        "lib.rs",
    ],
    cmd = """
        NETSIM_PROTO_ROOT=$$(dirname $$(dirname $(location netsim/common.proto)))
        ROOTCANAL_PROTO_ROOT=$$(dirname $$(dirname $(location @rootcanal//:proto/rootcanal/configuration.proto)))
        PROTOBUF_ROOT=external/protobuf+/src

        $(location :build_proto) \\
            --protoc=$(location @protobuf//:protoc) \\
            --plugin=$(location @protobuf-codegen//:protoc-gen-rust) \\
            --out-dir=$(@D) \\
            --proto-path=$$NETSIM_PROTO_ROOT \\
            --proto-path=$$ROOTCANAL_PROTO_ROOT \\
            --proto-path=$$PROTOBUF_ROOT \\
            $(locations :netsim_proto_srcs) \\
            $(location @rootcanal//:proto/rootcanal/configuration.proto) \\
            --grpc-srcs $(locations :netsim_grpc_gen)
    """,
    tools = [
        ":build_proto",
        "@protobuf-codegen//:protoc-gen-rust",
        "@protobuf//:protoc",
    ],
)

rust_library(
    name = "proto",
    srcs = [
        "lib.rs",
        "netsim/access_point.rs",
        "netsim/access_point_grpc.rs",
        "netsim/common.rs",
        "netsim/config.rs",
        "netsim/frontend.rs",
        "netsim/frontend_grpc.rs",
        "netsim/hci_packet.rs",
        "netsim/model.rs",
        "netsim/packet_streamer.rs",
        "netsim/packet_streamer_grpc.rs",
        "netsim/startup.rs",
        "netsim/stats.rs",
        "rootcanal/configuration.rs",
    ],
    crate_name = "netsim_proto",
    edition = "2021",
    deps = [
        "@grpcio",
        "@protobuf-rust",
    ],
)

cc_library(
    name = "netsim_cc_grpc",
    srcs = [":netsim_cc_grpc_gen"],
    hdrs = [":netsim_cc_grpc_gen"],
    includes = ["."],
    deps = [
        ":netsim_cc_proto",
        "@grpc//:grpc++",
    ],
)

genrule(
    name = "netsim_cc_grpc_gen",
    srcs = [
        ":netsim_proto_srcs",
        "netsim/common.proto",
        "@protobuf//:well_known_type_protos",
        "@rootcanal//:proto/rootcanal/configuration.proto",
    ],
    outs = [
        "netsim/frontend.grpc.pb.h",
        "netsim/frontend.grpc.pb.cc",
        "netsim/packet_streamer.grpc.pb.h",
        "netsim/packet_streamer.grpc.pb.cc",
    ],
    cmd = """
        SRCS="netsim/frontend.proto netsim/packet_streamer.proto"

        # Calculate include paths
        # We need the root of the 'netsim' proto package
        NETSIM_PROTO_ROOT=$$(dirname $$(dirname $(location netsim/common.proto)))

        # We need the root of 'rootcanal' package
        ROOTCANAL_PROTO_ROOT=$$(dirname $$(dirname $(location @rootcanal//:proto/rootcanal/configuration.proto)))

        # Protobuf well known types root
        PROTOBUF_ROOT="external/protobuf+/src"

        for f in $$SRCS; do
            $(location @protobuf//:protoc) \\
                --plugin=protoc-gen-grpc=$(location @grpc//src/compiler:grpc_cpp_plugin) \\
                --grpc_out=$(@D) \\
                --proto_path=$$NETSIM_PROTO_ROOT \\
                --proto_path=$$ROOTCANAL_PROTO_ROOT \\
                --proto_path=$$PROTOBUF_ROOT \\
                $$NETSIM_PROTO_ROOT/$$f
        done
    """,
    tools = [
        "@grpc//src/compiler:grpc_cpp_plugin",
        "@protobuf//:protoc",
    ],
)
