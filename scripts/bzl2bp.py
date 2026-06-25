# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

import os
import sys

soong_targets = []
CURRENT_REL_PATH = ""

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
NEXT_DIR = os.path.join(os.path.dirname(SCRIPT_DIR), "next")


EXACT_DEP_MAPPING = {
    "@protobuf-json-mapping": "libprotobuf_json_mapping",
    "@protobuf-rust": "libprotobuf",
    "//proto": "libnetsim_proto",
    "//rootcanal": "librootcanal",
    "@base64": "libbase64_rust",
    "@rootcanal//:lib_rootcanal_ffi": "lib_rootcanal_ffi",
    "@cxx.rs//:cxx": "libcxx",
    "@casimir//:casimir_lib": "libcasimir",
}

IGNORED_DEPS = {"slirp", "wifi-actor", "ap-actor", "@libglib", "ethernet-actor"}


def translate_dep(dep):
  for ignored in IGNORED_DEPS:
    if ignored in dep:
      return None

  if dep in EXACT_DEP_MAPPING:
    return EXACT_DEP_MAPPING[dep]

  if dep.startswith("@"):
    return "lib" + dep[1:].replace("-", "_")

  if dep.startswith("//next/"):
    if ":" in dep:
      base = dep.split(":")[-1]
    else:
      base = dep.split("/")[-1]
    # Avoid double netsim_next prefix if already there
    if base.startswith("netsim_next_"):
      return "lib" + base.replace("-", "_")
    return "libnetsim_next_" + base.replace("-", "_")

  if dep.startswith("//"):
    if ":" in dep:
      base = dep.split(":")[-1]
    else:
      base = dep.split("/")[-1]
    if base.startswith("netsim_next_"):
      return "lib" + base.replace("-", "_")
    return "libnetsim_next_" + base.replace("-", "_")

  if dep.startswith(":"):
    base = dep[1:]
    if base.startswith("netsim_next_"):
      return "lib" + base.replace("-", "_")
    return "libnetsim_next_" + base.replace("-", "_")

  return dep.replace("-", "_")


def transform_deps(deps):
  out = set()
  for d in deps:
    t = translate_dep(d)
    if t:
      out.add(t)
  return sorted(list(out))


def glob(include, exclude=None):
  import glob as pyglob
  import os as pyos

  out = []
  for pattern in include:
    if pattern.count("**") > 1:
      if pattern.endswith("**"):
        pattern = pattern + "/*"
      base_dir = pyos.path.join(NEXT_DIR, CURRENT_REL_PATH)
      matched = pyglob.glob(pyos.path.join(base_dir, pattern), recursive=True)
      for p in matched:
        if pyos.path.isfile(p):
          rel = pyos.path.relpath(p, base_dir)
          out.append(rel)
    else:
      out.append(pattern)
  return out


def select(items):
  if "//conditions:default" in items:
    return items["//conditions:default"]
  return list(items.values())[0]


def load(label, *args, **kwargs):
  pass


def package(*args, **kwargs):
  pass


def alias(*args, **kwargs):
  pass


def filegroup(*args, **kwargs):
  pass


def netsim_cc_library_static(*args, **kwargs):
  pass


def cc_library_static(*args, **kwargs):
  pass


def cc_library(*args, **kwargs):
  pass


def exports_files(*args, **kwargs):
  pass


def rust_test(*args, **kwargs):
  pass


def rust_test_suite(*args, **kwargs):
  pass


def rust_protobuf(*args, **kwargs):
  pass


def genrule(*args, **kwargs):
  pass


def cc_binary(*args, **kwargs):
  pass


def kt_android_library(*args, **kwargs):
  pass


def rust_cxx_bridge(*args, **kwargs):
  pass


def sh_test(*args, **kwargs):
  pass


def android_binary(*args, **kwargs):
  pass


def rust_clippy(*args, **kwargs):
  pass


def rust_doc(*args, **kwargs):
  pass


def rust_doc_test(*args, **kwargs):
  pass


def stripped_binaries(*args, **kwargs):
  pass


