#!/usr/bin/env python3
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

import os
import re
import sys

# Match the legacy Apache 2.0 license string gracefully regardless of wrapping
pat_license = re.compile(
    r"^([ \t]*)(//|#|\*|/\*)\s*Licensed under the Apache License, Version"
    r" 2\.0.*?limitations under.*?the License\.",
    re.DOTALL | re.MULTILINE,
)


def process_file(filepath):
  try:
    with open(filepath, "r", encoding="utf-8") as file:
      content = file.read()
  except UnicodeDecodeError:
    print(
        f"format_licenses: Skipping {filepath} (UnicodeDecodeError)",
        file=sys.stderr,
    )
    return

  orig = content

  # 1. Strip useless empty comment blocks at the top of the file
  # This fixes time_display.rs: `//\n// Copyright` -> `// Copyright`
  content = re.sub(
      r"^([ \t]*(?://|#)[ \t]*\n)+([ \t]*(?://|#)[" r" \t]*Copyright)",
      r"\2",
      content,
  )

  # Standard Apache 2.0 conversion
  content = pat_license.sub(
      r"\1\2 SPDX-License-Identifier: Apache-2.0", content
  )

  # Tightly pack the headers
  content = re.sub(
      r"^([ \t]*(?://|#|\*|/\*) Copyright.*?(?:The Android Open Source"
      r" Project|Google"
      r" LLC|Google, Inc\.))"
      r"\n(?:[ \t]*(?://|#|\*|/\*)[ \t]*\n)+"
      r"([ \t]*(?://|#|\*|/\*) SPDX-License)",
      lambda m: f"{m.group(1)}\n{m.group(2)}",
      content,
      flags=re.MULTILINE,
  )

  # Rename legacy generic entities to AOSP namespace
  content = re.sub(
      r"Copyright (\d{4}.*?) Google, Inc\.",
      r"Copyright \1 The Android Open Source Project",
      content,
  )
  content = re.sub(
      r"Copyright (\d{4}.*?) Google LLC",
      r"Copyright \1 The Android Open Source Project",
      content,
  )

  # Remove awkward top-level spacer if trailing whitespace
  content = re.sub(r"^\s*(//|#)\n([ \t]*\1 Copyright)", r"\2", content)

  # Auto-inject missing SPDX identifier if Copyright exists
  if "Copyright" in content and "SPDX-License-Identifier" not in content:
    if re.search(r"Licensed under the Apache License", content, re.IGNORECASE):
      print(
          "format_licenses: WARNING - Unstripped boilerplate anomaly in"
          f" {filepath}. Skipping injection.",
          file=sys.stderr,
      )
    else:
      match = re.search(
          r"^(?P<prefix>[ \t]*(?://|#|\*|/\*)[ \t]*)Copyright",
          content,
          re.MULTILINE,
      )
      if match:
        prefix = match.group("prefix")
        spdx_line = f"{prefix}SPDX-License-Identifier: Apache-2.0"
        content = re.sub(
            r"^([ \t]*(?://|#|\*|/\*)[ \t]*Copyright[^\n]*)$",
            lambda m: m.group(1) + "\n" + spdx_line,
            content,
            count=1,
            flags=re.MULTILINE,
        )

  # 2. Extract out-of-place headers and move them to absolute top
  # This fixes test_dual_fd_final.rs where header was inside `mod tests {`
  block_match = re.search(
      r"^([ \t]*((?://|#|\*|/\*)[ \t]*Copyright.*?\n(?:["
      r" \t]*(?://|#|\*|/\*).*?\n)*?[ \t]*(?://|#|\*|/\*)["
      r" \t]*SPDX-License-Identifier:[^\n]*\n))",
      content,
      re.MULTILINE,
  )
  if block_match:
    if block_match.start() > 0:
      # Prevent matching if it's already top or under shebang
      is_under_shebang = (
          content.startswith("#!")
          and block_match.start() == content.find("\n") + 1
      )
      prefix_str = content[: block_match.start()].strip()
      is_top_level = prefix_str == "" or prefix_str in ("/*", "/**")

      if not is_under_shebang and not is_top_level:
        block = block_match.group(1)
        content = content[: block_match.start()] + content[block_match.end() :]

        clean_block = ""
        for line in block.splitlines(True):
          clean_block += re.sub(r"^[ \t]+", "", line)

        clean_block = clean_block.rstrip() + "\n\n"
        content = content.lstrip("\n")  # Remove extra newlines at start

        if content.startswith("#!"):
          parts = content.split("\n", 1)
          if len(parts) > 1:
            content = parts[0] + "\n" + clean_block + parts[1].lstrip("\n")
          else:
            content = parts[0] + "\n" + clean_block
        else:
          content = clean_block + content

  # 3. Inject full standard block into completely header-less files
  # This fixes lifecycle_test.rs which had no header whatsoever.
  has_copyright = re.search(
      r"^[ \t]*(#|//|\*|/\*)[ \t]*Copyright", content, re.MULTILINE
  )
  if not has_copyright and "SPDX-License-Identifier" not in content:
    basename = os.path.basename(filepath)
    ext = os.path.splitext(filepath)[1]
    if ext in [".rs", ".java", ".ts", ".cc", ".h", ".kt", ".proto"]:
      prefix = "//"
    elif ext in [".py", ".toml", ".sh", ".bzl"]:
      prefix = "#"
    elif basename.startswith("BUILD") or basename == "CMakeLists.txt":
      prefix = "#"
    else:
      prefix = "//"  # Default

    header = (
        f"{prefix} Copyright 2026 The Android Open Source Project\n{prefix}"
        " SPDX-License-Identifier: Apache-2.0\n\n"
    )
    content = content.lstrip("\n")  # Remove extra newlines at start
    if content.startswith("#!"):
      parts = content.split("\n", 1)
      if len(parts) > 1:
        content = parts[0] + "\n" + header + parts[1].lstrip("\n")
      else:
        content = parts[0] + "\n" + header
    else:
      content = header + content

  if content != orig:
    with open(filepath, "w", encoding="utf-8") as file:
      file.write(content)
    print(f"format_licenses: Fixed {filepath}")


def main():
  if len(sys.argv) < 2:
    return

  for filepath in sys.argv[1:]:
    if os.path.isfile(filepath):
      process_file(filepath)


if __name__ == "__main__":
  main()
