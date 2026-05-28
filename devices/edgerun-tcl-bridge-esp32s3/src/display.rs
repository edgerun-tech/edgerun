use core::ptr::{read_volatile, write_volatile};

const GPIO_BASE: usize = 0x6000_4000;
const IO_MUX_BASE: usize = 0x6000_9000;
const SPI2_BASE: usize = 0x6002_4000;
const SYSTEM_BASE: usize = 0x600c_0000;

const GPIO_OUT_W1TS: *mut u32 = (GPIO_BASE + 0x08) as *mut u32;
const GPIO_OUT_W1TC: *mut u32 = (GPIO_BASE + 0x0c) as *mut u32;
const GPIO_OUT1_W1TS: *mut u32 = (GPIO_BASE + 0x14) as *mut u32;
const GPIO_OUT1_W1TC: *mut u32 = (GPIO_BASE + 0x18) as *mut u32;
const GPIO_ENABLE_W1TS: *mut u32 = (GPIO_BASE + 0x24) as *mut u32;
const GPIO_ENABLE_W1TC: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;
const GPIO_ENABLE1_W1TS: *mut u32 = (GPIO_BASE + 0x30) as *mut u32;
const GPIO_ENABLE1_W1TC: *mut u32 = (GPIO_BASE + 0x34) as *mut u32;
const GPIO_IN: *const u32 = (GPIO_BASE + 0x3c) as *const u32;
const GPIO_IN1: *const u32 = (GPIO_BASE + 0x40) as *const u32;
const GPIO_PIN_BASE: usize = GPIO_BASE + 0x74;
const GPIO_FUNC_IN_SEL_CFG: usize = GPIO_BASE + 0x154;
const GPIO_FUNC_OUT_SEL_CFG: usize = GPIO_BASE + 0x554;

const SYSTEM_PERIP_CLK_EN0: *mut u32 = (SYSTEM_BASE + 0x18) as *mut u32;
const SYSTEM_PERIP_RST_EN0: *mut u32 = (SYSTEM_BASE + 0x20) as *mut u32;
const SYSTEM_SPI2_CLK_EN: u32 = 1 << 6;
const SYSTEM_SPI2_RST: u32 = 1 << 6;

const SPI_CMD: *mut u32 = (SPI2_BASE) as *mut u32;
const SPI_CTRL: *mut u32 = (SPI2_BASE + 0x08) as *mut u32;
const SPI_CLOCK: *mut u32 = (SPI2_BASE + 0x0c) as *mut u32;
const SPI_USER: *mut u32 = (SPI2_BASE + 0x10) as *mut u32;
const SPI_USER1: *mut u32 = (SPI2_BASE + 0x14) as *mut u32;
const SPI_USER2: *mut u32 = (SPI2_BASE + 0x18) as *mut u32;
const SPI_MS_DLEN: *mut u32 = (SPI2_BASE + 0x1c) as *mut u32;
const SPI_MISC: *mut u32 = (SPI2_BASE + 0x20) as *mut u32;
const SPI_DMA_CONF: *mut u32 = (SPI2_BASE + 0x30) as *mut u32;
const SPI_DMA_INT_CLR: *mut u32 = (SPI2_BASE + 0x38) as *mut u32;
const SPI_W0: usize = SPI2_BASE + 0x98;
const SPI_SLAVE: *mut u32 = (SPI2_BASE + 0xe0) as *mut u32;
const SPI_CLK_GATE: *mut u32 = (SPI2_BASE + 0xe8) as *mut u32;

const MCU_SEL_S: u32 = 12;
const MCU_SEL_M: u32 = 0x7 << MCU_SEL_S;
const FUN_PD: u32 = 1 << 7;
const FUN_PU: u32 = 1 << 8;
const FUN_DRV_S: u32 = 10;
const FUN_DRV_M: u32 = 0x3 << FUN_DRV_S;
const FUN_IE: u32 = 1 << 9;
const GPIO_FUNC: u32 = 1;
const DRIVE_3: u32 = 3;
const GPIO_PIN_PAD_DRIVER: u32 = 1 << 2;

