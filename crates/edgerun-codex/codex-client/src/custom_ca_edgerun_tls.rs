use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::rustls_provider::ensure_rustls_crypto_provider;
use edgerun_error::Error;
use edgerun_rustls_pki_types::CertificateDer;
use edgerun_rustls_pki_types::pem::{self, PemObject, SectionKind};
use edgerun_rusttls::{ClientConfig, RootCertStore};
use tracing::{info, warn};

pub const CODEX_CA_CERT_ENV: &str = "CODEX_CA_CERTIFICATE";
pub const SSL_CERT_FILE_ENV: &str = "SSL_CERT_FILE";
const CA_CERT_HINT: &str = "If you set CODEX_CA_CERTIFICATE or SSL_CERT_FILE, ensure it points to a PEM file containing one or more CERTIFICATE blocks, or unset it to use system roots.";
type PemSection = (SectionKind, Vec<u8>);

#[derive(Debug, Error)]
pub enum BuildCustomCaTransportError {
    #[error("Failed to read CA certificate file {} selected by {}: {source}. {hint}", path.display(), source_env, hint = CA_CERT_HINT)]
    ReadCaFile { source_env: &'static str, path: PathBuf, source: io::Error },
    #[error("Failed to load CA certificates from {} selected by {}: {detail}. {hint}", path.display(), source_env, hint = CA_CERT_HINT)]
    InvalidCaFile { source_env: &'static str, path: PathBuf, detail: String },
    #[error("Failed to parse certificate #{certificate_index} from {} selected by {}: {source}. {hint}", path.display(), source_env, hint = CA_CERT_HINT)]
    RegisterCertificate { source_env: &'static str, path: PathBuf, certificate_index: usize, source: edgerun_reqwest::Error },
    #[error("Failed to build HTTP client while using CA bundle from {} ({}): {source}", source_env, path.display())]
    BuildClientWithCustomCa { source_env: &'static str, path: PathBuf, #[source] source: edgerun_reqwest::Error },
    #[error("Failed to build HTTP client while using system root certificates: {0}")]
    BuildClientWithSystemRoots(#[source] edgerun_reqwest::Error),
    #[error("Failed to register certificate #{certificate_index} from {} selected by {} in Edgerun TLS root store: {source}. {hint}", path.display(), source_env, hint = CA_CERT_HINT)]
    RegisterTlsCertificate { source_env: &'static str, path: PathBuf, certificate_index: usize, source: edgerun_rusttls::Error },
}

impl From<BuildCustomCaTransportError> for io::Error {
    fn from(error: BuildCustomCaTransportError) -> Self {
        match error {
            BuildCustomCaTransportError::ReadCaFile { ref source, .. } => io::Error::new(source.kind(), error),
            BuildCustomCaTransportError::InvalidCaFile { .. }
            | BuildCustomCaTransportError::RegisterCertificate { .. }
            | BuildCustomCaTransportError::RegisterTlsCertificate { .. } => io::Error::new(io::ErrorKind::InvalidData, error),
            BuildCustomCaTransportError::BuildClientWithCustomCa { .. }
            | BuildCustomCaTransportError::BuildClientWithSystemRoots(_) => io::Error::other(error),
        }
    }
}

pub fn build_reqwest_client_with_custom_ca(builder: edgerun_reqwest::ClientBuilder) -> Result<edgerun_reqwest::Client, BuildCustomCaTransportError> {
    build_reqwest_client_with_env(&ProcessEnv, builder)
}

pub fn build_reqwest_client_for_subprocess_tests(builder: edgerun_reqwest::ClientBuilder) -> Result<edgerun_reqwest::Client, BuildCustomCaTransportError> {
    build_reqwest_client_with_env(&ProcessEnv, builder.no_proxy())
}

pub fn maybe_build_edgerun_tls_client_config_with_custom_ca() -> Result<Option<Arc<ClientConfig>>, BuildCustomCaTransportError> {
    maybe_build_edgerun_tls_client_config_with_env(&ProcessEnv)
}

pub fn maybe_build_rustls_client_config_with_custom_ca() -> Result<Option<Arc<ClientConfig>>, BuildCustomCaTransportError> {
    maybe_build_edgerun_tls_client_config_with_custom_ca()
}

fn maybe_build_edgerun_tls_client_config_with_env(env_source: &dyn EnvSource) -> Result<Option<Arc<ClientConfig>>, BuildCustomCaTransportError> {
    let Some(bundle) = env_source.configured_ca_bundle() else { return Ok(None); };
    ensure_rustls_crypto_provider();

    let mut root_store = RootCertStore::empty();
    let edgerun_rustls_native_certs::CertificateResult { certs, errors, .. } = edgerun_rustls_native_certs::load_native_certs();
    if !errors.is_empty() {
        warn!(native_root_error_count = errors.len(), "encountered errors while loading native root certificates");
    }
    let _ = root_store.add_parsable_certificates(certs);

    for (idx, cert) in bundle.load_certificates()?.into_iter().enumerate() {
        root_store.add(cert).map_err(|source| BuildCustomCaTransportError::RegisterTlsCertificate {
            source_env: bundle.source_env,
            path: bundle.path.clone(),
            certificate_index: idx + 1,
            source,
        })?;
    }

    Ok(Some(Arc::new(ClientConfig::builder().with_root_certificates(root_store).with_no_client_auth())))
}

fn build_reqwest_client_with_env(env_source: &dyn EnvSource, mut builder: edgerun_reqwest::ClientBuilder) -> Result<edgerun_reqwest::Client, BuildCustomCaTransportError> {
    let Some(bundle) = env_source.configured_ca_bundle() else {
        return builder.build().map_err(BuildCustomCaTransportError::BuildClientWithSystemRoots);
    };

    ensure_rustls_crypto_provider();
    info!(source_env = bundle.source_env, ca_path = %bundle.path.display(), "building HTTP client with Edgerun TLS backend for custom CA bundle");
    builder = builder.use_rustls_tls();

    for (idx, cert) in bundle.load_certificates()?.iter().enumerate() {
        let cert = edgerun_reqwest::Certificate::from_der(cert.as_ref()).map_err(|source| BuildCustomCaTransportError::RegisterCertificate {
            source_env: bundle.source_env,
            path: bundle.path.clone(),
            certificate_index: idx + 1,
            source,
        })?;
        builder = builder.add_root_certificate(cert);
    }

    builder.build().map_err(|source| BuildCustomCaTransportError::BuildClientWithCustomCa {
        source_env: bundle.source_env,
        path: bundle.path,
        source,
    })
}

trait EnvSource {
    fn var(&self, key: &str) -> Option<String>;
    fn non_empty_path(&self, key: &str) -> Option<PathBuf> {
        self.var(key).filter(|value| !value.is_empty()).map(PathBuf::from)
    }
    fn configured_ca_bundle(&self) -> Option<ConfiguredCaBundle> {
        self.non_empty_path(CODEX_CA_CERT_ENV)
            .map(|path| ConfiguredCaBundle { source_env: CODEX_CA_CERT_ENV, path })
            .or_else(|| self.non_empty_path(SSL_CERT_FILE_ENV).map(|path| ConfiguredCaBundle { source_env: SSL_CERT_FILE_ENV, path }))
    }
}

struct ProcessEnv;
impl EnvSource for ProcessEnv {
    fn var(&self, key: &str) -> Option<String> { env::var(key).ok() }
}

struct ConfiguredCaBundle {
    source_env: &'static str,
    path: PathBuf,
}

impl ConfiguredCaBundle {
    fn load_certificates(&self) -> Result<Vec<CertificateDer<'static>>, BuildCustomCaTransportError> {
        let pem_data = fs::read(&self.path).map_err(|source| BuildCustomCaTransportError::ReadCaFile {
            source_env: self.source_env,
            path: self.path.clone(),
            source,
        })?;
        let normalized = NormalizedPem::from_pem_data(self.source_env, &self.path, &pem_data);
        let mut certificates = Vec::new();
        let mut logged_crl = false;
        for section in normalized.sections() {
            let (kind, der) = section.map_err(|error| self.pem_parse_error(&error))?;
            match kind {
                SectionKind::Certificate => {
                    let cert = normalized.certificate_der(&der).ok_or_else(|| self.invalid_ca_file("invalid TRUSTED CERTIFICATE DER length"))?;
                    certificates.push(CertificateDer::from(cert.to_vec()));
                }
                SectionKind::Crl if !logged_crl => {
                    info!(source_env = self.source_env, ca_path = %self.path.display(), "ignoring X509 CRL entries found in custom CA bundle");
                    logged_crl = true;
                }
                _ => {}
            }
        }
        if certificates.is_empty() {
            return Err(self.pem_parse_error(&pem::Error::NoItemsFound));
        }
        Ok(certificates)
    }

    fn pem_parse_error(&self, error: &pem::Error) -> BuildCustomCaTransportError {
        let detail = match error {
            pem::Error::NoItemsFound => "no certificates found in PEM file".to_string(),
            _ => format!("failed to parse PEM file: {error}"),
        };
        self.invalid_ca_file(detail)
    }

    fn invalid_ca_file(&self, detail: impl std::fmt::Display) -> BuildCustomCaTransportError {
        BuildCustomCaTransportError::InvalidCaFile { source_env: self.source_env, path: self.path.clone(), detail: detail.to_string() }
    }
}

enum NormalizedPem {
    Standard(String),
    TrustedCertificate(String),
}

impl NormalizedPem {
    fn from_pem_data(source_env: &'static str, path: &Path, pem_data: &[u8]) -> Self {
        let pem = String::from_utf8_lossy(pem_data);
        if pem.contains("TRUSTED CERTIFICATE") {
            info!(source_env, ca_path = %path.display(), "normalizing OpenSSL TRUSTED CERTIFICATE labels in custom CA bundle");
            Self::TrustedCertificate(pem.replace("BEGIN TRUSTED CERTIFICATE", "BEGIN CERTIFICATE").replace("END TRUSTED CERTIFICATE", "END CERTIFICATE"))
        } else {
            Self::Standard(pem.into_owned())
        }
    }

    fn contents(&self) -> &str {
        match self { Self::Standard(contents) | Self::TrustedCertificate(contents) => contents }
    }

    fn sections(&self) -> impl Iterator<Item = Result<PemSection, pem::Error>> + '_ {
        PemSection::pem_slice_iter(self.contents().as_bytes())
    }

    fn certificate_der<'a>(&self, der: &'a [u8]) -> Option<&'a [u8]> {
        match self { Self::Standard(_) => Some(der), Self::TrustedCertificate(_) => first_der_item(der) }
    }
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
    if length_octets == 0 { return None; }
    let length_start = 2usize;
    let length_end = length_start.checked_add(length_octets)?;
    let mut content_length = 0usize;
    for &byte in der.get(length_start..length_end)? {
        content_length = content_length.checked_mul(256)?.checked_add(usize::from(byte))?;
    }
    length_end.checked_add(content_length).filter(|length| *length <= der.len())
}
