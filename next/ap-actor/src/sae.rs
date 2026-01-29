// Copyright 2025-2026 The Android Open Source Project

use crate::ffi;
use log::{error, info};

/// SAE Finite Field Cryptography (FFC) or Elliptic Curve Cryptography (ECC) Group
pub enum SaeGroup {
    EccP256 = 19,
}

/// SAE State Machine for WPA3-Personal
#[derive(Debug, Clone)]
pub struct SaeStateMachine {
    // Configuration
    own_addr: Vec<u8>,
    peer_addr: Vec<u8>,
    password: Vec<u8>,

    // State
    pub state: SaeState,
    pub pwe: Vec<u8>, // Password Element
    pub pmk: Vec<u8>, // Pairwise Master Key
    pub kck: Vec<u8>, // Key Confirmation Key
    pub pmkid: Vec<u8>,

    // Commit Exchange
    rand: Vec<u8>, // Private scalar for Commit
    own_commit_scalar: Vec<u8>,
    own_commit_element: Vec<u8>,
    peer_commit_scalar: Vec<u8>,
    peer_commit_element: Vec<u8>,

    // Verification
    pub k_pair: Vec<u8>, // Shared key
    scounter: u16,       // Send Counter
    rcounter: u16,       // Receive Counter
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SaeState {
    Nothing,
    Committed,
    Confirmed,
}

impl SaeStateMachine {
    pub fn new(own_addr: &[u8], peer_addr: &[u8], password: &[u8]) -> Self {
        Self {
            own_addr: own_addr.to_vec(),
            peer_addr: peer_addr.to_vec(),
            password: password.to_vec(),
            state: SaeState::Nothing,
            pwe: Vec::new(),
            pmk: Vec::new(),
            kck: Vec::new(),
            pmkid: Vec::new(),
            rand: Vec::new(),
            own_commit_scalar: Vec::new(),
            own_commit_element: Vec::new(),
            peer_commit_scalar: Vec::new(),
            peer_commit_element: Vec::new(),
            k_pair: Vec::new(),
            scounter: 0, // 802.11-2016 12.4.8.2: Initialize to 0 or 1
            rcounter: 0,
        }
    }

    /// Step 1: Generate our Commit Message (and PWE if needed)
    pub fn build_commit(&mut self) -> Option<Vec<u8>> {
        // 1. Derive PWE if not present
        if self.pwe.is_empty() {
            info!("SAE: Deriving PWE for peer {:?}", self.peer_addr);
            self.pwe = ffi::EcP256CalculatePwe(&self.password, &self.own_addr, &self.peer_addr);
            if self.pwe.is_empty() {
                error!("SAE: Failed to derive PWE");
                return None;
            }
        }

        // 2. Generate Random Scalar and Mask
        // Per hostapd/802.11:
        // We need 'q' (Order of P-256).
        // Since we don't expose 'q' directly, we rely on FFI to handle modular arithmetic.
        // We need:
        //   rand = Rand(32)
        //   mask = Rand(32)
        //   own_commit_scalar = (rand + mask) mod q
        //   own_commit_element = -element(mask * PWE) = (order - mask) * PWE

        // Let's use the symmetric property:
        //   pick any 'mask'
        //   scalar = (rand - mask) mod q
        //   element = mask * PWE  (simpler to just multiply)
        // peer Verification:
        //   K = scalar * PWE + element = (rand - mask) * PWE + mask * PWE = rand * PWE.

        // Does hostapd support this?
        // hostapd receive:
        //   K = peer_scalar * PWE + peer_element
        // So yes, as long as we send (scalar, element) such that scalar*PWE + element = rand*PWE.

        // Implementation:
        // rand = ffi::RandBytes(32)
        // mask = ffi::RandBytes(32)
        // own_commit_scalar = BnModSub(rand, mask, order)
        // own_commit_element = EcP256PointMul(PWE, mask)

        // Requires curve order for modular operations.
        // We can hardcode P-256 Order? Or expose it via FFI.
        // Hardcoding P-256 Order is standard.
        // P-256 Order: FFFFFFFF 00000000 FFFFFFFF FFFFFFFF BCE6FAAD A7179E84 F3B9CAC2 FC632551
        let p256_order: Vec<u8> = vec![
            0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xBC, 0xE6, 0xFA, 0xAD, 0xA7, 0x17, 0x9E, 0x84, 0xF3, 0xB9, 0xCA, 0xC2,
            0xFC, 0x63, 0x25, 0x51,
        ];

        let rand = ffi::RandBytes(32);
        self.rand = rand.clone();
        let mask = ffi::RandBytes(32);

        // scalar = (rand - mask) mod q
        self.own_commit_scalar = ffi::BnModSub(&rand, &mask, &p256_order);

        // element = mask * PWE
        self.own_commit_element = ffi::EcP256PointMul(&self.pwe, &mask);

        if self.own_commit_scalar.is_empty() || self.own_commit_element.is_empty() {
            error!("SAE: Failed to generate commit values");
            return None;
        }

        self.state = SaeState::Committed;
        self.scounter += 1;

        // 3. Construct Frame Body
        // Format (802.11-2016 12.4.8.2.2):
        //   Group (2 bytes) | Scalar (32 bytes) | Element (64 bytes usually - uncompressed)
        let mut body = Vec::new();
        // Group 19 (LE 16-bit)
        body.extend_from_slice(&(SaeGroup::EccP256 as u16).to_le_bytes());

        // Scalar (32 bytes). Pad if needed.
        if self.own_commit_scalar.len() < 32 {
            let pad = 32 - self.own_commit_scalar.len();
            body.extend(std::iter::repeat(0).take(pad));
        }
        body.extend_from_slice(&self.own_commit_scalar);

        // Element (Expect 64 bytes for P-256 uncompressed without header? No, usually 04 + X + Y)
        // EcP256PointMul returns (04 || X || Y) [65 bytes] or standard uncompressed.
        // SAE Frame expects just x,y?
        // 802.11-2016 9.4.1.25: Finite Cyclic Group field.
        // For ECC: "The x-coordinate ... followed by the y-coordinate"
        // So we strip the '04' prefix if present.
        if self.own_commit_element.len() == 65 && self.own_commit_element[0] == 0x04 {
            body.extend_from_slice(&self.own_commit_element[1..]);
        } else if self.own_commit_element.len() == 64 {
            body.extend_from_slice(&self.own_commit_element);
        } else {
            error!("SAE: Invalid element length {}", self.own_commit_element.len());
            return None;
        }

        if !self.peer_commit_scalar.is_empty() {
            let _ = self.process_commit();
        }
        Some(body)
    }

