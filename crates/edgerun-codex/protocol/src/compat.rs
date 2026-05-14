pub mod absolute_path {
    use std::borrow::Cow;
    use std::fmt;
    use std::ops::Deref;
    use std::path::Path;
    use std::path::PathBuf;

    use edgerun_json::FromJson;
    use edgerun_json::JsonValueError;
    use edgerun_json::ToJson;
    use edgerun_json::Value;
    use edgerun_serde::Deserialize;
    use edgerun_serde::Deserializer;
    use edgerun_serde::Serialize;
    use edgerun_serde::de::Error as _;
    use schemars::JsonSchema;
    use ts_rs::TS;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema, TS)]
    pub struct AbsolutePathBuf(PathBuf);

    impl AbsolutePathBuf {
        pub fn resolve_path_against_base<P: AsRef<Path>, B: AsRef<Path>>(
            path: P,
            base_path: B,
        ) -> Self {
            let path = path.as_ref();
            if path.is_absolute() {
                Self(normalize(path))
            } else {
                Self(normalize(&base_path.as_ref().join(path)))
            }
        }

        pub fn from_absolute_path<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
            let path = normalize(path.as_ref());
            if !path.is_absolute() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("path is not absolute: {}", path.display()),
                ));
            }
            Ok(Self(path))
        }

        pub fn current_dir() -> std::io::Result<Self> {
            Self::from_absolute_path(std::env::current_dir()?)
        }

        pub fn join<P: AsRef<Path>>(&self, path: P) -> Self {
            Self::resolve_path_against_base(path, &self.0)
        }

        pub fn parent(&self) -> Option<Self> {
            self.0.parent().map(|p| Self(p.to_path_buf()))
        }

        pub fn ancestors(&self) -> impl Iterator<Item = Self> + '_ {
            self.0.ancestors().map(|p| Self(p.to_path_buf()))
        }

        pub fn as_path(&self) -> &Path {
            &self.0
        }

        pub fn path(&self) -> &Path {
            &self.0
        }

        pub fn into_path_buf(self) -> PathBuf {
            self.0
        }

        pub fn to_path_buf(&self) -> PathBuf {
            self.0.clone()
        }

        pub fn to_string_lossy(&self) -> Cow<'_, str> {
            self.0.to_string_lossy()
        }

        pub fn display(&self) -> std::path::Display<'_> {
            self.0.display()
        }
    }

    impl AsRef<Path> for AbsolutePathBuf {
        fn as_ref(&self) -> &Path {
            self.as_path()
        }
    }

    impl Deref for AbsolutePathBuf {
        type Target = Path;

        fn deref(&self) -> &Self::Target {
            self.as_path()
        }
    }

    impl fmt::Display for AbsolutePathBuf {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0.display())
        }
    }

    impl TryFrom<PathBuf> for AbsolutePathBuf {
        type Error = std::io::Error;

        fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
            Self::from_absolute_path(value)
        }
    }

    impl TryFrom<&Path> for AbsolutePathBuf {
        type Error = std::io::Error;

        fn try_from(value: &Path) -> Result<Self, Self::Error> {
            Self::from_absolute_path(value)
        }
    }

    impl TryFrom<&str> for AbsolutePathBuf {
        type Error = std::io::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            Self::from_absolute_path(value)
        }
    }

    impl<'de> Deserialize<'de> for AbsolutePathBuf {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let path = PathBuf::deserialize(deserializer)?;
            Self::from_absolute_path(path).map_err(D::Error::custom)
        }
    }

    impl ToJson for AbsolutePathBuf {
        fn to_json(&self) -> Value {
            self.to_string_lossy().into_owned().to_json()
        }
    }

    impl FromJson for AbsolutePathBuf {
        fn from_json(value: Value) -> Result<Self, JsonValueError> {
            let path = String::from_json(value)?;
            Self::from_absolute_path(PathBuf::from(path))
                .map_err(|err| JsonValueError::WrongType(err.to_string()))
        }
    }

    pub fn canonicalize_preserving_symlinks(path: &Path) -> std::io::Result<PathBuf> {
        if path.exists() {
            std::fs::canonicalize(path)
        } else {
            Ok(AbsolutePathBuf::from_absolute_path(path)?.into_path_buf())
        }
    }

    fn normalize(path: &Path) -> PathBuf {
        let mut out = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    out.pop();
                }
                _ => out.push(component.as_os_str()),
            }
        }
        out
    }

    #[cfg(test)]
    pub mod test_support {
        use super::AbsolutePathBuf;
        use std::path::PathBuf;

        pub trait PathBufExt {
            fn abs(self) -> AbsolutePathBuf;
        }

        impl PathBufExt for PathBuf {
            fn abs(self) -> AbsolutePathBuf {
                AbsolutePathBuf::from_absolute_path(self).expect("absolute test path")
            }
        }

        pub fn test_path_buf(path: &str) -> PathBuf {
            PathBuf::from(path)
        }
    }
}

