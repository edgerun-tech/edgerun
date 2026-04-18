#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Pin {
    Gnd = 0,
    VbatBb = 1,
    VbatRf = 2,
    VddExt = 3,
    ResetN = 4,
    PwrKey = 5,
    WDisable = 6,
    GpsPwr = 7,
    USimDet = 8,
    USimVdd = 9,
    USimData = 10,
    USimClk = 11,
    USimRst = 12,
    Uart1Rxd = 13,
    Uart1Txd = 14,
    Uart1RtsN = 15,
    Uart1CtsN = 16,
    Uart2Rxd = 17,
    Uart2Txd = 18,
    Adc0 = 19,
    Adc1 = 20,
    NetMode = 21,
    NetStatus = 22,
    Gpio1 = 23,
    Gpio2 = 24,
    Gpio3 = 25,
    Gpio4 = 26,
    Gpio5 = 27,
    AntMain = 28,
    GndAnt = 29,
}

impl Pin {
    pub fn name(self) -> &'static str {
        match self {
            Pin::Gnd => "GND",
            Pin::VbatBb => "VBAT_BB",
            Pin::VbatRf => "VBAT_RF",
            Pin::VddExt => "VDD_EXT",
            Pin::ResetN => "RESET_N",
            Pin::PwrKey => "PWRKEY",
            Pin::WDisable => "WDISABLE",
            Pin::GpsPwr => "GPS_PWR",
            Pin::USimDet => "USIM_DET",
            Pin::USimVdd => "USIM_VDD",
            Pin::USimData => "USIM_DATA",
            Pin::USimClk => "USIM_CLK",
            Pin::USimRst => "USIM_RST",
            Pin::Uart1Rxd => "UART1_RXD",
            Pin::Uart1Txd => "UART1_TXD",
            Pin::Uart1RtsN => "UART1_RTS_N",
            Pin::Uart1CtsN => "UART1_CTS_N",
            Pin::Uart2Rxd => "UART2_RXD",
            Pin::Uart2Txd => "UART2_TXD",
            Pin::Adc0 => "ADC0",
            Pin::Adc1 => "ADC1",
            Pin::NetMode => "NET_MODE",
            Pin::NetStatus => "NET_STATUS",
            Pin::Gpio1 => "GPIO1",
            Pin::Gpio2 => "GPIO2",
            Pin::Gpio3 => "GPIO3",
            Pin::Gpio4 => "GPIO4",
            Pin::Gpio5 => "GPIO5",
            Pin::AntMain => "ANT_MAIN",
            Pin::GndAnt => "GND_ANT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PinFunction {
    Power,
    Reset,
    Sim,
    Uart,
    Gpio,
    Adc,
    Antenna,
    Rf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinConfig {
    pub pin: Pin,
    pub function: PinFunction,
    pub voltage_level_mv: u32,
    pub description: &'static str,
}

impl PinConfig {
    pub const fn new(
        pin: Pin,
        function: PinFunction,
        voltage_level_mv: u32,
        description: &'static str,
    ) -> Self {
        Self {
            pin,
            function,
            voltage_level_mv,
            description,
        }
    }
}

pub const PINS: &[PinConfig] = &[
    PinConfig::new(
        Pin::VbatBb,
        PinFunction::Power,
        3300,
        "Baseband power supply (3.3-4.3V)",
    ),
    PinConfig::new(
        Pin::VbatRf,
        PinFunction::Power,
        3300,
        "RF power supply (3.3-4.3V)",
    ),
    PinConfig::new(
        Pin::VddExt,
        PinFunction::Power,
        1800,
        "External circuit supply (1.8V)",
    ),
    PinConfig::new(
        Pin::ResetN,
        PinFunction::Reset,
        1800,
        "Module reset (active low)",
    ),
    PinConfig::new(
        Pin::PwrKey,
        PinFunction::Reset,
        1800,
        "Power key (active low to power on)",
    ),
    PinConfig::new(
        Pin::USimDet,
        PinFunction::Sim,
        1800,
        "USIM card detect (hot-plug support)",
    ),
    PinConfig::new(
        Pin::USimVdd,
        PinFunction::Sim,
        1800,
        "USIM card supply (1.8/3.0V)",
    ),
    PinConfig::new(Pin::USimData, PinFunction::Sim, 1800, "USIM data I/O"),
    PinConfig::new(Pin::USimClk, PinFunction::Sim, 1800, "USIM clock"),
    PinConfig::new(Pin::USimRst, PinFunction::Sim, 1800, "USIM reset"),
    PinConfig::new(Pin::Uart1Rxd, PinFunction::Uart, 1800, "UART1 receive data"),
    PinConfig::new(
        Pin::Uart1Txd,
        PinFunction::Uart,
        1800,
        "UART1 transmit data",
    ),
    PinConfig::new(
        Pin::Uart1RtsN,
        PinFunction::Uart,
        1800,
        "UART1 request to send",
    ),
    PinConfig::new(
        Pin::Uart1CtsN,
        PinFunction::Uart,
        1800,
        "UART1 clear to send",
    ),
    PinConfig::new(Pin::Uart2Rxd, PinFunction::Uart, 1800, "UART2 receive data"),
    PinConfig::new(
        Pin::Uart2Txd,
        PinFunction::Uart,
        1800,
        "UART2 transmit data",
    ),
    PinConfig::new(Pin::Adc0, PinFunction::Adc, 0, "ADC input channel 0"),
    PinConfig::new(Pin::Adc1, PinFunction::Adc, 0, "ADC input channel 1"),
    PinConfig::new(Pin::NetMode, PinFunction::Gpio, 1800, "Network mode select"),
    PinConfig::new(
        Pin::NetStatus,
        PinFunction::Gpio,
        1800,
        "Network status indicator",
    ),
    PinConfig::new(
        Pin::Gpio1,
        PinFunction::Gpio,
        1800,
        "General purpose GPIO 1",
    ),
    PinConfig::new(
        Pin::Gpio2,
        PinFunction::Gpio,
        1800,
        "General purpose GPIO 2",
    ),
    PinConfig::new(
        Pin::Gpio3,
        PinFunction::Gpio,
        1800,
        "General purpose GPIO 3",
    ),
    PinConfig::new(
        Pin::Gpio4,
        PinFunction::Gpio,
        1800,
        "General purpose GPIO 4",
    ),
    PinConfig::new(
        Pin::Gpio5,
        PinFunction::Gpio,
        1800,
        "General purpose GPIO 5",
    ),
    PinConfig::new(
        Pin::AntMain,
        PinFunction::Antenna,
        0,
        "Main antenna connection",
    ),
];
