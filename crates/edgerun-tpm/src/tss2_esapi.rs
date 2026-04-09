//! Vendored TSS2 ESAPI FFI bindings.
//!
//! Minimal set of types and extern "C" function declarations needed by
//! `TpmDevice::create_ecdsa_p256_signing_key`.  Replaces the `tss-esapi-sys`
//! dependency with ~200 lines of raw `#[repr(C)]` structs and `extern "C"`
//! declarations.

// ---------------------------------------------------------------------------
// TPM 2.0 algorithm / attribute constants
// ---------------------------------------------------------------------------

pub const TPM2_ALG_ECC: u16 = 0x0023;
pub const TPM2_ALG_SHA256: u16 = 0x000B;
pub const TPM2_ALG_ECDSA: u16 = 0x0018;
pub const TPM2_ALG_NULL: u16 = 0x0010;
pub const TPM2_ECC_NIST_P256: u16 = 0x0003;
pub const TPM2_RH_OWNER: u32 = 0x4000_0001;

pub const TPMA_OBJECT_SIGN_ENCRYPT: u32 = 0x0004_0000;
pub const TPMA_OBJECT_FIXED_TPM: u32 = 0x0000_0002;
pub const TPMA_OBJECT_FIXED_PARENT: u32 = 0x0000_0010;
pub const TPMA_OBJECT_SENSITIVE_DATA_ORIGIN: u32 = 0x0000_0020;
pub const TPMA_OBJECT_USER_WITH_AUTH: u32 = 0x0000_0040;
pub const TPMA_OBJECT_NODA: u32 = 0x0000_0400;

// ---------------------------------------------------------------------------
// ESYS handles
// ---------------------------------------------------------------------------

pub type ESYS_TR = u32;
pub const ESYS_TR_NONE: ESYS_TR = 0x0000_0FFF;
pub const ESYS_TR_PASSWORD: ESYS_TR = 0x0040_0009;

