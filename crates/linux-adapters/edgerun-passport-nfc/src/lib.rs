#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use edgerun_crypto::des::{
    iso9797_alg3_mac_3des2, iso9797_method2_pad, iso9797_method2_unpad, Tdes2,
};
use edgerun_crypto::fill_random;
use edgerun_crypto::sha1::{Digest, Sha1};
use edgerun_crypto::{sha256, sha384, sha512};
use edgerun_nfc::NfcReader;

pub const EMRTD_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x02, 0x47, 0x10, 0x01];
pub const EF_COM: u16 = 0x011E;
pub const EF_SOD: u16 = 0x011D;
pub const EF_CARD_ACCESS: u16 = 0x011C;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataGroup {
    Dg1,
    Dg2,
    Dg3,
    Dg4,
    Dg5,
    Dg6,
    Dg7,
    Dg8,
    Dg9,
    Dg10,
    Dg11,
    Dg12,
    Dg13,
    Dg14,
    Dg15,
    Dg16,
}

impl DataGroup {
    pub fn file_id(self) -> u16 {
        match self {
            DataGroup::Dg1 => 0x0101,
            DataGroup::Dg2 => 0x0102,
            DataGroup::Dg3 => 0x0103,
            DataGroup::Dg4 => 0x0104,
            DataGroup::Dg5 => 0x0105,
            DataGroup::Dg6 => 0x0106,
            DataGroup::Dg7 => 0x0107,
            DataGroup::Dg8 => 0x0108,
            DataGroup::Dg9 => 0x0109,
            DataGroup::Dg10 => 0x010A,
            DataGroup::Dg11 => 0x010B,
            DataGroup::Dg12 => 0x010C,
            DataGroup::Dg13 => 0x010D,
            DataGroup::Dg14 => 0x010E,
            DataGroup::Dg15 => 0x010F,
            DataGroup::Dg16 => 0x0110,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0x61 => Some(DataGroup::Dg1),
            0x75 => Some(DataGroup::Dg2),
            0x63 => Some(DataGroup::Dg3),
            0x76 => Some(DataGroup::Dg4),
            0x65 => Some(DataGroup::Dg5),
            0x66 => Some(DataGroup::Dg6),
            0x67 => Some(DataGroup::Dg7),
            0x68 => Some(DataGroup::Dg8),
            0x69 => Some(DataGroup::Dg9),
            0x6A => Some(DataGroup::Dg10),
            0x6B => Some(DataGroup::Dg11),
            0x6C => Some(DataGroup::Dg12),
            0x6D => Some(DataGroup::Dg13),
            0x6E => Some(DataGroup::Dg14),
            0x6F => Some(DataGroup::Dg15),
            0x70 => Some(DataGroup::Dg16),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusWord(pub u8, pub u8);

impl StatusWord {
    pub const OK: Self = Self(0x90, 0x00);

    pub fn is_ok(self) -> bool {
        self == Self::OK
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PassportError {
    Transport(String),
    Status(StatusWord),
    MalformedResponse,
    MalformedTlv,
    MalformedMrz,
    InvalidCheckDigit { field: &'static str },
    FileTooLarge,
    Unsupported(&'static str),
    RandomFailed,
    AuthenticationFailed,
    SecureMessagingFailed,
}

impl From<edgerun_capabilities::CapabilityError> for PassportError {
    fn from(value: edgerun_capabilities::CapabilityError) -> Self {
        PassportError::Transport(value.to_string())
    }
}

pub type Result<T> = core::result::Result<T, PassportError>;

impl X509CertificateInfo {
    pub fn is_issued_by(&self, issuer: &X509CertificateInfo) -> bool {
        self.issuer_der == issuer.subject_der
    }

    pub fn is_self_issued(&self) -> bool {
        self.issuer_der == self.subject_der
    }
}

impl CmsSignedDataInfo {
    pub fn signer_certificate_candidates(&self) -> Vec<&CmsCertificate> {
        self.certificates
            .iter()
            .filter(|certificate| certificate.info.is_some())
            .collect()
    }
}

pub fn verify_cms_signatures(cms: &CmsSignedDataInfo) -> Vec<CmsSignatureVerification> {
    let mut out = Vec::new();
    for (signer_index, signer) in cms.signer_infos.iter().enumerate() {
        let algorithm = cms_signature_algorithm(signer);
        let Some(message) = signer.signed_attributes_signature_input_der.as_deref() else {
            out.push(CmsSignatureVerification {
                signer_index,
                certificate_index: None,
                algorithm,
                verified: false,
                error: Some("missing signed attributes".to_string()),
            });
            continue;
        };

        let mut found_candidate = false;
        for (certificate_index, certificate) in cms.certificates.iter().enumerate() {
            let Some(cert) = &certificate.info else {
                continue;
            };
            if !is_supported_signature_public_key_algorithm(
                cert.subject_public_key_algorithm_oid.as_deref(),
            ) {
                continue;
            }
            found_candidate = true;
            let result = verify_signature(
                algorithm,
                &cert.subject_public_key_der,
                message,
                &signer.signature,
            );
            let verified = result.is_ok();
            out.push(CmsSignatureVerification {
                signer_index,
                certificate_index: Some(certificate_index),
                algorithm,
                verified,
                error: result.err(),
            });
            if verified {
                break;
            }
        }

        if !found_candidate {
            out.push(CmsSignatureVerification {
                signer_index,
                certificate_index: None,
                algorithm,
                verified: false,
                error: Some("no supported signer certificate candidate".to_string()),
            });
        }
    }
    out
}

pub fn verify_document_signer_certificates(
    cms: &CmsSignedDataInfo,
    trust_anchors: &[X509CertificateInfo],
) -> Vec<CertificateChainVerification> {
    let mut out = Vec::new();
    for (certificate_index, certificate) in cms.certificates.iter().enumerate() {
        let Some(cert) = &certificate.info else {
            out.push(CertificateChainVerification {
                certificate_index,
                trust_anchor_index: None,
                algorithm: CmsSignatureAlgorithm::Unsupported,
                verified: false,
                error: Some("unparsed certificate".to_string()),
            });
            continue;
        };

        let algorithm = certificate_signature_algorithm(cert);
        let mut found_anchor = false;
        for (trust_anchor_index, anchor) in trust_anchors.iter().enumerate() {
            if !cert.is_issued_by(anchor) {
                continue;
            }
            if !is_supported_signature_public_key_algorithm(
                anchor.subject_public_key_algorithm_oid.as_deref(),
            ) {
                continue;
            }
            found_anchor = true;
            let result = verify_signature(
                algorithm,
                &anchor.subject_public_key_der,
                &cert.tbs_certificate_der,
                &cert.certificate_signature,
            );
            let verified = result.is_ok();
            out.push(CertificateChainVerification {
                certificate_index,
                trust_anchor_index: Some(trust_anchor_index),
                algorithm,
                verified,
                error: result.err(),
            });
            if verified {
                break;
            }
        }

        if !found_anchor {
            out.push(CertificateChainVerification {
                certificate_index,
                trust_anchor_index: None,
                algorithm,
                verified: false,
                error: Some("no matching trust anchor".to_string()),
            });
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassportFile {
    pub fid: u16,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PassportData {
    pub ef_card_access: Option<PassportFile>,
    pub ef_com: Option<PassportFile>,
    pub ef_sod: Option<PassportFile>,
    pub data_groups: Vec<(DataGroup, PassportFile)>,
    pub pace_infos: Vec<PaceInfo>,
    pub dg2_images: Vec<FaceImage>,
    pub passive_auth: Option<PassiveAuthReport>,
    pub mrz: Option<MrzRecord>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EfCom {
    pub lds_version: Option<String>,
    pub unicode_version: Option<String>,
    pub data_groups: Vec<DataGroup>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CardAccess {
    pub pace_infos: Vec<PaceInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaceInfo {
    pub protocol_oid: String,
    pub version: u64,
    pub parameter_id: Option<u64>,
    pub mapping: PaceMapping,
    pub cipher: PaceCipher,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaceMapping {
    GenericMapping,
    IntegratedMapping,
    ChipAuthenticationMapping,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaceCipher {
    Des3,
    Aes128,
    Aes192,
    Aes256,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceImage {
    pub format: FaceImageFormat,
    pub offset: usize,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceImageFormat {
    Jpeg,
    Jpeg2000,
    Jpeg2000Codestream,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PassiveAuthReport {
    pub cms: Option<CmsSignedDataInfo>,
    pub lds_security_object: LdsSecurityObject,
    pub data_group_hashes: Vec<DataGroupHash>,
    pub signature_verifications: Vec<CmsSignatureVerification>,
    pub verified_groups: Vec<DataGroup>,
    pub mismatched_groups: Vec<DataGroup>,
    pub missing_groups: Vec<DataGroup>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CmsSignedDataInfo {
    pub lds_security_object: LdsSecurityObject,
    pub digest_algorithm_oids: Vec<String>,
    pub certificates: Vec<CmsCertificate>,
    pub signer_infos: Vec<CmsSignerInfo>,
    pub signed_attr_message_digest: Option<Vec<u8>>,
    pub signed_attr_digest_algorithm: DigestAlgorithm,
    pub signed_attr_digest_matches_econtent: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CmsCertificate {
    pub der: Vec<u8>,
    pub info: Option<X509CertificateInfo>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct X509CertificateInfo {
    pub version: u64,
    pub serial_number: Vec<u8>,
    pub tbs_certificate_der: Vec<u8>,
    pub tbs_certificate_sha256: [u8; 32],
    pub signature_algorithm_oid: Option<String>,
    pub signature_algorithm_params_der: Option<Vec<u8>>,
    pub issuer_der: Vec<u8>,
    pub issuer_sha256: [u8; 32],
    pub not_before: String,
    pub not_after: String,
    pub subject_der: Vec<u8>,
    pub subject_sha256: [u8; 32],
    pub subject_public_key_algorithm_oid: Option<String>,
    pub subject_public_key_der: Vec<u8>,
    pub subject_public_key_sha256: [u8; 32],
    pub certificate_signature_algorithm_oid: Option<String>,
    pub certificate_signature_algorithm_params_der: Option<Vec<u8>>,
    pub certificate_signature: Vec<u8>,
    pub certificate_sha256: [u8; 32],
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CmsSignerInfo {
    pub version: u64,
    pub digest_algorithm_oid: Option<String>,
    pub signature_algorithm_oid: Option<String>,
    pub signature_algorithm_params_der: Option<Vec<u8>>,
    pub signature: Vec<u8>,
    pub signed_attributes_der: Option<Vec<u8>>,
    pub signed_attributes_signature_input_der: Option<Vec<u8>>,
    pub signed_attributes_sha256: Option<[u8; 32]>,
    pub signed_attr_message_digest: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CmsSignatureVerification {
    pub signer_index: usize,
    pub certificate_index: Option<usize>,
    pub algorithm: CmsSignatureAlgorithm,
    pub verified: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertificateChainVerification {
    pub certificate_index: usize,
    pub trust_anchor_index: Option<usize>,
    pub algorithm: CmsSignatureAlgorithm,
    pub verified: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmsSignatureAlgorithm {
    RsaPkcs1Sha256,
    RsaPkcs1Sha384,
    RsaPkcs1Sha512,
    RsaPssSha256,
    EcdsaP256Sha256,
    EcdsaP256Sha384,
    EcdsaP256Sha512,
    Unsupported,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LdsSecurityObject {
    pub version: u64,
    pub digest_algorithm: DigestAlgorithm,
    pub data_group_hashes: Vec<DataGroupHash>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataGroupHash {
    pub group: DataGroup,
    pub digest: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DigestAlgorithm {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MrzAccessData {
    pub document_number: String,
    pub date_of_birth: String,
    pub date_of_expiry: String,
}

impl MrzAccessData {
    pub fn new(document_number: &str, date_of_birth: &str, date_of_expiry: &str) -> Self {
        Self {
            document_number: document_number.to_string(),
            date_of_birth: date_of_birth.to_string(),
            date_of_expiry: date_of_expiry.to_string(),
        }
    }

    pub fn mrz_information(&self) -> Result<String> {
        let document_number = normalize_mrz_field(&self.document_number, 9);
        let birth = normalize_mrz_field(&self.date_of_birth, 6);
        let expiry = normalize_mrz_field(&self.date_of_expiry, 6);
        let mut out = String::new();
        out.push_str(&document_number);
        out.push(check_digit(document_number.as_bytes())?);
        out.push_str(&birth);
        out.push(check_digit(birth.as_bytes())?);
        out.push_str(&expiry);
        out.push(check_digit(expiry.as_bytes())?);
        Ok(out)
    }

    pub fn bac_key_seed(&self) -> Result<[u8; 16]> {
        let mut hasher = Sha1::new();
        hasher.update(self.mrz_information()?.as_bytes());
        let digest = hasher.finalize();
        let mut out = [0u8; 16];
        out.copy_from_slice(&digest[..16]);
        Ok(out)
    }

    pub fn bac_document_basic_keys(&self) -> Result<BacDocumentBasicKeys> {
        let seed = self.bac_key_seed()?;
        Ok(BacDocumentBasicKeys {
            kenc: derive_bac_3des_key(&seed, 1),
            kmac: derive_bac_3des_key(&seed, 2),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacDocumentBasicKeys {
    pub kenc: [u8; 16],
    pub kmac: [u8; 16],
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MrzRecord {
    pub raw: String,
    pub document_code: String,
    pub issuing_state: String,
    pub primary_identifier: String,
    pub secondary_identifiers: Vec<String>,
    pub document_number: String,
    pub nationality: String,
    pub date_of_birth: String,
    pub sex: Option<char>,
    pub date_of_expiry: String,
    pub personal_number: String,
}

pub trait SecureMessaging {
    fn wrap_command(&mut self, command: &[u8]) -> Result<Vec<u8>>;
    fn unwrap_response(&mut self, response: &[u8]) -> Result<Vec<u8>>;
}

#[derive(Default)]
pub struct PlainMessaging;

impl SecureMessaging for PlainMessaging {
    fn wrap_command(&mut self, command: &[u8]) -> Result<Vec<u8>> {
        Ok(command.to_vec())
    }

    fn unwrap_response(&mut self, response: &[u8]) -> Result<Vec<u8>> {
        Ok(response.to_vec())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacSecureMessaging {
    ksenc: [u8; 16],
    ksmac: [u8; 16],
    ssc: [u8; 8],
}

impl BacSecureMessaging {
    pub fn authenticate<R: NfcReader>(
        reader: &R,
        target_id: &str,
        access_data: &MrzAccessData,
    ) -> Result<Self> {
        let mut rnd_ifd = [0u8; 8];
        let mut kifd = [0u8; 16];
        fill_random(&mut rnd_ifd).map_err(|_| PassportError::RandomFailed)?;
        fill_random(&mut kifd).map_err(|_| PassportError::RandomFailed)?;
        Self::authenticate_with_entropy(reader, target_id, access_data, rnd_ifd, kifd)
    }

    pub fn authenticate_with_entropy<R: NfcReader>(
        reader: &R,
        target_id: &str,
        access_data: &MrzAccessData,
        rnd_ifd: [u8; 8],
        kifd: [u8; 16],
    ) -> Result<Self> {
        let keys = access_data.bac_document_basic_keys()?;
        let rnd_icc = {
            let response = reader.transceive(target_id, &get_challenge())?;
            let data = split_response(&response)?;
            if data.len() != 8 {
                return Err(PassportError::MalformedResponse);
            }
            let mut out = [0u8; 8];
            out.copy_from_slice(&data);
            out
        };

        let mut s = Vec::with_capacity(32);
        s.extend_from_slice(&rnd_ifd);
        s.extend_from_slice(&rnd_icc);
        s.extend_from_slice(&kifd);
        let cipher = Tdes2::new(&keys.kenc);
        let eifd = cipher
            .cbc_encrypt(&[0u8; 8], &s)
            .ok_or(PassportError::AuthenticationFailed)?;
        let mifd = iso9797_alg3_mac_3des2(&keys.kmac, &eifd);
        let mut cmd_data = eifd;
        cmd_data.extend_from_slice(&mifd);

        let response = reader.transceive(target_id, &mutual_authenticate(&cmd_data))?;
        let data = split_response(&response)?;
        if data.len() != 40 {
            return Err(PassportError::AuthenticationFailed);
        }
        let (eicc, micc) = data.split_at(32);
        if iso9797_alg3_mac_3des2(&keys.kmac, eicc) != micc {
            return Err(PassportError::AuthenticationFailed);
        }
        let decrypted = cipher
            .cbc_decrypt(&[0u8; 8], eicc)
            .ok_or(PassportError::AuthenticationFailed)?;
        if decrypted.len() != 32 || decrypted[0..8] != rnd_icc || decrypted[8..16] != rnd_ifd {
            return Err(PassportError::AuthenticationFailed);
        }
        let mut kicc = [0u8; 16];
        kicc.copy_from_slice(&decrypted[16..32]);
        let mut seed = [0u8; 16];
        for i in 0..16 {
            seed[i] = kifd[i] ^ kicc[i];
        }

        let mut ssc = [0u8; 8];
        ssc[..4].copy_from_slice(&rnd_icc[4..8]);
        ssc[4..].copy_from_slice(&rnd_ifd[4..8]);

        Ok(Self {
            ksenc: derive_bac_3des_key(&seed, 1),
            ksmac: derive_bac_3des_key(&seed, 2),
            ssc,
        })
    }

    pub fn session_send_counter(&self) -> [u8; 8] {
        self.ssc
    }
}

impl SecureMessaging for BacSecureMessaging {
    fn wrap_command(&mut self, command: &[u8]) -> Result<Vec<u8>> {
        let parsed = ParsedApdu::parse(command)?;
        increment_ssc(&mut self.ssc);

        let protected_header = [parsed.cla | 0x0C, parsed.ins, parsed.p1, parsed.p2];
        let mut dos = Vec::new();
        if !parsed.data.is_empty() {
            let padded = iso9797_method2_pad(parsed.data, 8);
            let encrypted = Tdes2::new(&self.ksenc)
                .cbc_encrypt(&[0u8; 8], &padded)
                .ok_or(PassportError::SecureMessagingFailed)?;
            let mut value = Vec::with_capacity(encrypted.len() + 1);
            value.push(0x01);
            value.extend_from_slice(&encrypted);
            push_tlv(&mut dos, 0x87, &value)?;
        }
        if let Some(le) = parsed.le {
            push_tlv(&mut dos, 0x97, &[le])?;
        }

        let mut mac_input = Vec::new();
        mac_input.extend_from_slice(&self.ssc);
        let mut mac_body = Vec::new();
        mac_body.extend_from_slice(&protected_header);
        mac_body.extend_from_slice(&dos);
        mac_input.extend_from_slice(&iso9797_method2_pad(&mac_body, 8));
        let mac = iso9797_alg3_mac_3des2(&self.ksmac, &mac_input);
        push_tlv(&mut dos, 0x8E, &mac)?;
        if dos.len() > u8::MAX as usize {
            return Err(PassportError::Unsupported("extended protected APDUs"));
        }

        let mut out = Vec::with_capacity(5 + dos.len());
        out.extend_from_slice(&protected_header);
        out.push(dos.len() as u8);
        out.extend_from_slice(&dos);
        Ok(out)
    }

    fn unwrap_response(&mut self, response: &[u8]) -> Result<Vec<u8>> {
        let protected = split_response(response)?;
        increment_ssc(&mut self.ssc);

        let mut encrypted_data = None;
        let mut status = None;
        let mut mac = None;
        let mut mac_end = 0usize;
        let mut pos = 0usize;
        while pos < protected.len() {
            let tlv = parse_tlv_at(&protected, pos)?;
            if tlv.tag == 0x8E {
                mac = Some(tlv.value);
                mac_end = pos;
            } else if tlv.tag == 0x87 {
                encrypted_data = Some(tlv.value);
            } else if tlv.tag == 0x99 {
                if tlv.value.len() != 2 {
                    return Err(PassportError::MalformedResponse);
                }
                status = Some(StatusWord(tlv.value[0], tlv.value[1]));
            }
            pos += tlv.total_len;
        }

        let mac = mac.ok_or(PassportError::SecureMessagingFailed)?;
        let mut mac_input = Vec::new();
        mac_input.extend_from_slice(&self.ssc);
        mac_input.extend_from_slice(&iso9797_method2_pad(&protected[..mac_end], 8));
        if iso9797_alg3_mac_3des2(&self.ksmac, &mac_input) != mac {
            return Err(PassportError::SecureMessagingFailed);
        }

        let mut out = Vec::new();
        if let Some(value) = encrypted_data {
            if value.first() != Some(&0x01) || (value.len() - 1) % 8 != 0 {
                return Err(PassportError::MalformedResponse);
            }
            let decrypted = Tdes2::new(&self.ksenc)
                .cbc_decrypt(&[0u8; 8], &value[1..])
                .ok_or(PassportError::SecureMessagingFailed)?;
            out.extend_from_slice(
                &iso9797_method2_unpad(&decrypted).ok_or(PassportError::MalformedResponse)?,
            );
        }
        let status = status.ok_or(PassportError::SecureMessagingFailed)?;
        out.push(status.0);
        out.push(status.1);
        Ok(out)
    }
}

pub struct PassportSession<'a, R, S = PlainMessaging>
where
    R: NfcReader,
    S: SecureMessaging,
{
    reader: &'a R,
    target_id: String,
    secure_messaging: S,
    max_chunk_len: u8,
}

impl<'a, R> PassportSession<'a, R, PlainMessaging>
where
    R: NfcReader,
{
    pub fn new(reader: &'a R, target_id: &str) -> Self {
        Self::with_secure_messaging(reader, target_id, PlainMessaging)
    }

    pub fn authenticate_bac(
        self,
        access_data: &MrzAccessData,
    ) -> Result<PassportSession<'a, R, BacSecureMessaging>> {
        let secure_messaging =
            BacSecureMessaging::authenticate(self.reader, &self.target_id, access_data)?;
        Ok(PassportSession {
            reader: self.reader,
            target_id: self.target_id,
            secure_messaging,
            max_chunk_len: self.max_chunk_len,
        })
    }
}

impl<'a, R, S> PassportSession<'a, R, S>
where
    R: NfcReader,
    S: SecureMessaging,
{
    pub fn with_secure_messaging(reader: &'a R, target_id: &str, secure_messaging: S) -> Self {
        Self {
            reader,
            target_id: target_id.to_string(),
            secure_messaging,
            max_chunk_len: 0xE0,
        }
    }

    pub fn set_max_chunk_len(&mut self, len: u8) {
        self.max_chunk_len = len.max(1);
    }

    pub fn select_passport_application(&mut self) -> Result<()> {
        self.send_checked(&select_by_name(EMRTD_AID)).map(|_| ())
    }

    pub fn select_file(&mut self, fid: u16) -> Result<()> {
        self.send_checked(&select_file(fid)).map(|_| ())
    }

    pub fn read_selected_file(&mut self) -> Result<Vec<u8>> {
        let header = self.read_binary(0, 8)?;
        let total_len = ber_total_len(&header)?;
        if total_len > u16::MAX as usize {
            return Err(PassportError::FileTooLarge);
        }

        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(&header[..core::cmp::min(header.len(), total_len)]);
        while out.len() < total_len {
            let remaining = total_len - out.len();
            let le = core::cmp::min(remaining, self.max_chunk_len as usize) as u8;
            let chunk = self.read_binary(out.len() as u16, le)?;
            if chunk.is_empty() {
                return Err(PassportError::MalformedResponse);
            }
            out.extend_from_slice(&chunk[..core::cmp::min(chunk.len(), remaining)]);
        }
        out.truncate(total_len);
        Ok(out)
    }

    pub fn read_file(&mut self, fid: u16) -> Result<PassportFile> {
        self.select_file(fid)?;
        Ok(PassportFile {
            fid,
            bytes: self.read_selected_file()?,
        })
    }

    pub fn read_data_group(&mut self, group: DataGroup) -> Result<PassportFile> {
        self.read_file(group.file_id())
    }

    pub fn read_card_access(&mut self) -> Result<PassportFile> {
        self.read_file(EF_CARD_ACCESS)
    }

    pub fn read_basic_data_groups(&mut self) -> Result<PassportData> {
        let card_access = self.read_card_access().ok();
        let pace_infos = card_access
            .as_ref()
            .and_then(|file| parse_ef_card_access(&file.bytes).ok())
            .map(|card_access| card_access.pace_infos)
            .unwrap_or_default();

        self.select_passport_application()?;
        let ef_com_file = self.read_file(EF_COM)?;
        let ef_com = parse_ef_com(&ef_com_file.bytes)?;
        let ef_sod = self.read_file(EF_SOD).ok();
        let mut data = PassportData {
            ef_card_access: card_access,
            ef_com: Some(ef_com_file),
            ef_sod,
            pace_infos,
            ..PassportData::default()
        };

        for group in &ef_com.data_groups {
            let Ok(file) = self.read_data_group(*group) else {
                continue;
            };
            if *group == DataGroup::Dg1 {
                data.mrz = parse_dg1_mrz(&file.bytes).ok();
            } else if *group == DataGroup::Dg2 {
                data.dg2_images = extract_dg2_face_images(&file.bytes);
            }
            data.data_groups.push((*group, file));
        }
        data.passive_auth = match &data.ef_sod {
            Some(ef_sod) => verify_passive_auth_hashes(&ef_sod.bytes, &data.data_groups).ok(),
            None => None,
        };
        Ok(data)
    }

    fn read_binary(&mut self, offset: u16, le: u8) -> Result<Vec<u8>> {
        self.send_checked(&read_binary(offset, le))
    }

    fn send_checked(&mut self, command: &[u8]) -> Result<Vec<u8>> {
        let wrapped = self.secure_messaging.wrap_command(command)?;
        let raw = self.reader.transceive(&self.target_id, &wrapped)?;
        let response = self.secure_messaging.unwrap_response(&raw)?;
        split_response(&response)
    }
}

pub fn select_by_name(aid: &[u8]) -> Vec<u8> {
    let mut out = vec![0x00, 0xA4, 0x04, 0x0C, aid.len() as u8];
    out.extend_from_slice(aid);
    out
}

pub fn select_file(fid: u16) -> Vec<u8> {
    vec![0x00, 0xA4, 0x02, 0x0C, 0x02, (fid >> 8) as u8, fid as u8]
}

pub fn read_binary(offset: u16, le: u8) -> Vec<u8> {
    vec![0x00, 0xB0, (offset >> 8) as u8, offset as u8, le]
}

pub fn get_challenge() -> Vec<u8> {
    vec![0x00, 0x84, 0x00, 0x00, 0x08]
}

pub fn mutual_authenticate(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x00, 0x82, 0x00, 0x00, data.len() as u8];
    out.extend_from_slice(data);
    out.push(0x28);
    out
}

pub fn split_response(response: &[u8]) -> Result<Vec<u8>> {
    if response.len() < 2 {
        return Err(PassportError::MalformedResponse);
    }
    let data_len = response.len() - 2;
    let status = StatusWord(response[data_len], response[data_len + 1]);
    if !status.is_ok() {
        return Err(PassportError::Status(status));
    }
    Ok(response[..data_len].to_vec())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParsedApdu<'a> {
    cla: u8,
    ins: u8,
    p1: u8,
    p2: u8,
    data: &'a [u8],
    le: Option<u8>,
}

impl<'a> ParsedApdu<'a> {
    fn parse(command: &'a [u8]) -> Result<Self> {
        if command.len() < 4 {
            return Err(PassportError::MalformedResponse);
        }
        let mut out = ParsedApdu {
            cla: command[0],
            ins: command[1],
            p1: command[2],
            p2: command[3],
            data: &[],
            le: None,
        };
        match command.len() {
            4 => Ok(out),
            5 => {
                out.le = Some(command[4]);
                Ok(out)
            }
            _ => {
                let lc = command[4] as usize;
                if command.len() == 5 + lc {
                    out.data = &command[5..5 + lc];
                    Ok(out)
                } else if command.len() == 6 + lc {
                    out.data = &command[5..5 + lc];
                    out.le = Some(command[5 + lc]);
                    Ok(out)
                } else {
                    Err(PassportError::MalformedResponse)
                }
            }
        }
    }
}

fn increment_ssc(ssc: &mut [u8; 8]) {
    for byte in ssc.iter_mut().rev() {
        let (next, carry) = byte.overflowing_add(1);
        *byte = next;
        if !carry {
            break;
        }
    }
}

fn push_tlv(out: &mut Vec<u8>, tag: u8, value: &[u8]) -> Result<()> {
    out.push(tag);
    push_len(out, value.len())?;
    out.extend_from_slice(value);
    Ok(())
}

fn encode_tlv(tag: u8, value: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    push_tlv(&mut out, tag, value)?;
    Ok(out)
}

fn push_len(out: &mut Vec<u8>, len: usize) -> Result<()> {
    if len < 0x80 {
        out.push(len as u8);
    } else if len <= 0xFF {
        out.extend_from_slice(&[0x81, len as u8]);
    } else if len <= 0xFFFF {
        out.extend_from_slice(&[0x82, (len >> 8) as u8, len as u8]);
    } else {
        return Err(PassportError::Unsupported("large BER length"));
    }
    Ok(())
}

pub fn parse_ef_com(bytes: &[u8]) -> Result<EfCom> {
    let root = Tlv::parse(bytes)?;
    if root.tag != 0x60 {
        return Err(PassportError::MalformedTlv);
    }

    let mut out = EfCom::default();
    for child in TlvIter::new(root.value) {
        let child = child?;
        match child.tag {
            0x5F01 => out.lds_version = Some(tlv_text(child.value)?),
            0x5F36 => out.unicode_version = Some(tlv_text(child.value)?),
            0x5C => {
                out.data_groups = child
                    .value
                    .iter()
                    .filter_map(|tag| DataGroup::from_tag(*tag))
                    .collect();
            }
            _ => {}
        }
    }
    Ok(out)
}

pub fn parse_dg1_mrz(bytes: &[u8]) -> Result<MrzRecord> {
    let root = Tlv::parse(bytes)?;
    if root.tag != 0x61 {
        return Err(PassportError::MalformedTlv);
    }
    for child in TlvIter::new(root.value) {
        let child = child?;
        if child.tag == 0x5F1F {
            return parse_td3_mrz(&tlv_text(child.value)?);
        }
    }
    Err(PassportError::MalformedMrz)
}

pub fn parse_ef_card_access(bytes: &[u8]) -> Result<CardAccess> {
    let root = Tlv::parse(bytes)?;
    let mut out = CardAccess::default();
    collect_pace_infos(root.value, &mut out)?;
    Ok(out)
}

fn collect_pace_infos(bytes: &[u8], out: &mut CardAccess) -> Result<()> {
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tlv = parse_tlv_at(bytes, pos)?;
        if tlv.tag == 0x30 {
            if let Some(info) = parse_pace_info_sequence(tlv.value)? {
                out.pace_infos.push(info);
            }
        }
        if matches!(tlv.tag, 0x30 | 0x31 | 0x7F4C) {
            collect_pace_infos(tlv.value, out)?;
        }
        pos += tlv.total_len;
    }
    Ok(())
}

fn parse_pace_info_sequence(bytes: &[u8]) -> Result<Option<PaceInfo>> {
    let mut iter = TlvIter::new(bytes);
    let Some(protocol) = iter.next() else {
        return Ok(None);
    };
    let protocol = protocol?;
    if protocol.tag != 0x06 {
        return Ok(None);
    }
    let protocol_oid = parse_oid(protocol.value)?;
    if !protocol_oid.starts_with("0.4.0.127.0.7.2.2.4") {
        return Ok(None);
    }

    let Some(version) = iter.next() else {
        return Err(PassportError::MalformedTlv);
    };
    let version = version?;
    if version.tag != 0x02 {
        return Err(PassportError::MalformedTlv);
    }
    let version = parse_der_integer(version.value)?;
    let parameter_id = match iter.next() {
        Some(Ok(tlv)) if tlv.tag == 0x02 => Some(parse_der_integer(tlv.value)?),
        Some(Ok(_)) => None,
        Some(Err(e)) => return Err(e),
        None => None,
    };

    Ok(Some(PaceInfo {
        mapping: pace_mapping_from_oid(&protocol_oid),
        cipher: pace_cipher_from_oid(&protocol_oid),
        protocol_oid,
        version,
        parameter_id,
    }))
}

pub fn extract_dg2_face_images(bytes: &[u8]) -> Vec<FaceImage> {
    let mut images = Vec::new();
    let mut pos = 0usize;
    while pos + 4 <= bytes.len() {
        if bytes[pos..].starts_with(&[0xFF, 0xD8, 0xFF]) {
            if let Some(end) = find_subslice(&bytes[pos + 2..], &[0xFF, 0xD9]) {
                let end = pos + 2 + end + 2;
                images.push(FaceImage {
                    format: FaceImageFormat::Jpeg,
                    offset: pos,
                    bytes: bytes[pos..end].to_vec(),
                });
                pos = end;
                continue;
            }
        } else if bytes[pos..].starts_with(&[
            0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A,
        ]) {
            images.push(FaceImage {
                format: FaceImageFormat::Jpeg2000,
                offset: pos,
                bytes: bytes[pos..].to_vec(),
            });
            break;
        } else if bytes[pos..].starts_with(&[0xFF, 0x4F, 0xFF, 0x51]) {
            images.push(FaceImage {
                format: FaceImageFormat::Jpeg2000Codestream,
                offset: pos,
                bytes: bytes[pos..].to_vec(),
            });
            break;
        }
        pos += 1;
    }
    images
}

pub fn verify_passive_auth_hashes(
    ef_sod: &[u8],
    data_groups: &[(DataGroup, PassportFile)],
) -> Result<PassiveAuthReport> {
    let cms = parse_ef_sod_cms(ef_sod).ok();
    let signature_verifications = cms.as_ref().map(verify_cms_signatures).unwrap_or_default();
    let lds_security_object = cms
        .as_ref()
        .map(|cms| cms.lds_security_object.clone())
        .unwrap_or(parse_ef_sod_hashes(ef_sod)?);
    let mut verified_groups = Vec::new();
    let mut mismatched_groups = Vec::new();
    let mut missing_groups = Vec::new();

    for expected in &lds_security_object.data_group_hashes {
        let Some((_, file)) = data_groups
            .iter()
            .find(|(group, _)| *group == expected.group)
        else {
            missing_groups.push(expected.group);
            continue;
        };
        match digest_bytes(lds_security_object.digest_algorithm, &file.bytes) {
            Some(actual) if actual == expected.digest => verified_groups.push(expected.group),
            Some(_) => mismatched_groups.push(expected.group),
            None => missing_groups.push(expected.group),
        }
    }

    Ok(PassiveAuthReport {
        cms,
        data_group_hashes: lds_security_object.data_group_hashes.clone(),
        lds_security_object,
        signature_verifications,
        verified_groups,
        mismatched_groups,
        missing_groups,
    })
}

pub fn parse_ef_sod_cms(bytes: &[u8]) -> Result<CmsSignedDataInfo> {
    let sod = Tlv::parse(bytes)?;
    let cms = if sod.tag == 0x77 {
        Tlv::parse(sod.value)?
    } else {
        sod
    };
    if cms.tag != 0x30 {
        return Err(PassportError::MalformedTlv);
    }
    let signed_data = find_cms_signed_data(cms.value).ok_or(PassportError::MalformedTlv)?;
    let econtent =
        find_octet_string_containing_lso(cms.value).ok_or(PassportError::MalformedTlv)?;
    let lds_security_object = parse_lds_security_object(econtent)?;
    let digest_algorithm_oids = collect_signed_data_digest_algorithm_oids(signed_data)?;
    let certificates = collect_signed_data_certificates(signed_data)?;
    let signer_infos = collect_signed_data_signer_infos(signed_data)?;
    let signed_attr_message_digest = signer_infos
        .iter()
        .find_map(|info| info.signed_attr_message_digest.clone())
        .or_else(|| find_cms_signed_attr_message_digest(cms.value));
    let signed_attr_digest_algorithm = signed_attr_message_digest
        .as_ref()
        .map_or(DigestAlgorithm::Unknown, |digest| {
            digest_algorithm_from_len(digest.len())
        });
    let signed_attr_digest_matches_econtent =
        signed_attr_message_digest.as_ref().and_then(|expected| {
            digest_bytes(signed_attr_digest_algorithm, econtent).map(|actual| actual == *expected)
        });

    Ok(CmsSignedDataInfo {
        lds_security_object,
        digest_algorithm_oids,
        certificates,
        signer_infos,
        signed_attr_message_digest,
        signed_attr_digest_algorithm,
        signed_attr_digest_matches_econtent,
    })
}

pub fn parse_ef_sod_hashes(bytes: &[u8]) -> Result<LdsSecurityObject> {
    let sod = Tlv::parse(bytes)?;
    let cms = if sod.tag == 0x77 {
        Tlv::parse(sod.value)?
    } else {
        sod
    };
    if cms.tag != 0x30 {
        return Err(PassportError::MalformedTlv);
    }
    let econtent =
        find_octet_string_containing_lso(cms.value).ok_or(PassportError::MalformedTlv)?;
    parse_lds_security_object(econtent)
}

fn find_octet_string_containing_lso(bytes: &[u8]) -> Option<&[u8]> {
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tlv = parse_tlv_at(bytes, pos).ok()?;
        if tlv.tag == 0x04 && looks_like_lds_security_object(tlv.value) {
            return Some(tlv.value);
        }
        if is_constructed_tag(tlv.tag) {
            if let Some(found) = find_octet_string_containing_lso(tlv.value) {
                return Some(found);
            }
        }
        pos += tlv.total_len;
    }
    None
}

fn find_cms_signed_data(bytes: &[u8]) -> Option<&[u8]> {
    let mut saw_signed_data_oid = false;
    for child in TlvIter::new(bytes) {
        let child = child.ok()?;
        if child.tag == 0x06 {
            saw_signed_data_oid = parse_oid(child.value).ok()? == "1.2.840.113549.1.7.2";
        } else if saw_signed_data_oid && child.tag == 0xA0 {
            let signed_data = Tlv::parse(child.value).ok()?;
            if signed_data.tag == 0x30 {
                return Some(signed_data.value);
            }
        }
    }
    None
}

fn collect_signed_data_digest_algorithm_oids(signed_data: &[u8]) -> Result<Vec<String>> {
    let fields = collect_sequence_fields(signed_data)?;
    let Some(digest_algorithms) = fields.get(1) else {
        return Err(PassportError::MalformedTlv);
    };
    if digest_algorithms.tag != 0x31 {
        return Err(PassportError::MalformedTlv);
    }
    let mut out = Vec::new();
    for algorithm in TlvIter::new(digest_algorithms.value) {
        let algorithm = algorithm?;
        if algorithm.tag == 0x30 {
            for child in TlvIter::new(algorithm.value) {
                let child = child?;
                if child.tag == 0x06 {
                    out.push(parse_oid(child.value)?);
                    break;
                }
            }
        }
    }
    Ok(out)
}

fn collect_signed_data_certificates(signed_data: &[u8]) -> Result<Vec<CmsCertificate>> {
    let fields = collect_sequence_fields(signed_data)?;
    let mut out = Vec::new();
    for field in &fields {
        if field.tag == 0xA0 {
            let mut pos = 0usize;
            while pos < field.value.len() {
                let cert = parse_tlv_at(field.value, pos)?;
                if cert.tag == 0x30 {
                    let der = field.value[pos..pos + cert.total_len].to_vec();
                    let info = parse_x509_certificate_info(&der).ok();
                    out.push(CmsCertificate { der, info });
                }
                pos += cert.total_len;
            }
        }
    }
    Ok(out)
}

fn collect_signed_data_signer_infos(signed_data: &[u8]) -> Result<Vec<CmsSignerInfo>> {
    let fields = collect_sequence_fields(signed_data)?;
    let Some(signer_infos) = fields.iter().rev().find(|field| field.tag == 0x31) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for signer_info in TlvIter::new(signer_infos.value) {
        let signer_info = signer_info?;
        if signer_info.tag == 0x30 {
            out.push(parse_signer_info(signer_info.value)?);
        }
    }
    Ok(out)
}

fn parse_signer_info(bytes: &[u8]) -> Result<CmsSignerInfo> {
    let fields = collect_sequence_fields(bytes)?;
    let version = fields
        .first()
        .filter(|field| field.tag == 0x02)
        .ok_or(PassportError::MalformedTlv)
        .and_then(|field| parse_der_integer(field.value))?;
    let digest_algorithm_oid = fields
        .get(2)
        .filter(|field| field.tag == 0x30)
        .and_then(|field| parse_algorithm_oid(field.value).ok());
    let signed_attrs = fields.iter().find(|field| field.tag == 0xA0);
    let signed_attributes_der = signed_attrs.map(|field| field.value.to_vec());
    let signed_attributes_signature_input_der =
        signed_attrs.and_then(|field| encode_tlv(0x31, field.value).ok());
    let signed_attributes_sha256 = signed_attributes_signature_input_der
        .as_ref()
        .map(|der| sha256(der));
    let signed_attr_message_digest =
        signed_attrs.and_then(|field| find_cms_signed_attr_message_digest(field.value));
    let signature_algorithm_oid = fields
        .iter()
        .skip_while(|field| field.tag != 0xA0)
        .skip(1)
        .find(|field| field.tag == 0x30)
        .and_then(|field| parse_algorithm_oid(field.value).ok());
    let signature_algorithm_params_der = fields
        .iter()
        .skip_while(|field| field.tag != 0xA0)
        .skip(1)
        .find(|field| field.tag == 0x30)
        .and_then(|field| parse_algorithm_params_der(field.value).ok().flatten());
    let signature = fields
        .iter()
        .find(|field| field.tag == 0x04)
        .map(|field| field.value.to_vec())
        .unwrap_or_default();

    Ok(CmsSignerInfo {
        version,
        digest_algorithm_oid,
        signature_algorithm_oid,
        signature_algorithm_params_der,
        signature,
        signed_attributes_der,
        signed_attributes_signature_input_der,
        signed_attributes_sha256,
        signed_attr_message_digest,
    })
}

fn collect_sequence_fields(bytes: &[u8]) -> Result<Vec<Tlv<'_>>> {
    let mut out = Vec::new();
    for field in TlvIter::new(bytes) {
        out.push(field?);
    }
    Ok(out)
}

fn parse_algorithm_oid(bytes: &[u8]) -> Result<String> {
    for child in TlvIter::new(bytes) {
        let child = child?;
        if child.tag == 0x06 {
            return parse_oid(child.value);
        }
    }
    Err(PassportError::MalformedTlv)
}

fn parse_algorithm_params_der(bytes: &[u8]) -> Result<Option<Vec<u8>>> {
    let fields = collect_sequence_fields(bytes)?;
    let Some(params) = fields.get(1) else {
        return Ok(None);
    };
    let offset = tbs_field_offset(bytes, 1)?;
    Ok(Some(bytes[offset..offset + params.total_len].to_vec()))
}

pub fn parse_x509_certificate_info(der: &[u8]) -> Result<X509CertificateInfo> {
    let cert = Tlv::parse(der)?;
    if cert.tag != 0x30 {
        return Err(PassportError::MalformedTlv);
    }
    let cert_fields = collect_sequence_fields(cert.value)?;
    if cert_fields.len() < 3 {
        return Err(PassportError::MalformedTlv);
    }
    let tbs = cert_fields[0];
    if tbs.tag != 0x30 {
        return Err(PassportError::MalformedTlv);
    }
    let tbs_certificate_der = der[..tbs.total_len].to_vec();
    let tbs_certificate_sha256 = sha256(&tbs_certificate_der);
    let tbs_fields = collect_sequence_fields(tbs.value)?;
    let mut idx = 0usize;
    let version = if tbs_fields.first().map(|field| field.tag) == Some(0xA0) {
        let inner = Tlv::parse(tbs_fields[0].value)?;
        if inner.tag != 0x02 {
            return Err(PassportError::MalformedTlv);
        }
        idx = 1;
        parse_der_integer(inner.value)? + 1
    } else {
        1
    };
    let serial = tbs_fields.get(idx).ok_or(PassportError::MalformedTlv)?;
    idx += 1;
    if serial.tag != 0x02 {
        return Err(PassportError::MalformedTlv);
    }
    let signature_algorithm = tbs_fields.get(idx).ok_or(PassportError::MalformedTlv)?;
    idx += 1;
    let signature_algorithm_oid = parse_algorithm_oid(signature_algorithm.value).ok();
    let signature_algorithm_params_der = parse_algorithm_params_der(signature_algorithm.value)
        .ok()
        .flatten();
    let issuer = tbs_fields.get(idx).ok_or(PassportError::MalformedTlv)?;
    idx += 1;
    let issuer_der =
        tbs.value[tbs_field_offset(tbs.value, idx - 1)?..][..issuer.total_len].to_vec();
    let issuer_sha256 = sha256(&issuer_der);
    let validity = tbs_fields.get(idx).ok_or(PassportError::MalformedTlv)?;
    idx += 1;
    let (not_before, not_after) = parse_x509_validity(validity.value)?;
    let subject = tbs_fields.get(idx).ok_or(PassportError::MalformedTlv)?;
    idx += 1;
    let subject_der =
        tbs.value[tbs_field_offset(tbs.value, idx - 1)?..][..subject.total_len].to_vec();
    let subject_sha256 = sha256(&subject_der);
    let spki = tbs_fields.get(idx).ok_or(PassportError::MalformedTlv)?;
    let (subject_public_key_algorithm_oid, subject_public_key_der) = parse_x509_spki(spki.value)?;
    let subject_public_key_sha256 = sha256(&subject_public_key_der);
    let certificate_signature_algorithm_oid = parse_algorithm_oid(cert_fields[1].value).ok();
    let certificate_signature_algorithm_params_der =
        parse_algorithm_params_der(cert_fields[1].value)
            .ok()
            .flatten();
    let certificate_signature = parse_der_bit_string(cert_fields[2].value)?;
    let certificate_sha256 = sha256(der);

    Ok(X509CertificateInfo {
        version,
        serial_number: serial.value.to_vec(),
        tbs_certificate_der,
        tbs_certificate_sha256,
        signature_algorithm_oid,
        signature_algorithm_params_der,
        issuer_der,
        issuer_sha256,
        not_before,
        not_after,
        subject_der,
        subject_sha256,
        subject_public_key_algorithm_oid,
        subject_public_key_der,
        subject_public_key_sha256,
        certificate_signature_algorithm_oid,
        certificate_signature_algorithm_params_der,
        certificate_signature,
        certificate_sha256,
    })
}

fn tbs_field_offset(bytes: &[u8], field_index: usize) -> Result<usize> {
    let mut pos = 0usize;
    for i in 0..=field_index {
        let tlv = parse_tlv_at(bytes, pos)?;
        if i == field_index {
            return Ok(pos);
        }
        pos += tlv.total_len;
    }
    Err(PassportError::MalformedTlv)
}

fn parse_x509_validity(bytes: &[u8]) -> Result<(String, String)> {
    let fields = collect_sequence_fields(bytes)?;
    if fields.len() < 2 {
        return Err(PassportError::MalformedTlv);
    }
    Ok((parse_x509_time(fields[0])?, parse_x509_time(fields[1])?))
}

fn parse_x509_time(tlv: Tlv<'_>) -> Result<String> {
    match tlv.tag {
        0x17 | 0x18 => tlv_text(tlv.value),
        _ => Err(PassportError::MalformedTlv),
    }
}

fn parse_x509_spki(bytes: &[u8]) -> Result<(Option<String>, Vec<u8>)> {
    let fields = collect_sequence_fields(bytes)?;
    if fields.len() < 2 {
        return Err(PassportError::MalformedTlv);
    }
    let algorithm_oid = parse_algorithm_oid(fields[0].value).ok();
    let key = parse_der_bit_string(fields[1].value)?;
    Ok((algorithm_oid, key))
}

fn parse_der_bit_string(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.is_empty() || bytes[0] != 0 {
        return Err(PassportError::MalformedTlv);
    }
    Ok(bytes[1..].to_vec())
}

fn find_cms_signed_attr_message_digest(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tlv = parse_tlv_at(bytes, pos).ok()?;
        if tlv.tag == 0x30 {
            if let Some(value) = parse_cms_attribute_message_digest(tlv.value) {
                return Some(value);
            }
        }
        if is_constructed_tag(tlv.tag) {
            if let Some(value) = find_cms_signed_attr_message_digest(tlv.value) {
                return Some(value);
            }
        }
        pos += tlv.total_len;
    }
    None
}

fn parse_cms_attribute_message_digest(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut iter = TlvIter::new(bytes);
    let oid = iter.next()?.ok()?;
    if oid.tag != 0x06 || parse_oid(oid.value).ok()? != "1.2.840.113549.1.9.4" {
        return None;
    }
    let values = iter.next()?.ok()?;
    if values.tag != 0x31 {
        return None;
    }
    for value in TlvIter::new(values.value) {
        let value = value.ok()?;
        if value.tag == 0x04 {
            return Some(value.value.to_vec());
        }
    }
    None
}

fn looks_like_lds_security_object(bytes: &[u8]) -> bool {
    let Ok(root) = Tlv::parse(bytes) else {
        return false;
    };
    if root.tag != 0x30 {
        return false;
    }
    let mut iter = TlvIter::new(root.value);
    matches!(iter.next(), Some(Ok(tlv)) if tlv.tag == 0x02)
}

fn parse_lds_security_object(bytes: &[u8]) -> Result<LdsSecurityObject> {
    let root = Tlv::parse(bytes)?;
    if root.tag != 0x30 {
        return Err(PassportError::MalformedTlv);
    }
    let mut iter = TlvIter::new(root.value);
    let version = iter.next().ok_or(PassportError::MalformedTlv)??;
    if version.tag != 0x02 {
        return Err(PassportError::MalformedTlv);
    }
    let version = parse_der_integer(version.value)?;

    let digest_algorithm = iter.next().ok_or(PassportError::MalformedTlv)??;
    let digest_algorithm = parse_digest_algorithm_identifier(digest_algorithm.value)?;

    let hashes = iter.next().ok_or(PassportError::MalformedTlv)??;
    if hashes.tag != 0x30 {
        return Err(PassportError::MalformedTlv);
    }
    let mut data_group_hashes = Vec::new();
    for hash_value in TlvIter::new(hashes.value) {
        let hash_value = hash_value?;
        if hash_value.tag != 0x30 {
            continue;
        }
        let mut hash_iter = TlvIter::new(hash_value.value);
        let group = hash_iter.next().ok_or(PassportError::MalformedTlv)??;
        let digest = hash_iter.next().ok_or(PassportError::MalformedTlv)??;
        if group.tag != 0x02 || digest.tag != 0x04 {
            return Err(PassportError::MalformedTlv);
        }
        let Some(group) = data_group_from_number(parse_der_integer(group.value)?) else {
            continue;
        };
        data_group_hashes.push(DataGroupHash {
            group,
            digest: digest.value.to_vec(),
        });
    }

    Ok(LdsSecurityObject {
        version,
        digest_algorithm,
        data_group_hashes,
    })
}

fn parse_digest_algorithm_identifier(bytes: &[u8]) -> Result<DigestAlgorithm> {
    let algorithm = Tlv::parse(bytes)?;
    if algorithm.tag == 0x30 {
        for child in TlvIter::new(algorithm.value) {
            let child = child?;
            if child.tag == 0x06 {
                return digest_algorithm_from_oid(&parse_oid(child.value)?);
            }
        }
        Err(PassportError::MalformedTlv)
    } else if algorithm.tag == 0x06 {
        digest_algorithm_from_oid(&parse_oid(algorithm.value)?)
    } else {
        Err(PassportError::MalformedTlv)
    }
}

fn digest_algorithm_from_oid(oid: &str) -> Result<DigestAlgorithm> {
    Ok(match oid {
        "1.3.14.3.2.26" => DigestAlgorithm::Sha1,
        "2.16.840.1.101.3.4.2.1" => DigestAlgorithm::Sha256,
        "2.16.840.1.101.3.4.2.2" => DigestAlgorithm::Sha384,
        "2.16.840.1.101.3.4.2.3" => DigestAlgorithm::Sha512,
        _ => DigestAlgorithm::Unknown,
    })
}

fn parse_rsa_pss_hash_algorithm(params_der: &[u8]) -> Option<DigestAlgorithm> {
    let params = Tlv::parse(params_der).ok()?;
    let value = if params.tag == 0x30 {
        params.value
    } else {
        params_der
    };
    for field in TlvIter::new(value) {
        let field = field.ok()?;
        if field.tag == 0xA0 {
            let algorithm = Tlv::parse(field.value).ok()?;
            if algorithm.tag != 0x30 {
                return None;
            }
            return parse_algorithm_oid(algorithm.value)
                .ok()
                .and_then(|oid| digest_algorithm_from_oid(&oid).ok());
        }
    }
    Some(DigestAlgorithm::Sha1)
}

fn digest_algorithm_from_len(len: usize) -> DigestAlgorithm {
    match len {
        20 => DigestAlgorithm::Sha1,
        32 => DigestAlgorithm::Sha256,
        48 => DigestAlgorithm::Sha384,
        64 => DigestAlgorithm::Sha512,
        _ => DigestAlgorithm::Unknown,
    }
}

fn digest_bytes(algorithm: DigestAlgorithm, bytes: &[u8]) -> Option<Vec<u8>> {
    match algorithm {
        DigestAlgorithm::Sha1 => {
            let mut hasher = Sha1::new();
            hasher.update(bytes);
            Some(hasher.finalize().to_vec())
        }
        DigestAlgorithm::Sha256 => Some(sha256(bytes).to_vec()),
        DigestAlgorithm::Sha384 => Some(sha384(bytes).to_vec()),
        DigestAlgorithm::Sha512 => Some(sha512(bytes).to_vec()),
        DigestAlgorithm::Unknown => None,
    }
}

fn cms_signature_algorithm(signer: &CmsSignerInfo) -> CmsSignatureAlgorithm {
    let signature_oid = signer.signature_algorithm_oid.as_deref();
    let digest_oid = signer.digest_algorithm_oid.as_deref();
    let pss_hash = signer
        .signature_algorithm_params_der
        .as_deref()
        .and_then(parse_rsa_pss_hash_algorithm);
    match (signature_oid, digest_oid) {
        (Some("1.2.840.113549.1.1.11"), _) => CmsSignatureAlgorithm::RsaPkcs1Sha256,
        (Some("1.2.840.113549.1.1.12"), _) => CmsSignatureAlgorithm::RsaPkcs1Sha384,
        (Some("1.2.840.113549.1.1.13"), _) => CmsSignatureAlgorithm::RsaPkcs1Sha512,
        (Some("1.2.840.113549.1.1.1"), Some("2.16.840.1.101.3.4.2.1")) => {
            CmsSignatureAlgorithm::RsaPkcs1Sha256
        }
        (Some("1.2.840.113549.1.1.1"), Some("2.16.840.1.101.3.4.2.2")) => {
            CmsSignatureAlgorithm::RsaPkcs1Sha384
        }
        (Some("1.2.840.113549.1.1.1"), Some("2.16.840.1.101.3.4.2.3")) => {
            CmsSignatureAlgorithm::RsaPkcs1Sha512
        }
        (Some("1.2.840.113549.1.1.10"), _) if pss_hash == Some(DigestAlgorithm::Sha256) => {
            CmsSignatureAlgorithm::RsaPssSha256
        }
        (Some("1.2.840.113549.1.1.10"), Some("2.16.840.1.101.3.4.2.1")) => {
            CmsSignatureAlgorithm::RsaPssSha256
        }
        (Some("1.2.840.10045.4.3.2"), _) => CmsSignatureAlgorithm::EcdsaP256Sha256,
        (Some("1.2.840.10045.4.3.3"), _) => CmsSignatureAlgorithm::EcdsaP256Sha384,
        (Some("1.2.840.10045.4.3.4"), _) => CmsSignatureAlgorithm::EcdsaP256Sha512,
        _ => CmsSignatureAlgorithm::Unsupported,
    }
}

fn certificate_signature_algorithm(cert: &X509CertificateInfo) -> CmsSignatureAlgorithm {
    let pss_hash = cert
        .certificate_signature_algorithm_params_der
        .as_deref()
        .and_then(parse_rsa_pss_hash_algorithm);
    match cert.certificate_signature_algorithm_oid.as_deref() {
        Some("1.2.840.113549.1.1.11") => CmsSignatureAlgorithm::RsaPkcs1Sha256,
        Some("1.2.840.113549.1.1.12") => CmsSignatureAlgorithm::RsaPkcs1Sha384,
        Some("1.2.840.113549.1.1.13") => CmsSignatureAlgorithm::RsaPkcs1Sha512,
        Some("1.2.840.113549.1.1.10") if pss_hash == Some(DigestAlgorithm::Sha256) => {
            CmsSignatureAlgorithm::RsaPssSha256
        }
        Some("1.2.840.113549.1.1.10") => CmsSignatureAlgorithm::Unsupported,
        Some("1.2.840.10045.4.3.2") => CmsSignatureAlgorithm::EcdsaP256Sha256,
        Some("1.2.840.10045.4.3.3") => CmsSignatureAlgorithm::EcdsaP256Sha384,
        Some("1.2.840.10045.4.3.4") => CmsSignatureAlgorithm::EcdsaP256Sha512,
        _ => CmsSignatureAlgorithm::Unsupported,
    }
}

fn is_supported_signature_public_key_algorithm(oid: Option<&str>) -> bool {
    matches!(
        oid,
        Some("1.2.840.113549.1.1.1") | Some("1.2.840.10045.2.1")
    )
}

fn verify_signature(
    algorithm: CmsSignatureAlgorithm,
    public_key_der: &[u8],
    message: &[u8],
    signature: &[u8],
) -> core::result::Result<(), String> {
    match algorithm {
        CmsSignatureAlgorithm::RsaPkcs1Sha256
        | CmsSignatureAlgorithm::RsaPkcs1Sha384
        | CmsSignatureAlgorithm::RsaPkcs1Sha512 => {
            verify_rsa_pkcs1_signature(algorithm, public_key_der, message, signature)
        }
        CmsSignatureAlgorithm::RsaPssSha256 => {
            verify_rsa_pss_signature(algorithm, public_key_der, message, signature)
        }
        CmsSignatureAlgorithm::EcdsaP256Sha256
        | CmsSignatureAlgorithm::EcdsaP256Sha384
        | CmsSignatureAlgorithm::EcdsaP256Sha512 => {
            verify_ecdsa_p256_signature(algorithm, public_key_der, message, signature)
        }
        CmsSignatureAlgorithm::Unsupported => Err("unsupported signature algorithm".to_string()),
    }
}

fn verify_rsa_pkcs1_signature(
    algorithm: CmsSignatureAlgorithm,
    public_key_der: &[u8],
    message: &[u8],
    signature: &[u8],
) -> core::result::Result<(), String> {
    use edgerun_crypto::rsa::pkcs1::DecodeRsaPublicKey;
    use edgerun_crypto::rsa::signature::Verifier;

    let public_key = edgerun_crypto::rsa::RsaPublicKey::from_pkcs1_der(public_key_der)
        .map_err(|e| format!("failed to parse RSA public key: {e}"))?;
    let signature = edgerun_crypto::rsa::pkcs1v15::Signature::try_from(signature)
        .map_err(|e| format!("failed to parse RSA PKCS#1 signature: {e}"))?;
    match algorithm {
        CmsSignatureAlgorithm::RsaPkcs1Sha256 => {
            let verifying_key = edgerun_crypto::rsa::pkcs1v15::VerifyingKey::<
                edgerun_crypto::rsa::sha2::Sha256,
            >::new(public_key);
            verifying_key
                .verify(message, &signature)
                .map_err(|e| format!("RSA PKCS#1 SHA-256 verification failed: {e}"))
        }
        CmsSignatureAlgorithm::RsaPkcs1Sha384 => {
            let verifying_key = edgerun_crypto::rsa::pkcs1v15::VerifyingKey::<
                edgerun_crypto::rsa::sha2::Sha384,
            >::new(public_key);
            verifying_key
                .verify(message, &signature)
                .map_err(|e| format!("RSA PKCS#1 SHA-384 verification failed: {e}"))
        }
        CmsSignatureAlgorithm::RsaPkcs1Sha512 => {
            let verifying_key = edgerun_crypto::rsa::pkcs1v15::VerifyingKey::<
                edgerun_crypto::rsa::sha2::Sha512,
            >::new(public_key);
            verifying_key
                .verify(message, &signature)
                .map_err(|e| format!("RSA PKCS#1 SHA-512 verification failed: {e}"))
        }
        CmsSignatureAlgorithm::RsaPssSha256
        | CmsSignatureAlgorithm::EcdsaP256Sha256
        | CmsSignatureAlgorithm::EcdsaP256Sha384
        | CmsSignatureAlgorithm::EcdsaP256Sha512
        | CmsSignatureAlgorithm::Unsupported => {
            Err("unsupported CMS signature algorithm".to_string())
        }
    }
}

fn verify_rsa_pss_signature(
    algorithm: CmsSignatureAlgorithm,
    public_key_der: &[u8],
    message: &[u8],
    signature: &[u8],
) -> core::result::Result<(), String> {
    use edgerun_crypto::rsa::pkcs1::DecodeRsaPublicKey;
    use edgerun_crypto::rsa::signature::Verifier;

    let public_key = edgerun_crypto::rsa::RsaPublicKey::from_pkcs1_der(public_key_der)
        .map_err(|e| format!("failed to parse RSA public key: {e}"))?;
    let signature = edgerun_crypto::rsa::pss::Signature::try_from(signature)
        .map_err(|e| format!("failed to parse RSA-PSS signature: {e}"))?;
    match algorithm {
        CmsSignatureAlgorithm::RsaPssSha256 => {
            let verifying_key = edgerun_crypto::rsa::pss::VerifyingKey::<
                edgerun_crypto::rsa::sha2::Sha256,
            >::new(public_key);
            verifying_key
                .verify(message, &signature)
                .map_err(|e| format!("RSA-PSS SHA-256 verification failed: {e}"))
        }
        _ => Err("unsupported RSA-PSS signature algorithm".to_string()),
    }
}

fn verify_ecdsa_p256_signature(
    algorithm: CmsSignatureAlgorithm,
    public_key_der: &[u8],
    message: &[u8],
    signature: &[u8],
) -> core::result::Result<(), String> {
    let verifying_key = edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(public_key_der)
        .map_err(|e| format!("failed to parse P-256 public key: {e}"))?;
    let signature = edgerun_crypto::p256::ecdsa::Signature::from_der(signature)
        .map_err(|e| format!("failed to parse ECDSA signature: {e}"))?;
    use edgerun_crypto::signature::hazmat::PrehashVerifier;
    match algorithm {
        CmsSignatureAlgorithm::EcdsaP256Sha256 => verifying_key
            .verify_prehash(&sha256(message), &signature)
            .map_err(|e| format!("ECDSA P-256 SHA-256 verification failed: {e}")),
        CmsSignatureAlgorithm::EcdsaP256Sha384 => verifying_key
            .verify_prehash(&sha384(message), &signature)
            .map_err(|e| format!("ECDSA P-256 SHA-384 verification failed: {e}")),
        CmsSignatureAlgorithm::EcdsaP256Sha512 => verifying_key
            .verify_prehash(&sha512(message), &signature)
            .map_err(|e| format!("ECDSA P-256 SHA-512 verification failed: {e}")),
        _ => Err("unsupported ECDSA signature algorithm".to_string()),
    }
}

fn data_group_from_number(number: u64) -> Option<DataGroup> {
    match number {
        1 => Some(DataGroup::Dg1),
        2 => Some(DataGroup::Dg2),
        3 => Some(DataGroup::Dg3),
        4 => Some(DataGroup::Dg4),
        5 => Some(DataGroup::Dg5),
        6 => Some(DataGroup::Dg6),
        7 => Some(DataGroup::Dg7),
        8 => Some(DataGroup::Dg8),
        9 => Some(DataGroup::Dg9),
        10 => Some(DataGroup::Dg10),
        11 => Some(DataGroup::Dg11),
        12 => Some(DataGroup::Dg12),
        13 => Some(DataGroup::Dg13),
        14 => Some(DataGroup::Dg14),
        15 => Some(DataGroup::Dg15),
        16 => Some(DataGroup::Dg16),
        _ => None,
    }
}

pub fn parse_td3_mrz(raw: &str) -> Result<MrzRecord> {
    let compact: String = raw.chars().filter(|c| *c != '\n' && *c != '\r').collect();
    if compact.len() != 88 {
        return Err(PassportError::MalformedMrz);
    }
    let line1 = &compact[0..44];
    let line2 = &compact[44..88];

    require_check("document_number", &line2[0..9], line2.as_bytes()[9] as char)?;
    require_check(
        "date_of_birth",
        &line2[13..19],
        line2.as_bytes()[19] as char,
    )?;
    require_check(
        "date_of_expiry",
        &line2[21..27],
        line2.as_bytes()[27] as char,
    )?;
    require_check(
        "personal_number",
        &line2[28..42],
        line2.as_bytes()[42] as char,
    )?;
    let composite = [&line2[0..10], &line2[13..20], &line2[21..43]].concat();
    require_check("composite", &composite, line2.as_bytes()[43] as char)?;

    let names = &line1[5..44];
    let mut name_parts = names.split("<<");
    let primary_identifier = trim_fillers(name_parts.next().unwrap_or_default());
    let secondary_identifiers = name_parts
        .next()
        .unwrap_or_default()
        .split('<')
        .filter(|part| !part.is_empty())
        .map(trim_fillers)
        .collect();

    Ok(MrzRecord {
        raw: compact.clone(),
        document_code: trim_fillers(&line1[0..2]),
        issuing_state: trim_fillers(&line1[2..5]),
        primary_identifier,
        secondary_identifiers,
        document_number: trim_fillers(&line2[0..9]),
        nationality: trim_fillers(&line2[10..13]),
        date_of_birth: line2[13..19].to_string(),
        sex: match line2.as_bytes()[20] as char {
            '<' => None,
            c => Some(c),
        },
        date_of_expiry: line2[21..27].to_string(),
        personal_number: trim_fillers(&line2[28..42]),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tlv<'a> {
    pub tag: u16,
    pub value: &'a [u8],
    pub total_len: usize,
}

impl<'a> Tlv<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        parse_tlv_at(bytes, 0)
    }
}

pub struct TlvIter<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> TlvIter<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
}

impl<'a> Iterator for TlvIter<'a> {
    type Item = Result<Tlv<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        match parse_tlv_at(self.bytes, self.pos) {
            Ok(tlv) => {
                self.pos += tlv.total_len;
                Some(Ok(tlv))
            }
            Err(e) => {
                self.pos = self.bytes.len();
                Some(Err(e))
            }
        }
    }
}

fn parse_tlv_at(bytes: &[u8], offset: usize) -> Result<Tlv<'_>> {
    if offset >= bytes.len() {
        return Err(PassportError::MalformedTlv);
    }
    let mut pos = offset;
    let first = bytes[pos];
    pos += 1;
    let tag = if first & 0x1F == 0x1F {
        if pos >= bytes.len() {
            return Err(PassportError::MalformedTlv);
        }
        let second = bytes[pos];
        pos += 1;
        ((first as u16) << 8) | second as u16
    } else {
        first as u16
    };

    if pos >= bytes.len() {
        return Err(PassportError::MalformedTlv);
    }
    let len_first = bytes[pos];
    pos += 1;
    let len = if len_first & 0x80 == 0 {
        len_first as usize
    } else {
        let count = (len_first & 0x7F) as usize;
        if count == 0 || count > 3 || pos + count > bytes.len() {
            return Err(PassportError::MalformedTlv);
        }
        let mut len = 0usize;
        for b in &bytes[pos..pos + count] {
            len = (len << 8) | (*b as usize);
        }
        pos += count;
        len
    };
    if pos + len > bytes.len() {
        return Err(PassportError::MalformedTlv);
    }
    Ok(Tlv {
        tag,
        value: &bytes[pos..pos + len],
        total_len: pos + len - offset,
    })
}

fn is_constructed_tag(tag: u16) -> bool {
    let first = if tag > 0xFF {
        (tag >> 8) as u8
    } else {
        tag as u8
    };
    first & 0x20 != 0
}

fn ber_total_len(bytes: &[u8]) -> Result<usize> {
    Ok(Tlv::parse(bytes)?.total_len)
}

fn tlv_text(bytes: &[u8]) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| PassportError::MalformedTlv)
}

fn parse_der_integer(bytes: &[u8]) -> Result<u64> {
    if bytes.is_empty() || bytes.len() > 9 {
        return Err(PassportError::MalformedTlv);
    }
    let mut out = 0u64;
    let mut start = 0usize;
    if bytes[0] == 0 && bytes.len() > 1 {
        start = 1;
    }
    for byte in &bytes[start..] {
        out = (out << 8) | (*byte as u64);
    }
    Ok(out)
}

fn parse_oid(bytes: &[u8]) -> Result<String> {
    if bytes.is_empty() {
        return Err(PassportError::MalformedTlv);
    }
    let first = bytes[0];
    let mut arcs = Vec::new();
    arcs.push((first / 40) as u64);
    arcs.push((first % 40) as u64);

    let mut value = 0u64;
    let mut in_arc = false;
    for byte in &bytes[1..] {
        in_arc = true;
        value = (value << 7) | (byte & 0x7F) as u64;
        if byte & 0x80 == 0 {
            arcs.push(value);
            value = 0;
            in_arc = false;
        }
    }
    if in_arc {
        return Err(PassportError::MalformedTlv);
    }

    let mut out = String::new();
    for (i, arc) in arcs.iter().enumerate() {
        if i > 0 {
            out.push('.');
        }
        push_u64_decimal(&mut out, *arc);
    }
    Ok(out)
}

fn push_u64_decimal(out: &mut String, mut value: u64) {
    if value == 0 {
        out.push('0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut pos = buf.len();
    while value > 0 {
        pos -= 1;
        buf[pos] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    for byte in &buf[pos..] {
        out.push(*byte as char);
    }
}

fn pace_mapping_from_oid(oid: &str) -> PaceMapping {
    if oid.contains(".4.1.")
        || oid.ends_with(".4.1")
        || oid.contains(".4.2.")
        || oid.ends_with(".4.2")
    {
        PaceMapping::GenericMapping
    } else if oid.contains(".4.3.")
        || oid.ends_with(".4.3")
        || oid.contains(".4.4.")
        || oid.ends_with(".4.4")
    {
        PaceMapping::IntegratedMapping
    } else if oid.contains(".4.6.") || oid.ends_with(".4.6") {
        PaceMapping::ChipAuthenticationMapping
    } else {
        PaceMapping::Unknown
    }
}

fn pace_cipher_from_oid(oid: &str) -> PaceCipher {
    if oid.ends_with(".1") || oid.ends_with(".2") || oid.ends_with(".3") {
        PaceCipher::Des3
    } else if oid.ends_with(".4") || oid.ends_with(".5") {
        PaceCipher::Aes128
    } else if oid.ends_with(".6") || oid.ends_with(".7") {
        PaceCipher::Aes192
    } else if oid.ends_with(".8") || oid.ends_with(".9") {
        PaceCipher::Aes256
    } else {
        PaceCipher::Unknown
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|candidate| candidate == needle)
}

fn normalize_mrz_field(value: &str, width: usize) -> String {
    let mut out: String = value
        .chars()
        .map(|c| {
            if c == ' ' {
                '<'
            } else {
                c.to_ascii_uppercase()
            }
        })
        .take(width)
        .collect();
    while out.len() < width {
        out.push('<');
    }
    out
}

pub fn check_digit(value: &[u8]) -> Result<char> {
    const WEIGHTS: [u32; 3] = [7, 3, 1];
    let mut sum = 0u32;
    for (i, byte) in value.iter().enumerate() {
        let digit = match *byte {
            b'0'..=b'9' => (*byte - b'0') as u32,
            b'A'..=b'Z' => (*byte - b'A') as u32 + 10,
            b'<' => 0,
            _ => return Err(PassportError::MalformedMrz),
        };
        sum += digit * WEIGHTS[i % 3];
    }
    Ok((b'0' + (sum % 10) as u8) as char)
}

fn require_check(field: &'static str, value: &str, expected: char) -> Result<()> {
    if check_digit(value.as_bytes())? == expected {
        Ok(())
    } else {
        Err(PassportError::InvalidCheckDigit { field })
    }
}

fn trim_fillers(value: &str) -> String {
    value.trim_end_matches('<').replace('<', " ")
}

fn derive_bac_3des_key(seed: &[u8; 16], counter: u32) -> [u8; 16] {
    let mut hasher = Sha1::new();
    hasher.update(seed);
    hasher.update(counter.to_be_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 16];
    key.copy_from_slice(&digest[..16]);
    for byte in &mut key {
        *byte = with_odd_parity(*byte);
    }
    key
}

fn with_odd_parity(mut byte: u8) -> u8 {
    let ones = (byte >> 1).count_ones();
    if ones % 2 == 0 {
        byte |= 1;
    } else {
        byte &= 0xFE;
    }
    byte
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::VecDeque;
    use core::cell::RefCell;
    use edgerun_capabilities::{
        capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
        CapabilityOperation, CapabilityProvider, CapabilityRole,
    };

    #[test]
    fn builds_standard_apdus() {
        assert_eq!(
            select_by_name(EMRTD_AID),
            vec![0x00, 0xA4, 0x04, 0x0C, 0x07, 0xA0, 0x00, 0x00, 0x02, 0x47, 0x10, 0x01]
        );
        assert_eq!(
            select_file(EF_COM),
            vec![0x00, 0xA4, 0x02, 0x0C, 0x02, 0x01, 0x1E]
        );
        assert_eq!(
            read_binary(0x0102, 0x20),
            vec![0x00, 0xB0, 0x01, 0x02, 0x20]
        );
    }

    #[test]
    fn parses_ef_com_data_group_tags() {
        let ef_com = [
            0x60, 0x12, 0x5F, 0x01, 0x04, b'0', b'1', b'0', b'7', 0x5F, 0x36, 0x04, b'0', b'4',
            b'0', b'0', 0x5C, 0x02, 0x61, 0x75,
        ];
        let parsed = parse_ef_com(&ef_com).unwrap();
        assert_eq!(parsed.lds_version.as_deref(), Some("0107"));
        assert_eq!(parsed.unicode_version.as_deref(), Some("0400"));
        assert_eq!(parsed.data_groups, vec![DataGroup::Dg1, DataGroup::Dg2]);
    }

    #[test]
    fn parses_card_access_pace_info() {
        let card_access = [
            0x31, 0x14, 0x30, 0x12, 0x06, 0x0A, 0x04, 0x00, 0x7F, 0x00, 0x07, 0x02, 0x02, 0x04,
            0x02, 0x02, 0x02, 0x01, 0x02, 0x02, 0x01, 0x0D,
        ];
        let parsed = parse_ef_card_access(&card_access).unwrap();
        assert_eq!(parsed.pace_infos.len(), 1);
        assert_eq!(parsed.pace_infos[0].protocol_oid, "0.4.0.127.0.7.2.2.4.2.2");
        assert_eq!(parsed.pace_infos[0].version, 2);
        assert_eq!(parsed.pace_infos[0].parameter_id, Some(13));
        assert_eq!(parsed.pace_infos[0].mapping, PaceMapping::GenericMapping);
        assert_eq!(parsed.pace_infos[0].cipher, PaceCipher::Des3);
    }

    #[test]
    fn extracts_jpeg_from_dg2_payload() {
        let dg2 = [
            0x75, 0x0B, 0x7F, 0x61, 0x07, 0xAA, 0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0xFF, 0xD9,
        ];
        let images = extract_dg2_face_images(&dg2);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].format, FaceImageFormat::Jpeg);
        assert_eq!(images[0].offset, 6);
        assert_eq!(
            images[0].bytes,
            vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0xFF, 0xD9]
        );
    }

    #[test]
    fn parses_ef_sod_lds_security_object_hashes() {
        let dg1 = [0x61, 0x03, 0x5F, 0x1F, 0x00];
        let ef_sod = minimal_ef_sod_with_sha256_dg1(&dg1);
        let parsed = parse_ef_sod_hashes(&ef_sod).unwrap();
        assert_eq!(parsed.version, 0);
        assert_eq!(parsed.digest_algorithm, DigestAlgorithm::Sha256);
        assert_eq!(parsed.data_group_hashes.len(), 1);
        assert_eq!(parsed.data_group_hashes[0].group, DataGroup::Dg1);
        assert_eq!(parsed.data_group_hashes[0].digest, sha256(&dg1).to_vec());
    }

    #[test]
    fn parses_ef_sod_cms_signed_attr_message_digest() {
        let dg1 = [0x61, 0x03, 0x5F, 0x1F, 0x00];
        let ef_sod = minimal_ef_sod_with_sha256_dg1(&dg1);
        let parsed = parse_ef_sod_cms(&ef_sod).unwrap();
        assert_eq!(
            parsed.digest_algorithm_oids,
            vec!["2.16.840.1.101.3.4.2.1".to_string()]
        );
        assert_eq!(parsed.certificates.len(), 1);
        assert_eq!(parsed.signer_certificate_candidates().len(), 1);
        let cert = parsed.certificates[0].info.as_ref().unwrap();
        assert_eq!(cert.version, 3);
        assert_eq!(cert.serial_number, vec![0x01]);
        assert_eq!(cert.certificate_sha256, sha256(&parsed.certificates[0].der));
        assert_eq!(
            cert.tbs_certificate_sha256,
            sha256(&cert.tbs_certificate_der)
        );
        assert_eq!(cert.issuer_sha256, sha256(&cert.issuer_der));
        assert_eq!(cert.subject_sha256, sha256(&cert.subject_der));
        assert!(cert.is_self_issued());
        assert_eq!(
            cert.signature_algorithm_oid.as_deref(),
            Some("1.2.840.113549.1.1.11")
        );
        assert_eq!(cert.not_before, "240101000000Z");
        assert_eq!(cert.not_after, "300101000000Z");
        assert_eq!(
            cert.subject_public_key_algorithm_oid.as_deref(),
            Some("1.2.840.113549.1.1.1")
        );
        assert_eq!(
            cert.certificate_signature_algorithm_oid.as_deref(),
            Some("1.2.840.113549.1.1.11")
        );
        assert_eq!(cert.certificate_signature, vec![0xCA, 0xFE]);
        assert_eq!(parsed.signer_infos.len(), 1);
        assert_eq!(parsed.signer_infos[0].version, 1);
        assert_eq!(
            parsed.signer_infos[0].digest_algorithm_oid.as_deref(),
            Some("2.16.840.1.101.3.4.2.1")
        );
        assert_eq!(
            parsed.signer_infos[0].signature_algorithm_oid.as_deref(),
            Some("1.2.840.113549.1.1.1")
        );
        assert_eq!(parsed.signer_infos[0].signature, vec![0xAA, 0x55]);
        let signed_input = parsed.signer_infos[0]
            .signed_attributes_signature_input_der
            .as_ref()
            .unwrap();
        assert_eq!(signed_input[0], 0x31);
        assert_eq!(
            parsed.signer_infos[0].signed_attributes_sha256,
            Some(sha256(signed_input))
        );
        let verification = verify_cms_signatures(&parsed);
        assert_eq!(verification.len(), 1);
        assert!(!verification[0].verified);
        assert_eq!(parsed.signed_attr_digest_algorithm, DigestAlgorithm::Sha256);
        assert_eq!(parsed.signed_attr_digest_matches_econtent, Some(true));
        assert_eq!(
            parsed.signed_attr_message_digest.as_deref(),
            Some(sha256(&der_lds_security_object_sha256_dg1(&dg1)).as_slice())
        );
    }

    #[test]
    fn verifies_rsa_pkcs1_sha256_cms_signature() {
        use edgerun_crypto::rsa::pkcs1::EncodeRsaPublicKey;
        use edgerun_crypto::rsa::signature::{SignatureEncoding, Signer};

        let mut rng = edgerun_crypto::rng::OsRng;
        let private_key = edgerun_crypto::rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let public_key = edgerun_crypto::rsa::RsaPublicKey::from(&private_key);
        let public_key_der = public_key.to_pkcs1_der().unwrap().as_bytes().to_vec();
        let signed_attributes = der_sequence(&[
            der_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04]),
            der_set(&[der_octet_string(&[0x11; 32])]),
        ]);
        let signed_input = encode_tlv(0x31, &signed_attributes).unwrap();
        let signing_key =
            edgerun_crypto::rsa::pkcs1v15::SigningKey::<edgerun_crypto::rsa::sha2::Sha256>::new(
                private_key,
            );
        let signature = signing_key.sign(&signed_input).to_vec();
        let cms = CmsSignedDataInfo {
            lds_security_object: LdsSecurityObject::default(),
            digest_algorithm_oids: vec!["2.16.840.1.101.3.4.2.1".to_string()],
            certificates: vec![CmsCertificate {
                der: Vec::new(),
                info: Some(X509CertificateInfo {
                    subject_public_key_algorithm_oid: Some("1.2.840.113549.1.1.1".to_string()),
                    subject_public_key_der: public_key_der,
                    ..X509CertificateInfo::default()
                }),
            }],
            signer_infos: vec![CmsSignerInfo {
                version: 1,
                digest_algorithm_oid: Some("2.16.840.1.101.3.4.2.1".to_string()),
                signature_algorithm_oid: Some("1.2.840.113549.1.1.1".to_string()),
                signature_algorithm_params_der: None,
                signature,
                signed_attributes_der: Some(signed_attributes),
                signed_attributes_signature_input_der: Some(signed_input),
                signed_attributes_sha256: None,
                signed_attr_message_digest: None,
            }],
            signed_attr_message_digest: None,
            signed_attr_digest_algorithm: DigestAlgorithm::Unknown,
            signed_attr_digest_matches_econtent: None,
        };
        let verification = verify_cms_signatures(&cms);
        assert_eq!(verification.len(), 1);
        assert!(verification[0].verified);
        assert_eq!(verification[0].certificate_index, Some(0));
        assert_eq!(
            verification[0].algorithm,
            CmsSignatureAlgorithm::RsaPkcs1Sha256
        );
    }

    #[test]
    fn verifies_rsa_pss_sha256_cms_signature() {
        use edgerun_crypto::rsa::pkcs1::EncodeRsaPublicKey;
        use edgerun_crypto::rsa::signature::{RandomizedSigner, SignatureEncoding};

        let mut rng = edgerun_crypto::rng::OsRng;
        let private_key = edgerun_crypto::rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let public_key = edgerun_crypto::rsa::RsaPublicKey::from(&private_key);
        let public_key_der = public_key.to_pkcs1_der().unwrap().as_bytes().to_vec();
        let signed_attributes = der_sequence(&[
            der_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04]),
            der_set(&[der_octet_string(&[0x33; 32])]),
        ]);
        let signed_input = encode_tlv(0x31, &signed_attributes).unwrap();
        let signing_key =
            edgerun_crypto::rsa::pss::SigningKey::<edgerun_crypto::rsa::sha2::Sha256>::new(private_key);
        let signature = signing_key.sign_with_rng(&mut rng, &signed_input).to_vec();
        let cms = CmsSignedDataInfo {
            lds_security_object: LdsSecurityObject::default(),
            digest_algorithm_oids: vec!["2.16.840.1.101.3.4.2.1".to_string()],
            certificates: vec![CmsCertificate {
                der: Vec::new(),
                info: Some(X509CertificateInfo {
                    subject_public_key_algorithm_oid: Some("1.2.840.113549.1.1.1".to_string()),
                    subject_public_key_der: public_key_der,
                    ..X509CertificateInfo::default()
                }),
            }],
            signer_infos: vec![CmsSignerInfo {
                version: 1,
                digest_algorithm_oid: Some("2.16.840.1.101.3.4.2.1".to_string()),
                signature_algorithm_oid: Some("1.2.840.113549.1.1.10".to_string()),
                signature_algorithm_params_der: Some(der_rsa_pss_sha256_params()),
                signature,
                signed_attributes_der: Some(signed_attributes),
                signed_attributes_signature_input_der: Some(signed_input),
                signed_attributes_sha256: None,
                signed_attr_message_digest: None,
            }],
            signed_attr_message_digest: None,
            signed_attr_digest_algorithm: DigestAlgorithm::Unknown,
            signed_attr_digest_matches_econtent: None,
        };
        let verification = verify_cms_signatures(&cms);
        assert_eq!(verification.len(), 1);
        assert!(verification[0].verified);
        assert_eq!(verification[0].certificate_index, Some(0));
        assert_eq!(
            verification[0].algorithm,
            CmsSignatureAlgorithm::RsaPssSha256
        );
    }

    #[test]
    fn verifies_ecdsa_p256_sha256_cms_signature() {
        use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
        use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;

        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::random(&mut edgerun_crypto::rng::OsRng);
        let public_key = signing_key.verifying_key().to_encoded_point(false);
        let signed_attributes = der_sequence(&[
            der_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04]),
            der_set(&[der_octet_string(&[0x22; 32])]),
        ]);
        let signed_input = encode_tlv(0x31, &signed_attributes).unwrap();
        let signature: edgerun_crypto::p256::ecdsa::Signature =
            signing_key.sign_prehash(&sha256(&signed_input)).unwrap();
        let cms = CmsSignedDataInfo {
            lds_security_object: LdsSecurityObject::default(),
            digest_algorithm_oids: vec!["2.16.840.1.101.3.4.2.1".to_string()],
            certificates: vec![CmsCertificate {
                der: Vec::new(),
                info: Some(X509CertificateInfo {
                    subject_public_key_algorithm_oid: Some("1.2.840.10045.2.1".to_string()),
                    subject_public_key_der: public_key.as_bytes().to_vec(),
                    ..X509CertificateInfo::default()
                }),
            }],
            signer_infos: vec![CmsSignerInfo {
                version: 1,
                digest_algorithm_oid: Some("2.16.840.1.101.3.4.2.1".to_string()),
                signature_algorithm_oid: Some("1.2.840.10045.4.3.2".to_string()),
                signature_algorithm_params_der: None,
                signature: signature.to_der().as_bytes().to_vec(),
                signed_attributes_der: Some(signed_attributes),
                signed_attributes_signature_input_der: Some(signed_input),
                signed_attributes_sha256: None,
                signed_attr_message_digest: None,
            }],
            signed_attr_message_digest: None,
            signed_attr_digest_algorithm: DigestAlgorithm::Unknown,
            signed_attr_digest_matches_econtent: None,
        };
        let verification = verify_cms_signatures(&cms);
        assert_eq!(verification.len(), 1);
        assert!(verification[0].verified);
        assert_eq!(
            verification[0].algorithm,
            CmsSignatureAlgorithm::EcdsaP256Sha256
        );
    }

    #[test]
    fn verifies_document_signer_certificate_against_trust_anchor() {
        use edgerun_crypto::rsa::pkcs1::EncodeRsaPublicKey;
        use edgerun_crypto::rsa::signature::{SignatureEncoding, Signer};

        let mut rng = edgerun_crypto::rng::OsRng;
        let anchor_private_key = edgerun_crypto::rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let anchor_public_key = edgerun_crypto::rsa::RsaPublicKey::from(&anchor_private_key);
        let anchor_public_key_der = anchor_public_key
            .to_pkcs1_der()
            .unwrap()
            .as_bytes()
            .to_vec();
        let tbs = der_sequence(&[
            der_integer(1),
            der_sha256_algorithm_identifier(),
            der_sequence(&[]),
        ]);
        let signing_key =
            edgerun_crypto::rsa::pkcs1v15::SigningKey::<edgerun_crypto::rsa::sha2::Sha256>::new(
                anchor_private_key,
            );
        let signature = signing_key.sign(&tbs).to_vec();
        let anchor_subject = der_sequence(&[]);
        let signer_cert = X509CertificateInfo {
            tbs_certificate_der: tbs,
            issuer_der: anchor_subject.clone(),
            certificate_signature_algorithm_oid: Some("1.2.840.113549.1.1.11".to_string()),
            certificate_signature: signature,
            ..X509CertificateInfo::default()
        };
        let trust_anchor = X509CertificateInfo {
            subject_der: anchor_subject,
            subject_public_key_algorithm_oid: Some("1.2.840.113549.1.1.1".to_string()),
            subject_public_key_der: anchor_public_key_der,
            ..X509CertificateInfo::default()
        };
        let cms = CmsSignedDataInfo {
            certificates: vec![CmsCertificate {
                der: Vec::new(),
                info: Some(signer_cert),
            }],
            ..CmsSignedDataInfo::default()
        };

        let verification = verify_document_signer_certificates(&cms, &[trust_anchor]);
        assert_eq!(verification.len(), 1);
        assert!(verification[0].verified);
        assert_eq!(verification[0].certificate_index, 0);
        assert_eq!(verification[0].trust_anchor_index, Some(0));
        assert_eq!(
            verification[0].algorithm,
            CmsSignatureAlgorithm::RsaPkcs1Sha256
        );
    }

    #[test]
    fn verifies_passive_auth_hashes() {
        let dg1 = PassportFile {
            fid: DataGroup::Dg1.file_id(),
            bytes: vec![0x61, 0x03, 0x5F, 0x1F, 0x00],
        };
        let ef_sod = minimal_ef_sod_with_sha256_dg1(&dg1.bytes);
        let report = verify_passive_auth_hashes(&ef_sod, &[(DataGroup::Dg1, dg1)]).unwrap();
        assert_eq!(report.verified_groups, vec![DataGroup::Dg1]);
        assert!(report.mismatched_groups.is_empty());
        assert!(report.missing_groups.is_empty());
    }

    #[test]
    fn reports_passive_auth_hash_mismatch() {
        let expected_dg1 = [0x61, 0x03, 0x5F, 0x1F, 0x00];
        let actual_dg1 = PassportFile {
            fid: DataGroup::Dg1.file_id(),
            bytes: vec![0x61, 0x03, 0x5F, 0x1F, 0x01],
        };
        let ef_sod = minimal_ef_sod_with_sha256_dg1(&expected_dg1);
        let report = verify_passive_auth_hashes(&ef_sod, &[(DataGroup::Dg1, actual_dg1)]).unwrap();
        assert!(report.verified_groups.is_empty());
        assert_eq!(report.mismatched_groups, vec![DataGroup::Dg1]);
    }

    #[test]
    fn derives_mrz_information_and_bac_keys() {
        let mrz = MrzAccessData::new("L898902C3", "740812", "120415");
        assert_eq!(mrz.mrz_information().unwrap(), "L898902C3674081221204159");
        let keys = mrz.bac_document_basic_keys().unwrap();
        assert_ne!(keys.kenc, keys.kmac);
        assert!(keys.kenc.iter().all(|b| b.count_ones() % 2 == 1));
    }

    #[test]
    fn parses_td3_mrz() {
        let raw = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\
                   L898902C36UTO7408122F1204159ZE184226B<<<<<10";
        let parsed = parse_td3_mrz(raw).unwrap();
        assert_eq!(parsed.primary_identifier, "ERIKSSON");
        assert_eq!(
            parsed.secondary_identifiers,
            vec!["ANNA".to_string(), "MARIA".to_string()]
        );
        assert_eq!(parsed.document_number, "L898902C3");
        assert_eq!(parsed.nationality, "UTO");
        assert_eq!(parsed.sex, Some('F'));
    }

    #[test]
    fn reads_file_through_generic_nfc_reader() {
        let reader = MockReader::new(vec![
            (select_by_name(EMRTD_AID), sw_ok(&[])),
            (select_file(EF_COM), sw_ok(&[])),
            (
                read_binary(0, 8),
                sw_ok(&[0x60, 0x04, 0x5C, 0x02, 0x61, 0x75]),
            ),
        ]);
        let mut session = PassportSession::new(&reader, "target-1");
        session.select_passport_application().unwrap();
        let file = session.read_file(EF_COM).unwrap();
        assert_eq!(file.bytes, vec![0x60, 0x04, 0x5C, 0x02, 0x61, 0x75]);
    }

    #[test]
    fn authenticates_bac_with_deterministic_entropy() {
        let access = MrzAccessData::new("L898902C3", "740812", "120415");
        let keys = access.bac_document_basic_keys().unwrap();
        let rnd_icc = [0x46, 0x08, 0xF9, 0x19, 0x88, 0x70, 0x22, 0x12];
        let rnd_ifd = [0x78, 0x17, 0x23, 0x86, 0x0C, 0x06, 0xC2, 0x26];
        let kifd = [
            0x0B, 0x79, 0x52, 0x40, 0xCB, 0x70, 0x49, 0xB0, 0x1C, 0x19, 0xB3, 0x3E, 0x32, 0x80,
            0x4F, 0x0B,
        ];
        let kicc = [
            0x0B, 0x4F, 0x80, 0x32, 0x3E, 0xB3, 0x19, 0x1C, 0xB0, 0x49, 0x70, 0xCB, 0x40, 0x52,
            0x79, 0x0B,
        ];

        let mut s_ifd = Vec::new();
        s_ifd.extend_from_slice(&rnd_ifd);
        s_ifd.extend_from_slice(&rnd_icc);
        s_ifd.extend_from_slice(&kifd);
        let cipher = Tdes2::new(&keys.kenc);
        let eifd = cipher.cbc_encrypt(&[0u8; 8], &s_ifd).unwrap();
        let mifd = iso9797_alg3_mac_3des2(&keys.kmac, &eifd);
        let mut ifd_auth_data = eifd;
        ifd_auth_data.extend_from_slice(&mifd);

        let mut s_icc = Vec::new();
        s_icc.extend_from_slice(&rnd_icc);
        s_icc.extend_from_slice(&rnd_ifd);
        s_icc.extend_from_slice(&kicc);
        let eicc = cipher.cbc_encrypt(&[0u8; 8], &s_icc).unwrap();
        let micc = iso9797_alg3_mac_3des2(&keys.kmac, &eicc);
        let mut icc_auth_data = eicc;
        icc_auth_data.extend_from_slice(&micc);

        let reader = MockReader::new(vec![
            (get_challenge(), sw_ok(&rnd_icc)),
            (mutual_authenticate(&ifd_auth_data), sw_ok(&icc_auth_data)),
        ]);

        let sm = BacSecureMessaging::authenticate_with_entropy(
            &reader, "target-1", &access, rnd_ifd, kifd,
        )
        .unwrap();
        assert_eq!(
            sm.session_send_counter(),
            [0x88, 0x70, 0x22, 0x12, 0x0C, 0x06, 0xC2, 0x26]
        );
    }

    #[test]
    fn bac_secure_messaging_wraps_and_unwraps_read_binary() {
        let mut sm = BacSecureMessaging {
            ksenc: [
                0x97, 0x9E, 0xC1, 0x3B, 0x1C, 0xC1, 0xB0, 0x42, 0xF6, 0x59, 0xCA, 0x2E, 0xA6, 0x1E,
                0x3D, 0x1E,
            ],
            ksmac: [
                0xF1, 0xCB, 0x1F, 0x1F, 0xB5, 0xAD, 0xF2, 0x08, 0x80, 0x6B, 0x89, 0xDC, 0x57, 0x9D,
                0xC1, 0xF8,
            ],
            ssc: [0x88, 0x70, 0x22, 0x12, 0x0C, 0x06, 0xC2, 0x26],
        };

        let protected_command = sm.wrap_command(&read_binary(0, 8)).unwrap();
        assert_eq!(&protected_command[..5], &[0x0C, 0xB0, 0x00, 0x00, 0x0D]);
        assert!(protected_command.windows(2).any(|w| w == [0x97, 0x01]));
        assert!(protected_command.windows(2).any(|w| w == [0x8E, 0x08]));

        let mut response_ssc = sm.ssc;
        increment_ssc(&mut response_ssc);
        let encrypted = Tdes2::new(&sm.ksenc)
            .cbc_encrypt(&[0u8; 8], &iso9797_method2_pad(b"ABCDEFGH", 8))
            .unwrap();
        let mut protected = Vec::new();
        let mut do87 = vec![0x01];
        do87.extend_from_slice(&encrypted);
        push_tlv(&mut protected, 0x87, &do87).unwrap();
        push_tlv(&mut protected, 0x99, &[0x90, 0x00]).unwrap();
        let mut mac_input = Vec::new();
        mac_input.extend_from_slice(&response_ssc);
        mac_input.extend_from_slice(&iso9797_method2_pad(&protected, 8));
        let mac = iso9797_alg3_mac_3des2(&sm.ksmac, &mac_input);
        push_tlv(&mut protected, 0x8E, &mac).unwrap();
        protected.extend_from_slice(&[0x90, 0x00]);

        assert_eq!(sm.unwrap_response(&protected).unwrap(), sw_ok(b"ABCDEFGH"));
    }

    fn sw_ok(data: &[u8]) -> Vec<u8> {
        let mut out = data.to_vec();
        out.extend_from_slice(&[0x90, 0x00]);
        out
    }

    fn minimal_ef_sod_with_sha256_dg1(dg1: &[u8]) -> Vec<u8> {
        let lds_security_object = der_lds_security_object_sha256_dg1(dg1);
        let econtent_digest = sha256(&lds_security_object);
        let digest_algorithm = der_sha256_algorithm_identifier();
        let signed_attrs = der_context(
            0,
            &der_sequence(&[
                der_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04]),
                der_set(&[der_octet_string(&econtent_digest)]),
            ]),
        );
        let signer_info = der_sequence(&[
            der_integer(1),
            der_sequence(&[der_sequence(&[]), der_integer(1)]),
            digest_algorithm.clone(),
            signed_attrs,
            der_sequence(&[
                der_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01]),
                der_null(),
            ]),
            der_octet_string(&[0xAA, 0x55]),
        ]);
        let certificates = der_context(0, &minimal_x509_certificate_der());
        let encap_content_info = der_sequence(&[
            der_oid(&[0x67, 0x81, 0x08, 0x01, 0x01, 0x01]),
            der_context(0, &der_octet_string(&lds_security_object)),
        ]);
        let signed_data = der_sequence(&[
            der_integer(3),
            der_set(&[digest_algorithm]),
            encap_content_info,
            certificates,
            der_set(&[signer_info]),
        ]);
        let content_info = der_sequence(&[
            der_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02]),
            der_context(0, &signed_data),
        ]);
        let mut ef_sod = vec![0x77];
        push_test_len(&mut ef_sod, content_info.len());
        ef_sod.extend_from_slice(&content_info);
        ef_sod
    }

    fn der_lds_security_object_sha256_dg1(dg1: &[u8]) -> Vec<u8> {
        let dg_hash = sha256(dg1);
        let data_group_hash_value = der_sequence(&[der_integer(1), der_octet_string(&dg_hash)]);
        let data_group_hash_values = der_sequence(&[data_group_hash_value]);
        der_sequence(&[
            der_integer(0),
            der_sha256_algorithm_identifier(),
            data_group_hash_values,
        ])
    }

    fn der_sha256_algorithm_identifier() -> Vec<u8> {
        der_sequence(&[
            der_oid(&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01]),
            der_null(),
        ])
    }

    fn der_rsa_pss_sha256_params() -> Vec<u8> {
        der_sequence(&[der_context(0, &der_sha256_algorithm_identifier())])
    }

    fn minimal_x509_certificate_der() -> Vec<u8> {
        let rsa_encryption = der_sequence(&[
            der_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01]),
            der_null(),
        ]);
        let sha256_rsa = der_sequence(&[
            der_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B]),
            der_null(),
        ]);
        let name = der_sequence(&[]);
        let validity = der_sequence(&[der_utctime("240101000000Z"), der_utctime("300101000000Z")]);
        let spki = der_sequence(&[rsa_encryption, der_bit_string(&[0x01, 0x02, 0x03])]);
        let tbs = der_sequence(&[
            der_context(0, &der_integer(2)),
            der_integer(1),
            sha256_rsa.clone(),
            name.clone(),
            validity,
            name,
            spki,
        ]);
        der_sequence(&[tbs, sha256_rsa, der_bit_string(&[0xCA, 0xFE])])
    }

    fn der_sequence(items: &[Vec<u8>]) -> Vec<u8> {
        der_constructed(0x30, items)
    }

    fn der_set(items: &[Vec<u8>]) -> Vec<u8> {
        der_constructed(0x31, items)
    }

    fn der_context(number: u8, inner: &[u8]) -> Vec<u8> {
        let mut out = vec![0xA0 + number];
        push_test_len(&mut out, inner.len());
        out.extend_from_slice(inner);
        out
    }

    fn der_constructed(tag: u8, items: &[Vec<u8>]) -> Vec<u8> {
        let len = items.iter().map(|item| item.len()).sum();
        let mut out = vec![tag];
        push_test_len(&mut out, len);
        for item in items {
            out.extend_from_slice(item);
        }
        out
    }

    fn der_integer(value: u8) -> Vec<u8> {
        vec![0x02, 0x01, value]
    }

    fn der_octet_string(value: &[u8]) -> Vec<u8> {
        let mut out = vec![0x04];
        push_test_len(&mut out, value.len());
        out.extend_from_slice(value);
        out
    }

    fn der_bit_string(value: &[u8]) -> Vec<u8> {
        let mut out = vec![0x03];
        push_test_len(&mut out, value.len() + 1);
        out.push(0x00);
        out.extend_from_slice(value);
        out
    }

    fn der_utctime(value: &str) -> Vec<u8> {
        let mut out = vec![0x17];
        push_test_len(&mut out, value.len());
        out.extend_from_slice(value.as_bytes());
        out
    }

    fn der_oid(value: &[u8]) -> Vec<u8> {
        let mut out = vec![0x06];
        push_test_len(&mut out, value.len());
        out.extend_from_slice(value);
        out
    }

    fn der_null() -> Vec<u8> {
        vec![0x05, 0x00]
    }

    fn push_test_len(out: &mut Vec<u8>, len: usize) {
        if len < 0x80 {
            out.push(len as u8);
        } else if len <= 0xFF {
            out.extend_from_slice(&[0x81, len as u8]);
        } else {
            out.extend_from_slice(&[0x82, (len >> 8) as u8, len as u8]);
        }
    }

    struct MockReader {
        exchanges: RefCell<VecDeque<(Vec<u8>, Vec<u8>)>>,
    }

    impl MockReader {
        fn new(exchanges: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
            Self {
                exchanges: RefCell::new(exchanges.into()),
            }
        }
    }

    impl CapabilityProvider for MockReader {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "mock-nfc",
                "mock",
                CapabilityRole::Communication,
                &[CapabilityModality::Radio],
                &[CapabilityEventKind::Radio],
                &[CapabilityOperation::Invoke],
                Vec::new(),
            )
        }
    }

    impl NfcReader for MockReader {
        fn read_ndef(
            &self,
            _target_id: &str,
        ) -> core::result::Result<
            Option<edgerun_nfc::NdefMessage>,
            edgerun_capabilities::CapabilityError,
        > {
            Ok(None)
        }

        fn write_ndef(
            &self,
            _target_id: &str,
            _message: &edgerun_nfc::NdefMessage,
        ) -> core::result::Result<(), edgerun_capabilities::CapabilityError> {
            Ok(())
        }

        fn transceive(
            &self,
            _target_id: &str,
            command: &[u8],
        ) -> core::result::Result<Vec<u8>, edgerun_capabilities::CapabilityError> {
            let (expected, response) = self.exchanges.borrow_mut().pop_front().unwrap();
            assert_eq!(expected, command);
            Ok(response)
        }
    }
}
