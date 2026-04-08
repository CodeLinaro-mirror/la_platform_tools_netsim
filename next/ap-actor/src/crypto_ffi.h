// Copyright 2025-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#pragma once

#include <vector>

#include "cxx.h"

namespace netsim {
namespace hostap {

enum class DigestType : uint8_t { MD5, SHA1, SHA256, SHA384, SHA512 };

// Hmac
rust::Vec<uint8_t> Hmac(uint8_t digest_type, const rust::Vec<uint8_t> &key,
                        const rust::Vec<uint8_t> &data);
// SHA Hash
rust::Vec<uint8_t> Sha(uint8_t digest_type, const rust::Vec<uint8_t> &data);
rust::Vec<uint8_t> AesWrap(const rust::Vec<uint8_t> &kek,
                           const rust::Vec<uint8_t> &plain);
rust::Vec<uint8_t> AesUnwrap(const rust::Vec<uint8_t> &kek,
                             const rust::Vec<uint8_t> &cipher);

bool AesCcmEncrypt(const rust::Vec<uint8_t> &key,
                   const rust::Vec<uint8_t> &nonce,
                   const rust::Vec<uint8_t> &aad,
                   const rust::Vec<uint8_t> &plain,
                   rust::Vec<uint8_t> &out_cipher, rust::Vec<uint8_t> &out_tag,
                   size_t tag_len);

bool AesCcmDecrypt(const rust::Vec<uint8_t> &key,
                   const rust::Vec<uint8_t> &nonce,
                   const rust::Vec<uint8_t> &aad,
                   const rust::Vec<uint8_t> &cipher,
                   const rust::Vec<uint8_t> &tag,
                   rust::Vec<uint8_t> &out_plain);

// PBKDF2
rust::Vec<uint8_t> Pbkdf2HmacSha1(const rust::Vec<uint8_t> &password,
                                  const rust::Vec<uint8_t> &salt,
                                  uint32_t iterations, size_t key_len);

// SAE / ECC P-256 Primitives
rust::Vec<uint8_t> EcP256CalculatePwe(const rust::Vec<uint8_t> &password,
                                      const rust::Vec<uint8_t> &address1,
                                      const rust::Vec<uint8_t> &address2);

// Returns (x, y) concatenated (32 bytes each) or empty on failure
rust::Vec<uint8_t> EcP256PointMul(const rust::Vec<uint8_t> &point_src,
                                  const rust::Vec<uint8_t> &scalar);

// Returns (x, y) concatenated (32 bytes each) or empty on failure
rust::Vec<uint8_t> EcP256PointAdd(const rust::Vec<uint8_t> &point_a,
                                  const rust::Vec<uint8_t> &point_b);

// Returns (a + b) mod m, big-endian byte array
rust::Vec<uint8_t> BnModAdd(const rust::Vec<uint8_t> &a,
                            const rust::Vec<uint8_t> &b,
                            const rust::Vec<uint8_t> &m);

// Returns (a - b) mod m, big-endian byte array
rust::Vec<uint8_t> BnModSub(const rust::Vec<uint8_t> &a,
                            const rust::Vec<uint8_t> &b,
                            const rust::Vec<uint8_t> &m);

rust::Vec<uint8_t> RandBytes(size_t len);

}  // namespace hostap
}  // namespace netsim
