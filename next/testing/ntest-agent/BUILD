load("@rules_android//rules:rules.bzl", "android_binary")
load("@rules_kotlin//kotlin:android.bzl", "kt_android_library")

package(default_visibility = ["//visibility:public"])

kt_android_library(
    name = "ntest-agent-lib",
    testonly = True,
    srcs = glob(["src/main/java/**/*.kt"]),
    manifest = "AndroidManifest.xml",
    tags = ["manual"],
    deps = [
        "@maven//:androidx_test_monitor",
        "@maven//:androidx_test_runner",
    ],
)

android_binary(
    name = "ntest-agent",
    testonly = True,
    manifest = "AndroidManifest.xml",
    tags = ["manual"],
    deps = [":ntest-agent-lib"],
)
