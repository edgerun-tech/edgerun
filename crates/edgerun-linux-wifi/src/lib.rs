//! Linux WiFi management via nl80211 netlink and wireless extensions.

mod socket;
mod nla;
mod scan;
mod connect;
mod ap;
mod getifindex;
mod interface;
mod backend;
mod eapol;

pub use interface::{
    LinuxWifiInterface, LinuxWifiBackend, discover_wifi_interfaces,
    observe_current_network,
    observe_access_point_state,
    wifi_mode_to_u32,
    wifi_mode_from_u32,
    read_wireless_quality,
    set_rfkill_block,
};
pub use eapol::{
    EapolKeyMessageType,
    ParsedEapolKey,
    parse_eapol_key_frame,
    derive_ptk,
    calculate_eapol_mic,
    verify_eapol_mic,
    derive_wpa_pmk,
    parse_mac_string,
};
pub use socket::Nl80211Socket;
pub use nla::{put_nla, put_nla_u32, put_nla_string, put_nla_u16, put_nla_u8, put_nla_nested};
pub use scan::{
    nl80211_scan,
    parse_scan_results,
    parse_bss_info,
    build_rsn_ie,
    nl80211_connect_wpa,
};
pub use connect::{
    nl80211_disconnect,
    nl80211_set_interface_type,
    nl80211_get_interface_type,
    nl80211_get_station,
    nl80211_dump_stations,
};
pub use ap::{
    nl80211_get_regulatory_domain,
    nl80211_set_regulatory_domain,
    nl80211_get_wiphy_info,
    nl80211_set_bss,
    nl80211_start_ap,
    nl80211_stop_ap,
    nl80211_set_wiphy_freq,
};
pub use getifindex::get_ifindex;

#[cfg(test)]
mod tests;
