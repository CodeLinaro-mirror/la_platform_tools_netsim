// Copyright 2025-2026 The Android Open Source Project

use crate::shared::SharedKeyStore;
use crate::{ApError, ApState};
use actor_framework::DynContext;

use netsim_packets::ieee80211::{
    ie::IeIterator, management_subtype, tags, write_ie, AssociationResponseFixedFields,
    AuthenticationFixedFields, BeaconFixedFields, BeaconFrameHeader, FrameControl, Ieee80211,
    MacHeader3Addr, SequenceControl,
};
use netsim_packets::llc::{control_field, sap, LlcSnapHeader};
use zerocopy::{IntoBytes, U16};

/// Handles 802.11 Management Frames
#[derive(Clone, Debug)]
pub struct Ieee80211Manager {
    // We might need some state here, or pass it in
}

impl Ieee80211Manager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn generate_beacon(&self, ap: &ApState) -> Result<Vec<bytes::Bytes>, ApError> {
        // Beacon Header
        let header = BeaconFrameHeader {
            frame_control: FrameControl::new(0x0080), // Mgmt (00), Beacon (1000) -> 0x0080 (LE: 80 00)
            duration: U16::new(0),
            da: netsim_packets::ethernet::MacAddr { bytes: [0xFF; 6] },
            sa: ap.config.bssid,
            bssid: ap.config.bssid,
            sequence_control: SequenceControl::new(0),
        };

        let mut frame = header.as_bytes().to_vec();

        // Fixed Fields
        let fixed = BeaconFixedFields {
            timestamp: [0; 8],
            beacon_interval: U16::new(ap.config.beacon_interval),
            capabilities: U16::new(0x0001), // ESS
        };
        frame.extend_from_slice(fixed.as_bytes());

        // IEs
        self.append_beacon_ies(&mut frame, ap);

