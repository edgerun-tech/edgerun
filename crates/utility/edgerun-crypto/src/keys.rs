//! EdgeRun-owned key wrapper types.

#[cfg(feature = "ed25519")]
#[derive(Clone)]
pub struct Ed25519SigningKey {
    inner: crate::ed25519_dalek::SigningKey,
}

#[cfg(feature = "ed25519")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ed25519VerifyingKey {
    inner: crate::ed25519_dalek::VerifyingKey,
}

#[cfg(feature = "ed25519")]
impl Ed25519SigningKey {
    pub fn from_bytes(seed: &[u8; 32]) -> Self {
        Self {
            inner: crate::ed25519_dalek::SigningKey::from_bytes(seed),
        }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        self.inner.as_bytes()
    }

    pub fn to_scalar_bytes(&self) -> [u8; 32] {
        self.inner.to_scalar_bytes()
    }

    pub fn verifying_key(&self) -> Ed25519VerifyingKey {
        Ed25519VerifyingKey {
            inner: self.inner.verifying_key(),
        }
    }

    pub fn sign(&self, message: &[u8]) -> crate::ed25519::Signature {
        use crate::signature::Signer;

        self.inner.sign(message)
    }

    pub fn sign_bytes(&self, message: &[u8]) -> [u8; 64] {
        self.sign(message).to_bytes()
    }

    pub(crate) fn as_dalek(&self) -> &crate::ed25519_dalek::SigningKey {
        &self.inner
    }
}

#[cfg(feature = "ed25519")]
impl core::fmt::Debug for Ed25519SigningKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Ed25519SigningKey")
            .field("verifying_key", &self.verifying_key())
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "ed25519")]
impl Ed25519VerifyingKey {
    pub fn from_bytes(bytes: &[u8; 32]) -> crate::signature::Result<Self> {
        crate::ed25519_dalek::VerifyingKey::from_bytes(bytes).map(|inner| Self { inner })
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        self.inner.as_bytes()
    }

    pub fn verify(&self, message: &[u8], signature: &[u8]) -> crate::Result<()> {
        crate::verification::ed25519_verify(self.as_bytes(), message, signature)
    }

    pub fn verify_strict(&self, message: &[u8], signature: &[u8]) -> crate::Result<()> {
        crate::verification::ed25519_verify_strict(self.as_bytes(), message, signature)
    }

    pub fn to_montgomery_bytes(&self) -> [u8; 32] {
        self.inner.to_montgomery().to_bytes()
    }

    pub(crate) fn as_dalek(&self) -> &crate::ed25519_dalek::VerifyingKey {
        &self.inner
    }
}

#[cfg(feature = "ed25519")]
impl crate::signature::Signer<crate::ed25519::Signature> for Ed25519SigningKey {
    fn try_sign(
        &self,
        msg: &[u8],
    ) -> core::result::Result<crate::ed25519::Signature, crate::signature::Error> {
        use crate::signature::Signer;

        self.inner.try_sign(msg)
    }
}

#[cfg(feature = "ed25519")]
impl crate::signature::Verifier<crate::ed25519::Signature> for Ed25519VerifyingKey {
    fn verify(
        &self,
        msg: &[u8],
        signature: &crate::ed25519::Signature,
    ) -> core::result::Result<(), crate::signature::Error> {
        use crate::signature::Verifier;

        self.inner.verify(msg, signature)
    }
}

#[cfg(feature = "p256")]
#[derive(Clone)]
pub struct P256SigningKey {
    inner: crate::p256::ecdsa::SigningKey,
}

#[cfg(feature = "p256")]
#[derive(Clone, Debug)]
pub struct P256VerifyingKey {
    inner: crate::p256::ecdsa::VerifyingKey,
}

