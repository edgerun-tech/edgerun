#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UartPort {
    Uart1 = 1,
    Uart2 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaudRate {
    Baud9600 = 9600,
    Baud19200 = 19200,
    Baud38400 = 38400,
    Baud57600 = 57600,
    #[default]
    Baud115200 = 115200,
    Baud230400 = 230400,
    Baud460800 = 460800,
    Baud921600 = 921600,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataBits {
    #[default]
    Bits8 = 8,
    Bits7 = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StopBits {
    #[default]
    One = 1,
    Two = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Parity {
    #[default]
    None = 0,
    Odd = 1,
    Even = 2,
}

pub struct UartConfig {
    pub port: UartPort,
    pub baud: BaudRate,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: bool,
}

impl UartConfig {
    pub const fn new(port: UartPort) -> Self {
        Self {
            port,
            baud: BaudRate::Baud115200,
            data_bits: DataBits::Bits8,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow_control: false,
        }
    }

    pub const fn with_baud(mut self, baud: BaudRate) -> Self {
        self.baud = baud;
        self
    }

    pub const fn with_flow_control(mut self, enable: bool) -> Self {
        self.flow_control = enable;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpioMode {
    #[default]
    Input = 0,
    Output = 1,
    Alternate = 2,
    Analog = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpioPull {
    #[default]
    None = 0,
    Up = 1,
    Down = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioState {
    Low = 0,
    High = 1,
}

pub struct GpioConfig {
    pub pin: u8,
    pub mode: GpioMode,
    pub pull: GpioPull,
    pub initial_state: GpioState,
}

impl GpioConfig {
    pub const fn new(pin: u8) -> Self {
        Self {
            pin,
            mode: GpioMode::Input,
            pull: GpioPull::None,
            initial_state: GpioState::Low,
        }
    }

    pub const fn as_output(mut self, initial: GpioState) -> Self {
        self.mode = GpioMode::Output;
        self.initial_state = initial;
        self
    }

    pub const fn with_pull(mut self, pull: GpioPull) -> Self {
        self.pull = pull;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcChannel {
    Adc0 = 0,
    Adc1 = 1,
}

impl AdcChannel {
    pub fn voltage_divider(ohms: u32) -> u32 {
        1800 * ohms / (1800 + ohms)
    }
}

pub struct AdcConfig {
    pub channel: AdcChannel,
    pub vref: u32,
    pub resolution: u32,
}

impl AdcConfig {
    pub const fn new(channel: AdcChannel) -> Self {
        Self {
            channel,
            vref: 1800,
            resolution: 12,
        }
    }

    pub const fn with_vref(mut self, vref: u32) -> Self {
        self.vref = vref;
        self
    }
}

pub struct Interfaces {
    pub uart: UartConfig,
    pub gpio: GpioConfig,
    pub adc: AdcConfig,
}

impl Default for Interfaces {
    fn default() -> Self {
        Self::new()
    }
}

impl Interfaces {
    pub fn new() -> Self {
        Self {
            uart: UartConfig::new(UartPort::Uart1),
            gpio: GpioConfig::new(1),
            adc: AdcConfig::new(AdcChannel::Adc0),
        }
    }
}
use crate::prelude::v1::*;
