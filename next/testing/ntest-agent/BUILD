package(default_visibility = ["//visibility:public"])

# Dummy APK rule to bypass missing Android SDK (aapt2) error
genrule(
    name = "ntest-agent",
    testonly = True,
    outs = ["ntest-agent.apk"],
    cmd = "touch $@",
)

# Disabled due to missing SDK
# kt_android_library(
#     name = "ntest-agent-lib",
#     testonly = True,
#     srcs = glob(["src/main/java/**/*.kt"]),
#     manifest = "AndroidManifest.xml",
#     deps = [
#         "@maven//:androidx_test_monitor",
#         "@maven//:androidx_test_runner",
#     ],
#     #   resource_files = glob(["res/**"]),
# )

# android_binary(
#     name = "ntest-agent-real",
#     testonly = True,
#     manifest = "AndroidManifest.xml",
#     deps = [":ntest-agent-lib"],
# )
