// Copyright 2025-2026 The Android Open Source Project
#![allow(non_snake_case)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DigestType {
    MD5 = 0,
    SHA1 = 1,
    SHA256 = 2,
    SHA384 = 3,
    SHA512 = 4,
}

#[cxx::bridge(namespace = "netsim::hostap")]
mod hostap_ffi {
    // SAFETY:
    // 1. Necessity: We use C++ FFI because the `bssl-crypto` Rust crate (and
    //    `bssl-sys` bindings) currently lack complete support for:
    //    - Generic AES-CCM (Required for WPA/EAPOL). `bssl-crypto` only supports
    //      GCM.
    //    - AES Key Wrap (RFC 3394). `AES_wrap_key` is not exposed in safe wrappers.
    //    - Legacy HMAC-MD5/SHA1/SHA384. `bssl-crypto` only supports
    //      HMAC-SHA256/512.
    //    Rewriting these using raw `bssl-sys` FFI is redundant given the existing
    // C++ shim.
    // 2. Soundness:
    //    - The `cxx` crate handles ABI compatibility and type safety for `Vec<u8>`
    //      <-> `std::vector`.
    //    - The C++ implementation (`crypto_ffi.cc`) checks all buffer sizes and
    //      OpenSSL return values, returning empty vectors or `false` on failure
    //      rather than triggering undefined behavior.
    //    - No raw pointers are exposed or manipulated without length checks in the
    //      C++ shim.
    unsafe extern "C++" {
        include!("crypto_ffi.h");

        fn Hmac(digest_type: u8, key: &Vec<u8>, data: &Vec<u8>) -> Vec<u8>;
        fn Sha(digest_type: u8, data: &Vec<u8>) -> Vec<u8>;
        fn AesWrap(kek: &Vec<u8>, plain: &Vec<u8>) -> Vec<u8>;
        fn AesUnwrap(kek: &Vec<u8>, cipher: &Vec<u8>) -> Vec<u8>;
        fn AesCcmEncrypt(
            key: &Vec<u8>,
            nonce: &Vec<u8>,
            aad: &Vec<u8>,
            plain: &Vec<u8>,
            out_cipher: &mut Vec<u8>,
            out_tag: &mut Vec<u8>,
            tag_len: usize,
        ) -> bool;
        fn AesCcmDecrypt(
            key: &Vec<u8>,
            nonce: &Vec<u8>,
            aad: &Vec<u8>,
            cipher: &Vec<u8>,
            tag: &Vec<u8>,
            out_plain: &mut Vec<u8>,
        ) -> bool;
        fn RandBytes(len: usize) -> Vec<u8>;
        // PBKDF2
        fn Pbkdf2HmacSha1(
            password: &Vec<u8>,
            salt: &Vec<u8>,
            iterations: u32,
            key_len: usize,
        ) -> Vec<u8>;
        // SAE / ECC P-256
        fn EcP256CalculatePwe(
            password: &Vec<u8>,
            address1: &Vec<u8>,
            address2: &Vec<u8>,
        ) -> Vec<u8>;
        fn EcP256PointMul(point_src: &Vec<u8>, scalar: &Vec<u8>) -> Vec<u8>;
        fn EcP256PointAdd(point_a: &Vec<u8>, point_b: &Vec<u8>) -> Vec<u8>;
        fn BnModAdd(a: &Vec<u8>, b: &Vec<u8>, m: &Vec<u8>) -> Vec<u8>;
        fn BnModSub(a: &Vec<u8>, b: &Vec<u8>, m: &Vec<u8>) -> Vec<u8>;
    }
}

pub fn Hmac(digest_type: DigestType, key: &Vec<u8>, data: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::Hmac(digest_type as u8, key, data)
}

pub fn Sha(digest_type: DigestType, data: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::Sha(digest_type as u8, data)
}

pub fn AesWrap(kek: &Vec<u8>, plain: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::AesWrap(kek, plain)
}

pub fn AesUnwrap(kek: &Vec<u8>, cipher: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::AesUnwrap(kek, cipher)
}

pub fn AesCcmEncrypt(
    key: &Vec<u8>,
    nonce: &Vec<u8>,
    aad: &Vec<u8>,
    plain: &Vec<u8>,
    out_cipher: &mut Vec<u8>,
    out_tag: &mut Vec<u8>,
    tag_len: usize,
) -> bool {
    self::hostap_ffi::AesCcmEncrypt(key, nonce, aad, plain, out_cipher, out_tag, tag_len)
}

pub fn AesCcmDecrypt(
    key: &Vec<u8>,
    nonce: &Vec<u8>,
    aad: &Vec<u8>,
    cipher: &Vec<u8>,
    tag: &Vec<u8>,
    out_plain: &mut Vec<u8>,
) -> bool {
    self::hostap_ffi::AesCcmDecrypt(key, nonce, aad, cipher, tag, out_plain)
}

pub fn RandBytes(len: usize) -> Vec<u8> {
    self::hostap_ffi::RandBytes(len)
}

pub fn Pbkdf2HmacSha1(
    password: &Vec<u8>,
    salt: &Vec<u8>,
    iterations: u32,
    key_len: usize,
) -> Vec<u8> {
    self::hostap_ffi::Pbkdf2HmacSha1(password, salt, iterations, key_len)
}

pub fn EcP256CalculatePwe(password: &Vec<u8>, address1: &Vec<u8>, address2: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::EcP256CalculatePwe(password, address1, address2)
}

pub fn EcP256PointMul(point_src: &Vec<u8>, scalar: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::EcP256PointMul(point_src, scalar)
}

pub fn EcP256PointAdd(point_a: &Vec<u8>, point_b: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::EcP256PointAdd(point_a, point_b)
}

pub fn BnModAdd(a: &Vec<u8>, b: &Vec<u8>, m: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::BnModAdd(a, b, m)
}

pub fn BnModSub(a: &Vec<u8>, b: &Vec<u8>, m: &Vec<u8>) -> Vec<u8> {
    self::hostap_ffi::BnModSub(a, b, m)
}