pub const LCD_WIDTH: u16 = 320;
pub const LCD_HEIGHT: u16 = 480;

const PIN_BL: u8 = 1;
const PIN_DATA0: u8 = 21;
const PIN_DATA3: u8 = 39;
const PIN_DATA2: u8 = 40;
const PIN_CS: u8 = 45;
const PIN_CLK: u8 = 47;
const PIN_DATA1: u8 = 48;

const LCD_OPCODE_WRITE_CMD: u32 = 0x02;
const LCD_OPCODE_WRITE_COLOR: u32 = 0x32;

const SPI_USR: u32 = 1 << 24;
const SPI_UPDATE: u32 = 1 << 23;
const SPI_USR_MOSI: u32 = 1 << 27;
const SPI_FWRITE_QUAD: u32 = 1 << 13;
const SPI_CK_IDLE_EDGE: u32 = 1 << 29;
const SPI_CS_KEEP_ACTIVE: u32 = 1 << 30;
const SPI_CLK_EN: u32 = 1 << 0;
const SPI_MST_CLK_ACTIVE: u32 = 1 << 1;
const SPI_MST_CLK_SEL: u32 = 1 << 2;

const FSPICLK_OUT: u32 = 101;
const FSPICLK_IN: u32 = 101;
const FSPIQ_OUT: u32 = 102;
const FSPIQ_IN: u32 = 102;
const FSPID_OUT: u32 = 103;
const FSPID_IN: u32 = 103;
const FSPIHD_OUT: u32 = 104;
const FSPIHD_IN: u32 = 104;
const FSPIWP_OUT: u32 = 105;
const FSPIWP_IN: u32 = 105;
const FSPICS0_OUT: u32 = 110;
const FSPICS0_IN: u32 = 110;

pub const CHAR_W: usize = 5;
pub const CHAR_H: usize = 7;
pub const CHAR_STEP: usize = 6;

const FONT5X7: [[u8; 5]; 59] = [
    [0x00, 0x00, 0x00, 0x00, 0x00], // ' ' (idx 0)
    [0x00, 0x00, 0x00, 0x00, 0x00], // '!'  (idx 1) - unused, placeholder
    [0x00, 0x00, 0x00, 0x00, 0x00], // '"'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '#'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '$'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '%'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '&'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '''
    [0x00, 0x00, 0x00, 0x00, 0x00], // '('
    [0x00, 0x00, 0x00, 0x00, 0x00], // ')'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '*'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '+'
    [0x00, 0x36, 0x36, 0x00, 0x00], // ',' -> reuse ':' as comma approximation
    [0x08, 0x08, 0x08, 0x08, 0x08], // '-'
    [0x00, 0x60, 0x60, 0x00, 0x00], // '.'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '/'
    [0x3e, 0x51, 0x49, 0x45, 0x3e], // '0' (idx 16)
    [0x00, 0x42, 0x7f, 0x40, 0x00], // '1'
    [0x42, 0x61, 0x51, 0x49, 0x46], // '2'
    [0x21, 0x41, 0x45, 0x4b, 0x31], // '3'
    [0x18, 0x14, 0x12, 0x7f, 0x10], // '4'
    [0x27, 0x45, 0x45, 0x45, 0x39], // '5'
    [0x3c, 0x4a, 0x49, 0x49, 0x30], // '6'
    [0x01, 0x71, 0x09, 0x05, 0x03], // '7'
    [0x36, 0x49, 0x49, 0x49, 0x36], // '8'
    [0x06, 0x49, 0x49, 0x29, 0x1e], // '9' (idx 25)
    [0x00, 0x00, 0x00, 0x00, 0x00], // ':'
    [0x00, 0x00, 0x00, 0x00, 0x00], // ';'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '<'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '='
    [0x00, 0x00, 0x00, 0x00, 0x00], // '>'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '?'
    [0x00, 0x00, 0x00, 0x00, 0x00], // '@'
    [0x7e, 0x11, 0x11, 0x11, 0x7e], // 'A' (idx 33)
    [0x7f, 0x49, 0x49, 0x49, 0x36], // 'B'
    [0x3e, 0x41, 0x41, 0x41, 0x22], // 'C'
    [0x7f, 0x41, 0x41, 0x22, 0x1c], // 'D'
    [0x7f, 0x49, 0x49, 0x49, 0x41], // 'E'
    [0x7f, 0x09, 0x09, 0x09, 0x01], // 'F'
    [0x3e, 0x41, 0x49, 0x49, 0x7a], // 'G'
    [0x7f, 0x08, 0x08, 0x08, 0x7f], // 'H'
    [0x00, 0x41, 0x7f, 0x41, 0x00], // 'I'
    [0x20, 0x40, 0x41, 0x3f, 0x01], // 'J'
    [0x7f, 0x08, 0x14, 0x22, 0x41], // 'K'
    [0x7f, 0x40, 0x40, 0x40, 0x40], // 'L'
    [0x7f, 0x02, 0x0c, 0x02, 0x7f], // 'M'
    [0x7f, 0x04, 0x08, 0x10, 0x7f], // 'N'
    [0x3e, 0x41, 0x41, 0x41, 0x3e], // 'O'
    [0x7f, 0x09, 0x09, 0x09, 0x06], // 'P'
    [0x3e, 0x41, 0x51, 0x21, 0x5e], // 'Q'
    [0x7f, 0x09, 0x19, 0x29, 0x46], // 'R'
    [0x46, 0x49, 0x49, 0x49, 0x31], // 'S'
    [0x01, 0x01, 0x7f, 0x01, 0x01], // 'T'
    [0x3f, 0x40, 0x40, 0x40, 0x3f], // 'U'
    [0x1f, 0x20, 0x40, 0x20, 0x1f], // 'V'
    [0x3f, 0x40, 0x38, 0x40, 0x3f], // 'W'
    [0x63, 0x14, 0x08, 0x14, 0x63], // 'X'
    [0x07, 0x08, 0x70, 0x08, 0x07], // 'Y'
    [0x61, 0x51, 0x49, 0x45, 0x43], // 'Z' (idx 58)
];

