// TPM2 wire protocol constants (from tpm2-tss tss2_tpm2_types.h)

// Structure tags
pub const TPM_ST_NO_SESSIONS: u16 = 0x8001;
pub const TPM_ST_SESSIONS: u16 = 0x8002;
pub const TPM_ST_HASHCHECK: u16 = 0x8024;

// Response codes
pub const TPM_RC_SUCCESS: u32 = 0;

// Command codes
pub const TPM_CC_READ_PUBLIC: u32 = 0x0000_0173;
pub const TPM_CC_SIGN: u32 = 0x0000_015D;
pub const TPM_CC_HASH: u32 = 0x0000_017D;
pub const TPM_CC_GET_RANDOM: u32 = 0x0000_017B;
pub const TPM_CC_VERIFY_SIGNATURE: u32 = 0x0000_0177;
pub const TPM_CC_START_AUTH_SESSION: u32 = 0x0000_0176;
pub const TPM_CC_POLICY_COMMAND_CODE: u32 = 0x0000_016C;
pub const TPM_CC_POLICY_PCR: u32 = 0x0000_017F;
pub const TPM_CC_POLICY_AUTHORIZE: u32 = 0x0000_016A;
pub const TPM_CC_CREATE_PRIMARY: u32 = 0x0000_0131;
pub const TPM_CC_EVICT_CONTROL: u32 = 0x0000_0120;
pub const TPM_CC_FLUSH_CONTEXT: u32 = 0x0000_0165;
pub const TPM_CC_STARTUP: u32 = 0x0000_0144;
pub const TPM_CC_SHUTDOWN: u32 = 0x0000_0145;

// Startup types
pub const TPM_SU_CLEAR: u16 = 0x0000;
pub const TPM_SU_STATE: u16 = 0x0001;

// Handle constants
pub const TPM_PERSISTENT_FIRST: u32 = 0x8100_0000;
pub const TPM_RS_PW: u32 = 0x4000_0009;
pub const TPM_RH_OWNER: u32 = 0x4000_0001;
pub const TPM_RH_NULL: u32 = 0x4000_0007;

// Session types
pub const TPM_SE_HMAC: u8 = 0x00;

// ECC curves
pub const TPM_ECC_NIST_P256: u16 = 0x0003;
pub const TPM_ALG_ECDSA: u16 = 0x0018;

// Algorithm IDs
pub const TPM_ALG_SHA256: u16 = 0x000B;
pub const TPM_ALG_NULL: u16 = 0x0010;
pub const TPM_ALG_ECC: u16 = 0x0023;

// TPMA_OBJECT bits
pub const TPMA_OBJECT_FIXED_TPM: u32 = 0x0000_0002;
pub const TPMA_OBJECT_FIXED_PARENT: u32 = 0x0000_0010;
pub const TPMA_OBJECT_SENSITIVE_DATA_ORIGIN: u32 = 0x0000_0020;
pub const TPMA_OBJECT_USER_WITH_AUTH: u32 = 0x0000_0040;
pub const TPMA_OBJECT_NODA: u32 = 0x0000_0400;
pub const TPMA_OBJECT_SIGN_ENCRYPT: u32 = 0x0004_0000;
pub const TPMA_OBJECT_DECRYPT: u32 = 0x0002_0000;
pub const TPMA_OBJECT_RESTRICTED: u32 = 0x0001_0000;
