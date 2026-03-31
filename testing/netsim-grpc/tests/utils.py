# Copyright 2020 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
import google.protobuf.text_format


def fmt_proto(msg: google.protobuf.Message) -> str:
  """Formats a `google.protobuf.Message` object as a string.

  Parameters:
      msg: A `google.protobuf.Message` object.

  Returns:
      A string representation of the `msg` object, with each line on a separate
      line.

  Raises:
      TypeError: If `msg` is not a `google.protobuf.Message` object.

  Example:

      >>> from google.protobuf import message
      >>> msg = message.Message()
      >>> msg.set_field1("value1")
      >>> msg.set_field2(123)
      >>> fmt_proto(msg)
      'field1: value1\nfield2: 123'
  """
  return google.protobuf.text_format.MessageToString(msg, as_one_line=True)