pub fn glyph_column(ch: u8, col: usize) -> u8 {
    let idx = match ch {
        b' ' => 0usize,
        b'-' => 13,
        b'.' => 14,
        b'0'..=b'9' => (ch - b'0') as usize + 16,
        b':' => 0,
        b'A'..=b'Z' => (ch - b'A') as usize + 33,
        _ => 0,
    };
    if col >= CHAR_W {
        return 0;
    }
    FONT5X7[idx][col]
}

fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    (((r as u16) & 0xf8) << 8) | (((g as u16) & 0xfc) << 3) | ((b as u16) >> 3)
}

pub fn text_width(text: &str, scale: u8) -> u16 {
    text.len() as u16 * CHAR_STEP as u16 * scale as u16
}

pub struct Display;

impl Display {
    pub unsafe fn init() {
        configure_output(PIN_BL);
        set_backlight(false);

        spi2_init();
        delay_ms(120);

        tx_cmd(0x01, &[]);
        delay_ms(150);
        tx_cmd(0x36, &[0x70]);
        tx_cmd(0x3A, &[0x55]);

        for init in AXS15231B_INIT {
            tx_cmd(init.cmd, init.data);
            if init.delay_ms != 0 {
                delay_ms(init.delay_ms);
            }
        }

        tx_cmd(0x29, &[]);
        delay_ms(200);
        Self::fill_rgb565(0xf800);
        set_backlight(true);
    }

    pub unsafe fn fill_rgb565(color: u16) {
        tx_cmd(
            0x2A,
            &[
                0x00,
                0x00,
                ((LCD_WIDTH - 1) >> 8) as u8,
                (LCD_WIDTH - 1) as u8,
            ],
        );
        tx_command_word(0x2C, LCD_OPCODE_WRITE_COLOR, false, true);
        let hi = (color >> 8) as u8;
        let lo = color as u8;
        let mut chunk = [0u8; 64];
        let mut offset = 0;
        while offset < chunk.len() {
            chunk[offset] = hi;
            chunk[offset + 1] = lo;
            offset += 2;
        }

        let mut remaining = LCD_WIDTH as usize * LCD_HEIGHT as usize * 2;
        while remaining != 0 {
            let len = if remaining < chunk.len() {
                remaining
            } else {
                chunk.len()
            };
            let keep_cs = remaining > len;
            spi2_write(&chunk[..len], true, keep_cs);
            remaining -= len;
        }
    }

