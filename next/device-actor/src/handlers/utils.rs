pub fn chip_kind_to_network_kind(
    kind: &netsim_model::chip::ChipKind,
) -> netsim_model::chip::NetworkKind {
    match kind {
        netsim_model::chip::ChipKind::BLUETOOTH | netsim_model::chip::ChipKind::BleBeacon => {
            netsim_model::chip::NetworkKind::Bluetooth
        }
        netsim_model::chip::ChipKind::WIFI => netsim_model::chip::NetworkKind::Wifi,
        netsim_model::chip::ChipKind::UWB => netsim_model::chip::NetworkKind::Uwb,
        netsim_model::chip::ChipKind::CELLULAR => netsim_model::chip::NetworkKind::Cell,
        _ => netsim_model::chip::NetworkKind::Bluetooth, // Fallback
    }
}