SHARED_LIB_OVERRIDES = {"lib_rootcanal_ffi"}
PLATFORM_RUSTLIBS = {
    "daemon": ["libcommand_fds"],
    "daemon-lib": [
        "libcommand_fds",
        "libbytes",
        "libserde_json",
        "libtokio_util",
        "libnetsim_next_rootcanal_server",
    ],
    "packet-stream": ["libcommand_fds"],
}
IGNORED_TARGETS = {
    "slirp-actor",
    "libslirp-rs",
    "ap-actor",
    "wifi-actor",
    "http-proxy",
    "ethernet-actor",
}


def netsim_rust_library(
    name,
    srcs=None,
    deps=None,
    proc_macro_deps=None,
    compile_data=None,
    crate_name=None,
    **kwargs,
):
  if name in IGNORED_TARGETS:
    return
  if deps is None:
    deps = []
  if proc_macro_deps is None:
    proc_macro_deps = []
  if compile_data is None:
    compile_data = []
  if srcs is None or type(srcs) is not list:
    srcs = ["src/**/*.rs"]
  if crate_name is None:
    crate_name = name.replace("-", "_")

  rustlibs = []
  shared_libs = []
  for lib in transform_deps(deps):
    if lib in SHARED_LIB_OVERRIDES:
      shared_libs.append(lib)
    else:
      rustlibs.append(lib)

  if name in PLATFORM_RUSTLIBS:
    rustlibs.extend(PLATFORM_RUSTLIBS[name])
  proc_macros = transform_deps(proc_macro_deps)
  srcs_content = resolve_rust_srcs(srcs, "lib.rs")
  data_content = resolve_rust_srcs(compile_data, "") if compile_data else []

  soong_targets.append({
      "type": "rust_library_host",
      "name": f"libnetsim_next_{name.replace('-', '_')}",
      "crate_name": crate_name,
      "stem": f"lib{crate_name}",
      "edition": kwargs.get("edition"),
      "crate_root": srcs_content[0],
      "srcs": srcs_content + data_content,
      "rustlibs": rustlibs,
      "shared_libs": shared_libs,
      "proc_macros": proc_macros,
      "features": ["cuttlefish"],
  })


def resolve_rust_srcs(srcs, default_file):
  mapped_srcs = []
  for src in srcs:
    if src == "//:netsim_link_layer_packets_rust_gen":
      mapped_srcs.append(":rootcanal_link_layer_packets_rust_gen")
    elif "**/*.rs" in src:
      mapped_srcs.append(src.replace("**/*.rs", default_file))
    elif "*.rs" in src:
      mapped_srcs.append(src.replace("*.rs", default_file))
    elif "main.rs" in src or "lib.rs" in src or not src.endswith(".rs"):
      mapped_srcs.append(src)
  return mapped_srcs


def netsim_rust_binary(name, srcs=None, deps=None, **kwargs):
  if deps is None:
    deps = []
  if srcs is None or type(srcs) is not list:
    srcs = ["src/**/*.rs"]

  rustlibs = transform_deps(deps)
  # Add platform-specific rustlibs
  if name in PLATFORM_RUSTLIBS:
    rustlibs.extend(PLATFORM_RUSTLIBS[name])

  srcs_content = resolve_rust_srcs(srcs, "main.rs")
  data_content = resolve_rust_srcs(kwargs.get("compile_data", []), "")

  soong_name = (
      "netsimd" if name == "daemon" else f"netsim_next_{name.replace('-', '_')}"
  )

  soong_targets.append({
      "type": "rust_binary_host",
      "name": soong_name,
      "stem": soong_name,
      "edition": kwargs.get("edition"),
      "crate_root": srcs_content[0],
      "srcs": srcs_content + data_content,
      "rustlibs": rustlibs,
      "features": ["cuttlefish"],
  })


def rust_proc_macro(name, srcs=None, deps=None, **kwargs):
  if deps is None:
    deps = []
  if srcs is None or type(srcs) is not list:
    srcs = ["src/**/*.rs"]

  rustlibs = transform_deps(deps)
  srcs_content = resolve_rust_srcs(srcs, "lib.rs")
  data_content = resolve_rust_srcs(kwargs.get("compile_data", []), "")

  soong_targets.append({
      "type": "rust_proc_macro",
      "name": f"libnetsim_next_{name.replace('-', '_')}",
      "crate_name": name.replace("-", "_"),
      "stem": f"lib{name.replace('-', '_')}",
      "edition": kwargs.get("edition"),
      "crate_root": srcs_content[0],
      "srcs": srcs_content + data_content,
      "rustlibs": rustlibs,
      "features": ["cuttlefish"],
  })