    pub unsafe fn draw_pixel(x: u16, y: u16, color: u16) {
        if x >= LCD_WIDTH || y >= LCD_HEIGHT {
            return;
        }
        tx_cmd(
            0x2A,
            &[
                (x >> 8) as u8,
                x as u8,
                (x >> 8) as u8,
                x as u8,
            ],
        );
        tx_cmd(
            0x2B,
            &[
                (y >> 8) as u8,
                y as u8,
                (y >> 8) as u8,
                y as u8,
            ],
        );
        tx_command_word(0x2C, LCD_OPCODE_WRITE_COLOR, false, false);
        spi2_write(&[(color >> 8) as u8, color as u8], true, false);
    }

    pub unsafe fn draw_rgb565_with<F>(width: u16, height: u16, mut pixel_rgb565: F)
    where
        F: FnMut(u16, u16) -> u16,
    {
        if width != LCD_WIDTH || height != LCD_HEIGHT {
            return;
        }

        tx_cmd(
            0x2A,
            &[
                0x00,
                0x00,
                ((LCD_WIDTH - 1) >> 8) as u8,
                (LCD_WIDTH - 1) as u8,
            ],
        );
        tx_command_word(0x2C, LCD_OPCODE_WRITE_COLOR, false, true);

        let mut chunk = [0u8; 64];
        let mut py = 0u16;
        while py < height {
            let mut px = 0u16;
            while px < width {
                let mut offset = 0usize;
                while offset < chunk.len() && px < width {
                    let color = pixel_rgb565(px, py);
                    chunk[offset] = (color >> 8) as u8;
                    chunk[offset + 1] = color as u8;
                    offset += 2;
                    px += 1;
                }
                let keep_cs = py + 1 < height || px < width;
                spi2_write(&chunk[..offset], true, keep_cs);
            }
            py += 1;
        }
    }

    pub unsafe fn draw_text(x: u16, y: u16, text: &str, scale: u8, fg: u16, bg: u16) {
        if scale == 0 {
            return;
        }
        for (ci, ch) in text.bytes().enumerate() {
            let mut col = 0usize;
            while col < CHAR_W {
                let bits = glyph_column(ch, col);
                let mut row = 0usize;
                while row < CHAR_H {
                    let on = (bits >> row) & 1;
                    if on != 0 {
                        let px = x + (ci * CHAR_STEP + col) as u16 * scale as u16;
                        let py = y + row as u16 * scale as u16;
                        fill_rect(px, py, scale as u16, scale as u16, fg);
                    }
                    row += 1;
                }
                col += 1;
            }
        }
    }

    pub unsafe fn fill_rect(x: u16, y: u16, w: u16, h: u16, color: u16) {
        fill_rect(x, y, w, h, color);
    }
}

unsafe fn fill_rect(x: u16, y: u16, w: u16, h: u16, color: u16) {
    if w == 0 || h == 0 {
        return;
    }
    let x1 = x + w - 1;
    let y1 = y + h - 1;
    tx_cmd(0x2A, &[(x >> 8) as u8, x as u8, (x1 >> 8) as u8, x1 as u8]);
    tx_cmd(0x2B, &[(y >> 8) as u8, y as u8, (y1 >> 8) as u8, y1 as u8]);
    tx_command_word(0x2C, LCD_OPCODE_WRITE_COLOR, false, true);

    let hi = (color >> 8) as u8;
    let lo = color as u8;
    let mut chunk = [0u8; 64];
    let mut offset = 0;
    while offset < chunk.len() {
        chunk[offset] = hi;
        chunk[offset + 1] = lo;
        offset += 2;
    }

    let total = w as u32 * h as u32 * 2;
    let mut remaining = total;
    while remaining != 0 {
        let len = if remaining < chunk.len() as u32 {
            remaining as usize
        } else {
            chunk.len()
        };
        let keep_cs = remaining > len as u32;
        spi2_write(&chunk[..len], true, keep_cs);
        remaining -= len as u32;
    }
}

