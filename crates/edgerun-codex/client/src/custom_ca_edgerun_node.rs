use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use edgerun_encoding::base64::standard_decode;
use edgerun_error::Error;
use tracing::info;

pub const CODEX_CA_CERT_ENV: &str = "CODEX_CA_CERTIFICATE";
pub const SSL_CERT_FILE_ENV: &str = "SSL_CERT_FILE";
const CA_CERT_HINT: &str = "If you set CODEX_CA_CERTIFICATE or SSL_CERT_FILE, ensure it points to a PEM file containing one or more CERTIFICATE blocks, or unset it to use system roots.";

#[derive(Debug, Error)]
pub enum BuildCustomCaTransportError {
    #[error("Failed to read CA certificate file {} selected by {}: {source}. {hint}", path.display(), source_env, hint = CA_CERT_HINT)]
    ReadCaFile {
        source_env: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    #[error("Failed to load CA certificates from {} selected by {}: {detail}. {hint}", path.display(), source_env, hint = CA_CERT_HINT)]
    InvalidCaFile {
        source_env: &'static str,
        path: PathBuf,
        detail: String,
    },
    #[error("Failed to parse certificate #{certificate_index} from {} selected by {}: {source}. {hint}", path.display(), source_env, hint = CA_CERT_HINT)]
    RegisterCertificate {
        source_env: &'static str,
        path: PathBuf,
        certificate_index: usize,
        source: edgerun_reqwest::Error,
    },
    #[error("Failed to build HTTP client while using CA bundle from {} ({}): {source}", source_env, path.display())]
    BuildClientWithCustomCa {
        source_env: &'static str,
        path: PathBuf,
        #[source]
        source: edgerun_reqwest::Error,
    },
    #[error("Failed to build HTTP client while using system root certificates: {0}")]
    BuildClientWithSystemRoots(#[source] edgerun_reqwest::Error),
}

impl From<BuildCustomCaTransportError> for io::Error {
    fn from(error: BuildCustomCaTransportError) -> Self {
        match error {
            BuildCustomCaTransportError::ReadCaFile { ref source, .. } => {
                io::Error::new(source.kind(), error)
            }
            BuildCustomCaTransportError::InvalidCaFile { .. }
            | BuildCustomCaTransportError::RegisterCertificate { .. } => {
                io::Error::new(io::ErrorKind::InvalidData, error)
            }
            BuildCustomCaTransportError::BuildClientWithCustomCa { .. }
            | BuildCustomCaTransportError::BuildClientWithSystemRoots(_) => io::Error::other(error),
        }
    }
}

pub fn build_reqwest_client_with_custom_ca(
    builder: edgerun_reqwest::ClientBuilder,
) -> Result<edgerun_reqwest::Client, BuildCustomCaTransportError> {
    build_reqwest_client_with_env(&ProcessEnv, builder)
}

pub fn build_reqwest_client_for_subprocess_tests(
    builder: edgerun_reqwest::ClientBuilder,
) -> Result<edgerun_reqwest::Client, BuildCustomCaTransportError> {
    build_reqwest_client_with_env(&ProcessEnv, builder.no_proxy())
}

fn build_reqwest_client_with_env(
    env_source: &dyn EnvSource,
    mut builder: edgerun_reqwest::ClientBuilder,
) -> Result<edgerun_reqwest::Client, BuildCustomCaTransportError> {
    let Some(bundle) = env_source.configured_ca_bundle() else {
        return builder
            .build()
            .map_err(BuildCustomCaTransportError::BuildClientWithSystemRoots);
    };

    info!(source_env = bundle.source_env, ca_path = %bundle.path.display(), "building HTTP client with Edgerun node TLS custom CA roots");

    for (idx, cert) in bundle.load_certificates()?.iter().enumerate() {
        let cert = edgerun_reqwest::Certificate::from_der(cert.as_ref()).map_err(|source| {
            BuildCustomCaTransportError::RegisterCertificate {
                source_env: bundle.source_env,
                path: bundle.path.clone(),
                certificate_index: idx + 1,
                source,
            }
        })?;
        builder = builder.add_root_certificate(cert);
    }

    builder.build().map_err(
        |source| BuildCustomCaTransportError::BuildClientWithCustomCa {
            source_env: bundle.source_env,
            path: bundle.path,
            source,
        },
    )
}

trait EnvSource {
    fn var(&self, key: &str) -> Option<String>;
    fn non_empty_path(&self, key: &str) -> Option<PathBuf> {
        self.var(key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }
    fn configured_ca_bundle(&self) -> Option<ConfiguredCaBundle> {
        self.non_empty_path(CODEX_CA_CERT_ENV)
            .map(|path| ConfiguredCaBundle {
                source_env: CODEX_CA_CERT_ENV,
                path,
            })
            .or_else(|| {
                self.non_empty_path(SSL_CERT_FILE_ENV)
                    .map(|path| ConfiguredCaBundle {
                        source_env: SSL_CERT_FILE_ENV,
                        path,
                    })
            })
    }
}

struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn var(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }
}

struct ConfiguredCaBundle {
    source_env: &'static str,
    path: PathBuf,
}

impl ConfiguredCaBundle {
    fn load_certificates(&self) -> Result<Vec<Vec<u8>>, BuildCustomCaTransportError> {
        let pem_data =
            fs::read(&self.path).map_err(|source| BuildCustomCaTransportError::ReadCaFile {
                source_env: self.source_env,
                path: self.path.clone(),
                source,
            })?;
        let normalized = NormalizedPem::from_pem_data(self.source_env, &self.path, &pem_data);
        let mut certificates = Vec::new();
        let mut logged_crl = false;
        for section in normalized
            .sections()
            .map_err(|error| self.pem_parse_error(&error))?
        {
            match section.kind {
                PemSectionKind::Certificate => {
                    let der = section.der;
                    let cert = normalized.certificate_der(&der).ok_or_else(|| {
                        self.invalid_ca_file("invalid TRUSTED CERTIFICATE DER length")
                    })?;
                    certificates.push(cert.to_vec());
                }
                PemSectionKind::Crl if !logged_crl => {
                    info!(source_env = self.source_env, ca_path = %self.path.display(), "ignoring X509 CRL entries found in custom CA bundle");
                    logged_crl = true;
                }
                _ => {}
            }
        }
        if certificates.is_empty() {
            return Err(self.invalid_ca_file("no certificates found in PEM file"));
        }
        Ok(certificates)
    }

    fn pem_parse_error(&self, error: &str) -> BuildCustomCaTransportError {
        self.invalid_ca_file(format!("failed to parse PEM file: {error}"))
    }

    fn invalid_ca_file(&self, detail: impl std::fmt::Display) -> BuildCustomCaTransportError {
        BuildCustomCaTransportError::InvalidCaFile {
            source_env: self.source_env,
            path: self.path.clone(),
            detail: detail.to_string(),
        }
    }
}

enum NormalizedPem {
    Standard(String),
    TrustedCertificate(String),
}

impl NormalizedPem {
    fn from_pem_data(_source_env: &'static str, _path: &Path, pem_data: &[u8]) -> Self {
        let pem = String::from_utf8_lossy(pem_data);
        if pem.contains("TRUSTED CERTIFICATE") {
            info!(_source_env, ca_path = %_path.display(), "normalizing OpenSSL TRUSTED CERTIFICATE labels in custom CA bundle");
            Self::TrustedCertificate(
                pem.replace("BEGIN TRUSTED CERTIFICATE", "BEGIN CERTIFICATE")
                    .replace("END TRUSTED CERTIFICATE", "END CERTIFICATE"),
            )
        } else {
            Self::Standard(pem.into_owned())
        }
    }

    fn contents(&self) -> &str {
        match self {
            Self::Standard(contents) | Self::TrustedCertificate(contents) => contents,
        }
    }

    fn sections(&self) -> Result<Vec<PemSection>, String> {
        parse_pem_sections(self.contents())
    }

    fn certificate_der<'a>(&self, der: &'a [u8]) -> Option<&'a [u8]> {
        match self {
            Self::Standard(_) => Some(der),
            Self::TrustedCertificate(_) => first_der_item(der),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PemSectionKind {
    Certificate,
    Crl,
    Other,
}

struct PemSection {
    kind: PemSectionKind,
    der: Vec<u8>,
}

fn parse_pem_sections(contents: &str) -> Result<Vec<PemSection>, String> {
    let mut sections = Vec::new();
    let mut lines = contents.lines().enumerate();

    while let Some((line_index, line)) = lines.next() {
        let line = line.trim();
        let Some(label) = line
            .strip_prefix("-----BEGIN ")
            .and_then(|rest| rest.strip_suffix("-----"))
        else {
            continue;
        };

        let mut encoded = String::new();
        let end_marker = format!("-----END {label}-----");
        let mut found_end = false;

        for (_, body_line) in lines.by_ref() {
            let body_line = body_line.trim();
            if body_line == end_marker {
                found_end = true;
                break;
            }
            if !body_line.is_empty() {
                encoded.push_str(body_line);
            }
        }

        if !found_end {
            return Err(format!(
                "missing END marker for PEM block {label} starting at line {}",
                line_index + 1
            ));
        }

        let der = standard_decode(&encoded)
            .map_err(|error| format!("{label} block has invalid base64: {error}"))?;
        sections.push(PemSection {
            kind: match label {
                "CERTIFICATE" => PemSectionKind::Certificate,
                "X509 CRL" => PemSectionKind::Crl,
                _ => PemSectionKind::Other,
            },
            der,
        });
    }

    Ok(sections)
}

fn first_der_item(der: &[u8]) -> Option<&[u8]> {
    der_item_length(der).map(|length| &der[..length])
}

fn der_item_length(der: &[u8]) -> Option<usize> {
    let length_octet = *der.get(1)?;
    if length_octet & 0x80 == 0 {
        return Some(2 + usize::from(length_octet)).filter(|length| *length <= der.len());
    }
    let length_octets = usize::from(length_octet & 0x7f);
    if length_octets == 0 {
        return None;
    }
    let length_start = 2usize;
    let length_end = length_start.checked_add(length_octets)?;
    let mut content_length = 0usize;
    for &byte in der.get(length_start..length_end)? {
        content_length = content_length
            .checked_mul(256)?
            .checked_add(usize::from(byte))?;
    }
    length_end
        .checked_add(content_length)
        .filter(|length| *length <= der.len())
}
