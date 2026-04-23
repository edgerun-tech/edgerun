/// Provider Registry State
/// 
/// Data layout (128 bytes):
///   [0..32]   authority (32)
///   [32..40]  collateral_staked (8)  
///   [40..44]  cpu_cores (4)
///   [44..52]  memory_bytes (8)
///   [52..60]  storage_bytes (8)
///   [60..64]  network_mbits (4)
///   [64..72]  total_earnings (8)
///   [72..76]  slash_count (4)
///   [76..80]  uptime_percent (4)
///   [80]      status (1)
///   [81..128] reserved (47)

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ProviderStatus {
    Active = 0,
    Paused = 1,
    Slashed = 2,
}

pub const PROVIDER_SIZE: usize = 128;

// Collateral constants (USDC micro-units)
pub const COLLATERAL_PER_CORE: u64 = 100_000_000;      // 100 USDC per core
pub const COLLATERAL_PER_GIB_RAM: u64 = 50_000_000;  
pub const COLLATERAL_PER_GIB_STORAGE: u64 = 10_000_000;
pub const COLLATERAL_PER_MBIT: u64 = 5_000_000;

/// Unpack provider from account data
pub fn unpack_provider(data: &[u8]) -> Result<ProviderData, &'static str> {
    if data.len() < PROVIDER_SIZE {
        return Err("data too small");
    }
    
    let mut p = ProviderData {
        authority: [0u8; 32],
        collateral_staked: 0,
        cpu_cores: 0,
        memory_bytes: 0,
        storage_bytes: 0,
        network_mbits: 0,
        total_earnings: 0,
        slash_count: 0,
        uptime_percent: 0,
        status: ProviderStatus::Active,
    };
    
    p.authority.copy_from_slice(&data[0..32]);
    p.collateral_staked = u64::from_le_bytes(data[32..40].try_into().unwrap());
    p.cpu_cores = u32::from_le_bytes(data[40..44].try_into().unwrap());
    p.memory_bytes = u64::from_le_bytes(data[44..52].try_into().unwrap());
    p.storage_bytes = u64::from_le_bytes(data[52..60].try_into().unwrap());
    p.network_mbits = u32::from_le_bytes(data[60..64].try_into().unwrap());
    p.total_earnings = u64::from_le_bytes(data[64..72].try_into().unwrap());
    p.slash_count = u32::from_le_bytes(data[72..76].try_into().unwrap());
    p.uptime_percent = u32::from_le_bytes(data[76..80].try_into().unwrap());
    p.status = match data[80] {
        0 => ProviderStatus::Active,
        1 => ProviderStatus::Paused,
        2 => ProviderStatus::Slashed,
        _ => ProviderStatus::Active,
    };
    
    Ok(p)
}

/// Pack provider into account data
pub fn pack_provider(p: &ProviderData, data: &mut [u8]) -> Result<(), &'static str> {
    if data.len() < PROVIDER_SIZE {
        return Err("data too small");
    }
    
    data[0..32].copy_from_slice(&p.authority);
    data[32..40].copy_from_slice(&p.collateral_staked.to_le_bytes());
    data[40..44].copy_from_slice(&p.cpu_cores.to_le_bytes());
    data[44..52].copy_from_slice(&p.memory_bytes.to_le_bytes());
    data[52..60].copy_from_slice(&p.storage_bytes.to_le_bytes());
    data[60..64].copy_from_slice(&p.network_mbits.to_le_bytes());
    data[64..72].copy_from_slice(&p.total_earnings.to_le_bytes());
    data[72..76].copy_from_slice(&p.slash_count.to_le_bytes());
    data[76..80].copy_from_slice(&p.uptime_percent.to_le_bytes());
    data[80] = p.status as u8;
    
    Ok(())
}

/// Provider data
#[derive(Debug, Default)]
pub struct ProviderData {
    pub authority: [u8; 32],
    pub collateral_staked: u64,
    pub cpu_cores: u32,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_mbits: u32,
    pub total_earnings: u64,
    pub slash_count: u32,
    pub uptime_percent: u32,
    pub status: ProviderStatus,
}

use ::core::convert::TryInto;