unsafe fn set_backlight(on: bool) {
    set_pin(PIN_BL, on);
}

unsafe fn tx_cmd(cmd: u8, data: &[u8]) {
    tx_command_word(cmd, LCD_OPCODE_WRITE_CMD, false, !data.is_empty());
    spi2_write(data, false, false);
}

unsafe fn tx_command_word(cmd: u8, opcode: u32, quad: bool, keep_cs: bool) {
    let word = [opcode as u8, 0x00, cmd, 0x00];
    spi2_write(&word, quad, keep_cs);
}

unsafe fn spi2_init() {
    let clk = read_volatile(SYSTEM_PERIP_CLK_EN0);
    write_volatile(SYSTEM_PERIP_CLK_EN0, clk | SYSTEM_SPI2_CLK_EN);

    let rst = read_volatile(SYSTEM_PERIP_RST_EN0);
    write_volatile(SYSTEM_PERIP_RST_EN0, rst | SYSTEM_SPI2_RST);
    write_volatile(SYSTEM_PERIP_RST_EN0, rst & !SYSTEM_SPI2_RST);

    route_spi_pin(PIN_CS, FSPICS0_OUT, FSPICS0_IN);
    route_spi_pin(PIN_CLK, FSPICLK_OUT, FSPICLK_IN);
    route_spi_pin(PIN_DATA0, FSPID_OUT, FSPID_IN);
    route_spi_pin(PIN_DATA1, FSPIQ_OUT, FSPIQ_IN);
    route_spi_pin(PIN_DATA2, FSPIWP_OUT, FSPIWP_IN);
    route_spi_pin(PIN_DATA3, FSPIHD_OUT, FSPIHD_IN);

    write_volatile(
        SPI_CLK_GATE,
        SPI_CLK_EN | SPI_MST_CLK_ACTIVE | SPI_MST_CLK_SEL,
    );
    write_volatile(SPI_SLAVE, 0);
    write_volatile(SPI_DMA_CONF, 0);
    write_volatile(SPI_DMA_INT_CLR, u32::MAX);

    write_volatile(SPI_CLOCK, (3 << 12) | (1 << 6) | 3);
    write_volatile(SPI_CTRL, 0);
    write_volatile(SPI_MISC, SPI_CK_IDLE_EDGE | 0x3e);
    write_volatile(SPI_USER1, 0);
    write_volatile(SPI_USER2, 0);
    write_volatile(SPI_USER, SPI_USR_MOSI);
    spi2_apply_config();
}

unsafe fn spi2_write(data: &[u8], quad: bool, keep_cs: bool) {
    if data.is_empty() {
        return;
    }

    let mut written = 0;
    while written < data.len() {
        let remaining = data.len() - written;
        let len = if remaining < 64 { remaining } else { 64 };
        spi2_write_chunk(&data[written..written + len], quad, keep_cs || remaining > len);
        written += len;
    }
}

unsafe fn spi2_write_chunk(data: &[u8], quad: bool, keep_cs: bool) {
    while (read_volatile(SPI_CMD) & SPI_USR) != 0 {}

    let mut word_index = 0;
    while word_index < 16 {
        write_volatile((SPI_W0 + word_index * 4) as *mut u32, 0);
        word_index += 1;
    }

    let mut byte_index = 0;
    while byte_index < data.len() {
        let mut word = 0u32;
        let mut shift = 0;
        while shift < 32 && byte_index < data.len() {
            word |= (data[byte_index] as u32) << shift;
            byte_index += 1;
            shift += 8;
        }
        write_volatile((SPI_W0 + ((byte_index - 1) / 4) * 4) as *mut u32, word);
    }

    write_volatile(SPI_MS_DLEN, data.len() as u32 * 8 - 1);
    let mut user = SPI_USR_MOSI;
    if quad {
        user |= SPI_FWRITE_QUAD;
    }
    let mut misc = read_volatile(SPI_MISC) & !SPI_CS_KEEP_ACTIVE;
    if keep_cs {
        misc |= SPI_CS_KEEP_ACTIVE;
    }
    write_volatile(SPI_MISC, misc);
    write_volatile(SPI_USER, user);
    spi2_apply_config();
    write_volatile(SPI_CMD, read_volatile(SPI_CMD) | SPI_USR);
    while (read_volatile(SPI_CMD) & SPI_USR) != 0 {}
}

