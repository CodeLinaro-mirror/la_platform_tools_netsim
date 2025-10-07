exports_files(["netsimd.cc"])

genrule(
    name = "cxx-bridge-header",
    outs = ["cxx.h"],
    cmd = "$(location @cxx.rs//:codegen) --header >> $(OUTS)",
    tools = ["@cxx.rs//:codegen"],
    visibility = ["//:__pkg__"],
)