#[cfg(feature = "p256")]
impl P256SigningKey {
    pub fn random() -> Self {
        loop {
            let mut bytes = [0u8; 32];
            crate::fill_random(&mut bytes)
                .expect("edgerun RNG should always provide fallback bytes");
            if let Ok(key) = Self::from_bytes(&bytes) {
                return key;
            }
        }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> crate::Result<Self> {
        crate::p256::ecdsa::SigningKey::from_bytes(bytes.into())
            .map(|inner| Self { inner })
            .map_err(|_| crate::CryptoError::InvalidKey)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes().into()
    }

    pub fn verifying_key(&self) -> P256VerifyingKey {
        P256VerifyingKey {
            inner: *self.inner.verifying_key(),
        }
    }

    pub fn public_key_sec1(&self) -> alloc::vec::Vec<u8> {
        self.verifying_key().to_sec1_bytes()
    }

    pub fn sign_sha256_fixed(&self, message: &[u8]) -> crate::Result<[u8; 64]> {
        crate::signing::p256_sign_sha256_fixed(self, message)
    }

    pub fn sign_prehash_fixed(&self, prehash: &[u8]) -> crate::Result<[u8; 64]> {
        crate::signing::p256_sign_prehash_fixed(self, prehash)
    }

    pub fn sign_sha256_der(&self, message: &[u8]) -> crate::Result<alloc::vec::Vec<u8>> {
        crate::signing::p256_sign_sha256_der(self, message)
    }

    pub fn sign_prehash_der(&self, prehash: &[u8]) -> crate::Result<alloc::vec::Vec<u8>> {
        crate::signing::p256_sign_prehash_der(self, prehash)
    }

    pub(crate) fn as_raw(&self) -> &crate::p256::ecdsa::SigningKey {
        &self.inner
    }
}

#[cfg(feature = "p256")]
impl core::fmt::Debug for P256SigningKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("P256SigningKey")
            .field("verifying_key", &self.verifying_key())
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "p256")]
impl P256VerifyingKey {
    pub fn from_sec1_bytes(bytes: &[u8]) -> crate::Result<Self> {
        crate::p256::ecdsa::VerifyingKey::from_sec1_bytes(bytes)
            .map(|inner| Self { inner })
            .map_err(|_| crate::CryptoError::InvalidKey)
    }

    pub fn from_public_key_pem(pem: &str) -> crate::Result<Self> {
        use crate::spki::DecodePublicKey;

        let (label, der) = crate::pem_rfc7468::decode_vec(pem.as_bytes())
            .map_err(|_| crate::CryptoError::InvalidKey)?;
        if label != "PUBLIC KEY" {
            return Err(crate::CryptoError::InvalidKey);
        }
        crate::p256::ecdsa::VerifyingKey::from_public_key_der(&der)
            .map(|inner| Self { inner })
            .map_err(|_| crate::CryptoError::InvalidKey)
    }

    pub fn to_sec1_bytes(&self) -> alloc::vec::Vec<u8> {
        use crate::p256::elliptic_curve::sec1::ToEncodedPoint;

        self.inner.to_encoded_point(false).as_bytes().to_vec()
    }

    pub fn verify_sha256_fixed(&self, message: &[u8], signature: &[u8]) -> crate::Result<()> {
        crate::verification::p256_verify_sha256_fixed(&self.to_sec1_bytes(), message, signature)
    }

    pub fn verify_prehash_fixed(&self, prehash: &[u8], signature: &[u8]) -> crate::Result<()> {
        crate::verification::p256_verify_prehash_fixed(&self.to_sec1_bytes(), prehash, signature)
    }

    pub fn verify_sha256_der(&self, message: &[u8], signature: &[u8]) -> crate::Result<()> {
        crate::verification::p256_verify_sha256_der(&self.to_sec1_bytes(), message, signature)
    }

    pub fn verify_prehash_der(&self, prehash: &[u8], signature: &[u8]) -> crate::Result<()> {
        crate::verification::p256_verify_prehash_der(&self.to_sec1_bytes(), prehash, signature)
    }

    pub(crate) fn as_raw(&self) -> &crate::p256::ecdsa::VerifyingKey {
        &self.inner
    }
}