unsafe fn spi2_apply_config() {
    write_volatile(SPI_CMD, read_volatile(SPI_CMD) | SPI_UPDATE);
    while (read_volatile(SPI_CMD) & SPI_UPDATE) != 0 {}
}

unsafe fn route_spi_pin(pin: u8, out_signal: u32, in_signal: u32) {
    configure_matrix_pin(pin);
    write_volatile(
        (GPIO_FUNC_OUT_SEL_CFG + pin as usize * 4) as *mut u32,
        out_signal,
    );
    write_volatile(
        (GPIO_FUNC_IN_SEL_CFG + in_signal as usize * 4) as *mut u32,
        pin as u32 | (1 << 7),
    );
}

unsafe fn configure_matrix_pin(pin: u8) {
    if let Some(mux) = io_mux_reg(pin) {
        let mut val = read_volatile(mux);
        val &= !(MCU_SEL_M | FUN_DRV_M | FUN_IE);
        val |= (GPIO_FUNC << MCU_SEL_S) | (DRIVE_3 << FUN_DRV_S) | FUN_IE;
        write_volatile(mux, val);
    }
}

unsafe fn configure_output(pin: u8) {
    if let Some(mux) = io_mux_reg(pin) {
        let mut val = read_volatile(mux);
        val &= !(MCU_SEL_M | FUN_DRV_M | FUN_IE);
        val |= (GPIO_FUNC << MCU_SEL_S) | (DRIVE_3 << FUN_DRV_S);
        write_volatile(mux, val);
    }

    if pin < 32 {
        write_volatile(GPIO_ENABLE_W1TS, 1u32 << pin);
    } else {
        write_volatile(GPIO_ENABLE1_W1TS, 1u32 << (pin - 32));
    }
}

unsafe fn set_pin(pin: u8, high: bool) {
    let (set, clear, bit) = if pin < 32 {
        (GPIO_OUT_W1TS, GPIO_OUT_W1TC, 1u32 << pin)
    } else {
        (GPIO_OUT1_W1TS, GPIO_OUT1_W1TC, 1u32 << (pin - 32))
    };
    write_volatile(if high { set } else { clear }, bit);
}

fn io_mux_reg(pin: u8) -> Option<*mut u32> {
    let offset = match pin {
        0..=14 => 0x04 + pin as usize * 4,
        19..=21 => 0x50 + (pin as usize - 19) * 4,
        33..=38 => 0x88 + (pin as usize - 33) * 4,
        39 => 0xa0,
        40 => 0xa4,
        41 => 0xa8,
        42 => 0xac,
        43 => 0xb0,
        44 => 0xb4,
        45 => 0xb8,
        46 => 0xbc,
        47 => 0xc0,
        48 => 0xc4,
        _ => return None,
    };
    Some((IO_MUX_BASE + offset) as *mut u32)
}

unsafe fn delay_ms(ms: u32) {
    let mut outer = 0;
    while outer < ms {
        let mut inner = 0;
        while inner < 20_000 {
            core::arch::asm!("nop");
            inner += 1;
        }
        outer += 1;
    }
}

#[derive(Clone, Copy)]
struct InitCommand {
    cmd: u8,
    data: &'static [u8],
    delay_ms: u32,
}

