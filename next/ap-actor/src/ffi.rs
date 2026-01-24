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
mod ffi {
    // SAFETY:
    // 1. Necessity: We use C++ FFI because the `bssl-crypto` Rust crate (and `bssl-sys` bindings)
    //    currently lack complete support for:
    //    - Generic AES-CCM (Required for WPA/EAPOL). `bssl-crypto` only supports GCM.
    //    - AES Key Wrap (RFC 3394). `AES_wrap_key` is not exposed in safe wrappers.
    //    - Legacy HMAC-MD5/SHA1/SHA384. `bssl-crypto` only supports HMAC-SHA256/512.
    //    Rewriting these using raw `bssl-sys` FFI is redundant given the existing C++ shim.
    // 2. Soundness:
    //    - The `cxx` crate handles ABI compatibility and type safety for `Vec<u8>` <-> `std::vector`.
    //    - The C++ implementation (`crypto_ffi.cc`) checks all buffer sizes and OpenSSL return values,
    //      returning empty vectors or `false` on failure rather than triggering undefined behavior.
    //    - No raw pointers are exposed or manipulated without length checks in the C++ shim.
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
    }
}

pub fn Hmac(digest_type: DigestType, key: &Vec<u8>, data: &Vec<u8>) -> Vec<u8> {
    ffi::Hmac(digest_type as u8, key, data)
}

pub fn Sha(digest_type: DigestType, data: &Vec<u8>) -> Vec<u8> {
    ffi::Sha(digest_type as u8, data)
}

pub fn AesWrap(kek: &Vec<u8>, plain: &Vec<u8>) -> Vec<u8> {
    ffi::AesWrap(kek, plain)
}

pub fn AesUnwrap(kek: &Vec<u8>, cipher: &Vec<u8>) -> Vec<u8> {
    ffi::AesUnwrap(kek, cipher)
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
    ffi::AesCcmEncrypt(key, nonce, aad, plain, out_cipher, out_tag, tag_len)
}

pub fn AesCcmDecrypt(
    key: &Vec<u8>,
    nonce: &Vec<u8>,
    aad: &Vec<u8>,
    cipher: &Vec<u8>,
    tag: &Vec<u8>,
    out_plain: &mut Vec<u8>,
) -> bool {
    ffi::AesCcmDecrypt(key, nonce, aad, cipher, tag, out_plain)
}

pub fn RandBytes(len: usize) -> Vec<u8> {
    ffi::RandBytes(len)
}
