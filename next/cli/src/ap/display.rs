// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_proto::access_point;

pub fn print_list_ap_response(response: &access_point::ListAccessPointsResponse, _verbose: bool) {
    if response.access_points.is_empty() {
        println!("No available Access Points found.");
        return;
    }

    let mut aps = response.access_points.clone();
    aps.sort_by_key(|ap| ap.id);

    println!(
        "{:<2} | {:<20} | {:<17} | {:<20} | {:<4} | {:<2} | {:<9} | {:<6} | {:<7}",
        "ID", "SSID", "BSSID", "Security", "Chan", "CC", "PHY Mode", "Hidden", "Clients"
    );
    println!("{:-<108}", "-");

    for ap in &aps {
        print_ap(ap);
    }
}

pub fn print_ap(ap: &access_point::AccessPoint) {
    let security = if ap.enterprise_enabled {
        "WPA2/WPA3 Enterprise"
    } else if ap.sae {
        "WPA3 Personal"
    } else if !ap.wpa_passphrase.is_empty() {
        "WPA2 Personal"
    } else {
        "None"
    };

    let phy_mode =
        if ap.hw_mode.is_empty() { "802.11".to_string() } else { format!("802.11{}", ap.hw_mode) };

    let hidden = if ap.hidden_ssid { "Yes" } else { "No" };
    let clients = ap.connected_devices.len();

    print!(
        "{:<2} | {:<20} | {:<17} | {:<20} | {:<4} | {:<2} | {:<9} | {:<6} | {:<7}",
        ap.id, ap.ssid, ap.bssid, security, ap.channel, ap.country_code, phy_mode, hidden, clients
    );

    if !ap.mac_acl_list.is_empty() {
        print!(" | Client MAC ACL List: {:?}", ap.mac_acl_list);
    }
    println!();
}
