//! Edgerun unikernel - bare shell with networking

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_arch = "xtensa", feature(asm_experimental_arch))]
#![cfg_attr(not(target_os = "none"), allow(dead_code, unused_imports))]

extern crate alloc;
extern crate edgerun_dhcp;
extern crate edgerun_http;
#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
extern crate edgerun_layout;
extern crate edgerun_oci;
extern crate edgerun_platform;
extern crate edgerun_rt as rt;
#[cfg(target_arch = "x86_64")]
extern crate edgerun_rtl8125;
extern crate edgerun_tftp;
extern crate edgerun_tpm;
#[cfg(target_arch = "x86_64")]
extern crate edgerun_virtio;
#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
extern crate edgerun_wifi;

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const DISPLAY_CONSOLE_LINES: usize = 8;
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const DISPLAY_CONSOLE_COLS: usize = 38;

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
struct DisplayConsole {
    lines: [[u8; DISPLAY_CONSOLE_COLS]; DISPLAY_CONSOLE_LINES],
    lens: [usize; DISPLAY_CONSOLE_LINES],
    next: usize,
    count: usize,
    dirty: bool,
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl DisplayConsole {
    const fn new() -> Self {
        Self {
            lines: [[0; DISPLAY_CONSOLE_COLS]; DISPLAY_CONSOLE_LINES],
            lens: [0; DISPLAY_CONSOLE_LINES],
            next: 0,
            count: 0,
            dirty: true,
        }
    }

    fn push(&mut self, message: &str) {
        let bytes = message.as_bytes();
        let mut offset = 0;
        while offset < bytes.len() {
            let end = next_line_end(bytes, offset);
            self.push_bytes(&bytes[offset..end]);
            offset = end;
            while offset < bytes.len() && (bytes[offset] == b'\n' || bytes[offset] == b'\r') {
                offset += 1;
            }
        }
        if bytes.is_empty() {
            self.push_bytes(b"");
        }
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        let line = &mut self.lines[self.next];
        let len = bytes.len().min(DISPLAY_CONSOLE_COLS);
        let mut i = 0;
        while i < len {
            line[i] = sanitize_console_byte(bytes[i]);
            i += 1;
        }
        while i < DISPLAY_CONSOLE_COLS {
            line[i] = 0;
            i += 1;
        }
        self.lens[self.next] = len;
        self.next = (self.next + 1) % DISPLAY_CONSOLE_LINES;
        self.count = (self.count + 1).min(DISPLAY_CONSOLE_LINES);
        self.dirty = true;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
struct DisplayConsoleCell(core::cell::UnsafeCell<DisplayConsole>);

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe impl Sync for DisplayConsoleCell {}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
impl DisplayConsoleCell {
    const fn new() -> Self {
        Self(core::cell::UnsafeCell::new(DisplayConsole::new()))
    }

    fn with<R>(&self, f: impl FnOnce(&mut DisplayConsole) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
static DISPLAY_CONSOLE: DisplayConsoleCell = DisplayConsoleCell::new();

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn display_console_log(message: &str) {
    DISPLAY_CONSOLE.with(|console| console.push(message));
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn next_line_end(bytes: &[u8], offset: usize) -> usize {
    let mut end = offset;
    while end < bytes.len() && end - offset < DISPLAY_CONSOLE_COLS {
        if bytes[end] == b'\n' || bytes[end] == b'\r' {
            break;
        }
        end += 1;
    }
    end
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn sanitize_console_byte(byte: u8) -> u8 {
    if (0x20..=0x7e).contains(&byte) {
        byte
    } else {
        b'?'
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[repr(C, packed)]
struct EspAppDesc {
    magic_word: u32,
    secure_version: u32,
    reserv1: [u32; 2],
    version: [u8; 32],
    project_name: [u8; 32],
    time: [u8; 16],
    date: [u8; 16],
    idf_ver: [u8; 32],
    app_elf_sha256: [u8; 32],
    min_efuse_blk_rev_full: u16,
    max_efuse_blk_rev_full: u16,
    mmu_page_size: u8,
    reserv3: [u8; 3],
    reserv2: [u32; 18],
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
const UI_HTML: &str = r#"
<main class="remote">
  <section class="status">
    <div class="label">TCL AC</div>
    <div class="temp">24</div>
    <div class="mode">Cool - Auto fan</div>
  </section>
  <section class="controls">
    <button class="power">Power</button>
    <button>Mode</button>
    <button>Fan</button>
    <button>Swing</button>
  </section>
</main>
"#;

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
const UI_CSS: &str = r#"
main { display: block; width: 320px; min-height: 480px; background-color: #f8fafc; color: #101828; padding: 16px; }
.status { display: block; background-color: #0f766e; color: white; border-radius: 14px; padding: 18px; margin-bottom: 14px; }
.label { display: block; font-size: 16px; margin-bottom: 8px; }
.temp { display: block; font-size: 72px; line-height: 1.0; margin-bottom: 8px; }
.mode { display: block; font-size: 18px; }
.controls { display: flex; flex-wrap: wrap; }
button { display: block; width: 132px; height: 72px; margin-right: 8px; margin-bottom: 10px; background-color: #e2e8f0; color: #0f172a; border-radius: 10px; padding: 18px; font-size: 18px; }
.power { background-color: #dc2626; color: white; }
"#;

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_initial_ui() {
    #[cfg(feature = "html-ui")]
    render_html_ui(None);
    #[cfg(not(feature = "html-ui"))]
    render_debug_pattern(None);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_console_if_dirty(touch: Option<(u16, u16)>) {
    let dirty = DISPLAY_CONSOLE.with(|console| {
        let dirty = console.dirty;
        console.dirty = false;
        dirty
    });
    if dirty {
        match touch {
            Some(touch) => render_touch_ui(touch),
            None => render_initial_ui(),
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_touch_ui(touch: (u16, u16)) {
    #[cfg(feature = "html-ui")]
    render_html_ui(Some(touch));
    #[cfg(not(feature = "html-ui"))]
    render_debug_pattern(Some(touch));
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn poll_serial_control(rx: &mut rt::serial_mux::Receiver<256>, last_touch: Option<(u16, u16)>) {
    let mut processed = 0;
    while processed < 8 {
        let frame = match rx.poll() {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(_) => {
                rt::serial_mux::write(rt::serial_mux::CHANNEL_CONTROL, b"err decode\n");
                break;
            }
        };
        processed += 1;
        if frame.channel != rt::serial_mux::CHANNEL_CONTROL {
            continue;
        }
        match frame.payload() {
            b"ping" | b"ping\n" => {
                display_console_log("ctl ping");
                rt::serial_mux::write_with_seq(
                    rt::serial_mux::CHANNEL_CONTROL,
                    frame.seq,
                    b"pong\n",
                );
            }
            b"status" | b"status\n" => {
                display_console_log("ctl status");
                rt::serial_mux::write_with_seq(
                    rt::serial_mux::CHANNEL_CONTROL,
                    frame.seq,
                    b"ok board=jc3248w535 display=up touch=up transport=usb-serial-jtag\n",
                );
                if let Some((x, y)) = last_touch {
                    write_touch_status(frame.seq, x, y);
                }
            }
            b"repaint" | b"repaint\n" => {
                display_console_log("ctl repaint");
                if let Some(touch) = last_touch {
                    render_touch_ui(touch);
                } else {
                    render_initial_ui();
                }
                rt::serial_mux::write_with_seq(
                    rt::serial_mux::CHANNEL_CONTROL,
                    frame.seq,
                    b"ok repaint\n",
                );
            }
            b"wifi" | b"wifi\n" | b"wifi start" | b"wifi start\n" => {
                display_console_log("ctl wifi start");
                if try_start_esp32s3_wifi_ap() {
                    rt::serial_mux::write_with_seq(
                        rt::serial_mux::CHANNEL_CONTROL,
                        frame.seq,
                        b"ok wifi-start\n",
                    );
                } else {
                    rt::serial_mux::write_with_seq(
                        rt::serial_mux::CHANNEL_CONTROL,
                        frame.seq,
                        b"err wifi-start\n",
                    );
                }
            }
            b"wifiinit" | b"wifiinit\n" => write_wifi_init(frame.seq),
            b"wifi1" | b"wifi1\n" => write_wifi_debug_step(frame.seq, 1),
            b"wifi0" | b"wifi0\n" => write_wifi_debug_step(frame.seq, 0),
            b"wifi2" | b"wifi2\n" => write_wifi_debug_step(frame.seq, 2),
            b"wifi3" | b"wifi3\n" => write_wifi_debug_step(frame.seq, 3),
            b"wifi4" | b"wifi4\n" => write_wifi_debug_step(frame.seq, 4),
            b"wifi5" | b"wifi5\n" => write_wifi_debug_step(frame.seq, 5),
            b"wifi6" | b"wifi6\n" => write_wifi_debug_step(frame.seq, 6),
            b"wifi7" | b"wifi7\n" => write_wifi_debug_step(frame.seq, 7),
            b"wifi8" | b"wifi8\n" => write_wifi_debug_step(frame.seq, 8),
            b"wifi9" | b"wifi9\n" => write_wifi_debug_step(frame.seq, 9),
            b"wifi10" | b"wifi10\n" => write_wifi_debug_step(frame.seq, 10),
            b"wifi11" | b"wifi11\n" => write_wifi_debug_step(frame.seq, 11),
            b"wifi12" | b"wifi12\n" => write_wifi_debug_step(frame.seq, 12),
            b"wifi13" | b"wifi13\n" => write_wifi_debug_step(frame.seq, 13),
            b"wifi14" | b"wifi14\n" => write_wifi_debug_step(frame.seq, 14),
            b"wifi15" | b"wifi15\n" => write_wifi_debug_step(frame.seq, 15),
            b"wifi16" | b"wifi16\n" => write_wifi_debug_step(frame.seq, 16),
            b"wifi17" | b"wifi17\n" => write_wifi_debug_step(frame.seq, 17),
            b"wifi18" | b"wifi18\n" => write_wifi_debug_step(frame.seq, 18),
            b"wifi19" | b"wifi19\n" => write_wifi_debug_step(frame.seq, 19),
            b"wifi20" | b"wifi20\n" => write_wifi_debug_step(frame.seq, 20),
            b"wifi21" | b"wifi21\n" => write_wifi_debug_step(frame.seq, 21),
            b"wifi22" | b"wifi22\n" => write_wifi_debug_step(frame.seq, 22),
            b"wifi23" | b"wifi23\n" => write_wifi_debug_step(frame.seq, 23),
            b"wifi24" | b"wifi24\n" => write_wifi_debug_step(frame.seq, 24),
            b"wifi25" | b"wifi25\n" => write_wifi_debug_step(frame.seq, 25),
            b"wifimmio" | b"wifimmio\n" => write_wifi_mmio_regs(frame.seq),
            b"wifitx" | b"wifitx\n" => write_wifi_tx_regs(frame.seq),
            b"wifirate" | b"wifirate\n" => write_wifi_rate_regs(frame.seq),
            b"wificrypto" | b"wificrypto\n" => write_wifi_crypto_regs(frame.seq),
            b"wifiant" | b"wifiant\n" => write_wifi_antenna_regs(frame.seq),
            b"wifiphy" | b"wifiphy\n" => write_wifi_phy_regs(frame.seq),
            b"wifiregs" | b"wifiregs\n" => write_wifi_debug_regs(frame.seq),
            b"wififuns" | b"wififuns\n" => write_wifi_phy_fun_slots(frame.seq),
            _ => {
                display_console_log("ctl unknown");
                rt::serial_mux::write_with_seq(
                    rt::serial_mux::CHANNEL_CONTROL,
                    frame.seq,
                    b"err unknown-command\n",
                );
            }
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_debug_regs(seq: u16) {
    display_console_log("ctl wifiregs");
    let mut buf = [0u8; 160];
    let mut len = 0;
    append_wifi_debug_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_init(seq: u16) {
    display_console_log("ctl wifiinit");
    let ok = try_wifi_init_known_good();
    let mut buf = [0u8; 48];
    let mut len = 0;
    if ok {
        append_bytes(&mut buf, &mut len, b"ok wifi-init status=");
    } else {
        append_bytes(&mut buf, &mut len, b"err wifi-init status=");
    }
    append_i32(&mut buf, &mut len, wifi_debug_status());
    append_bytes(&mut buf, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_phy_fun_slots(seq: u16) {
    display_console_log("ctl wififuns");
    let mut buf = [0u8; 320];
    let mut len = 0;
    append_wifi_phy_fun_slots(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_mmio_regs(seq: u16) {
    display_console_log("ctl wifimmio");
    let mut buf = [0u8; 512];
    let mut len = 0;
    append_wifi_mmio_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_tx_regs(seq: u16) {
    display_console_log("ctl wifitx");
    let mut buf = [0u8; 256];
    let mut len = 0;
    append_wifi_tx_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_rate_regs(seq: u16) {
    display_console_log("ctl wifirate");
    let mut buf = [0u8; 160];
    let mut len = 0;
    append_wifi_rate_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_crypto_regs(seq: u16) {
    display_console_log("ctl wificrypto");
    let mut buf = [0u8; 192];
    let mut len = 0;
    append_wifi_crypto_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_antenna_regs(seq: u16) {
    display_console_log("ctl wifiant");
    let mut buf = [0u8; 128];
    let mut len = 0;
    append_wifi_antenna_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

fn write_wifi_phy_regs(seq: u16) {
    display_console_log("ctl wifiphy");
    let mut buf = [0u8; 96];
    let mut len = 0;
    append_wifi_phy_regs(&mut buf, &mut len);
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_wifi_debug_step(seq: u16, step: u8) {
    display_console_log("ctl wifi step");
    let mut buf = [0u8; 48];
    let mut len = 0;
    if try_wifi_debug_step(step) {
        append_bytes(&mut buf, &mut len, b"ok wifi-step status=");
    } else {
        append_bytes(&mut buf, &mut len, b"err wifi-step status=");
    }
    append_u16(&mut buf, &mut len, wifi_debug_status() as u16);
    append_bytes(&mut buf, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn append_wifi_debug_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio::debug_regs();
    append_bytes(out, len, b"wifi regs pwc=0x");
    append_hex_u32(out, len, regs.rtc_dig_pwc);
    append_bytes(out, len, b" iso=0x");
    append_hex_u32(out, len, regs.rtc_dig_iso);
    append_bytes(out, len, b" clk=0x");
    append_hex_u32(out, len, regs.wifi_clk_en);
    append_bytes(out, len, b" rst=0x");
    append_hex_u32(out, len, regs.wifi_rst_en);
    append_bytes(out, len, b" mac=0x");
    append_hex_u32(out, len, regs.mac_reset_ctrl);
    append_bytes(out, len, b" funs=0x");
    append_hex_u32(out, len, regs.phy_funs);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_mmio_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_mac_regs();
    append_bytes(out, len, b"wifi mmio mac=0x");
    append_hex_u32(out, len, regs.mac_reset_ctrl);
    append_bytes(out, len, b" dma=0x");
    append_hex_u32(out, len, regs.dma_ctrl);
    append_bytes(out, len, b" rxp0=0x");
    append_hex_u32(out, len, regs.rx_policy0);
    append_bytes(out, len, b" rxp1=0x");
    append_hex_u32(out, len, regs.rx_policy1);
    append_bytes(out, len, b" rxp2=0x");
    append_hex_u32(out, len, regs.rx_policy2);
    append_bytes(out, len, b" rxp3=0x");
    append_hex_u32(out, len, regs.rx_policy3);
    append_bytes(out, len, b" c34=0x");
    append_hex_u32(out, len, regs.ctrl_33c34);
    append_bytes(out, len, b" c40=0x");
    append_hex_u32(out, len, regs.ctrl_33c40);
    append_bytes(out, len, b" c74=0x");
    append_hex_u32(out, len, regs.ctrl_33c74);
    append_bytes(out, len, b" r0=0x");
    append_hex_u32(out, len, regs.rx_ctrl0);
    append_bytes(out, len, b" r1=0x");
    append_hex_u32(out, len, regs.rx_ctrl1);
    append_bytes(out, len, b" r2=0x");
    append_hex_u32(out, len, regs.rx_ctrl2);
    append_bytes(out, len, b" r3=0x");
    append_hex_u32(out, len, regs.rx_ctrl3);
    append_bytes(out, len, b" rg=0x");
    append_hex_u32(out, len, regs.rx_global);
    append_bytes(out, len, b" cfg0=0x");
    append_hex_u32(out, len, regs.rx_cfg0);
    append_bytes(out, len, b" cfg1=0x");
    append_hex_u32(out, len, regs.rx_cfg1);
    append_bytes(out, len, b" cfg2=0x");
    append_hex_u32(out, len, regs.rx_cfg2);
    append_bytes(out, len, b" cfg3=0x");
    append_hex_u32(out, len, regs.rx_cfg3);
    append_bytes(out, len, b" base=0x");
    append_hex_u32(out, len, regs.rx_base);
    append_bytes(out, len, b" flt=0x");
    append_hex_u32(out, len, regs.rx_filter_count);
    append_bytes(out, len, b" f0=0x");
    append_hex_u32(out, len, regs.rx_filter_ctrl0);
    append_bytes(out, len, b" f5=0x");
    append_hex_u32(out, len, regs.rx_filter_ctrl5);
    append_bytes(out, len, b" p0=0x");
    append_hex_u32(out, len, regs.rx_filter_pattern0);
    append_bytes(out, len, b" p5=0x");
    append_hex_u32(out, len, regs.rx_filter_pattern5);
    append_bytes(out, len, b" m0=0x");
    append_hex_u32(out, len, regs.rx_filter_mask0);
    append_bytes(out, len, b" m5=0x");
    append_hex_u32(out, len, regs.rx_filter_mask5);
    append_bytes(out, len, b" c114=0x");
    append_hex_u32(out, len, regs.ctrl_33114);
    append_bytes(out, len, b" c118=0x");
    append_hex_u32(out, len, regs.ctrl_33118);
    append_bytes(out, len, b" coex=0x");
    append_hex_u32(out, len, regs.coex_ctrl);
    append_bytes(out, len, b" pti=0x");
    append_hex_u32(out, len, regs.coex_pti);
    append_bytes(out, len, b" dpti=0x");
    append_hex_u32(out, len, regs.coex_default_pti);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_tx_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_tx_regs();
    append_bytes(out, len, b"wifi tx c118=0x");
    append_hex_u32(out, len, regs.ctrl_33118);
    append_bytes(out, len, b" c78=0x");
    append_hex_u32(out, len, regs.ctrl_33c78);
    append_bytes(out, len, b" t10=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c10);
    append_bytes(out, len, b" t14=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c14);
    append_bytes(out, len, b" t18=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c18);
    append_bytes(out, len, b" t54=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c54);
    append_bytes(out, len, b" t88=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c88);
    append_bytes(out, len, b" t94=0x");
    append_hex_u32(out, len, regs.tx_ctrl_33c94);
    append_bytes(out, len, b" b8=0x");
    append_hex_u32(out, len, regs.ctrl_332b8);
    append_bytes(out, len, b" c84=0x");
    append_hex_u32(out, len, regs.ctrl_33084);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_rate_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_rate_regs();
    append_bytes(out, len, b"wifi rate r404=0x");
    append_hex_u32(out, len, regs.rate_33404);
    append_bytes(out, len, b" r408=0x");
    append_hex_u32(out, len, regs.rate_33408);
    append_bytes(out, len, b" r40c=0x");
    append_hex_u32(out, len, regs.rate_3340c);
    append_bytes(out, len, b" r410=0x");
    append_hex_u32(out, len, regs.rate_33410);
    append_bytes(out, len, b" r414=0x");
    append_hex_u32(out, len, regs.rate_33414);
    append_bytes(out, len, b" r418=0x");
    append_hex_u32(out, len, regs.rate_33418);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_crypto_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_crypto_regs();
    append_bytes(out, len, b"wifi crypto c800=0x");
    append_hex_u32(out, len, regs.crypto_33800);
    append_bytes(out, len, b" c804=0x");
    append_hex_u32(out, len, regs.crypto_33804);
    append_bytes(out, len, b" c808=0x");
    append_hex_u32(out, len, regs.crypto_33808);
    append_bytes(out, len, b" c80c=0x");
    append_hex_u32(out, len, regs.crypto_3380c);
    append_bytes(out, len, b" c810=0x");
    append_hex_u32(out, len, regs.crypto_33810);
    append_bytes(out, len, b" c840=0x");
    append_hex_u32(out, len, regs.crypto_33840);
    append_bytes(out, len, b"\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_crypto_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi crypto unavailable\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_antenna_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_antenna_regs();
    append_bytes(out, len, b"wifi ant a0=0x");
    append_hex_u32(out, len, regs.ant0);
    append_bytes(out, len, b" a1=0x");
    append_hex_u32(out, len, regs.ant1);
    append_bytes(out, len, b" a7=0x");
    append_hex_u32(out, len, regs.ant7);
    append_bytes(out, len, b" aux=0x");
    append_hex_u32(out, len, regs.ant_aux);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_phy_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_phy_regs();
    append_bytes(out, len, b"wifi phy lr0=0x");
    append_hex_u32(out, len, regs.low_rate_ctrl0);
    append_bytes(out, len, b" lr1=0x");
    append_hex_u32(out, len, regs.low_rate_ctrl1);
    append_bytes(out, len, b"\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_phy_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi phy unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_antenna_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi ant unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_rate_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi rate unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_tx_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi tx unavailable\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
)))]
fn append_wifi_mmio_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi mmio unavailable\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn append_wifi_debug_regs(out: &mut [u8], len: &mut usize) {
    let regs = edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_regs();
    append_bytes(out, len, b"wifi mmio regs pwc=0x");
    append_hex_u32(out, len, regs.rtc_dig_pwc);
    append_bytes(out, len, b" iso=0x");
    append_hex_u32(out, len, regs.rtc_dig_iso);
    append_bytes(out, len, b" clk=0x");
    append_hex_u32(out, len, regs.wifi_clk_en);
    append_bytes(out, len, b" rst=0x");
    append_hex_u32(out, len, regs.wifi_rst_en);
    append_bytes(out, len, b" mac=0x");
    append_hex_u32(out, len, regs.mac_reset_ctrl);
    append_bytes(out, len, b"\n");
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn append_wifi_phy_fun_slots(out: &mut [u8], len: &mut usize) {
    let slots = edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio::debug_phy_fun_slots();
    append_bytes(out, len, b"wifi funs 008=0x");
    append_hex_u32(out, len, slots.slot_008);
    append_bytes(out, len, b" 00c=0x");
    append_hex_u32(out, len, slots.slot_00c);
    append_bytes(out, len, b" 05c=0x");
    append_hex_u32(out, len, slots.slot_05c);
    append_bytes(out, len, b" 06c=0x");
    append_hex_u32(out, len, slots.slot_06c);
    append_bytes(out, len, b" 110=0x");
    append_hex_u32(out, len, slots.slot_110);
    append_bytes(out, len, b" 148=0x");
    append_hex_u32(out, len, slots.slot_148);
    append_bytes(out, len, b" 160=0x");
    append_hex_u32(out, len, slots.slot_160);
    append_bytes(out, len, b" 164=0x");
    append_hex_u32(out, len, slots.slot_164);
    append_bytes(out, len, b" 190=0x");
    append_hex_u32(out, len, slots.slot_190);
    append_bytes(out, len, b" 1a8=0x");
    append_hex_u32(out, len, slots.slot_1a8);
    append_bytes(out, len, b" 1d4=0x");
    append_hex_u32(out, len, slots.slot_1d4);
    append_bytes(out, len, b" 200=0x");
    append_hex_u32(out, len, slots.slot_200);
    append_bytes(out, len, b" 204=0x");
    append_hex_u32(out, len, slots.slot_204);
    append_bytes(out, len, b" 208=0x");
    append_hex_u32(out, len, slots.slot_208);
    append_bytes(out, len, b" 224=0x");
    append_hex_u32(out, len, slots.slot_224);
    append_bytes(out, len, b" 234=0x");
    append_hex_u32(out, len, slots.slot_234);
    append_bytes(out, len, b"\n");
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
)))]
fn append_wifi_phy_fun_slots(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi funs unavailable\n");
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn append_wifi_debug_regs(out: &mut [u8], len: &mut usize) {
    append_bytes(out, len, b"wifi regs unavailable\n");
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn write_touch_status(seq: u16, x: u16, y: u16) {
    let mut buf = [0u8; 32];
    let mut len = 0;
    append_bytes(&mut buf, &mut len, b"touch x=");
    append_u16(&mut buf, &mut len, x);
    append_bytes(&mut buf, &mut len, b" y=");
    append_u16(&mut buf, &mut len, y);
    append_bytes(&mut buf, &mut len, b"\n");
    rt::serial_mux::write_with_seq(rt::serial_mux::CHANNEL_CONTROL, seq, &buf[..len]);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn display_touch_status(x: u16, y: u16) {
    let mut buf = [0u8; 32];
    let mut len = 0;
    append_bytes(&mut buf, &mut len, b"touch x=");
    append_u16(&mut buf, &mut len, x);
    append_bytes(&mut buf, &mut len, b" y=");
    append_u16(&mut buf, &mut len, y);
    if let Ok(text) = core::str::from_utf8(&buf[..len]) {
        display_console_log(text);
    }
}

fn append_bytes(out: &mut [u8], len: &mut usize, bytes: &[u8]) {
    for &byte in bytes {
        if *len == out.len() {
            return;
        }
        out[*len] = byte;
        *len += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_u16(out: &mut [u8], len: &mut usize, mut value: u16) {
    let mut digits = [0u8; 5];
    let mut count = 0;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while count > 0 {
        count -= 1;
        append_bytes(out, len, &digits[count..count + 1]);
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_i32(out: &mut [u8], len: &mut usize, value: i32) {
    if value < 0 {
        append_bytes(out, len, b"-");
        append_u16(out, len, value.saturating_abs() as u16);
    } else {
        append_u16(out, len, value as u16);
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_hex_u32(out: &mut [u8], len: &mut usize, value: u32) {
    let mut shift = 28;
    loop {
        let nibble = ((value >> shift) & 0x0f) as u8;
        append_hex_nibble(out, len, nibble);
        if shift == 0 {
            break;
        }
        shift -= 4;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn append_hex_nibble(out: &mut [u8], len: &mut usize, nibble: u8) {
    let nibble = nibble & 0x0f;
    let byte = if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    };
    append_bytes(out, len, &[byte]);
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
type Esp32s3WifiAp = edgerun_platform::esp32s3_wifi::Esp32s3WifiOpenAp<
    edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio,
    4,
>;

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
struct Esp32s3WifiApCell(core::cell::UnsafeCell<core::mem::MaybeUninit<Esp32s3WifiAp>>);

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
unsafe impl Sync for Esp32s3WifiApCell {}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
impl Esp32s3WifiApCell {
    const fn new() -> Self {
        Self(core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()))
    }

    unsafe fn init(&self, ap: Esp32s3WifiAp) -> &'static mut Esp32s3WifiAp {
        unsafe { (&mut *self.0.get()).write(ap) }
    }
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
static ESP32S3_WIFI_AP: Esp32s3WifiApCell = Esp32s3WifiApCell::new();

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
#[inline(never)]
fn try_start_esp32s3_wifi_ap() -> bool {
    use edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio;
    use edgerun_wifi::ieee80211::{MacAddr, OpenApConfig};

    rt::log::log(1, "ESP32-S3 WiFi AP start begin");
    let config = match OpenApConfig::new(
        MacAddr::new([0x02, 0xed, 0x67, 0x75, 0x6e, 0x01]),
        b"edgerun-ac",
        6,
    ) {
        Ok(config) => config,
        Err(_) => {
            rt::log::log(3, "ESP32-S3 WiFi AP config failed");
            return false;
        }
    };

    let ap =
        unsafe { ESP32S3_WIFI_AP.init(Esp32s3WifiAp::new(EspressifPromiscRadio::new(), config)) };
    match ap.start() {
        Ok(()) => {
            rt::log::log(1, "ESP32-S3 WiFi AP start queued");
            true
        }
        Err(_) => {
            match EspressifPromiscRadio::last_start_status() {
                13289 => rt::log::log(3, "ESP32-S3 WiFi AP start failed: channel not initialized"),
                14289 => rt::log::log(3, "ESP32-S3 WiFi AP start failed: filter not initialized"),
                15289 => rt::log::log(3, "ESP32-S3 WiFi AP start failed: callback not initialized"),
                16289 => rt::log::log(
                    3,
                    "ESP32-S3 WiFi AP start failed: promiscuous not initialized",
                ),
                27289 => rt::log::log(3, "ESP32-S3 WiFi AP start failed: set mode not initialized"),
                37289 => rt::log::log(3, "ESP32-S3 WiFi AP start failed: start not initialized"),
                15000..=24999 => rt::log::log(3, "ESP32-S3 WiFi AP start failed: init stage"),
                25000..=34999 => rt::log::log(3, "ESP32-S3 WiFi AP start failed: mode stage"),
                35000..=44999 => rt::log::log(3, "ESP32-S3 WiFi AP start failed: start stage"),
                _ => rt::log::log(3, "ESP32-S3 WiFi AP start failed"),
            }
            false
        }
    }
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn try_wifi_debug_step(step: u8) -> bool {
    edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio::debug_step(step)
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn try_wifi_init_known_good() -> bool {
    false
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
))]
fn wifi_debug_status() -> i32 {
    edgerun_platform::esp32s3_wifi_blob::EspressifPromiscRadio::last_start_status()
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn try_wifi_debug_step(step: u8) -> bool {
    edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::debug_step(step)
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn try_wifi_init_known_good() -> bool {
    edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::init_known_good()
}

#[cfg(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-mmio",
    not(feature = "esp32s3-wifi-blob")
))]
fn wifi_debug_status() -> i32 {
    edgerun_platform::esp32s3_wifi_mmio::Esp32s3WifiMmio::last_status()
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn try_wifi_init_known_good() -> bool {
    false
}

#[cfg(not(all(
    target_arch = "xtensa",
    target_os = "none",
    feature = "esp32s3-wifi-blob"
)))]
fn try_start_esp32s3_wifi_ap() -> bool {
    rt::log::log(3, "ESP32-S3 WiFi AP backend disabled");
    false
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn try_wifi_debug_step(_step: u8) -> bool {
    rt::log::log(3, "ESP32-S3 WiFi debug backend disabled");
    false
}

#[cfg(not(any(
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-blob"
    ),
    all(
        target_arch = "xtensa",
        target_os = "none",
        feature = "esp32s3-wifi-mmio",
        not(feature = "esp32s3-wifi-blob")
    )
)))]
fn wifi_debug_status() -> i32 {
    0
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn render_html_ui(touch: Option<(u16, u16)>) {
    unsafe {
        edgerun_platform::esp32s3::Jc3248w535Display::draw_rgb565_with(320, 480, |x, y| {
            if let Some((tx, ty)) = touch {
                let dx = x.abs_diff(tx);
                let dy = y.abs_diff(ty);
                if dx <= 5 && dy <= 5 {
                    return 0xf800;
                }
            }
            if let Some(color) = console_pixel(x, y) {
                return color;
            }
            html_ui_pixel(x, y)
        });
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn console_pixel(x: u16, y: u16) -> Option<u16> {
    let x = x as u32;
    let y = y as u32;
    if y < 366 {
        return None;
    }
    if y == 366 {
        return Some(rgb565(20, 184, 166));
    }
    if y < 480 {
        if console_text_pixel(x, y) {
            return Some(rgb565(226, 232, 240));
        }
        let shade = if ((x / 12) + (y / 12)) & 1 == 0 { 0 } else { 3 };
        return Some(rgb565(3 + shade, 7 + shade, 18 + shade));
    }
    None
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn console_text_pixel(px: u32, py: u32) -> bool {
    const LEFT: u32 = 8;
    const TOP: u32 = 374;
    const SCALE: u32 = 1;
    const GLYPH_W: u32 = 5;
    const GLYPH_H: u32 = 7;
    const ROW_H: u32 = 12;
    if px < LEFT || py < TOP {
        return false;
    }
    let row = ((py - TOP) / ROW_H) as usize;
    if row >= DISPLAY_CONSOLE_LINES {
        return false;
    }
    let local_y = (py - TOP) % ROW_H;
    if local_y >= GLYPH_H {
        return false;
    }
    let col = ((px - LEFT) / (GLYPH_W + SCALE)) as usize;
    if col >= DISPLAY_CONSOLE_COLS {
        return false;
    }
    let local_x = (px - LEFT) % (GLYPH_W + SCALE);
    if local_x >= GLYPH_W {
        return false;
    }

    DISPLAY_CONSOLE.with(|console| {
        if row >= console.count {
            return false;
        }
        let line_index =
            (console.next + DISPLAY_CONSOLE_LINES - console.count + row) % DISPLAY_CONSOLE_LINES;
        if col >= console.lens[line_index] {
            return false;
        }
        let glyph = glyph_5x7(console.lines[line_index][col]);
        (glyph[local_y as usize] & (1 << (4 - local_x))) != 0
    })
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn html_ui_pixel(x: u16, y: u16) -> u16 {
    let x = x as u32;
    let y = y as u32;

    const TEXT: u16 = 0xe79f;
    const MUTED_TEXT: u16 = 0x9cf3;
    const PANEL_TEXT: u16 = 0xffff;

    if text_pixel(x, y, 32, 50, 16, "TCL AC") {
        return MUTED_TEXT;
    }
    if text_pixel(x, y, 30, 128, 72, "24") {
        return PANEL_TEXT;
    }
    if text_pixel(x, y, 34, 160, 18, "Cool - Auto fan") {
        return MUTED_TEXT;
    }
    if text_pixel(x, y, 57, 251, 18, "Power") {
        return PANEL_TEXT;
    }
    if text_pixel(x, y, 211, 251, 18, "Mode") {
        return TEXT;
    }
    if text_pixel(x, y, 69, 333, 18, "Fan") {
        return TEXT;
    }
    if text_pixel(x, y, 205, 333, 18, "Swing") {
        return TEXT;
    }

    if y == 188 && x >= 28 && x < 292 {
        return rgb565(22, 47, 55);
    }

    if in_rounded_rect(x, y, 16, 16, 288, 174, 14) {
        return rgb565(8, 37, 45);
    }
    if in_rounded_rect(x, y, 22, 210, 128, 68, 10) {
        return rgb565(173, 36, 48);
    }
    if in_rounded_rect(x, y, 170, 210, 128, 68, 10)
        || in_rounded_rect(x, y, 22, 292, 128, 68, 10)
        || in_rounded_rect(x, y, 170, 292, 128, 68, 10)
    {
        return rgb565(17, 31, 38);
    }

    if x < 320 && y < 480 {
        let shade = if ((x / 24) + (y / 24)) & 1 == 0 { 0 } else { 3 };
        return rgb565(5 + shade, 11 + shade, 15 + shade);
    }

    0
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn in_rounded_rect(px: u32, py: u32, x: u32, y: u32, w: u32, h: u32, radius: u32) -> bool {
    if px < x || py < y || px >= x.saturating_add(w) || py >= y.saturating_add(h) {
        return false;
    }
    let r = radius.min(w / 2).min(h / 2);
    let in_corner = (px < x + r || px >= x + w - r) && (py < y + r || py >= y + h - r);
    if !in_corner {
        return true;
    }
    let cx = if px < x + r { x + r } else { x + w - r - 1 };
    let cy = if py < y + r { y + r } else { y + h - r - 1 };
    let dx = px as i32 - cx as i32;
    let dy = py as i32 - cy as i32;
    dx * dx + dy * dy <= (r * r) as i32
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn text_pixel(px: u32, py: u32, x: u32, baseline_y: u32, font_size: u32, text: &str) -> bool {
    let scale = (font_size / 8).max(1);
    let glyph_w = 5 * scale;
    let glyph_h = 7 * scale;
    let top = baseline_y.saturating_sub(glyph_h);
    if py < top || py >= baseline_y {
        return false;
    }

    let mut cursor_x = x;
    for byte in text.bytes() {
        if px >= cursor_x && px < cursor_x.saturating_add(glyph_w) {
            let gx = (px - cursor_x) / scale;
            let gy = (py - top) / scale;
            let glyph = glyph_5x7(byte);
            return (glyph[gy as usize] & (1 << (4 - gx))) != 0;
        }
        cursor_x = cursor_x.saturating_add(glyph_w + scale);
    }
    false
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn paint_tile_commands(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    commands: &[edgerun_layout::UiRenderCommand],
) {
    for command in commands {
        match command {
            edgerun_layout::UiRenderCommand::FillRect { x, y, w, h, color } => {
                fill_rect_bgra(pixels, width, height, tile_y, *x, *y, *w, *h, *color);
            }
            edgerun_layout::UiRenderCommand::StrokeRect {
                x,
                y,
                w,
                h,
                color,
                style,
                thickness,
            } => {
                let t = (*thickness).max(1);
                fill_rect_bgra(pixels, width, height, tile_y, *x, *y, *w, t, *color);
                fill_rect_bgra(
                    pixels,
                    width,
                    height,
                    tile_y,
                    *x,
                    y + h.saturating_sub(t),
                    *w,
                    t,
                    *color,
                );
                fill_rect_bgra(pixels, width, height, tile_y, *x, *y, t, *h, *color);
                fill_rect_bgra(
                    pixels,
                    width,
                    height,
                    tile_y,
                    x + w.saturating_sub(t),
                    *y,
                    t,
                    *h,
                    *color,
                );
            }
            edgerun_layout::UiRenderCommand::Text {
                x,
                y,
                text,
                color,
                font_size,
            } => {
                draw_text_bgra(
                    pixels, width, height, tile_y, *x, *y, text, *color, *font_size,
                );
            }
            edgerun_layout::UiRenderCommand::RoundedRect {
                x,
                y,
                w,
                h,
                color,
                radius,
            } => {
                fill_rounded_rect_bgra(
                    pixels, width, height, tile_y, *x, *y, *w, *h, *radius, *color,
                );
            }
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn fill_rect_bgra(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    color: edgerun_layout::Color,
) {
    if w == 0 || h == 0 || color.a == 0 {
        return;
    }
    let x0 = x.min(width);
    let x1 = x.saturating_add(w).min(width);
    let y0 = y.max(tile_y);
    let y1 = y.saturating_add(h).min(tile_y.saturating_add(height));
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let mut py = y0;
    while py < y1 {
        let local_y = py - tile_y;
        let row = local_y as usize * width as usize * 4;
        let mut px = x0;
        while px < x1 {
            set_pixel_bgra(pixels, row + px as usize * 4, color);
            px += 1;
        }
        py += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn fill_rounded_rect_bgra(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    radius: u32,
    color: edgerun_layout::Color,
) {
    if radius == 0 {
        fill_rect_bgra(pixels, width, height, tile_y, x, y, w, h, color);
        return;
    }

    let x0 = x.min(width);
    let x1 = x.saturating_add(w).min(width);
    let y0 = y.max(tile_y);
    let y1 = y.saturating_add(h).min(tile_y.saturating_add(height));
    let r = radius.min(w / 2).min(h / 2);
    let r2 = (r * r) as i32;

    let mut py = y0;
    while py < y1 {
        let local_y = py - tile_y;
        let row = local_y as usize * width as usize * 4;
        let mut px = x0;
        while px < x1 {
            let in_corner = (px < x + r || px >= x + w - r) && (py < y + r || py >= y + h - r);
            let draw = if !in_corner {
                true
            } else {
                let cx = if px < x + r { x + r } else { x + w - r - 1 };
                let cy = if py < y + r { y + r } else { y + h - r - 1 };
                let dx = px as i32 - cx as i32;
                let dy = py as i32 - cy as i32;
                dx * dx + dy * dy <= r2
            };
            if draw {
                set_pixel_bgra(pixels, row + px as usize * 4, color);
            }
            px += 1;
        }
        py += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn draw_text_bgra(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    x: u32,
    baseline_y: u32,
    text: &str,
    color: edgerun_layout::Color,
    font_size: f32,
) {
    let scale = ((font_size as u32) / 8).max(1);
    let glyph_w = 5 * scale;
    let glyph_h = 7 * scale;
    let mut cursor_x = x;
    let top = baseline_y.saturating_sub(glyph_h);
    for byte in text.bytes() {
        draw_glyph_bgra(
            pixels, width, height, tile_y, cursor_x, top, scale, byte, color,
        );
        cursor_x = cursor_x.saturating_add(glyph_w + scale);
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn draw_glyph_bgra(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tile_y: u32,
    x: u32,
    y: u32,
    scale: u32,
    byte: u8,
    color: edgerun_layout::Color,
) {
    let glyph = glyph_5x7(byte);
    let mut gy = 0u32;
    while gy < 7 {
        let bits = glyph[gy as usize];
        let mut gx = 0u32;
        while gx < 5 {
            if (bits & (1 << (4 - gx))) != 0 {
                fill_rect_bgra(
                    pixels,
                    width,
                    height,
                    tile_y,
                    x + gx * scale,
                    y + gy * scale,
                    scale,
                    scale,
                    color,
                );
            }
            gx += 1;
        }
        gy += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn glyph_5x7(byte: u8) -> [u8; 7] {
    match byte {
        b'0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        b'1' => [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
        b'2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
        b'3' => [0x1e, 0x01, 0x01, 0x0e, 0x01, 0x01, 0x1e],
        b'4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        b'5' => [0x1f, 0x10, 0x10, 0x1e, 0x01, 0x01, 0x1e],
        b'6' => [0x0e, 0x10, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        b'7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        b'9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x01, 0x0e],
        b'A' | b'a' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        b'B' | b'b' => [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
        b'C' | b'c' => [0x0f, 0x10, 0x10, 0x10, 0x10, 0x10, 0x0f],
        b'D' | b'd' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        b'E' | b'e' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        b'F' | b'f' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        b'G' | b'g' => [0x0f, 0x10, 0x10, 0x13, 0x11, 0x11, 0x0f],
        b'H' | b'h' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        b'I' | b'i' => [0x0e, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0e],
        b'J' | b'j' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0e],
        b'K' | b'k' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'L' | b'l' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        b'M' | b'm' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' | b'n' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        b'O' | b'o' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        b'P' | b'p' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        b'Q' | b'q' => [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
        b'R' | b'r' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        b'S' | b's' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        b'T' | b't' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' | b'u' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        b'V' | b'v' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
        b'W' | b'w' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1b, 0x11],
        b'X' | b'x' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        b'Y' | b'y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
        b'Z' | b'z' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
        b'-' => [0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00],
        b':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
        b'=' => [0x00, 0x00, 0x1f, 0x00, 0x1f, 0x00, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x0c],
        b'/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        b'_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1f],
        b'[' => [0x0e, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0e],
        b']' => [0x0e, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0e],
        b' ' => [0x00; 7],
        _ => [0x1f, 0x11, 0x15, 0x15, 0x15, 0x11, 0x1f],
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn set_pixel_bgra(pixels: &mut [u8], i: usize, color: edgerun_layout::Color) {
    if i + 3 >= pixels.len() {
        return;
    }
    if color.a == 255 {
        pixels[i] = color.b;
        pixels[i + 1] = color.g;
        pixels[i + 2] = color.r;
        pixels[i + 3] = color.a;
        return;
    }
    let a = color.a as u32;
    let inv_a = 255 - a;
    pixels[i] = ((color.b as u32 * a + pixels[i] as u32 * inv_a) / 255) as u8;
    pixels[i + 1] = ((color.g as u32 * a + pixels[i + 1] as u32 * inv_a) / 255) as u8;
    pixels[i + 2] = ((color.r as u32 * a + pixels[i + 2] as u32 * inv_a) / 255) as u8;
    pixels[i + 3] = 255;
}

#[cfg(all(target_arch = "xtensa", target_os = "none", feature = "html-ui"))]
fn paint_touch_marker(pixels: &mut [u8], width: u32, height: u32, tile_y: u32, x: u32, y: u32) {
    if y.saturating_add(6) < tile_y || y > tile_y.saturating_add(height).saturating_add(6) {
        return;
    }

    let local_y = y.saturating_sub(tile_y);
    let x0 = x.saturating_sub(5);
    let y0 = local_y.saturating_sub(5);
    let x1 = x.saturating_add(6).min(width);
    let y1 = local_y.saturating_add(6).min(height);
    let mut py = y0;
    while py < y1 {
        let row = py as usize * width as usize * 4;
        let mut px = x0;
        while px < x1 {
            let i = row + px as usize * 4;
            pixels[i] = 0x00;
            pixels[i + 1] = 0x00;
            pixels[i + 2] = 0xff;
            pixels[i + 3] = 0xff;
            px += 1;
        }
        py += 1;
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn render_debug_pattern(touch: Option<(u16, u16)>) {
    unsafe {
        edgerun_platform::esp32s3::Jc3248w535Display::draw_rgb565_with(320, 480, |x, y| {
            if let Some((tx, ty)) = touch {
                let dx = x.abs_diff(tx);
                let dy = y.abs_diff(ty);
                if dx <= 5 && dy <= 5 {
                    return 0xf800;
                }
            }
            if let Some(color) = console_pixel(x, y) {
                return color;
            }
            debug_pixel(x, y)
        });
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    (((r as u16) & 0xf8) << 8) | (((g as u16) & 0xfc) << 3) | (b as u16 >> 3)
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn debug_pixel(x: u16, y: u16) -> u16 {
    if x == 0 || y == 0 || x == 319 || y == 479 || x == 159 || y == 239 {
        return 0x0000;
    }

    if y < 80 {
        return match x / 40 {
            0 => 0xf800,
            1 => 0x07e0,
            2 => 0x001f,
            3 => 0xffe0,
            4 => 0xf81f,
            5 => 0x07ff,
            6 => 0xffff,
            _ => 0x0000,
        };
    }

    if y < 160 {
        return rgb565(
            (x as u32 * 255 / 319) as u8,
            32,
            255u8.saturating_sub((x as u32 * 255 / 319) as u8),
        );
    }

    if y < 240 {
        return rgb565(32, (x as u32 * 255 / 319) as u8, 32);
    }

    if y < 320 {
        let shade = (((x / 16) + (y / 16)) & 1) as u8;
        return if shade == 0 { 0xffff } else { 0x8410 };
    }

    if y < 400 {
        return rgb565(
            (y as u32 * 255 / 479) as u8,
            (x as u32 * 255 / 319) as u8,
            96,
        );
    }

    if x < 80 {
        0x001f
    } else if x < 160 {
        0x07e0
    } else if x < 240 {
        0xf800
    } else {
        0xffff
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
const fn fixed_cstr<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    let mut i = 0;
    while i < bytes.len() && i + 1 < N {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[used]
#[no_mangle]
#[link_section = ".flash.appdesc"]
static esp_app_desc: EspAppDesc = EspAppDesc {
    magic_word: 0xABCD_5432,
    secure_version: 0,
    reserv1: [0; 2],
    version: fixed_cstr(b"0.1.0"),
    project_name: fixed_cstr(b"edgerun-unikernel"),
    time: fixed_cstr(b"00:00:00"),
    date: fixed_cstr(b"2026-04-28"),
    idf_ver: fixed_cstr(b"edgerun-bare"),
    app_elf_sha256: [0; 32],
    min_efuse_blk_rev_full: 0,
    max_efuse_blk_rev_full: u16::MAX,
    mmu_page_size: 16,
    reserv3: [0; 3],
    reserv2: [0; 18],
};

#[cfg(target_arch = "x86_64")]
use edgerun_dhcp::message::{DHCP_CLIENT_PORT, DHCP_SERVER_PORT};
#[cfg(target_arch = "x86_64")]
use edgerun_dhcp::{DhcpMessage, DhcpMessageType};
#[cfg(target_arch = "x86_64")]
use edgerun_tftp::message::{TftpMessage, TFTP_PORT};
#[cfg(target_arch = "x86_64")]
use rt::ip::{ParsedPacket, ARP_OP_REQUEST, ICMP_ECHO_REQUEST};
#[cfg(target_arch = "x86_64")]
use rt::{block_on, crc32, IpAddr, IpStack, Network, RingBuffer, Rng, TcpSocket};

#[cfg(target_arch = "x86_64")]
use core::future::Future;
#[cfg(target_arch = "x86_64")]
use core::pin::Pin;
#[cfg(target_arch = "x86_64")]
use core::sync::atomic::{AtomicPtr, Ordering};
#[cfg(target_arch = "x86_64")]
use core::task::{Context, Poll};

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[allow(dead_code)]
mod oci_syscall {
    use super::rt;
    use edgerun_oci::prelude::String;
    use edgerun_oci::rootfs_access::OciRootfs;
    use edgerun_oci::{
        dispatch_x86_64_linux_syscall_frame, prepare_and_load_oci_elf_program_with_load_bias,
        OciElfError, OciElfLoadBias, OciElfUnsafeIdentityMapper, OciPreparedLaunchState,
        OciSyscallAction, OciSyscallError, OciSyscallMemory, OciSyscallSink, OciX86_64SyscallFrame,
    };
    use edgerun_platform::arch::x86_64::{
        self, SyscallFrame, KERNEL_CODE_SELECTOR, USER_COMPAT_CODE_SELECTOR,
    };

    struct DirectMemory;

    impl OciSyscallMemory for DirectMemory {
        fn read_bytes(&self, addr: u64, len: usize, out: &mut [u8]) -> Result<(), OciSyscallError> {
            let ptr = usize::try_from(addr).map_err(|_| OciSyscallError::BadAddress)? as *const u8;
            let Some(out) = out.get_mut(..len) else {
                return Err(OciSyscallError::BadAddress);
            };
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, out.as_mut_ptr(), len);
            }
            Ok(())
        }
    }

    struct LogSink;

    impl OciSyscallSink for LogSink {
        fn write_fd(&mut self, fd: u64, bytes: &[u8]) -> Result<usize, OciSyscallError> {
            if fd == 1 || fd == 2 {
                if let Ok(text) = core::str::from_utf8(bytes) {
                    rt::log::log(1, text.trim_end_matches('\n'));
                } else {
                    rt::log::log(1, "container wrote non-UTF8 bytes");
                }
            }
            Ok(bytes.len())
        }
    }

    pub unsafe fn install(kernel_code_selector: u16, user_code_selector: u16) {
        x86_64::set_syscall_handler(handle_syscall);
        unsafe {
            x86_64::enable_syscall_entry(kernel_code_selector, user_code_selector);
        }
    }

    pub unsafe fn install_flat_gdt() {
        unsafe {
            x86_64::load_flat_gdt();
            install(KERNEL_CODE_SELECTOR, USER_COMPAT_CODE_SELECTOR);
        }
    }

    pub unsafe fn enter_launch_state(launch: &OciPreparedLaunchState) -> ! {
        unsafe {
            install_flat_gdt();
            x86_64::enter_user64(launch.entry_point, launch.stack_pointer);
        }
    }

    pub unsafe fn launch_rootfs<R: OciRootfs>(
        rootfs: &R,
        args: &[String],
        env: &[String],
        cwd: &str,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciElfError> {
        let mut mapper = OciElfUnsafeIdentityMapper::new();
        let Some(launch) = prepare_and_load_oci_elf_program_with_load_bias(
            rootfs,
            args,
            env,
            cwd,
            &mut mapper,
            scratch,
            stack_base,
            stack_top,
            stack,
            load_bias,
        )?
        else {
            return Err(OciElfError::NotFound(
                args.first().cloned().unwrap_or_default(),
            ));
        };

        unsafe {
            enter_launch_state(&launch);
        }
    }

    extern "C" fn handle_syscall(frame: &mut SyscallFrame) {
        let mut oci_frame = OciX86_64SyscallFrame {
            rax: frame.rax,
            rdi: frame.rdi,
            rsi: frame.rsi,
            rdx: frame.rdx,
            r10: frame.r10,
            r8: frame.r8,
            r9: frame.r9,
        };
        let memory = DirectMemory;
        let mut sink = LogSink;
        let mut scratch = [0u8; 256];

        match dispatch_x86_64_linux_syscall_frame(&memory, &mut sink, &mut scratch, &mut oci_frame)
        {
            Ok(OciSyscallAction::Return(_)) => {
                frame.rax = oci_frame.rax;
            }
            Ok(OciSyscallAction::Exit(code)) => {
                let _ = code;
                rt::log::log(1, "container exited");
                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
            Err(OciSyscallError::Unsupported(_)) => {
                frame.rax = (-38i64) as u64;
            }
            Err(OciSyscallError::BadAddress) => {
                frame.rax = (-14i64) as u64;
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[allow(dead_code)]
mod oci_image_boot {
    use super::boot_config::BootConfig;
    use super::disk_boot::{self, DiskBootError, OpenedEdgeFs};
    use super::oci_syscall;
    use edgerun_edgefs::EdgeFs;
    use edgerun_oci::prelude::String;
    use edgerun_oci::{
        EdgeFsImagePullReport, ImageRef, OciElfError, OciElfLoadBias, RegistryClient, RegistryError,
    };
    use edgerun_storage::{BlockStorage, PartitionBlockDevice};

    #[derive(Debug)]
    pub enum OciImageBootError {
        InvalidImageRef(String),
        Registry(RegistryError),
        Elf(OciElfError),
    }

    #[derive(Debug)]
    pub enum OciDiskImageBootError {
        Disk(DiskBootError),
        Image(OciImageBootError),
    }

    pub enum PulledConfiguredEdgeFs<S: BlockStorage> {
        Partition {
            fs: EdgeFs<PartitionBlockDevice<S>>,
            report: EdgeFsImagePullReport,
            config: BootConfig,
        },
        WholeDisk {
            fs: EdgeFs<S>,
            report: EdgeFsImagePullReport,
            config: BootConfig,
        },
    }

    impl From<RegistryError> for OciImageBootError {
        fn from(error: RegistryError) -> Self {
            Self::Registry(error)
        }
    }

    impl From<OciElfError> for OciImageBootError {
        fn from(error: OciElfError) -> Self {
            Self::Elf(error)
        }
    }

    pub fn image_boot_error_label(error: &OciImageBootError) -> &'static str {
        match error {
            OciImageBootError::InvalidImageRef(_) => "invalid image ref",
            OciImageBootError::Registry(RegistryError::HttpStatus(_)) => "registry http status",
            OciImageBootError::Registry(RegistryError::HttpError(_)) => "registry http error",
            OciImageBootError::Registry(RegistryError::AuthError(_)) => "registry auth error",
            OciImageBootError::Registry(RegistryError::ManifestNotFound(_)) => {
                "registry manifest not found"
            }
            OciImageBootError::Registry(RegistryError::NoManifests) => "registry no manifests",
            OciImageBootError::Registry(RegistryError::DigestMismatch { .. }) => {
                "registry digest mismatch"
            }
            OciImageBootError::Registry(RegistryError::ParseError(_)) => "registry parse error",
            OciImageBootError::Elf(_) => "elf error",
        }
    }

    pub fn image_boot_error_detail(error: &OciImageBootError) -> Option<&str> {
        match error {
            OciImageBootError::InvalidImageRef(detail) => Some(detail.as_str()),
            OciImageBootError::Registry(RegistryError::HttpError(detail)) => Some(detail.as_str()),
            OciImageBootError::Registry(RegistryError::AuthError(detail)) => Some(detail.as_str()),
            OciImageBootError::Registry(RegistryError::ManifestNotFound(detail)) => {
                Some(detail.as_str())
            }
            OciImageBootError::Registry(RegistryError::ParseError(detail)) => Some(detail.as_str()),
            _ => None,
        }
    }

    impl From<DiskBootError> for OciDiskImageBootError {
        fn from(error: DiskBootError) -> Self {
            Self::Disk(error)
        }
    }

    impl From<OciImageBootError> for OciDiskImageBootError {
        fn from(error: OciImageBootError) -> Self {
            Self::Image(error)
        }
    }

    pub async unsafe fn pull_image_into_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        image_ref: &str,
        rootfs_path: &str,
        registry_insecure_http: bool,
    ) -> Result<edgerun_oci::EdgeFsImagePullReport, OciImageBootError> {
        let image: ImageRef = image_ref
            .parse()
            .map_err(OciImageBootError::InvalidImageRef)?;
        let mut client = RegistryClient::new();
        if registry_insecure_http {
            client = client.insecure_http();
        }
        client
            .pull_into_edgefs(&image, rootfs_path, fs)
            .await
            .map_err(Into::into)
    }

    pub async unsafe fn pull_configured_image_into_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        config: &BootConfig,
    ) -> Result<edgerun_oci::EdgeFsImagePullReport, OciImageBootError> {
        unsafe {
            pull_image_into_edgefs(
                fs,
                &config.image,
                &config.rootfs_path,
                config.registry_insecure_http,
            )
            .await
        }
    }

    pub async unsafe fn pull_configured_disk_image<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<PulledConfiguredEdgeFs<S>, OciDiskImageBootError> {
        let mut device = device;
        let config = disk_boot::read_default_fat_boot_config_mut(&mut device)?;
        let opened = disk_boot::open_configured_edgefs(device, &config, key, fs_id)?;

        match opened {
            OpenedEdgeFs::Partition(mut fs) => {
                let report = unsafe { pull_configured_image_into_edgefs(&mut fs, &config).await? };
                Ok(PulledConfiguredEdgeFs::Partition { fs, report, config })
            }
            OpenedEdgeFs::WholeDisk(mut fs) => {
                let report = unsafe { pull_configured_image_into_edgefs(&mut fs, &config).await? };
                Ok(PulledConfiguredEdgeFs::WholeDisk { fs, report, config })
            }
        }
    }

    pub unsafe fn launch_pulled_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        report: &EdgeFsImagePullReport,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        let runtime = &report.plan.runtime;
        unsafe {
            oci_syscall::launch_rootfs(
                fs,
                &runtime.args,
                &runtime.env,
                runtime.cwd.as_str(),
                scratch,
                stack_base,
                stack_top,
                stack,
                load_bias,
            )
            .map_err(Into::into)
        }
    }

    pub async unsafe fn pull_and_launch_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        image_ref: &str,
        rootfs_path: &str,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        let report = unsafe { pull_image_into_edgefs(fs, image_ref, rootfs_path, false).await? };
        unsafe {
            launch_pulled_edgefs(
                fs, &report, scratch, stack_base, stack_top, stack, load_bias,
            )
        }
    }

    pub async unsafe fn pull_and_launch_configured_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        config: &BootConfig,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        unsafe {
            pull_and_launch_edgefs(
                fs,
                &config.image,
                &config.rootfs_path,
                scratch,
                stack_base,
                stack_top,
                stack,
                load_bias,
            )
            .await
        }
    }

    pub async unsafe fn pull_and_launch_configured_disk_image<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciDiskImageBootError> {
        let pulled = unsafe { pull_configured_disk_image(device, key, fs_id).await? };
        match pulled {
            PulledConfiguredEdgeFs::Partition { mut fs, report, .. } => unsafe {
                launch_pulled_edgefs(
                    &mut fs, &report, scratch, stack_base, stack_top, stack, load_bias,
                )
                .map_err(Into::into)
            },
            PulledConfiguredEdgeFs::WholeDisk { mut fs, report, .. } => unsafe {
                launch_pulled_edgefs(
                    &mut fs, &report, scratch, stack_base, stack_top, stack, load_bias,
                )
                .map_err(Into::into)
            },
        }
    }
}

#[cfg(any(target_os = "none", test))]
#[allow(dead_code)]
mod boot_config {
    use alloc::string::{String, ToString};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EdgeFsBootTarget {
        ExistingPartition,
        FormatFirstPartition,
        FormatDataPartition,
        FormatWholeDisk,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct BootConfig {
        pub image: String,
        pub edgefs: EdgeFsBootTarget,
        pub rootfs_path: String,
        pub http_smoke: Option<String>,
        pub oci_pull: bool,
        pub registry_insecure_http: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BootConfigError {
        InvalidUtf8,
        InvalidLine(usize),
        UnknownKey(String),
        InvalidEdgeFsTarget(String),
        InvalidRootfsPath(String),
        MissingImage,
    }

    impl core::fmt::Display for BootConfigError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::InvalidUtf8 => f.write_str("boot config is not valid UTF-8"),
                Self::InvalidLine(line) => write!(f, "invalid boot config line {line}"),
                Self::UnknownKey(key) => write!(f, "unknown boot config key: {key}"),
                Self::InvalidEdgeFsTarget(target) => {
                    write!(f, "invalid edgefs boot target: {target}")
                }
                Self::InvalidRootfsPath(path) => write!(f, "invalid rootfs path: {path}"),
                Self::MissingImage => f.write_str("boot config is missing image"),
            }
        }
    }

    impl core::error::Error for BootConfigError {}

    impl BootConfig {
        pub fn parse(bytes: &[u8]) -> Result<Self, BootConfigError> {
            let text = core::str::from_utf8(bytes).map_err(|_| BootConfigError::InvalidUtf8)?;
            let mut image = None;
            let mut edgefs = EdgeFsBootTarget::ExistingPartition;
            let mut rootfs_path = "/".to_string();
            let mut http_smoke = None;
            let mut oci_pull = false;
            let mut registry_insecure_http = false;

            for (index, raw_line) in text.lines().enumerate() {
                let line_no = index + 1;
                let line = raw_line
                    .split_once('#')
                    .map_or(raw_line, |(line, _)| line)
                    .trim();
                if line.is_empty() {
                    continue;
                }

                let Some((key, value)) = line.split_once('=') else {
                    return Err(BootConfigError::InvalidLine(line_no));
                };
                let key = key.trim();
                let value = value.trim();
                if value.is_empty() {
                    return Err(BootConfigError::InvalidLine(line_no));
                }

                match key {
                    "image" | "oci_image" | "ref" => image = Some(value.to_string()),
                    "edgefs" => edgefs = parse_edgefs_target(value)?,
                    "rootfs" | "rootfs_path" => rootfs_path = parse_rootfs_path(value)?,
                    "http_smoke" | "http_smoke_url" => http_smoke = Some(value.to_string()),
                    "oci_pull" | "pull_image" => oci_pull = parse_bool(value, line_no)?,
                    "registry_insecure_http" | "plain_http_registry" => {
                        registry_insecure_http = parse_bool(value, line_no)?
                    }
                    other => return Err(BootConfigError::UnknownKey(other.to_string())),
                }
            }

            let image = image.ok_or(BootConfigError::MissingImage)?;
            Ok(Self {
                image,
                edgefs,
                rootfs_path,
                http_smoke,
                oci_pull,
                registry_insecure_http,
            })
        }
    }

    fn parse_bool(value: &str, line_no: usize) -> Result<bool, BootConfigError> {
        match value {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(BootConfigError::InvalidLine(line_no)),
        }
    }

    fn parse_edgefs_target(value: &str) -> Result<EdgeFsBootTarget, BootConfigError> {
        match value {
            "partition" | "existing-partition" | "existing_partition" => {
                Ok(EdgeFsBootTarget::ExistingPartition)
            }
            "format-partition" | "format_partition" | "first-partition" | "first_partition" => {
                Ok(EdgeFsBootTarget::FormatFirstPartition)
            }
            "format-data-partition"
            | "format_data_partition"
            | "data-partition"
            | "data_partition"
            | "first-data-partition"
            | "first_data_partition" => Ok(EdgeFsBootTarget::FormatDataPartition),
            "whole-disk" | "whole_disk" | "format-whole-disk" | "format_whole_disk" => {
                Ok(EdgeFsBootTarget::FormatWholeDisk)
            }
            other => Err(BootConfigError::InvalidEdgeFsTarget(other.to_string())),
        }
    }

    fn parse_rootfs_path(value: &str) -> Result<String, BootConfigError> {
        if !value.starts_with('/') || value.as_bytes().contains(&0) {
            return Err(BootConfigError::InvalidRootfsPath(value.to_string()));
        }

        let without_root = &value[1..];
        if without_root.is_empty() {
            return Ok(value.to_string());
        }

        for component in without_root.split('/') {
            if component.is_empty() || component == "." || component == ".." {
                return Err(BootConfigError::InvalidRootfsPath(value.to_string()));
            }
        }

        Ok(value.to_string())
    }

    #[cfg(test)]
    mod tests {
        use super::{BootConfig, EdgeFsBootTarget};

        #[test]
        fn parses_required_image_and_defaults() {
            let config = BootConfig::parse(b"# edgeOS\nimage=registry.local/app:latest\n").unwrap();
            assert_eq!(config.image, "registry.local/app:latest");
            assert_eq!(config.edgefs, EdgeFsBootTarget::ExistingPartition);
            assert_eq!(config.rootfs_path, "/");
            assert_eq!(config.http_smoke, None);
            assert!(!config.oci_pull);
            assert!(!config.registry_insecure_http);
        }

        #[test]
        fn parses_edgefs_and_rootfs_options() {
            let config = BootConfig::parse(
                b"oci_image = example.com/ns/app:v1\nedgefs = whole-disk\nrootfs_path = /apps/app\n",
            )
            .unwrap();
            assert_eq!(config.image, "example.com/ns/app:v1");
            assert_eq!(config.edgefs, EdgeFsBootTarget::FormatWholeDisk);
            assert_eq!(config.rootfs_path, "/apps/app");
        }

        #[test]
        fn parses_data_partition_target() {
            let config =
                BootConfig::parse(b"image=registry.local/app:v1\nedgefs=format-data-partition\n")
                    .unwrap();

            assert_eq!(config.edgefs, EdgeFsBootTarget::FormatDataPartition);
        }

        #[test]
        fn parses_http_smoke_url() {
            let config = BootConfig::parse(
                b"image=registry.local/app:v1\nhttp_smoke=http://10.0.2.2:18080/edgerun-smoke\n",
            )
            .unwrap();

            assert_eq!(
                config.http_smoke.as_deref(),
                Some("http://10.0.2.2:18080/edgerun-smoke")
            );
        }

        #[test]
        fn parses_oci_pull_options() {
            let config = BootConfig::parse(
                b"image=10.0.2.2:18080/edge/app:v1\noci_pull=true\nregistry_insecure_http=yes\n",
            )
            .unwrap();

            assert!(config.oci_pull);
            assert!(config.registry_insecure_http);
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[allow(dead_code)]
mod disk_boot {
    use super::boot_config::{BootConfig, BootConfigError, EdgeFsBootTarget};
    use edgerun_edgefs::{EdgeFs, EdgeFsError, EdgeFsInfo};
    use edgerun_storage::{
        detect_partitions, probe_filesystem, BlockStorage, FatError, FatReadOnly, FileSystemKind,
        FileSystemProbe, FileSystemProbeError, PartitionBlockDevice, PartitionEntry,
        PartitionError, PartitionTable, StorageError,
    };

    pub const DEFAULT_BOOT_CONFIG_PATHS: &[&str] =
        &["/edgerun/boot.cfg", "/EDGERUN/BOOT.CFG", "/boot.cfg"];

    #[derive(Debug)]
    pub enum DiskBootError {
        Partition(PartitionError),
        Probe(FileSystemProbeError),
        Fat(FatError),
        BootConfig(BootConfigError),
        EdgeFs(EdgeFsError),
        NoEdgeFsPartition,
        NoFatPartition,
        NoBootConfig,
        NoPartition,
    }

    impl From<PartitionError> for DiskBootError {
        fn from(error: PartitionError) -> Self {
            Self::Partition(error)
        }
    }

    impl From<EdgeFsError> for DiskBootError {
        fn from(error: EdgeFsError) -> Self {
            Self::EdgeFs(error)
        }
    }

    impl From<FileSystemProbeError> for DiskBootError {
        fn from(error: FileSystemProbeError) -> Self {
            Self::Probe(error)
        }
    }

    impl From<FatError> for DiskBootError {
        fn from(error: FatError) -> Self {
            Self::Fat(error)
        }
    }

    impl From<BootConfigError> for DiskBootError {
        fn from(error: BootConfigError) -> Self {
            Self::BootConfig(error)
        }
    }

    #[derive(Debug, Clone)]
    pub struct DiskPartitionProbe {
        pub partition: PartitionEntry,
        pub filesystem: FileSystemProbe,
    }

    #[derive(Debug, Clone)]
    pub struct EdgeFsPartition {
        pub partition: PartitionEntry,
        pub info: EdgeFsInfo,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MountPlanKind {
        EdgeFsReadWrite,
        ForeignReadOnly,
        FutureNative,
        Unknown,
    }

    #[derive(Debug, Clone)]
    pub struct MountPlanEntry {
        pub partition: PartitionEntry,
        pub filesystem: FileSystemProbe,
        pub edgefs: Option<EdgeFsInfo>,
        pub kind: MountPlanKind,
    }

    #[derive(Debug, Clone)]
    pub struct DiskMountPlan {
        pub table: PartitionTable,
        pub entries: alloc::vec::Vec<MountPlanEntry>,
    }

    pub enum OpenedEdgeFs<S: BlockStorage> {
        Partition(EdgeFs<PartitionBlockDevice<S>>),
        WholeDisk(EdgeFs<S>),
    }

    pub struct RtBlockDeviceStorage<T> {
        device: T,
    }

    impl<T> RtBlockDeviceStorage<T> {
        pub fn new(device: T) -> Self {
            Self { device }
        }

        pub fn into_inner(self) -> T {
            self.device
        }
    }

    impl BlockStorage for RtBlockDeviceStorage<edgerun_virtio::VirtBlk> {
        fn sector_size(&self) -> usize {
            edgerun_virtio::SECTOR_SIZE
        }

        fn sectors(&self) -> u64 {
            self.device.sectors()
        }

        fn read_sector(
            &mut self,
            sector: u64,
            buf: &mut [u8],
        ) -> core::result::Result<(), StorageError> {
            if buf.len() != self.sector_size() {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::InvalidInput,
                    "sector buffer size mismatch",
                )));
            }
            if self.device.read_sector(sector, buf) {
                Ok(())
            } else {
                Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::Other,
                    "bare block read failed",
                )))
            }
        }

        fn write_sector(
            &mut self,
            sector: u64,
            buf: &[u8],
        ) -> core::result::Result<(), StorageError> {
            if buf.len() != self.sector_size() {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::InvalidInput,
                    "sector buffer size mismatch",
                )));
            }
            if self.device.write_sector(sector, buf) {
                Ok(())
            } else {
                Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::Other,
                    "bare block write failed",
                )))
            }
        }
    }

    pub struct RtPartitionStorage<'a, S: BlockStorage> {
        parent: &'a mut S,
        start_lba: u64,
        sectors: u64,
    }

    impl<'a, S: BlockStorage> RtPartitionStorage<'a, S> {
        pub fn new(parent: &'a mut S, start_lba: u64, sectors: u64) -> Self {
            Self {
                parent,
                start_lba,
                sectors,
            }
        }
    }

    impl<S: BlockStorage> BlockStorage for RtPartitionStorage<'_, S> {
        fn sector_size(&self) -> usize {
            BlockStorage::sector_size(&*self.parent)
        }

        fn sectors(&self) -> u64 {
            self.sectors
        }

        fn read_sector(
            &mut self,
            sector: u64,
            buf: &mut [u8],
        ) -> core::result::Result<(), StorageError> {
            if sector >= self.sectors {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::UnexpectedEof,
                    "partition sector index out of range",
                )));
            }
            let Some(parent_sector) = self.start_lba.checked_add(sector) else {
                return Err(StorageError::Io(edgerun_storage::io::Error::other(
                    "partition sector overflow",
                )));
            };
            BlockStorage::read_sector(&mut *self.parent, parent_sector, buf)
        }

        fn write_sector(
            &mut self,
            sector: u64,
            buf: &[u8],
        ) -> core::result::Result<(), StorageError> {
            if sector >= self.sectors {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::UnexpectedEof,
                    "partition sector index out of range",
                )));
            }
            let Some(parent_sector) = self.start_lba.checked_add(sector) else {
                return Err(StorageError::Io(edgerun_storage::io::Error::other(
                    "partition sector overflow",
                )));
            };
            BlockStorage::write_sector(&mut *self.parent, parent_sector, buf)
        }

        fn sync(&mut self) -> core::result::Result<(), StorageError> {
            BlockStorage::sync(&mut *self.parent)
        }
    }

    pub fn scan_partition_filesystems<S: BlockStorage>(
        device: &mut S,
    ) -> Result<(PartitionTable, alloc::vec::Vec<DiskPartitionProbe>), DiskBootError> {
        let table = detect_partitions(device)?;
        let mut probes = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let mut slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            probes.push(DiskPartitionProbe {
                partition: partition.clone(),
                filesystem: probe_filesystem(&mut slice)?,
            });
        }
        Ok((table, probes))
    }

    pub fn plan_partition_mounts<S: BlockStorage>(
        device: &mut S,
        key: [u8; 32],
    ) -> Result<DiskMountPlan, DiskBootError> {
        let table = detect_partitions(device)?;
        let mut entries = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let mut slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            let filesystem = probe_filesystem(&mut slice)?;

            let edgefs = if filesystem.kind == FileSystemKind::EdgeFs {
                let slice = PartitionBlockDevice::new(
                    &mut *device,
                    partition.start_lba,
                    partition.sectors,
                )?;
                EdgeFs::probe(slice, key).ok()
            } else {
                None
            };

            let kind = match (filesystem.kind, edgefs.is_some()) {
                (FileSystemKind::EdgeFs, true) => MountPlanKind::EdgeFsReadWrite,
                (FileSystemKind::Fat12 | FileSystemKind::Fat16 | FileSystemKind::Fat32, _) => {
                    MountPlanKind::ForeignReadOnly
                }
                (FileSystemKind::ExFat | FileSystemKind::Iso9660, _) => {
                    MountPlanKind::ForeignReadOnly
                }
                (FileSystemKind::Ext, _) => MountPlanKind::FutureNative,
                _ => MountPlanKind::Unknown,
            };

            entries.push(MountPlanEntry {
                partition: partition.clone(),
                filesystem,
                edgefs,
                kind,
            });
        }

        Ok(DiskMountPlan { table, entries })
    }

    pub fn scan_edgefs_partitions<S: BlockStorage>(
        device: &mut S,
        key: [u8; 32],
    ) -> Result<(PartitionTable, alloc::vec::Vec<EdgeFsPartition>), DiskBootError> {
        let table = detect_partitions(device)?;
        let mut matches = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            if let Ok(info) = EdgeFs::probe(slice, key) {
                matches.push(EdgeFsPartition {
                    partition: partition.clone(),
                    info,
                });
            }
        }
        Ok((table, matches))
    }

    pub fn open_first_edgefs_partition<S: BlockStorage>(
        device: S,
        key: [u8; 32],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let (_, matches) = scan_edgefs_partitions(&mut scan_device, key)?;
        let Some(first) = matches.first() else {
            return Err(DiskBootError::NoEdgeFsPartition);
        };
        let partition = PartitionBlockDevice::new(
            scan_device,
            first.partition.start_lba,
            first.partition.sectors,
        )?;
        EdgeFs::open(partition, key).map_err(Into::into)
    }

    pub fn open_whole_disk_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
    ) -> Result<EdgeFs<S>, DiskBootError> {
        EdgeFs::open(device, key).map_err(Into::into)
    }

    pub fn open_first_fat_partition_readonly<S: BlockStorage>(
        device: S,
    ) -> Result<FatReadOnly<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let (_, probes) = scan_partition_filesystems(&mut scan_device)?;
        let Some(first) = probes.iter().find(|probe| {
            matches!(
                probe.filesystem.kind,
                FileSystemKind::Fat12 | FileSystemKind::Fat16 | FileSystemKind::Fat32
            )
        }) else {
            return Err(DiskBootError::NoFatPartition);
        };
        let partition = PartitionBlockDevice::new(
            scan_device,
            first.partition.start_lba,
            first.partition.sectors,
        )?;
        FatReadOnly::open(partition).map_err(Into::into)
    }

    pub fn read_first_fat_file_8_3<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<alloc::vec::Vec<u8>, DiskBootError> {
        read_first_fat_file(device, path)
    }

    pub fn read_first_fat_file<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<alloc::vec::Vec<u8>, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(device)?;
        fat.read_file(path).map_err(Into::into)
    }

    pub fn read_first_fat_boot_config<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<BootConfig, DiskBootError> {
        let bytes = read_first_fat_file_8_3(device, path)?;
        BootConfig::parse(&bytes).map_err(Into::into)
    }

    pub fn read_default_fat_boot_config<S: BlockStorage>(
        device: S,
    ) -> Result<BootConfig, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(device)?;
        for path in DEFAULT_BOOT_CONFIG_PATHS {
            match fat.read_file(path) {
                Ok(bytes) => return BootConfig::parse(&bytes).map_err(Into::into),
                Err(FatError::NotFound(_)) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(DiskBootError::NoBootConfig)
    }

    pub fn read_default_fat_boot_config_mut<S: BlockStorage>(
        device: &mut S,
    ) -> Result<BootConfig, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(&mut *device)?;
        for path in DEFAULT_BOOT_CONFIG_PATHS {
            match fat.read_file(path) {
                Ok(bytes) => return BootConfig::parse(&bytes).map_err(Into::into),
                Err(FatError::NotFound(_)) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(DiskBootError::NoBootConfig)
    }

    pub fn open_configured_edgefs<S: BlockStorage>(
        device: S,
        config: &BootConfig,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<OpenedEdgeFs<S>, DiskBootError> {
        match config.edgefs {
            EdgeFsBootTarget::ExistingPartition => {
                open_first_edgefs_partition(device, key).map(OpenedEdgeFs::Partition)
            }
            EdgeFsBootTarget::FormatFirstPartition => {
                format_first_partition_as_edgefs(device, key, fs_id).map(OpenedEdgeFs::Partition)
            }
            EdgeFsBootTarget::FormatDataPartition => {
                format_first_data_partition_as_edgefs(device, key, fs_id)
                    .map(OpenedEdgeFs::Partition)
            }
            EdgeFsBootTarget::FormatWholeDisk => {
                format_whole_disk_edgefs(device, key, fs_id).map(OpenedEdgeFs::WholeDisk)
            }
        }
    }

    pub fn format_partition_as_edgefs<S: BlockStorage>(
        device: S,
        partition: &PartitionEntry,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let partition = PartitionBlockDevice::new(device, partition.start_lba, partition.sectors)?;
        EdgeFs::format_with_id(partition, key, fs_id).map_err(Into::into)
    }

    pub fn format_first_partition_as_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let table = detect_partitions(&mut scan_device)?;
        let Some(first) = table.partitions.first() else {
            return Err(DiskBootError::NoPartition);
        };
        format_partition_as_edgefs(scan_device, first, key, fs_id)
    }

    pub fn format_first_data_partition_as_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let (_, probes) = scan_partition_filesystems(&mut scan_device)?;
        let Some(first) = probes.iter().find(|probe| {
            !matches!(
                probe.filesystem.kind,
                FileSystemKind::Fat12
                    | FileSystemKind::Fat16
                    | FileSystemKind::Fat32
                    | FileSystemKind::ExFat
                    | FileSystemKind::Iso9660
            )
        }) else {
            return Err(DiskBootError::NoPartition);
        };
        format_partition_as_edgefs(scan_device, &first.partition, key, fs_id)
    }

    pub fn format_whole_disk_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<S>, DiskBootError> {
        EdgeFs::format_with_id(device, key, fs_id).map_err(Into::into)
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
core::arch::global_asm!(
    r#"
    .section .text.entry,"ax"
    .global _start
_start:
    lea rsp, [rip + _stack]
    xor rbp, rbp

    lea rdi, [rip + _bss_start]
    lea rcx, [rip + _bss_end]
    sub rcx, rdi
    xor eax, eax
    rep stosb

    call kernel_main

1:
    hlt
    jmp 1b
"#
);

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
core::arch::global_asm!(
    r#"
    .section .text.entry,"ax"
    .align 4
.Lstack_ptr:
    .word _stack
.Lbss_start_ptr:
    .word _bss_start
.Lbss_end_ptr:
    .word _bss_end
.Luart0_fifo_ptr:
    .word 0x60000000
.Lusb_ep1_ptr:
    .word 0x60038000
.Lusb_ep1_conf_ptr:
    .word 0x60038004
.Lusb_conf0_ptr:
    .word 0x60038018
.Lsystem_perip_clk_en1_ptr:
    .word 0x600c001c
.Lsystem_perip_rst_en1_ptr:
    .word 0x600c0024
.Lusb_conf0_default:
    .word 0x4200
.Lkernel_main_ptr:
    .word kernel_main

    .global _start
_start:
    l32r a5, .Lsystem_perip_clk_en1_ptr
    l32i a6, a5, 0
    movi a7, 1
    slli a7, a7, 10
    or a6, a6, a7
    s32i a6, a5, 0
    l32r a5, .Lsystem_perip_rst_en1_ptr
    l32i a6, a5, 0
    movi a7, 1
    slli a7, a7, 10
    movi a8, -1
    xor a7, a7, a8
    and a6, a6, a7
    s32i a6, a5, 0
    l32r a5, .Lusb_conf0_ptr
    l32r a6, .Lusb_conf0_default
    s32i a6, a5, 0

    l32r a5, .Luart0_fifo_ptr
    movi a6, 69
    s32i a6, a5, 0
    movi a6, 82
    s32i a6, a5, 0
    movi a6, 10
    s32i a6, a5, 0

    l32r a5, .Lusb_ep1_ptr
    l32r a7, .Lusb_ep1_conf_ptr
    movi a6, 69
    s32i a6, a5, 0
    movi a6, 85
    s32i a6, a5, 0
    movi a6, 10
    s32i a6, a5, 0
    l32i a6, a7, 0
    movi a8, 1
    or a6, a6, a8
    s32i a6, a7, 0
    movi a6, 100
4:
    addi a6, a6, -1
    bnez a6, 4b

    l32r a1, .Lstack_ptr
    l32r a2, .Lbss_start_ptr
    l32r a3, .Lbss_end_ptr
    sub a3, a3, a2
    movi a4, 0
1:
    beqz a3, 2f
    s8i a4, a2, 0
    addi a2, a2, 1
    addi a3, a3, -1
    j 1b
2:
    call8 kernel_main
3:
    waiti 0
    j 3b
"#
);

#[cfg(target_arch = "x86_64")]
struct PumpStats {
    arp_replies: u32,
    icmp_replies: u32,
}

#[cfg(target_arch = "x86_64")]
enum BareNic {
    Rtl8125(edgerun_rtl8125::Rtl8125),
    Virtio(edgerun_virtio::VirtNet),
}

#[cfg(target_arch = "x86_64")]
struct BareNicStats {
    tx_completed: u32,
    rx_received: u32,
}

#[cfg(target_arch = "x86_64")]
impl BareNic {
    fn init(&mut self) -> bool {
        match self {
            Self::Rtl8125(net) => net.init(),
            Self::Virtio(net) => net.init(),
        }
    }

    fn send(&mut self, data: &[u8]) -> bool {
        match self {
            Self::Rtl8125(net) => net.send(data),
            Self::Virtio(net) => net.send(data),
        }
    }

    fn recv(&mut self, out: &mut [u8]) -> Option<usize> {
        match self {
            Self::Rtl8125(net) => net.recv(out),
            Self::Virtio(net) => net.recv(out),
        }
    }

    fn get_mac(&self) -> [u8; 6] {
        match self {
            Self::Rtl8125(net) => net.get_mac(),
            Self::Virtio(net) => net.get_mac(),
        }
    }

    fn is_link_up(&self) -> bool {
        match self {
            Self::Rtl8125(net) => net.is_link_up(),
            Self::Virtio(net) => net.is_link_up(),
        }
    }

    fn stats(&mut self) -> BareNicStats {
        match self {
            Self::Rtl8125(net) => {
                let stats = net.stats();
                BareNicStats {
                    tx_completed: stats.tx_completed,
                    rx_received: stats.rx_received,
                }
            }
            Self::Virtio(net) => {
                let stats = net.stats();
                BareNicStats {
                    tx_completed: stats.tx_completed,
                    rx_received: stats.rx_received,
                }
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
struct NetPump<'net, 'stack> {
    net: &'net mut BareNic,
    network: Network<'stack>,
    rx_buf: [u8; 1514],
    logged_start: bool,
    logged_arp: bool,
    logged_icmp: bool,
}

#[cfg(target_arch = "x86_64")]
async fn run_http_smoke(url: &str) {
    let client = edgerun_http::HttpClient::new()
        .version(edgerun_http::HttpVersion::Http1)
        .with_connect_timeout(edgerun_http::runtime::time::Duration::from_secs(3))
        .with_read_timeout(edgerun_http::runtime::time::Duration::from_secs(3));
    match client.get(url).await {
        Ok(response) if response.status().as_u16() == 200 => {
            rt::log::log(1, "Bare HTTP smoke GET ok")
        }
        Ok(_) => rt::log::log(1, "Bare HTTP smoke GET bad status"),
        Err(_) => rt::log::log(1, "Bare HTTP smoke GET failed"),
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
async fn run_configured_oci_pull(config: &boot_config::BootConfig) {
    if !config.oci_pull {
        return;
    }

    rt::log::log(1, "Bare OCI pull into EdgeFS start");
    let Some(mut block) = edgerun_virtio::find_virtio_blk() else {
        rt::log::log(1, "Bare OCI pull no VirtIO block device");
        return;
    };
    if !block.init() {
        rt::log::log(1, "Bare OCI pull VirtIO block init failed");
        return;
    }

    let mut storage = disk_boot::RtBlockDeviceStorage::new(block);
    let edgefs_key = [0x42u8; 32];
    let edgefs_fs_id = [0x24u8; 16];
    match disk_boot::open_configured_edgefs(&mut storage, config, edgefs_key, edgefs_fs_id) {
        Ok(disk_boot::OpenedEdgeFs::Partition(mut fs)) => {
            match unsafe {
                oci_image_boot::pull_configured_image_into_edgefs(&mut fs, config).await
            } {
                Ok(_) => rt::log::log(1, "Bare OCI pull into EdgeFS ok"),
                Err(error) => {
                    rt::log::log(1, "Bare OCI pull into EdgeFS failed");
                    rt::log::log(1, oci_image_boot::image_boot_error_label(&error));
                    if let Some(detail) = oci_image_boot::image_boot_error_detail(&error) {
                        rt::log::log(1, detail);
                    }
                }
            }
        }
        Ok(disk_boot::OpenedEdgeFs::WholeDisk(mut fs)) => {
            match unsafe {
                oci_image_boot::pull_configured_image_into_edgefs(&mut fs, config).await
            } {
                Ok(_) => rt::log::log(1, "Bare OCI pull into EdgeFS ok"),
                Err(error) => {
                    rt::log::log(1, "Bare OCI pull into EdgeFS failed");
                    rt::log::log(1, oci_image_boot::image_boot_error_label(&error));
                    if let Some(detail) = oci_image_boot::image_boot_error_detail(&error) {
                        rt::log::log(1, detail);
                    }
                }
            }
        }
        Err(_) => rt::log::log(1, "Bare OCI pull EdgeFS open failed"),
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
struct KernelBareNetDriver {
    net: AtomicPtr<BareNic>,
    stack: AtomicPtr<IpStack>,
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
unsafe impl Sync for KernelBareNetDriver {}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
impl KernelBareNetDriver {
    const fn empty() -> Self {
        Self {
            net: AtomicPtr::new(core::ptr::null_mut()),
            stack: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    fn install(&self, net: *mut BareNic, stack: *mut IpStack) {
        self.net.store(net, Ordering::Release);
        self.stack.store(stack, Ordering::Release);
    }

    fn net(&self) -> *mut BareNic {
        self.net.load(Ordering::Acquire)
    }

    fn stack(&self) -> *mut IpStack {
        self.stack.load(Ordering::Acquire)
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
impl rt::BareNetDriver for KernelBareNetDriver {
    fn local_ipv4(&self) -> [u8; 4] {
        unsafe { *(*self.stack()).ip.as_bytes() }
    }

    fn gateway_ipv4(&self) -> [u8; 4] {
        unsafe { *(*self.stack()).gateway.as_bytes() }
    }

    fn local_mac(&self) -> [u8; 6] {
        unsafe { (*self.stack()).mac }
    }

    fn lookup_arp(&self, ipv4: [u8; 4]) -> Option<[u8; 6]> {
        unsafe { (*self.stack()).arp.lookup(IpAddr::from_slice(&ipv4)) }
    }

    fn send_frame(&self, frame: &[u8]) -> bool {
        unsafe { (*self.net()).send(frame) }
    }

    fn recv_frame(&self, out: &mut [u8]) -> Option<usize> {
        let len = unsafe { (*self.net()).recv(out)? };
        if let Some(ParsedPacket::Arp { header, .. }) =
            Network::new(unsafe { &mut *self.stack() }).recv(&out[..len])
        {
            if header.oper == ARP_OP_REQUEST
                && header.tpa == unsafe { *(*self.stack()).ip.as_bytes() }
            {
                let mut network = Network::new(unsafe { &mut *self.stack() });
                if let Some(reply) = network.send_arp_reply(&header) {
                    let _ = unsafe { (*self.net()).send(reply) };
                }
            }
        }
        Some(len)
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
static BARE_NET_DRIVER: KernelBareNetDriver = KernelBareNetDriver::empty();

#[cfg(target_arch = "x86_64")]
fn poll_network(
    net: &mut BareNic,
    network: &mut Network<'_>,
    rx_buf: &mut [u8; 1514],
) -> PumpStats {
    let mut stats = PumpStats {
        arp_replies: 0,
        icmp_replies: 0,
    };

    while let Some(len) = net.recv(rx_buf) {
        match network.recv(&rx_buf[..len]) {
            Some(ParsedPacket::Arp { header, .. }) => {
                if header.oper == ARP_OP_REQUEST
                    && header.tpa == *network.stack.ip.as_bytes()
                    && network
                        .send_arp_reply(&header)
                        .map(|packet| net.send(packet))
                        .unwrap_or(false)
                {
                    stats.arp_replies = stats.arp_replies.wrapping_add(1);
                }
            }
            Some(ParsedPacket::Icmp {
                eth,
                ip,
                header,
                payload,
            }) => {
                if header.icmp_type == ICMP_ECHO_REQUEST
                    && ip.dst == *network.stack.ip.as_bytes()
                    && network
                        .send_icmp_echo_reply(
                            eth.src,
                            IpAddr::from_slice(&ip.src),
                            &header,
                            payload,
                        )
                        .map(|packet| net.send(packet))
                        .unwrap_or(false)
                {
                    stats.icmp_replies = stats.icmp_replies.wrapping_add(1);
                }
            }
            _ => {}
        }
    }

    stats
}

#[cfg(target_arch = "x86_64")]
fn dhcp_ipv4_to_rt(ip: edgerun_dhcp::Ipv4Addr) -> IpAddr {
    let octets = ip.octets();
    IpAddr::new(octets[0], octets[1], octets[2], octets[3])
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
unsafe fn probe_tpm2() -> Option<[u8; 32]> {
    rt::log::log(1, "Looking for TPM2 ACPI table...");
    match unsafe { edgerun_tpm::CrbTpmTransport::discover_acpi() } {
        Ok(Some(transport)) => {
            rt::log::log(1, "TPM2 CRB transport discovered");
            return probe_tpm2_transport(transport.with_timeout_polls(10));
        }
        Ok(None) => {
            rt::log::log(1, "No TPM2 ACPI table found");
        }
        Err(_) => {
            rt::log::log(1, "TPM2 ACPI discovery failed");
        }
    }

    rt::log::log(1, "Trying TPM2 TIS transport...");
    let transport = unsafe { edgerun_tpm::TisTpmTransport::new_default_x86() };
    probe_tpm2_transport(transport.with_timeout_polls(100_000))
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn fill_bare_random_source(out: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    if out.is_empty() {
        return Ok(());
    }

    if let Some(mut rng) = edgerun_virtio::find_virtio_rng() {
        if rng.init() && rng.fill_bytes(out) {
            return Ok(());
        }
    }

    fill_tpm2_random_source(out)
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn fill_tpm2_random_source(out: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    if out.is_empty() {
        return Ok(());
    }

    if let Ok(Some(transport)) = unsafe { edgerun_tpm::CrbTpmTransport::discover_acpi() } {
        if fill_tpm2_random_transport(transport.with_timeout_polls(10), out) {
            return Ok(());
        }
    }

    let transport = unsafe { edgerun_tpm::TisTpmTransport::new_default_x86() };
    if fill_tpm2_random_transport(transport.with_timeout_polls(100_000), out) {
        Ok(())
    } else {
        Err(edgerun_crypto::error::CryptoError::TpmUnavailable)
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn fill_tpm2_random_transport<T>(transport: T, out: &mut [u8]) -> bool
where
    T: edgerun_tpm::FixedTpmTransport + edgerun_tpm::TpmTransport,
{
    let mut device = edgerun_tpm::TpmDevice::new(transport);
    let startup_code = device.startup_response_code(edgerun_tpm::TPM_SU_CLEAR);
    if startup_code != edgerun_tpm::TPM_RC_SUCCESS && startup_code != 0x100 && startup_code != 0x120
    {
        return false;
    }

    let mut offset = 0usize;
    while offset < out.len() {
        let end = core::cmp::min(offset + 2048, out.len());
        let n = device.get_random_into(&mut out[offset..end]);
        if n == 0 || offset + n > end {
            return false;
        }
        offset += n;
    }
    true
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn probe_tpm2_transport<T>(transport: T) -> Option<[u8; 32]>
where
    T: edgerun_tpm::FixedTpmTransport + edgerun_tpm::TpmTransport,
{
    let mut device = edgerun_tpm::TpmDevice::new(transport);
    let startup_code = device.startup_response_code(edgerun_tpm::TPM_SU_CLEAR);
    if startup_code == edgerun_tpm::TPM_RC_SUCCESS || startup_code == 0x100 || startup_code == 0x120
    {
        rt::log::log(1, "TPM2 startup ok");
    } else if startup_code == 0x144 {
        rt::log::log(1, "TPM2 startup failed: command size");
        return None;
    } else if startup_code == 0xffff_fffb {
        rt::log::log(1, "TPM2 startup failed: transport");
        return None;
    } else if startup_code == 0xffff_fffc {
        rt::log::log(1, "TPM2 startup failed: malformed response");
        return None;
    } else {
        rt::log::log(1, "TPM2 startup failed");
        return None;
    }

    let mut tpm_random = [0u8; 32];
    let random_len = device.get_random_into(&mut tpm_random);
    let entropy = if random_len != 0 {
        rt::log::log(1, "TPM2 random ok");
        edgerun_crypto::rng::mix_entropy(&tpm_random[..random_len]);
        Some(tpm_random)
    } else {
        rt::log::log(1, "TPM2 random failed");
        None
    };

    let Some(entropy) = entropy else {
        return None;
    };

    let read_public_code = device.read_public_response_code(edgerun_tpm::TpmHandle(0x8100_0001));
    if read_public_code == edgerun_tpm::TPM_RC_SUCCESS {
        rt::log::log(1, "TPM2 persistent key readable");
    } else {
        rt::log::log(1, "TPM2 persistent key not readable");
        return Some(entropy);
    }

    let digest = [0u8; 32];
    let mut signature = [0u8; 64];
    if device
        .sign_p256_sha256_into(edgerun_tpm::TpmHandle(0x8100_0001), &digest, &mut signature)
        .is_ok()
    {
        rt::log::log(1, "TPM2 sign ok");
    } else {
        rt::log::log(1, "TPM2 sign failed");
    }

    Some(entropy)
}

#[cfg(target_arch = "x86_64")]
impl Future for NetPump<'_, '_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if !this.logged_start {
            rt::log::log(1, "Net pump started");
            this.logged_start = true;
        }

        let stats = poll_network(this.net, &mut this.network, &mut this.rx_buf);
        if stats.arp_replies != 0 && !this.logged_arp {
            rt::log::log(1, "ARP reply sent");
            this.logged_arp = true;
        }
        if stats.icmp_replies != 0 && !this.logged_icmp {
            rt::log::log(1, "ICMP echo reply sent");
            this.logged_icmp = true;
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn find_initialized_bare_nic() -> Option<BareNic> {
    if let Some(rtl8125) = edgerun_rtl8125::find_rtl8125() {
        rt::log::log(1, "RTL8125 found");
        let mut net = BareNic::Rtl8125(rtl8125);
        if net.init() {
            rt::log::log(1, "RTL8125 init ok");
            return Some(net);
        }
        rt::log::log(1, "RTL8125 init failed");
    } else {
        rt::log::log(1, "No RTL8125 found");
    }

    if let Some(virtio) = edgerun_virtio::find_virtio_net() {
        rt::log::log(1, "VirtIO net found");
        let mut net = BareNic::Virtio(virtio);
        if net.init() {
            rt::log::log(1, "VirtIO net init ok");
            return Some(net);
        }
        rt::log::log(1, "VirtIO net init failed");
    } else {
        rt::log::log(1, "No VirtIO net found");
    }

    None
}

#[cfg(target_os = "none")]
#[panic_handler]
unsafe fn panic(_info: &core::panic::PanicInfo) -> ! {
    #[cfg(target_arch = "xtensa")]
    {
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"PANIC\n");
    }
    loop {
        #[cfg(target_arch = "x86_64")]
        core::arch::asm!("hlt");
        #[cfg(target_arch = "xtensa")]
        core::arch::asm!("waiti 0");
        #[cfg(not(any(target_arch = "x86_64", target_arch = "xtensa")))]
        core::hint::spin_loop();
    }
}

#[used]
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[link_section = ".multiboot"]
static MULTIBOOT_HEADER: [u32; 8] = [
    0x1BADB002, 0x00010000, 0xE4514FFE, 0x100000, 0x100000, 0, 0, 0,
];

#[no_mangle]
#[cfg(all(target_arch = "xtensa", target_os = "none"))]
pub unsafe extern "C" fn kernel_main() -> ! {
    unsafe {
        edgerun_platform::arch::xtensa::esp32s3_disable_watchdogs();
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_init();
    }
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM0\n");
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM1\n");
    rt::timer::set_now(0);
    rt::log::set_mirror_logger(display_console_log);
    rt::log::log(1, "Starting edgerun unikernel on Xtensa");
    rt::log::init_serial_logger();
    rt::log::log(1, "Xtensa serial mux online");
    rt::log::log(1, "Initializing JC3248W535 display");
    unsafe {
        edgerun_platform::esp32s3::Jc3248w535Display::init();
        edgerun_platform::esp32s3::Jc3248w535Touch::init();
    }
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM2\n");
    rt::log::log(1, "JC3248W535 display init complete");
    rt::log::log(1, "Rendering display UI");
    render_initial_ui();
    edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(b"KM3\n");
    rt::log::log(1, "Display UI rendered");
    rt::log::log(1, "ESP32-S3 WiFi AP command ready");
    rt::log::log(1, "JC3248W535 touch polling enabled");

    let mut last_touch: Option<(u16, u16)> = None;
    let mut serial_rx = rt::serial_mux::Receiver::<256>::new();
    loop {
        poll_serial_control(&mut serial_rx, last_touch);
        unsafe {
            if let Some(point) = edgerun_platform::esp32s3::Jc3248w535Touch::read_point() {
                let touch = (point.x, point.y);
                if last_touch != Some(touch) {
                    render_touch_ui(touch);
                    display_touch_status(point.x, point.y);
                    last_touch = Some(touch);
                }
            }
        }
        render_console_if_dirty(last_touch);

        let mut i = 0;
        while i < 1_000_000 {
            core::arch::asm!("nop");
            i += 1;
        }
    }
}

#[no_mangle]
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
pub unsafe extern "C" fn kernel_main() -> ! {
    rt::timer::set_now(0);
    rt::log::log(1, "Starting edgerun unikernel");
    let mut active_boot_config = None;

    edgerun_crypto::rng::register_random_source(fill_bare_random_source);

    let mut rng = Rng::new_from_entropy();
    if let Some(mut virtio_rng) = edgerun_virtio::find_virtio_rng() {
        if virtio_rng.init() {
            let mut virtio_entropy = [0u8; 32];
            if virtio_rng.fill_bytes(&mut virtio_entropy) {
                rng.mix_entropy(&virtio_entropy);
                edgerun_crypto::rng::mix_entropy(&virtio_entropy);
                rt::log::log(1, "RNG mixed VirtIO entropy");
            } else {
                rt::log::log(1, "VirtIO RNG read failed");
            }
        } else {
            rt::log::log(1, "VirtIO RNG init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO RNG found");
    }

    if let Some(tpm_entropy) = unsafe { probe_tpm2() } {
        rng.mix_entropy(&tpm_entropy);
        rt::log::log(1, "RNG mixed TPM entropy");
    }

    let test_crc = crc32(b"hello");
    let _ = test_crc;

    let mut rx_buf = RingBuffer::new(1024);
    rx_buf.push_slice(b"test packet");
    let _ = rx_buf.len();

    rt::log::log(1, "Looking for VirtIO...");

    if let Some(mut console) = edgerun_virtio::find_virtio_console() {
        if console.init() {
            let _ = console.write_all(b"edgerun: virtio-console online\n");
            rt::log::log(1, "VirtIO console init ok");
        } else {
            rt::log::log(1, "VirtIO console init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO console found");
    }

    if let Some(mut block) = edgerun_virtio::find_virtio_blk() {
        rt::log::log(1, "VirtIO block device found");
        if block.init() {
            rt::log::log(1, "VirtIO block init ok");
            let mut first_sector = [0u8; 512];
            if block.read_sector(0, &mut first_sector) {
                rt::log::log(1, "VirtIO block first sector read ok");
            } else {
                rt::log::log(1, "VirtIO block first sector read failed");
            }
            if block.read_sector(0, &mut first_sector) {
                rt::log::log(1, "VirtIO block second sector read ok");
            } else {
                rt::log::log(1, "VirtIO block second sector read failed");
            }
            if first_sector[510] == 0x55 && first_sector[511] == 0xaa {
                let mut partition_sector = [0u8; 512];
                if block.read_sector(2048, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block partition sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block partition sector read failed");
                }
                if block.read_sector(2112, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block probe sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block probe sector read failed");
                }
                if block.read_sector(2308, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block FAT root sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block FAT root sector read failed");
                }
            }
            rt::log::log(1, "VirtIO block scanning partitions");
            let mut storage = disk_boot::RtBlockDeviceStorage::new(block);
            match edgerun_storage::BlockStorage::read_sector(&mut storage, 0, &mut first_sector) {
                Ok(()) => rt::log::log(1, "VirtIO storage adapter read ok"),
                Err(_) => rt::log::log(1, "VirtIO storage adapter read failed"),
            }
            if first_sector[510] != 0x55 || first_sector[511] != 0xaa {
                rt::log::log(1, "VirtIO block has no partitions");
            } else {
                match edgerun_storage::detect_partitions(&mut storage) {
                    Ok(table) if !table.partitions.is_empty() => {
                        rt::log::log(1, "VirtIO block partition table detected");
                        let mut handled_boot_config = false;
                        for partition in &table.partitions {
                            if handled_boot_config {
                                break;
                            }
                            let mut partition_boot_sector = [0u8; 512];
                            match edgerun_storage::BlockStorage::read_sector(
                                &mut storage,
                                partition.start_lba,
                                &mut partition_boot_sector,
                            ) {
                                Ok(()) if partition_boot_sector.iter().all(|byte| *byte == 0) => {
                                    rt::log::log(1, "VirtIO partition is blank")
                                }
                                Ok(()) => {
                                    rt::log::log(1, "VirtIO block partitions detected");
                                    let mut partition_storage = disk_boot::RtPartitionStorage::new(
                                        &mut storage,
                                        partition.start_lba,
                                        partition.sectors,
                                    );
                                    let mut fat_root_sector = [0u8; 512];
                                    match edgerun_storage::BlockStorage::read_sector(
                                        &mut partition_storage,
                                        260,
                                        &mut fat_root_sector,
                                    ) {
                                        Ok(()) => {
                                            rt::log::log(1, "VirtIO FAT relative root read ok")
                                        }
                                        Err(_) => {
                                            rt::log::log(1, "VirtIO FAT relative root read failed")
                                        }
                                    }
                                    rt::log::log(1, "VirtIO opening FAT boot partition");
                                    let mut boot_config = None;
                                    match edgerun_storage::FatReadOnly::open(partition_storage) {
                                        Ok(mut fat) => {
                                            rt::log::log(1, "VirtIO FAT boot partition open ok");
                                            match fat.root_entries() {
                                                Ok(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT root directory read ok",
                                                ),
                                                Err(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT root directory read failed",
                                                ),
                                            }
                                            rt::log::log(1, "VirtIO FAT boot config read start");
                                            match fat.read_file("/edgerun/boot.cfg") {
                                                Ok(bytes) => {
                                                    rt::log::log(
                                                        1,
                                                        "VirtIO FAT boot config bytes read ok",
                                                    );
                                                    match boot_config::BootConfig::parse(&bytes) {
                                                        Ok(config) => {
                                                            rt::log::log(
                                                                1,
                                                                "VirtIO FAT boot config read ok",
                                                            );
                                                            active_boot_config =
                                                                Some(config.clone());
                                                            boot_config = Some(config);
                                                        }
                                                        Err(_) => rt::log::log(
                                                            1,
                                                            "VirtIO FAT boot config parse failed",
                                                        ),
                                                    }
                                                }
                                                Err(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT boot config missing",
                                                ),
                                            }
                                        }
                                        Err(_) => rt::log::log(1, "VirtIO partition is not FAT"),
                                    }
                                    if let Some(config) = boot_config.as_ref() {
                                        let edgefs_key = [0x42u8; 32];
                                        let edgefs_fs_id = [0x24u8; 16];
                                        match disk_boot::open_configured_edgefs(
                                            &mut storage,
                                            config,
                                            edgefs_key,
                                            edgefs_fs_id,
                                        ) {
                                            Ok(_) => {
                                                rt::log::log(1, "VirtIO EdgeFS boot target open ok")
                                            }
                                            Err(_) => rt::log::log(
                                                1,
                                                "VirtIO EdgeFS boot target open failed",
                                            ),
                                        }
                                        handled_boot_config = true;
                                    }
                                }
                                Err(_) => rt::log::log(1, "VirtIO partition read failed"),
                            }
                        }
                    }
                    Ok(_) => {
                        rt::log::log(1, "VirtIO block has no partitions");
                    }
                    Err(_) => {
                        rt::log::log(1, "VirtIO block partition scan failed");
                    }
                }
            }
        } else {
            rt::log::log(1, "VirtIO block init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO block device found");
    }

    let mut net = match find_initialized_bare_nic() {
        Some(net) => net,
        None => {
            rt::log::log(1, "No usable NIC found");
            loop {
                core::arch::asm!("hlt");
            }
        }
    };
    let mac = net.get_mac();

    let mut stack = IpStack::new();
    stack.configure(
        IpAddr::new(0, 0, 0, 0),
        IpAddr::new(255, 255, 255, 0),
        IpAddr::zero(),
        mac,
    );

    let dhcp_xid = 0x12345678;
    let mut dhcp_ip = IpAddr::zero();
    let mut dhcp_netmask = IpAddr::new(255, 255, 255, 0);
    let mut dhcp_gateway = IpAddr::zero();
    let mut offered_ip = None;
    let mut offered_netmask = None;
    let mut offered_gateway = None;
    let mut requested_lease = false;
    let mut network = Network::new(&mut stack);

    let discover = DhcpMessage::discover(dhcp_xid, mac).to_wire();
    rt::log::log(1, "Sending DHCP discover");
    if let Some(pkt) = network.send_udp(
        IpAddr::new(255, 255, 255, 255),
        DHCP_CLIENT_PORT,
        DHCP_SERVER_PORT,
        &discover,
    ) {
        if net.send(pkt) {
            rt::log::log(1, "DHCP discover queued");
        } else {
            rt::log::log(1, "DHCP discover send failed");
        }
    }

    let mut rx_buf = [0u8; 1514];
    let mut logged_rx = false;
    for _ in 0..1000 {
        if let Some(len) = net.recv(&mut rx_buf) {
            if !logged_rx {
                rt::log::log(1, "NIC RX packet observed");
                logged_rx = true;
            }
            if let Some(ParsedPacket::Udp {
                header, payload, ..
            }) = network.recv(&rx_buf[..len])
            {
                if header.src_port == DHCP_SERVER_PORT && header.dst_port == DHCP_CLIENT_PORT {
                    if let Ok(message) = DhcpMessage::from_wire(payload) {
                        if message.xid != dhcp_xid || message.chaddr[..6] != mac {
                            continue;
                        }

                        match message.options.message_type {
                            Some(DhcpMessageType::Offer) if !requested_lease => {
                                let Some(server_id) = message.options.server_id else {
                                    continue;
                                };
                                offered_ip = Some(message.yiaddr);
                                offered_netmask = message.options.subnet_mask;
                                offered_gateway = message.options.router;

                                let request =
                                    DhcpMessage::request(dhcp_xid, mac, message.yiaddr, server_id)
                                        .to_wire();
                                if let Some(pkt) = network.send_udp(
                                    IpAddr::new(255, 255, 255, 255),
                                    DHCP_CLIENT_PORT,
                                    DHCP_SERVER_PORT,
                                    &request,
                                ) {
                                    if net.send(pkt) {
                                        requested_lease = true;
                                        rt::log::log(1, "DHCP request queued");
                                    } else {
                                        rt::log::log(1, "DHCP request send failed");
                                    }
                                }
                            }
                            Some(DhcpMessageType::Ack) if requested_lease => {
                                dhcp_ip = dhcp_ipv4_to_rt(message.yiaddr);
                                if dhcp_ip == IpAddr::zero() {
                                    if let Some(ip) = offered_ip {
                                        dhcp_ip = dhcp_ipv4_to_rt(ip);
                                    }
                                }
                                if let Some(netmask) =
                                    message.options.subnet_mask.or(offered_netmask)
                                {
                                    dhcp_netmask = dhcp_ipv4_to_rt(netmask);
                                }
                                if let Some(gateway) = message.options.router.or(offered_gateway) {
                                    dhcp_gateway = dhcp_ipv4_to_rt(gateway);
                                }
                                rt::log::log(1, "DHCP lease accepted");
                                break;
                            }
                            Some(DhcpMessageType::Nak) => {
                                rt::log::log(1, "DHCP lease rejected");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let net_stats = net.stats();
    if net_stats.tx_completed != 0 {
        rt::log::log(1, "NIC TX completed");
    } else {
        rt::log::log(1, "NIC TX pending");
    }
    if net_stats.rx_received == 0 {
        rt::log::log(1, "NIC RX no packets");
    }

    drop(network);

    if dhcp_ip != IpAddr::zero() {
        stack.ip = dhcp_ip;
        stack.netmask = dhcp_netmask;
        stack.gateway = dhcp_gateway;
    } else {
        rt::log::log(1, "Using static fallback IP");
        stack.ip = IpAddr::new(192, 168, 1, 12);
    }

    BARE_NET_DRIVER.install(&mut net as *mut _, &mut stack as *mut _);
    rt::install_bare_net_driver(&BARE_NET_DRIVER);
    rt::log::log(1, "Bare async TCP driver installed");
    if let Some(url) = active_boot_config
        .as_ref()
        .and_then(|config| config.http_smoke.as_deref())
    {
        block_on(run_http_smoke(url));
    }
    if let Some(config) = active_boot_config.as_ref() {
        block_on(run_configured_oci_pull(config));
    }

    let mut network = Network::new(&mut stack);
    let tftp_server = if network.stack.gateway != IpAddr::zero() {
        network.stack.gateway
    } else {
        IpAddr::new(192, 168, 1, 1)
    };
    let rrq = TftpMessage::rrq("edgerun.bin").to_wire();
    rt::log::log(1, "Sending TFTP RRQ");
    if let Some(pkt) = network.send_udp(tftp_server, 2070, TFTP_PORT, &rrq) {
        if net.send(pkt) {
            rt::log::log(1, "TFTP RRQ queued");
        } else {
            rt::log::log(1, "TFTP RRQ send failed");
        }
    }
    let mut tcp = TcpSocket::new();

    let addr = rt::SocketAddr::new(0xC0A8010C, 8080);
    let _ = tcp.bind(addr);
    let _ = tcp.listen(10);
    let _ = rng.next();

    if net.is_link_up() {}

    block_on(NetPump {
        net: &mut net,
        network,
        rx_buf: [0; 1514],
        logged_start: false,
        logged_arp: false,
        logged_icmp: false,
    });

    loop {
        core::arch::asm!("hlt");
    }
}

#[cfg(not(target_os = "none"))]
fn main() {}
