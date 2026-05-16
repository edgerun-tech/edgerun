fn main() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    let ssid_value = std::env::var("EDGERUN_USB_AP_SSID").unwrap_or_else(|_| "EdgeRunAC".into());
    let password_value = std::env::var("EDGERUN_USB_AP_PASSWORD")
        .expect("EDGERUN_USB_AP_PASSWORD must be set for USB PPP AP bridge");
    const AP_CHANNEL: u8 = 6;

    let ssid = std::ffi::CString::new(ssid_value).expect("AP SSID must not contain NUL bytes");
    let password =
        std::ffi::CString::new(password_value).expect("AP password must not contain NUL bytes");
    let rc =
        unsafe { edgerun_usb_ppp_ap_bridge_start(ssid.as_ptr(), password.as_ptr(), AP_CHANNEL) };
    if rc != 0 {
        panic!("USB PPP AP bridge failed to start: {}", rc);
    }

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

extern "C" {
    fn edgerun_usb_ppp_ap_bridge_start(
        ssid: *const std::ffi::c_char,
        password: *const std::ffi::c_char,
        channel: u8,
    ) -> i32;
    fn edgerun_usb_ppp_ap_bridge_status() -> i32;
}