pub mod async_utils {
    #[derive(Debug)]
    pub struct CancelErr;
}

pub mod strings {
    pub fn approx_tokens_from_byte_count(bytes: usize) -> usize {
        bytes.div_ceil(4)
    }

    pub fn approx_bytes_for_tokens(tokens: usize) -> usize {
        tokens.saturating_mul(4)
    }

    pub fn truncate_middle_chars(content: &str, max_chars: usize) -> String {
        let char_count = content.chars().count();
        if char_count <= max_chars {
            return content.to_string();
        }
        if max_chars <= 1 {
            return "…".to_string();
        }
        let left = max_chars / 2;
        let right = max_chars.saturating_sub(left + 1);
        let prefix: String = content.chars().take(left).collect();
        let suffix: String = content
            .chars()
            .rev()
            .take(right)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{prefix}…{suffix}")
    }

    pub fn truncate_middle_with_token_budget(content: &str, max_tokens: usize) -> (String, usize) {
        let max_chars = approx_bytes_for_tokens(max_tokens);
        (truncate_middle_chars(content, max_chars), max_tokens)
    }
}

pub mod image {
    use std::path::Path;
    use std::path::PathBuf;

    use edgerun_encoding::base64::standard_encode;
    use edgerun_error::Error;

    #[derive(Debug, Clone)]
    pub struct EncodedImage {
        pub bytes: Vec<u8>,
        pub mime: String,
    }

    impl EncodedImage {
        pub fn into_data_url(self) -> String {
            format!("data:{};base64,{}", self.mime, standard_encode(&self.bytes))
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum PromptImageMode {
        ResizeToFit,
        Original,
    }

    #[derive(Debug, Error)]
    pub enum ImageProcessingError {
        #[error("failed to read image at {path}: {source}")]
        Read {
            path: PathBuf,
            #[source]
            source: std::io::Error,
        },
        #[error("failed to decode image at {path}: {source}")]
        Decode {
            path: PathBuf,
            #[source]
            source: std::io::Error,
        },
        #[error("failed to encode image: {source}")]
        Encode {
            #[source]
            source: std::io::Error,
        },
        #[error("unsupported image `{mime}`")]
        UnsupportedImageFormat { mime: String },
    }

    impl ImageProcessingError {
        pub fn is_invalid_image(&self) -> bool {
            matches!(self, ImageProcessingError::Decode { .. })
        }
    }

    pub fn load_for_prompt_bytes(
        path: &Path,
        file_bytes: Vec<u8>,
        _mode: PromptImageMode,
    ) -> Result<EncodedImage, ImageProcessingError> {
        let mime = match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("webp") => "image/webp",
            Some("gif") => "image/gif",
            _ => {
                return Err(ImageProcessingError::UnsupportedImageFormat {
                    mime: "unknown".to_string(),
                });
            }
        };
        Ok(EncodedImage {
            bytes: file_bytes,
            mime: mime.to_string(),
        })
    }
}

pub mod network_proxy {
    use edgerun_serde::Deserialize;

    #[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum NetworkDecisionSource {
        Decider,
        User,
        Config,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum NetworkPolicyDecision {
        Allow,
        Deny,
        Ask,
    }
}