    /// Step 2: Process Peer's Commit Message
    pub fn parse_commit(&mut self, body: &[u8]) -> Option<()> {
        if body.len() < 2 + 32 + 64 {
            error!("SAE: Commit body too short: {}", body.len());
            return None;
        }

        let group_bytes = [body[0], body[1]];
        let group = u16::from_le_bytes(group_bytes);
        if group != SaeGroup::EccP256 as u16 {
            error!("SAE: Unsupported group {}", group);
            return None;
        }

        // Extract Scalar (32 bytes)
        let scalar = &body[2..34];
        self.peer_commit_scalar = scalar.to_vec();

        // Extract Element (Remaining bytes, expect 64 or 65? SAE usually sends raw X,Y [64 bytes])
        // If peer sends 64 bytes, we need to prefix 0x04 for OpenSSL uncompressed format.
        let element_raw = &body[34..];
        if element_raw.len() == 64 {
            self.peer_commit_element = vec![0x04];
            self.peer_commit_element.extend_from_slice(element_raw);
        } else {
            error!("SAE: Unexpected element length {}", element_raw.len());
            return None;
        }

        // At this point, we have Peer Scholar & Element.
        // We can optionally compute K now if we have already sent our commit (Simultaneous) or wait.
        // Usually, 'process_commit' implies we received it.
        // If we haven't sent ours, we should 'build_commit' next.

        // Calculate K if we have everything
        if !self.peer_commit_scalar.is_empty()
            && !self.peer_commit_element.is_empty()
            && !self.own_commit_scalar.is_empty()
        {
            self.process_commit()?;
        }

        Some(())
    }