        Ok(vec![bytes::Bytes::from(frame)])
    }

    fn append_beacon_ies(&self, body: &mut Vec<u8>, ap: &ApState) {
        // SSID IE
        write_ie(body, tags::SSID, ap.config.ssid.as_bytes());

        // Supported Rates
        write_ie(body, tags::SUPPORTED_RATES, tags::SUPPORTED_RATES_DEFAULT);

        // DS Param (Channel)
        write_ie(body, tags::DS_PARAMETER_SET, &[ap.config.channel]);

        // RSN IE (WPA2)
        let rsn_ie = crate::rsn::build_rsn_ie(&ap.config);
        if !rsn_ie.is_empty() {
            body.extend_from_slice(&rsn_ie);
        }

        // WiFi 6 (HE) Support
        if ap.config.hw_mode == "ax" {
            // HE Capabilities (ID 255, ExtID 35)
            // Body: ExtID(35) + Caps(00 00)
            write_ie(body, tags::EXTENSION, &[tags::HE_CAPABILITIES, 0x00, 0x00]);
        }
    }

    /// Handles an incoming Management Frame
    pub fn handle_frame(
        &mut self,
        ap: &mut ApState,
        frame: &[u8],
        shared_keys: &SharedKeyStore,
        _ctx: &mut DynContext<u32>,
    ) -> Result<Vec<bytes::Bytes>, ApError> {
        let ieee80211_frame = match Ieee80211::decode(frame) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("ApActor: Failed to decode 802.11 frame: {}", e);
                return Ok(vec![]);
            }
        };

        log::debug!(
            "ApActor: Handling frame subtype={:?} src={}",
            ieee80211_frame.stype(),
            ieee80211_frame.get_source()
        );

        if !ieee80211_frame.is_mgmt() {
            return self.handle_data_frame(ap, frame, ieee80211_frame, shared_keys);
        }

        match ieee80211_frame.stype() {
            management_subtype::AUTHENTICATION => self.handle_auth(ap, &ieee80211_frame),
            management_subtype::ASSOCIATION_REQUEST => self.handle_assoc(ap, &ieee80211_frame),
            management_subtype::PROBE_REQUEST => self.handle_probe_req(ap, &ieee80211_frame, frame),
            management_subtype::DEAUTHENTICATION => {
                self.handle_deauth(ap, &ieee80211_frame, shared_keys)
            }
            _ => Ok(vec![]),
        }
    }

    fn handle_auth(
        &mut self,
        ap: &mut ApState,
        frame: &Ieee80211,
    ) -> Result<Vec<bytes::Bytes>, ApError> {
        let src = frame.get_source();
        log::info!("ApActor: Received Auth from {}", src);

        // Construct Auth Response (Seq 2)
        // Header (24 bytes) + Auth Body (6 bytes)
        // FC: Auth (mgmt, subtype 11)
        // DA: src
        // SA: BSSID (ap.config.bssid)
        // BSSID: BSSID
        // SC: 0

        let mut resp = Vec::new();

        // 802.11 Header
        let header = MacHeader3Addr {
            frame_control: FrameControl::new(0x00B0), // Mgmt(00), Auth(1011) -> 0x00B0
            duration_id: U16::new(0),
            addr1: src,                                     // DA
            addr2: ap.config.bssid,                         // SA
            addr3: ap.config.bssid,                         // BSSID
            sequence_control: SequenceControl::new(0x0010), // SC (Seq 1?)
        };
        resp.extend_from_slice(header.as_bytes());

        // Auth Body
        let body = AuthenticationFixedFields {
            algorithm: U16::new(0), // Open System
            sequence: U16::new(2),
            status: U16::new(0), // Success
        };
        resp.extend_from_slice(body.as_bytes());

        let msg = bytes::Bytes::from(resp);
        Ok(vec![msg])
    }

    fn handle_assoc(
        &mut self,
        ap: &mut ApState,
        frame: &Ieee80211,
    ) -> Result<Vec<bytes::Bytes>, ApError> {
        let src = frame.get_source();
        log::info!("ApActor: Received Assoc Req from {}", src);

        let mut msgs = Vec::new();

        // Construct Assoc Response (Seq 2)
        let mut resp = Vec::new();

        let header = MacHeader3Addr {
            frame_control: FrameControl::new(0x0010), // Mgmt(00), Assoc Resp(0001) -> 0x0010
            duration_id: U16::new(0),
            addr1: src,                                     // DA
            addr2: ap.config.bssid,                         // SA
            addr3: ap.config.bssid,                         // BSSID
            sequence_control: SequenceControl::new(0x0010), // SC
        };
        resp.extend_from_slice(header.as_bytes());

        // Body
        let body = AssociationResponseFixedFields {
            capabilities: U16::new(0x0001), // ESS
            status: U16::new(0),            // Success
            aid: U16::new(0xC001),          // AID 1 (with 0xC000 bits set?) matches valid range?
                                            // Usually AID is 0xC000 | id.
        };
        resp.extend_from_slice(body.as_bytes());

        // Rates IE
        // Rates IE
        write_ie(&mut resp, tags::SUPPORTED_RATES, tags::SUPPORTED_RATES_DEFAULT);

        msgs.push(bytes::Bytes::from(resp));

        // Init WPA if configured
        if let Some(passphrase) = &ap.config.wpa_passphrase {
            // Assume passphrase usage for now (PSK derived?)
            // We need to implement proper key derivation later.
            let rsn_ie = crate::rsn::build_rsn_ie(&ap.config);
            let mut authenticator = crate::wpa_auth::WpaAuthenticator::new(
                ap.config.bssid,
                src,
                passphrase.as_bytes(),
                &rsn_ie,
            );

            if let Ok(m1) = authenticator.initiate_handshake() {
                let m1_frame = self.wrap_eapol(ap, src, &m1);
                ap.wpa = Some(authenticator);
                msgs.push(bytes::Bytes::from(m1_frame));
            }
        }

        Ok(msgs)
    }

    fn handle_probe_req(
        &mut self,
        ap: &mut ApState,
        frame: &Ieee80211,
        raw_frame: &[u8],
    ) -> Result<Vec<bytes::Bytes>, ApError> {
        // Filter by Destination Address (DA)
        // Must be Broadcast (FF:...) or match our BSSID.
        let da = frame.get_destination();
        if !da.is_broadcast() && da != ap.config.bssid {
            return Ok(vec![]);
        }

        // Parse SSID from Probe Req
        // Frame: Header (24) + IEs.
        // Ieee80211 doesn't have `payload()`, but `decode` validates it.
        // We'll operate on `raw_frame` slice for IE parsing. Header is usually 24 bytes for Mgmt.
        if raw_frame.len() < 24 {
            return Ok(vec![]);
        }
        let ies = &raw_frame[24..];

        let mut requested_ssid = None;

        // Use IeIterator for safe parsing
        for ie in IeIterator::new(ies) {
            if ie.id == tags::SSID {
                let val = String::from_utf8_lossy(ie.body);
                requested_ssid = Some(val.to_string());
                break; // Found it
            }
        }

        // Filter: Respond if SSID matches or is Wildcard (empty)
        let respond = match requested_ssid {
            Some(s) if s.is_empty() => true,        // Wildcard
            Some(s) if s == ap.config.ssid => true, // Direct Match
            None => true,                           // No SSID IE? Assume wildcard or malformed.
            _ => false,                             // Mismatch
        };

        if !respond {
            return Ok(vec![]);
        }

        let src = frame.get_source();

        // Construct Probe Response
        let header = BeaconFrameHeader {
            frame_control: FrameControl::new(0x0050), // Mgmt(00), ProbeResp(0101) -> 0x0050
            duration: U16::new(0),
            da: src,                // DA
            sa: ap.config.bssid,    // SA
            bssid: ap.config.bssid, // BSSID
            sequence_control: SequenceControl::new(0),
        };

        let mut resp = header.as_bytes().to_vec();

        // Fixed Fields (Same as Beacon)
        let fixed = BeaconFixedFields {
            timestamp: [0; 8],
            beacon_interval: U16::new(ap.config.beacon_interval),
            capabilities: U16::new(0x0001), // ESS
        };
        resp.extend_from_slice(fixed.as_bytes());

        // IEs (Same as Beacon)
        self.append_beacon_ies(&mut resp, ap);

        Ok(vec![bytes::Bytes::from(resp)])
    }

    fn handle_deauth(
        &self,
        ap: &mut ApState,
        frame: &Ieee80211,
        shared_keys: &SharedKeyStore,
    ) -> Result<Vec<bytes::Bytes>, ApError> {
        let src = frame.get_source();
        log::info!("ApActor: Received Deauth from {}", src);

        // Check if we have a session for this source
        if let Some(_wpa) = &ap.wpa {
            // Since ApState currently supports only a single session/authenticator,
            // we clear it indiscriminately. Future multi-station support will need keyed lookup.
            ap.wpa = None;
        }

        shared_keys.remove_session(&src);

        Ok(vec![])
    }

    // Helper to wrap EAPOL in Data Frame
    fn wrap_eapol(
        &self,
        ap: &ApState,
        dest: netsim_packets::ethernet::MacAddr,
        payload: &[u8],
    ) -> Vec<u8> {
        let mut frame = Vec::new();

        // 802.11 Header
        // ToDS=0, FromDS=1 (AP to STA) -> FC 0x0208
        // Type=Data(10) Subtype=Data(0000)
        // FC: 0000 0010 0000 1000 = 0x0208
        let header = MacHeader3Addr {
            frame_control: FrameControl::new(0x0208),
            duration_id: U16::new(0),
            addr1: dest,            // DA
            addr2: ap.config.bssid, // BSSID
            addr3: ap.config.bssid, // SA
            sequence_control: SequenceControl::new(0),
        };
        frame.extend_from_slice(header.as_bytes());

        // LLC Header
        let llc = LlcSnapHeader::new(
            sap::SNAP,
            sap::SNAP,
            control_field::UI,
            [0x00, 0x00, 0x00],
            netsim_packets::ethernet::ether_type::EAPOL,
        );
        frame.extend_from_slice(llc.as_bytes());

        frame.extend_from_slice(payload);
        frame
    }

    fn handle_data_frame(
        &self,
        ap: &mut ApState,
        frame: &[u8],
        ieee80211_frame: Ieee80211,
        shared_keys: &SharedKeyStore,
    ) -> Result<Vec<bytes::Bytes>, ApError> {
        // Quick EAPOL check (Data Frame + length > 32 + LLC 802.1X Type)
        if frame.len() <= 32 {
            return Ok(vec![]);
        }
        // Offset 24: LLC Header (AA AA 03 OUI.. Type..)
        let is_eapol = frame[24] == sap::SNAP
            && frame[25] == sap::SNAP
            && frame[26] == control_field::UI
            && frame[30] == (netsim_packets::ethernet::ether_type::EAPOL >> 8) as u8
            && frame[31] == (netsim_packets::ethernet::ether_type::EAPOL & 0xFF) as u8;

        if !is_eapol {
            return Ok(vec![]);
        }

        let wpa = match &mut ap.wpa {
            Some(wpa) => wpa,
            None => {
                log::debug!("ApActor: Received EAPOL but WPA not configured");
                return Ok(vec![]);
            }
        };

        log::info!("ApActor: Received EAPOL frame from src={}", ieee80211_frame.get_source());
        let payload = &frame[32..]; // Skip Header(24) + LLC(8)

        let outputs = match wpa.handle_eapol(payload) {
            Ok(o) => o,
            Err(_) => {
                log::warn!("ApActor: WPA handle_eapol failed");
                return Ok(vec![]);
            }
        };

        let mut frames = Vec::new();
        let src = ieee80211_frame.get_source();
        log::info!("ApActor: EAPOL produced {} outputs for {}", outputs.len(), src);

        for out in outputs {
            match out {
                crate::wpa_auth::WpaOutput::Frame(data) => {
                    log::debug!("ApActor: Sending EAPOL response (len={})", data.len());
                    let wrapped = self.wrap_eapol(ap, src, &data);
                    frames.push(bytes::Bytes::from(wrapped));
                }
                crate::wpa_auth::WpaOutput::InstallKey { key_index, key, cipher } => {
                    if key_index == 0 {
                        log::info!("ApActor: Installing PTK for {} cipher={}", src, cipher);
                        shared_keys.add_session(src, key);
                    } else {
                        log::info!("ApActor: Installing GTK idx={}", key_index);
                    }
                }
            }
        }
        Ok(frames)
    }
}
