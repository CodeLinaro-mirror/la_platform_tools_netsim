// Copyright 2025-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#pragma once

#include <vector>

#include "rust/cxx.h"

namespace netsim {
namespace hostap {

enum class DigestType : uint8_t { MD5, SHA1, SHA256, SHA384, SHA512 };

// Hmac
rust::Vec<uint8_t> Hmac(uint8_t digest_type, rust::Slice<const uint8_t> key,
                        rust::Slice<const uint8_t> data);
// SHA Hash
rust::Vec<uint8_t> Sha(uint8_t digest_type, rust::Slice<const uint8_t> data);
rust::Vec<uint8_t> AesWrap(rust::Slice<const uint8_t> kek,
                           rust::Slice<const uint8_t> plain);
rust::Vec<uint8_t> AesUnwrap(rust::Slice<const uint8_t> kek,
                             rust::Slice<const uint8_t> cipher);

bool AesCcmEncrypt(rust::Slice<const uint8_t> key,
                   rust::Slice<const uint8_t> nonce,
                   rust::Slice<const uint8_t> aad,
                   rust::Slice<const uint8_t> plain,
                   rust::Vec<uint8_t> &out_data, size_t tag_len);

bool AesCcmDecrypt(rust::Slice<const uint8_t> key,
                   rust::Slice<const uint8_t> nonce,
                   rust::Slice<const uint8_t> aad,
                   rust::Slice<const uint8_t> cipher_with_tag,
                   rust::Vec<uint8_t> &out_plain, size_t tag_len);

// PBKDF2
rust::Vec<uint8_t> Pbkdf2HmacSha1(rust::Slice<const uint8_t> password,
                                  rust::Slice<const uint8_t> salt,
                                  uint32_t iterations, size_t key_len);

// SAE / ECC P-256 Primitives
rust::Vec<uint8_t> EcP256CalculatePwe(rust::Slice<const uint8_t> password,
                                      rust::Slice<const uint8_t> address1,
                                      rust::Slice<const uint8_t> address2);

// Returns (x, y) concatenated (32 bytes each) or empty on failure
rust::Vec<uint8_t> EcP256PointMul(rust::Slice<const uint8_t> point_src,
                                  rust::Slice<const uint8_t> scalar);

// Returns (x, y) concatenated (32 bytes each) or empty on failure
rust::Vec<uint8_t> EcP256PointAdd(rust::Slice<const uint8_t> point_a,
                                  rust::Slice<const uint8_t> point_b);

// Returns (a + b) mod m, big-endian byte array
rust::Vec<uint8_t> BnModAdd(rust::Slice<const uint8_t> a,
                            rust::Slice<const uint8_t> b,
                            rust::Slice<const uint8_t> m);

// Returns (a - b) mod m, big-endian byte array
rust::Vec<uint8_t> BnModSub(rust::Slice<const uint8_t> a,
                            rust::Slice<const uint8_t> b,
                            rust::Slice<const uint8_t> m);

rust::Vec<uint8_t> RandBytes(size_t len);

}  // namespace hostap
}  // namespace netsim