const AXS15231B_INIT: &[InitCommand] = &[
    InitCommand { cmd: 0xBB, data: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5A, 0xA5], delay_ms: 0 },
    InitCommand { cmd: 0xA0, data: &[0xC0, 0x10, 0x00, 0x02, 0x00, 0x00, 0x04, 0x3F, 0x20, 0x05, 0x3F, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00], delay_ms: 0 },
    InitCommand { cmd: 0xA2, data: &[0x30, 0x3C, 0x24, 0x14, 0xD0, 0x20, 0xFF, 0xE0, 0x40, 0x19, 0x80, 0x80, 0x80, 0x20, 0xF9, 0x10, 0x02, 0xFF, 0xFF, 0xF0, 0x90, 0x01, 0x32, 0xA0, 0x91, 0xE0, 0x20, 0x7F, 0xFF, 0x00, 0x5A], delay_ms: 0 },
    InitCommand { cmd: 0xD0, data: &[0xE0, 0x40, 0x51, 0x24, 0x08, 0x05, 0x10, 0x01, 0x20, 0x15, 0x42, 0xC2, 0x22, 0x22, 0xAA, 0x03, 0x10, 0x12, 0x60, 0x14, 0x1E, 0x51, 0x15, 0x00, 0x8A, 0x20, 0x00, 0x03, 0x3A, 0x12], delay_ms: 0 },
    InitCommand { cmd: 0xA3, data: &[0xA0, 0x06, 0xAA, 0x00, 0x08, 0x02, 0x0A, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x55, 0x55], delay_ms: 0 },
    InitCommand { cmd: 0xC1, data: &[0x31, 0x04, 0x02, 0x02, 0x71, 0x05, 0x24, 0x55, 0x02, 0x00, 0x41, 0x00, 0x53, 0xFF, 0xFF, 0xFF, 0x4F, 0x52, 0x00, 0x4F, 0x52, 0x00, 0x45, 0x3B, 0x0B, 0x02, 0x0D, 0x00, 0xFF, 0x40], delay_ms: 0 },
    InitCommand { cmd: 0xC3, data: &[0x00, 0x00, 0x00, 0x50, 0x03, 0x00, 0x00, 0x00, 0x01, 0x80, 0x01], delay_ms: 0 },
    InitCommand { cmd: 0xC4, data: &[0x00, 0x24, 0x33, 0x80, 0x00, 0xEA, 0x64, 0x32, 0xC8, 0x64, 0xC8, 0x32, 0x90, 0x90, 0x11, 0x06, 0xDC, 0xFA, 0x00, 0x00, 0x80, 0xFE, 0x10, 0x10, 0x00, 0x0A, 0x0A, 0x44, 0x50], delay_ms: 0 },
    InitCommand { cmd: 0xC5, data: &[0x18, 0x00, 0x00, 0x03, 0xFE, 0x3A, 0x4A, 0x20, 0x30, 0x10, 0x88, 0xDE, 0x0D, 0x08, 0x0F, 0x0F, 0x01, 0x3A, 0x4A, 0x20, 0x10, 0x10, 0x00], delay_ms: 0 },
    InitCommand { cmd: 0xC6, data: &[0x05, 0x0A, 0x05, 0x0A, 0x00, 0xE0, 0x2E, 0x0B, 0x12, 0x22, 0x12, 0x22, 0x01, 0x03, 0x00, 0x3F, 0x6A, 0x18, 0xC8, 0x22], delay_ms: 0 },
    InitCommand { cmd: 0xC7, data: &[0x50, 0x32, 0x28, 0x00, 0xA2, 0x80, 0x8F, 0x00, 0x80, 0xFF, 0x07, 0x11, 0x9C, 0x67, 0xFF, 0x24, 0x0C, 0x0D, 0x0E, 0x0F], delay_ms: 0 },
    InitCommand { cmd: 0xC9, data: &[0x33, 0x44, 0x44, 0x01], delay_ms: 0 },
    InitCommand { cmd: 0xCF, data: &[0x2C, 0x1E, 0x88, 0x58, 0x13, 0x18, 0x56, 0x18, 0x1E, 0x68, 0x88, 0x00, 0x65, 0x09, 0x22, 0xC4, 0x0C, 0x77, 0x22, 0x44, 0xAA, 0x55, 0x08, 0x08, 0x12, 0xA0, 0x08], delay_ms: 0 },
    InitCommand { cmd: 0xD5, data: &[0x40, 0x8E, 0x8D, 0x01, 0x35, 0x04, 0x92, 0x74, 0x04, 0x92, 0x74, 0x04, 0x08, 0x6A, 0x04, 0x46, 0x03, 0x03, 0x03, 0x03, 0x82, 0x01, 0x03, 0x00, 0xE0, 0x51, 0xA1, 0x00, 0x00, 0x00], delay_ms: 0 },
    InitCommand { cmd: 0xD6, data: &[0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE, 0x93, 0x00, 0x01, 0x83, 0x07, 0x07, 0x00, 0x07, 0x07, 0x00, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x00, 0x84, 0x00, 0x20, 0x01, 0x00], delay_ms: 0 },
    InitCommand { cmd: 0xD7, data: &[0x03, 0x01, 0x0B, 0x09, 0x0F, 0x0D, 0x1E, 0x1F, 0x18, 0x1D, 0x1F, 0x19, 0x40, 0x8E, 0x04, 0x00, 0x20, 0xA0, 0x1F], delay_ms: 0 },
    InitCommand { cmd: 0xD8, data: &[0x02, 0x00, 0x0A, 0x08, 0x0E, 0x0C, 0x1E, 0x1F, 0x18, 0x1D, 0x1F, 0x19], delay_ms: 0 },
    InitCommand { cmd: 0xD9, data: &[0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F], delay_ms: 0 },
    InitCommand { cmd: 0xDD, data: &[0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F], delay_ms: 0 },
    InitCommand { cmd: 0xDF, data: &[0x44, 0x73, 0x4B, 0x69, 0x00, 0x0A, 0x02, 0x90], delay_ms: 0 },
    InitCommand { cmd: 0xE0, data: &[0x3B, 0x28, 0x10, 0x16, 0x0C, 0x06, 0x11, 0x28, 0x5C, 0x21, 0x0D, 0x35, 0x13, 0x2C, 0x33, 0x28, 0x0D], delay_ms: 0 },
    InitCommand { cmd: 0xE1, data: &[0x37, 0x28, 0x10, 0x16, 0x0B, 0x06, 0x11, 0x28, 0x5C, 0x21, 0x0D, 0x35, 0x14, 0x2C, 0x33, 0x28, 0x0F], delay_ms: 0 },
    InitCommand { cmd: 0xE2, data: &[0x3B, 0x07, 0x12, 0x18, 0x0E, 0x0D, 0x17, 0x35, 0x44, 0x32, 0x0C, 0x14, 0x14, 0x36, 0x3A, 0x2F, 0x0D], delay_ms: 0 },
    InitCommand { cmd: 0xE3, data: &[0x37, 0x07, 0x12, 0x18, 0x0E, 0x0D, 0x17, 0x35, 0x44, 0x32, 0x0C, 0x14, 0x14, 0x36, 0x32, 0x2F, 0x0F], delay_ms: 0 },
    InitCommand { cmd: 0xE4, data: &[0x3B, 0x07, 0x12, 0x18, 0x0E, 0x0D, 0x17, 0x39, 0x44, 0x2E, 0x0C, 0x14, 0x14, 0x36, 0x3A, 0x2F, 0x0D], delay_ms: 0 },
    InitCommand { cmd: 0xE5, data: &[0x37, 0x07, 0x12, 0x18, 0x0E, 0x0D, 0x17, 0x39, 0x44, 0x2E, 0x0C, 0x14, 0x14, 0x36, 0x3A, 0x2F, 0x0F], delay_ms: 0 },
    InitCommand { cmd: 0xA4, data: &[0x85, 0x85, 0x95, 0x82, 0xAF, 0xAA, 0xAA, 0x80, 0x10, 0x30, 0x40, 0x40, 0x20, 0xFF, 0x60, 0x30], delay_ms: 0 },
    InitCommand { cmd: 0xA4, data: &[0x85, 0x85, 0x95, 0x85], delay_ms: 0 },
    InitCommand { cmd: 0xBB, data: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], delay_ms: 0 },
    InitCommand { cmd: 0x13, data: &[], delay_ms: 0 },
    InitCommand { cmd: 0x11, data: &[], delay_ms: 120 },
    InitCommand { cmd: 0x2C, data: &[0x00, 0x00, 0x00, 0x00], delay_ms: 0 },
];