// ---------------------------------------------------------------------------
// Opaque context pointers
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct TSS2_TCTI_CONTEXT {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

#[repr(C)]
pub struct ESYS_CONTEXT {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

// ---------------------------------------------------------------------------
// TPM2B types (size-prefixed buffers)
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct TPM2B_DIGEST {
    pub size: u16,
    pub buffer: [u8; 64],
}

#[repr(C)]
pub struct TPM2B_DATA {
    pub size: u16,
    pub buffer: [u8; 64],
}

#[repr(C)]
pub struct TPM2B_ECC_PARAMETER {
    pub size: u16,
    pub buffer: [u8; 128],
}

#[repr(C)]
pub struct TPM2B_PUBLIC_KEY_RSA {
    pub size: u16,
    pub buffer: [u8; 256],
}

#[repr(C)]
pub struct TPM2B_SENSITIVE_DATA {
    pub size: u16,
    pub buffer: [u8; 256],
}

#[repr(C)]
pub struct TPM2B_AUTH {
    pub size: u16,
    pub buffer: [u8; 64],
}

#[repr(C)]
pub struct TPM2B_PRIVATE {
    pub size: u16,
    pub buffer: [u8; 1344],
}

// ---------------------------------------------------------------------------
// Scheme / parameter unions
// ---------------------------------------------------------------------------

#[repr(C)]
pub union TPMU_ASYM_SCHEME {
    pub _padding: [u8; 8],
}

#[repr(C)]
pub struct TPMS_SIG_SCHEME_ECDSA {
    pub _padding: [u8; 8],
}

#[repr(C)]
pub struct TPMU_KEYEDHASH_SCHEME {
    pub _padding: [u8; 8],
}

#[repr(C)]
pub struct TPMU_PUBLIC_PARMS {
    pub eccDetail: TPMS_ECC_PARMS,
}

#[repr(C)]
pub struct TPMU_PUBLIC_ID {
    pub ecc: TPMS_ECC_POINT,
}

#[repr(C)]
pub struct TPMS_ECC_PARMS {
    pub symmetric: TPMT_SYM_DEF_OBJECT,
    pub scheme: TPMT_ECC_SCHEME,
    pub curveID: u16,
    pub kdf: TPMT_KDF_SCHEME,
}

#[repr(C)]
pub struct TPMT_SYM_DEF_OBJECT {
    pub algorithm: u16,
    pub keyBits: TPMT_SYM_DEF_OBJECT_UNION,
    pub mode: TPMT_SYM_DEF_OBJECT_MODE_UNION,
}

#[repr(C)]
pub union TPMT_SYM_DEF_OBJECT_UNION {
    pub _padding: [u8; 4],
}

#[repr(C)]
pub union TPMT_SYM_DEF_OBJECT_MODE_UNION {
    pub _padding: [u8; 4],
}

#[repr(C)]
pub struct TPMT_ECC_SCHEME {
    pub scheme: u16,
    pub hashAlg: u16,
}

#[repr(C)]
pub struct TPMT_KDF_SCHEME {
    pub scheme: u16,
    pub details: TPMT_KDF_SCHEME_UNION,
}

#[repr(C)]
pub union TPMT_KDF_SCHEME_UNION {
    pub _padding: [u8; 8],
}

#[repr(C)]
pub struct TPMS_ECC_POINT {
    pub x: TPM2B_ECC_PARAMETER,
    pub y: TPM2B_ECC_PARAMETER,
}

// ---------------------------------------------------------------------------
// TPMT_PUBLIC / TPM2B_PUBLIC
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct TPMT_PUBLIC {
    pub type_: u16,
    pub nameAlg: u16,
    pub objectAttributes: u32,
    pub authPolicy: TPM2B_DIGEST,
    pub parameters: TPMU_PUBLIC_PARMS,
    pub unique: TPMU_PUBLIC_ID,
}

#[repr(C)]
pub struct TPM2B_PUBLIC {
    pub size: u16,
    pub publicArea: TPMT_PUBLIC,
}

// ---------------------------------------------------------------------------
// TPM2B_SENSITIVE_CREATE
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct TPMS_SENSITIVE_CREATE {
    pub userAuth: TPM2B_AUTH,
    pub data: TPM2B_SENSITIVE_DATA,
}

#[repr(C)]
pub struct TPM2B_SENSITIVE_CREATE {
    pub size: u16,
    pub sensitive: TPMS_SENSITIVE_CREATE,
}

// ---------------------------------------------------------------------------
// TPML_PCR_SELECTION
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct TPMS_PCR_SELECTION {
    pub hash: u16,
    pub sizeofSelect: u8,
    pub pcrSelect: [u8; 3],
}

#[repr(C)]
pub struct TPML_PCR_SELECTION {
    pub count: u32,
    pub pcrSelections: [TPMS_PCR_SELECTION; 16],
}

// ---------------------------------------------------------------------------
// TPM2B_CREATION_DATA, TPM2B_ATTEST, TPMT_TK_CREATION (unused outs)
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct TPM2B_CREATION_DATA {
    pub size: u16,
    pub creationData: [u8; 1344],
}

#[repr(C)]
pub struct TPM2B_ATTEST {
    pub size: u16,
    pub attestationData: [u8; 2048],
}

#[repr(C)]
pub struct TPMT_TK_CREATION {
    pub tag: u16,
    pub hierarchy: u32,
    pub digest: TPM2B_DIGEST,
}

// ---------------------------------------------------------------------------
// TCTI Loader
// ---------------------------------------------------------------------------

extern "C" {
    pub fn Tss2_TctiLdr_Initialize(
        name: *const u8,
        tctiContext: *mut *mut TSS2_TCTI_CONTEXT,
    ) -> u32;

    pub fn Tss2_TctiLdr_Finalize(
        tctiContext: *mut *mut TSS2_TCTI_CONTEXT,
    );
}

// ---------------------------------------------------------------------------
// ESYS API
// ---------------------------------------------------------------------------

extern "C" {
    pub fn Esys_Initialize(
        esys_context: *mut *mut ESYS_CONTEXT,
        tcti: *mut TSS2_TCTI_CONTEXT,
        info: *const (),
    ) -> u32;

    pub fn Esys_Finalize(
        context: *mut *mut ESYS_CONTEXT,
    );

    pub fn Esys_Startup(
        esysContext: *mut ESYS_CONTEXT,
        startupType: u16,
    ) -> u32;

    pub fn Esys_CreatePrimary(
        esysContext: *mut ESYS_CONTEXT,
        primaryHandle: ESYS_TR,
        shandle1: ESYS_TR,
        shandle2: ESYS_TR,
        shandle3: ESYS_TR,
        inSensitive: *const TPM2B_SENSITIVE_CREATE,
        inPublic: *const TPM2B_PUBLIC,
        outsideInfo: *const TPM2B_DATA,
        creationPCR: *const TPML_PCR_SELECTION,
        objectHandle: *mut ESYS_TR,
        outPublic: *mut *mut TPM2B_PUBLIC,
        creationData: *mut *mut TPM2B_CREATION_DATA,
        creationHash: *mut *mut TPM2B_DIGEST,
        creationTicket: *mut *mut TPMT_TK_CREATION,
    ) -> u32;

    pub fn Esys_EvictControl(
        esysContext: *mut ESYS_CONTEXT,
        auth: ESYS_TR,
        objectHandle: ESYS_TR,
        shandle1: ESYS_TR,
        shandle2: ESYS_TR,
        shandle3: ESYS_TR,
        persistentHandle: u32,
        newObjectHandle: *mut ESYS_TR,
    ) -> u32;

    pub fn Esys_FlushContext(
        esysContext: *mut ESYS_CONTEXT,
        flushHandle: ESYS_TR,
    ) -> u32;
}
