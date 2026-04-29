use crate::prelude::*;

use super::errors::RegistryError;
use super::image_ref::ImageRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageTrustPolicy {
    RequireDigestReference,
    AllowTagReference,
}

impl Default for ImageTrustPolicy {
    fn default() -> Self {
        Self::RequireDigestReference
    }
}

impl ImageTrustPolicy {
    pub fn enforce(self, image: &ImageRef) -> Result<(), RegistryError> {
        match self {
            Self::RequireDigestReference if !image.is_digest_reference() => {
                Err(RegistryError::TrustPolicy(format!(
                    "refusing to pull mutable tag reference {image}; use repository@sha256:<digest> or explicitly allow unverified tag pulls"
                )))
            }
            Self::RequireDigestReference | Self::AllowTagReference => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_rejects_tag_references() {
        let image: ImageRef = "alpine:latest".parse().unwrap();

        let error = ImageTrustPolicy::RequireDigestReference
            .enforce(&image)
            .unwrap_err();

        assert!(matches!(error, RegistryError::TrustPolicy(_)));
        assert!(error.to_string().contains("mutable tag reference"));
    }

    #[test]
    fn default_policy_accepts_digest_references() {
        let image: ImageRef =
            "alpine@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .parse()
                .unwrap();

        assert!(ImageTrustPolicy::RequireDigestReference
            .enforce(&image)
            .is_ok());
    }

    #[test]
    fn explicit_policy_allows_tag_references() {
        let image: ImageRef = "alpine:latest".parse().unwrap();

        assert!(ImageTrustPolicy::AllowTagReference.enforce(&image).is_ok());
    }
}
