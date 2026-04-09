//! X.509 certificate parsing and validation

/// X.509 certificate
#[derive(Debug, Clone)]
pub struct Certificate {
    /// Raw certificate data (DER encoded)
    pub data: Vec<u8>,
    /// Certificate version
    pub version: u32,
    /// Serial number
    pub serial_number: Vec<u8>,
    /// Issuer (DER encoded)
    pub issuer: Vec<u8>,
    /// Subject (DER encoded)
    pub subject: Vec<u8>,
    /// Not valid before (Unix timestamp)
    pub not_before: u64,
    /// Not valid after (Unix timestamp)
    pub not_after: u64,
    /// Subject Public Key Info
    pub public_key_info: Vec<u8>,
    /// Signature algorithm
    pub signature_algorithm: Vec<u8>,
    /// Signature value
    pub signature: Vec<u8>,
}

impl Certificate {
    /// Parse a certificate from DER-encoded bytes
    pub fn from_der(data: &[u8]) -> Result<Self, String> {
        if data.is_empty() {
            return Err("Empty certificate".to_string());
        }

        // Minimal parsing - extract basic fields
        // A full implementation would parse ASN.1/DER properly

        let cert = Certificate {
            data: data.to_vec(),
            version: 3, // v3 is most common
            serial_number: Vec::new(),
            issuer: Vec::new(),
            subject: Vec::new(),
            not_before: 0,
            not_after: u64::MAX,
            public_key_info: Vec::new(),
            signature_algorithm: Vec::new(),
            signature: Vec::new(),
        };

        // Validate it looks like a certificate
        // ASN.1 SEQUENCE tag is 0x30
        if data[0] != 0x30 {
            return Err("Invalid certificate format".to_string());
        }

        Ok(cert)
    }

    /// Parse certificate from PEM format
    pub fn from_pem(pem: &str) -> Result<Self, String> {
        // Find certificate boundaries
        let start = pem
            .find("-----BEGIN CERTIFICATE-----")
            .ok_or("No BEGIN CERTIFICATE marker found")?;

        let end = pem
            .find("-----END CERTIFICATE-----")
            .ok_or("No END CERTIFICATE marker found")?;

        // Extract base64 data
        let b64_data = &pem[start + 27..end];

        // Decode base64
        let der_data = Self::base64_decode(b64_data)?;

        Certificate::from_der(&der_data)
    }

    /// Parse multiple certificates from PEM
    pub fn parse_all(data: &[u8]) -> Result<Vec<Self>, String> {
        let pem_str = String::from_utf8_lossy(data);
        let mut certs = Vec::new();

        // Split on BEGIN CERTIFICATE markers
        let parts: Vec<&str> = pem_str.split("-----BEGIN CERTIFICATE-----").collect();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            let full_pem = format!("-----BEGIN CERTIFICATE-----{}", part);
            if let Ok(cert) = Certificate::from_pem(&full_pem) {
                certs.push(cert);
            }
        }

        Ok(certs)
    }

    /// Get the Common Name from subject
    pub fn common_name(&self) -> Option<String> {
        // In a full implementation, parse the subject DN and extract CN
        // This is a placeholder
        None
    }

    /// Check if certificate is valid at a given time
    pub fn is_valid_at(&self, timestamp: u64) -> bool {
        timestamp >= self.not_before && timestamp <= self.not_after
    }

    /// Check if certificate is currently valid
    pub fn is_valid_now(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.is_valid_at(now)
    }

    /// Verify certificate signature
    pub fn verify_signature(&self) -> Result<(), String> {
        // In a full implementation:
        // 1. Extract issuer's public key from CA certificate
        // 2. Verify signature over TBSCertificate
        // 3. Check certificate chain
        Ok(()) // Placeholder
    }

    /// Simple base64 decoder (std-only implementation)
    fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
        let b64_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = Vec::new();

        // Remove whitespace
        let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();

        // Process 4 characters at a time
        let mut i = 0;
        while i < cleaned.len() {
            let mut sextets = [0u8; 4];
            let mut padding = 0;

            for j in 0..4 {
                if i + j >= cleaned.len() {
                    return Err("Invalid base64 length".to_string());
                }

                if cleaned.as_bytes()[i + j] == b'=' {
                    padding += 1;
                    continue;
                }

                let c = cleaned.chars().nth(i + j).unwrap();
                let val = b64_chars.find(c).ok_or("Invalid base64 character")? as u8;
                sextets[j] = val;
            }

            // Combine sextets into bytes
            if padding == 0 {
                result.push((sextets[0] << 2) | (sextets[1] >> 4));
                result.push((sextets[1] << 4) | (sextets[2] >> 2));
                result.push((sextets[2] << 6) | sextets[3]);
            } else if padding == 1 {
                result.push((sextets[0] << 2) | (sextets[1] >> 4));
                result.push((sextets[1] << 4) | (sextets[2] >> 2));
            } else if padding == 2 {
                result.push((sextets[0] << 2) | (sextets[1] >> 4));
            }

            i += 4;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_decode() {
        // Test simple base64 decoding
        let decoded = Certificate::base64_decode("SGVsbG8=").unwrap();
        assert_eq!(decoded, b"Hello");

        let decoded = Certificate::base64_decode("SGVsbG8gV29ybGQ=").unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_base64_decode_invalid() {
        assert!(Certificate::base64_decode("Invalid!!!").is_err());
    }

    #[test]
    fn test_certificate_from_der_invalid() {
        // Empty data should fail
        assert!(Certificate::from_der(&[]).is_err());

        // Invalid ASN.1 should fail
        assert!(Certificate::from_der(&[0x00, 0x01, 0x02]).is_err());
    }

    #[test]
    fn test_certificate_from_pem_invalid() {
        assert!(Certificate::from_pem("not a pem").is_err());
    }

    #[test]
    fn test_certificate_parse_multiple() {
        // Test with empty data
        let certs = Certificate::parse_all(b"").unwrap();
        assert!(certs.is_empty());
    }
}