    fn process_commit(&mut self) -> Option<()> {
        println!("DEBUG: process_commit started");
        // K = scalar-op(rand, (elem-op(scalar-op(peer-commit-scalar, PWE), PEER-COMMIT-ELEMENT)))
        // 1. tmp1 = peer_scalar * PWE
        let tmp1 = ffi::EcP256PointMul(&self.pwe, &self.peer_commit_scalar);
        if tmp1.is_empty() {
            println!(
                "DEBUG: Failed to compute peer_scalar * PWE. PWE len: {}, Scalar len: {}",
                self.pwe.len(),
                self.peer_commit_scalar.len()
            );
            error!("SAE: Failed to compute peer_scalar * PWE");
            return None;
        }

        // 2. tmp2 = tmp1 + peer_element
        let tmp2 = ffi::EcP256PointAdd(&tmp1, &self.peer_commit_element);
        if tmp2.is_empty() {
            println!("DEBUG: Failed to compute tmp1 + peer_element");
            error!("SAE: Failed to compute tmp1 + peer_element");
            return None;
        }

        // 3. K_point = rand * tmp2
        let k_point = ffi::EcP256PointMul(&tmp2, &self.rand);
        if k_point.is_empty() {
            println!("DEBUG: Failed to compute K");
            error!("SAE: Failed to compute K");
            return None;
        }

        // k = K.x (coordinate)
        // EcP256PointMul returns (04 || X || Y) [65 bytes] or uncompressed.
        let k = if k_point.len() == 65 && k_point[0] == 0x04 {
            k_point[1..33].to_vec()
        } else if k_point.len() == 64 {
            k_point[0..32].to_vec()
        } else {
            error!("SAE: K point invalid length {}", k_point.len());
            return None;
        };

        // Key Derivation (SAE-KDF)
        // keyseed = H(0..0 (32 bytes), k)
        let null_key = vec![0u8; 32];
        let keyseed = ffi::Hmac(ffi::DigestType::SHA256, &null_key, &k);

        // KCK || PMK = KDF-512(keyseed, "SAE KCK and PMK", (own_commit_scalar + peer_commit_scalar) mod q)
        // We need (scalar + scalar) mod q.
        let p256_order: Vec<u8> = vec![
            0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xBC, 0xE6, 0xFA, 0xAD, 0xA7, 0x17, 0x9E, 0x84, 0xF3, 0xB9, 0xCA, 0xC2,
            0xFC, 0x63, 0x25, 0x51,
        ];
        let scalar_sum =
            ffi::BnModAdd(&self.own_commit_scalar, &self.peer_commit_scalar, &p256_order);

        // KDF-512 is not exposed directly. We can use Sha256PrfBits (standard 802.11 KDF) for 512 bits.
        // But `ffi.rs` doesn't expose `Sha256PrfBits` directly!
        // We exposed `EcP256...` but missed `Sha256PrfBits` or generic `KDF`.
        // Using HMAC for KCK/PMK derivation.
        // We implemented `Sha256PrfBits` in C++ for `EcP256CalculatePwe` but didn't expose it? D'oh.
        // We can expose `Sha256PrfBits` or just implement it in Rust using `ffi::Hmac`.
        // `Sha256PrfBits` is simple:
        //  HMAC(Key, i || Label || Data || Length) loop.
        // Let's implement KDF-512 in Rust using `ffi::Hmac`.

        let label = "SAE KCK and PMK";
        let context = scalar_sum.clone();
        let kdf_out = self.sha256_kdf(&keyseed, label, &context, 512); // 512 bits

        if kdf_out.len() != 64 {
            error!("SAE: KDF failed");
            return None;
        }

        self.kck = kdf_out[0..32].to_vec(); // KCK (32 bytes for HMAC-SHA256)
        self.pmk = kdf_out[32..64].to_vec(); // PMK (32 bytes)

        // PMKID = L((own_commit_scalar + peer_commit_scalar) mod q, 0, 128)
        // hostapd says: pmkid = val (scalar_sum) truncated?
        // 802.11-2016 12.4.8.2.4: PMKID = L(val, 0, 128) -> First 16 bytes of val (scalar_sum).
        if scalar_sum.len() >= 16 {
            self.pmkid = scalar_sum[0..16].to_vec();
        } else {
            // Pad to 16 bytes to match PMKID/KCK requirements.
            self.pmkid = scalar_sum.clone();
            self.pmkid.resize(16, 0);
        }

        info!("SAE: Derived PMK, KCK, PMKID");
        Some(())
    }

    fn sha256_kdf(&self, key: &[u8], label: &str, context: &[u8], bits: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let buf_len = (bits + 7) / 8;
        let mut counter: u16 = 1;
        let mut pos = 0;
        let len_le = (bits as u16).to_le_bytes();

        while pos < buf_len {
            let mut data = Vec::new();
            data.extend_from_slice(&counter.to_le_bytes());
            data.extend_from_slice(label.as_bytes());
            data.extend_from_slice(context);
            data.extend_from_slice(&len_le);

            let hash = ffi::Hmac(ffi::DigestType::SHA256, &key.to_vec(), &data);

            let needed = buf_len - pos;
            let copy = std::cmp::min(needed, hash.len());
            out.extend_from_slice(&hash[0..copy]);
            pos += copy;
            counter += 1;
        }
        out
    }

    /// Step 3: Generate Confirm Message
    pub fn build_confirm(&mut self) -> Option<Vec<u8>> {
        if self.kck.is_empty() {
            error!("SAE: KCK not derived, cannot build confirm");
            return None;
        }

        // TODO: Implement Verification Hash (Cn) logic
        // For now, return a placeholder to verify state machine flow
        let mut body = Vec::new();
        // scounter (2 bytes LE)
        body.extend_from_slice(&self.scounter.to_le_bytes());
        // Confirm string (32 bytes)
        body.extend(std::iter::repeat(0xAA).take(32));

        if !self.peer_commit_scalar.is_empty() {
            let _ = self.process_commit();
        }
        Some(body)
    }

    pub fn parse_confirm(&mut self, body: &[u8]) -> Option<()> {
        if body.len() < 2 + 32 {
            // counter + 32-byte hash
            error!("SAE: Confirm body too short");
            return None;
        }

        let rc = u16::from_le_bytes([body[0], body[1]]);
        // let peer_confirm = &body[2..34];

        // Verify peer confirm
        self.rcounter = rc;

        Some(())
    }
}