# Map rust_binary exactly to netsim_rust_binary
def rust_binary(*args, **kwargs):
  netsim_rust_binary(*args, **kwargs)


SANDBOX = {
    "glob": glob,
    "select": select,
    "load": load,
    "package": package,
    "alias": alias,
    "filegroup": filegroup,
    "netsim_cc_library_static": netsim_cc_library_static,
    "cc_library_static": cc_library_static,
    "cc_library": cc_library,
    "exports_files": exports_files,
    "rust_test": rust_test,
    "rust_test_suite": rust_test_suite,
    "rust_protobuf": rust_protobuf,
    "netsim_rust_library": netsim_rust_library,
    "netsim_rust_binary": netsim_rust_binary,
    "rust_binary": rust_binary,
    "rust_proc_macro": rust_proc_macro,
    "genrule": genrule,
    "cc_binary": cc_binary,
    "kt_android_library": kt_android_library,
    "rust_cxx_bridge": rust_cxx_bridge,
    "sh_test": sh_test,
    "android_binary": android_binary,
    "rust_clippy": rust_clippy,
    "rust_doc": rust_doc,
    "rust_doc_test": rust_doc_test,
    "stripped_binaries": stripped_binaries,
    "True": True,
    "False": False,
    "None": None,
}


def main():
  global CURRENT_REL_PATH
  repo_dir = NEXT_DIR
  for root, dirs, files in os.walk(repo_dir):
    if "BUILD" in files:
      path = os.path.join(root, "BUILD")
      rel_path = os.path.relpath(root, repo_dir)
      if rel_path == ".":
        rel_path = ""
      else:
        rel_path += "/"

      # Skip test and verification binaries for daemon MVP
      if "verify/" in rel_path or "testing/" in rel_path:
        continue

      global soong_targets
      soong_targets = []
      CURRENT_REL_PATH = rel_path
      code = open(path).read()

      try:
        exec(code, SANDBOX)
      except Exception as e:
        print(f"Error parsing {path}: {e}", file=sys.stderr)
        sys.exit(1)

      if soong_targets:
        with open(os.path.join(root, "Android.bp"), "w") as f:
          f.write("// This file is generated by bzl2bp.py. DO NOT EDIT.\n\n")
          f.write(
              "package {\n    default_applicable_licenses:"
              ' ["tools_netsim_license"],\n}\n\n'
          )

          for i, tgt in enumerate(soong_targets):
            f.write(f'{tgt["type"]} {{\n')
            f.write(f'    name: "{tgt["name"]}",\n')

            if "crate_name" in tgt:
              f.write(f'    crate_name: "{tgt["crate_name"]}",\n')

            if "edition" in tgt and tgt["edition"]:
              f.write(f'    edition: "{tgt["edition"]}",\n')

            if "stem" in tgt:
              f.write(f'    stem: "{tgt["stem"]}",\n')

            if "crate_root" in tgt:
              f.write(f'    crate_root: "{tgt["crate_root"]}",\n')

            f.write("    srcs: [\n")
            for src in tgt["srcs"]:
              f.write(f'        "{src}",\n')
            f.write("    ],\n")

            if tgt.get("data_empty"):
              pass

            f.write('    features: ["cuttlefish"],\n')
            f.write('    lints: "none",\n')

            if tgt.get("rustlibs"):
              f.write("    rustlibs: [\n")
              for lib in sorted(list(set(tgt["rustlibs"]))):
                f.write(f'        "{lib}",\n')
              f.write("    ],\n")
            if tgt.get("shared_libs"):
              f.write("    shared_libs: [\n")
              for lib in sorted(list(set(tgt["shared_libs"]))):
                f.write(f'        "{lib}",\n')
              f.write("    ],\n")

            if tgt.get("proc_macros"):
              f.write("    proc_macros: [\n")
              for mac in sorted(list(set(tgt["proc_macros"]))):
                f.write(f'        "{mac}",\n')
              f.write("    ],\n")

            if i == len(soong_targets) - 1:
              f.write("}\n")
            else:
              f.write("}\n\n")


if __name__ == "__main__":
  main()
