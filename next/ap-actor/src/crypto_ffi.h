// Copyright 2025-2026 The Android Open Source Project

#pragma once

#include "rust/cxx.h"
#include <vector>

namespace netsim {
namespace hostap {

enum class DigestType : uint8_t {
    MD5,
    SHA1,
    SHA256,
    SHA384,
    SHA512
};

// Hmac
rust::Vec<uint8_t> Hmac(uint8_t digest_type, const rust::Vec<uint8_t>& key, const rust::Vec<uint8_t>& data);
// SHA Hash
rust::Vec<uint8_t> Sha(uint8_t digest_type, const rust::Vec<uint8_t>& data);
rust::Vec<uint8_t> AesWrap(const rust::Vec<uint8_t>& kek, const rust::Vec<uint8_t>& plain);
rust::Vec<uint8_t> AesUnwrap(const rust::Vec<uint8_t>& kek, const rust::Vec<uint8_t>& cipher);

bool AesCcmEncrypt(const rust::Vec<uint8_t>& key, const rust::Vec<uint8_t>& nonce,
                   const rust::Vec<uint8_t>& aad, const rust::Vec<uint8_t>& plain,
                   rust::Vec<uint8_t>& out_cipher, rust::Vec<uint8_t>& out_tag,
                   size_t tag_len);


bool AesCcmDecrypt(const rust::Vec<uint8_t>& key, const rust::Vec<uint8_t>& nonce,
                   const rust::Vec<uint8_t>& aad, const rust::Vec<uint8_t>& cipher,
                   const rust::Vec<uint8_t>& tag, rust::Vec<uint8_t>& out_plain);

rust::Vec<uint8_t> RandBytes(size_t len);

} // namespace hostap
} // namespace netsim
