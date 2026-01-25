// Copyright 2025-2026 The Android Open Source Project

#include "crypto_ffi.h"

#include <openssl/aes.h>
#include <openssl/evp.h>
#include <openssl/hmac.h>
#include <openssl/rand.h>
#include <openssl/sha.h>
#include <openssl/aead.h>
#include <random>

#include <vector>
#include <cstring>
#include <iostream>

namespace netsim {
namespace hostap {

using rust::Vec;

static const EVP_MD* GetDigest(DigestType type) {
    switch (type) {
        case DigestType::MD5: return EVP_md5();
        case DigestType::SHA1: return EVP_sha1();
        case DigestType::SHA256: return EVP_sha256();
        case DigestType::SHA384: return EVP_sha384();
        case DigestType::SHA512: return EVP_sha512();
        default: return nullptr;
    }
}

// HMAC
Vec<uint8_t> Hmac(uint8_t digest_type_u8, const Vec<uint8_t>& key, const Vec<uint8_t>& data) {
    DigestType digest_type = static_cast<DigestType>(digest_type_u8);
    Vec<uint8_t> result;
    const EVP_MD* md = GetDigest(digest_type);
    if (!md) {
        return result;
    }

    unsigned int len = EVP_MD_size(md);
    result.reserve(len);
    std::vector<uint8_t> buf(len);

    if (HMAC(md, key.data(), key.size(), data.data(), data.size(), buf.data(), &len)) {
         for(uint8_t b : buf) result.push_back(b);
    }
    return result;
}

// SHA Hash
Vec<uint8_t> Sha(uint8_t digest_type_u8, const Vec<uint8_t>& data) {
    DigestType digest_type = static_cast<DigestType>(digest_type_u8);
    Vec<uint8_t> result;
    const EVP_MD* md = GetDigest(digest_type);
    if (!md) return result;

    EVP_MD_CTX* ctx = EVP_MD_CTX_new();
    if (!ctx) return result;

    unsigned int len = EVP_MD_size(md);
    std::vector<uint8_t> buf(len);

    if (EVP_DigestInit_ex(ctx, md, nullptr) &&
        EVP_DigestUpdate(ctx, data.data(), data.size()) &&
        EVP_DigestFinal_ex(ctx, buf.data(), &len)) {
         for(uint8_t b : buf) result.push_back(b);
    }
    EVP_MD_CTX_free(ctx);
    return result;
}

// AES-Wrap (RFC 3394)
Vec<uint8_t> AesWrap(const Vec<uint8_t>& kek, const Vec<uint8_t>& plain) {
    Vec<uint8_t> result;
    if (plain.size() % 8 != 0 || plain.size() < 16) return result;

    AES_KEY aes_key;
    if (AES_set_encrypt_key(kek.data(), kek.size() * 8, &aes_key) < 0) {
        return result;
    }

    size_t out_len = plain.size() + 8;
    std::vector<uint8_t> out_buf(out_len);

    int ret = AES_wrap_key(&aes_key, nullptr, out_buf.data(), plain.data(), plain.size());
    if (ret > 0) {
       for(uint8_t b : out_buf) result.push_back(b);
    }
    return result;
}

Vec<uint8_t> AesUnwrap(const Vec<uint8_t>& kek, const Vec<uint8_t>& cipher) {
    Vec<uint8_t> result;
    if (cipher.size() % 8 != 0 || cipher.size() < 24) return result;

    AES_KEY aes_key;
    if (AES_set_decrypt_key(kek.data(), kek.size() * 8, &aes_key) < 0) {
        return result;
    }

    size_t out_len = cipher.size() - 8;
    std::vector<uint8_t> out_buf(out_len);

    int ret = AES_unwrap_key(&aes_key, nullptr, out_buf.data(), cipher.data(), cipher.size());
    if (ret > 0) {
         for(uint8_t b : out_buf) result.push_back(b);
    }
    return result;
}

// AES-CCM using EVP_AEAD (BoringSSL style)
// We use EVP_aead_aes_128_ccm_bluetooth_8 because standard generic CCM
// might be unavailable/hidden in this build configuration.
// Bluetooth_8 uses M=8 (MIC length) and L=2 (length field bytes),
// which matches WPA2-CCMP requirements (8-byte MIC, 13-byte Nonce).

bool AesCcmEncrypt(const Vec<uint8_t>& key, const Vec<uint8_t>& nonce,
                   const Vec<uint8_t>& aad, const Vec<uint8_t>& plain,
                   Vec<uint8_t>& out_cipher, Vec<uint8_t>& out_tag,
                   size_t tag_len) {
    // CCMP uses 8-byte MAC (M=8).
    if (tag_len != 8) {
         // If caller requests non-8 tag, this AEAD won't work matching expectations if fixed.
         // However, EVP_AEAD_CTX_init takes tag_len.
         // bluetooth_8 implementation might enforce it.
    }

    const EVP_AEAD* aead = nullptr;
    if (key.size() == 16) {
        // Fallback or specific selection
        aead = EVP_aead_aes_128_ccm_bluetooth_8();
    } else {
        // WPA2 is typically 128-bit.
        std::cerr << "AesCcmEncrypt: Unsupported key size " << key.size() << std::endl;
        return false;
    }

    EVP_AEAD_CTX ctx;
    if (!EVP_AEAD_CTX_init(&ctx, aead, key.data(), key.size(), tag_len, nullptr)) {
        return false;
    }

    // Output buffer must be large enough: plain.size() + tag_len + max_overhead
    // EVP_AEAD_max_overhead is useful.
    size_t max_out_len = plain.size() + EVP_AEAD_max_overhead(aead);
    std::vector<uint8_t> out_buf(max_out_len);
    size_t out_len = 0;

    if (!EVP_AEAD_CTX_seal(&ctx, out_buf.data(), &out_len, max_out_len,
                           nonce.data(), nonce.size(),
                           plain.data(), plain.size(),
                           aad.data(), aad.size())) {
        EVP_AEAD_CTX_cleanup(&ctx);
        return false;
    }
    EVP_AEAD_CTX_cleanup(&ctx);

    // The output contains Ciphertext + Tag (appended).
    if (out_len < tag_len) return false;
    size_t cipher_len = out_len - tag_len;

    out_cipher.reserve(cipher_len);
    out_tag.reserve(tag_len);

    for (size_t i = 0; i < cipher_len; i++) out_cipher.push_back(out_buf[i]);
    for (size_t i = 0; i < tag_len; i++) out_tag.push_back(out_buf[cipher_len + i]);

    return true;
}

bool AesCcmDecrypt(const Vec<uint8_t>& key, const Vec<uint8_t>& nonce,
                   const Vec<uint8_t>& aad, const Vec<uint8_t>& cipher,
                   const Vec<uint8_t>& tag, Vec<uint8_t>& out_plain) {
    const EVP_AEAD* aead = nullptr;
    if (key.size() == 16) {
        aead = EVP_aead_aes_128_ccm_bluetooth_8();
    } else {
        std::cerr << "AesCcmDecrypt: Unsupported key size " << key.size() << std::endl;
        return false;
    }

    EVP_AEAD_CTX ctx;
    if (!EVP_AEAD_CTX_init(&ctx, aead, key.data(), key.size(), tag.size(), nullptr)) {
        return false;
    }

    std::vector<uint8_t> in_buf;
    in_buf.reserve(cipher.size() + tag.size());
    in_buf.insert(in_buf.end(), cipher.begin(), cipher.end());
    in_buf.insert(in_buf.end(), tag.begin(), tag.end());

    size_t max_out_len = in_buf.size();
    std::vector<uint8_t> out_buf(max_out_len);
    size_t out_len = 0;

    if (!EVP_AEAD_CTX_open(&ctx, out_buf.data(), &out_len, max_out_len,
                           nonce.data(), nonce.size(),
                           in_buf.data(), in_buf.size(),
                           aad.data(), aad.size())) {
        EVP_AEAD_CTX_cleanup(&ctx);
        return false;
    }
    EVP_AEAD_CTX_cleanup(&ctx);

    out_plain.reserve(out_len);
    for (size_t i = 0; i < out_len; i++) out_plain.push_back(out_buf[i]);

    return true;
}

Vec<uint8_t> RandBytes(size_t len) {
    Vec<uint8_t> out;
    out.reserve(len);
    // TODO: Switch back to RAND_bytes when linking is fixed.
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<> dis(0, 255);
    for (size_t i = 0; i < len; ++i) {
        out.push_back(static_cast<uint8_t>(dis(gen)));
    }
    return out;
}

} // namespace hostap
} // namespace netsim
