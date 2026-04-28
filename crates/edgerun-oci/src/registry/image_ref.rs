use crate::prelude::*;
use core::{fmt, str::FromStr};

use crate::validate_digest_reference;

/// An OCI image reference (e.g., `docker.io/library/alpine:latest`).
#[derive(Clone, Debug)]
pub struct ImageRef {
    pub registry: String,
    pub repository: String,
    pub tag: String,
}

impl FromStr for ImageRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("image reference is empty".into());
        }
        let mut registry = "docker.io".to_string();
        let mut rest = s;

        if let Some((prefix, remaining)) = s.split_once('/') {
            if prefix.contains('.') || prefix.contains(':') || prefix == "localhost" {
                registry = prefix.to_string();
                rest = remaining;
            }
        }

        let (mut repository, tag) = if let Some((repo, digest)) = rest.rsplit_once('@') {
            if repo.is_empty() || digest.is_empty() {
                return Err("digest image references must be repository@algorithm:hex".into());
            }
            if !validate_digest_reference(digest) {
                return Err("digest image references must use algorithm:hex digest syntax".into());
            }
            let repository = split_repository_tag(repo)
                .map(|(repository, _tag)| repository)
                .unwrap_or(repo);
            (repository.to_string(), digest.to_string())
        } else if let Some((repo, tag)) = split_repository_tag(rest) {
            if repo.is_empty() || tag.is_empty() {
                return Err("tag image references must be repository:tag".into());
            }
            if !valid_tag(tag) {
                return Err("image tag contains invalid characters".into());
            }
            (repo.to_string(), tag.to_string())
        } else {
            (rest.to_string(), "latest".to_string())
        };

        if !valid_registry(&registry) {
            return Err("image registry contains invalid characters".into());
        }
        if !valid_repository(&repository) {
            return Err("image repository contains invalid characters".into());
        }
        if registry == "docker.io" && !repository.contains('/') {
            repository = format!("library/{}", repository);
        }

        Ok(ImageRef {
            registry,
            repository,
            tag,
        })
    }
}

impl ImageRef {
    pub fn reference(&self) -> &str {
        &self.tag
    }

    pub fn is_digest_reference(&self) -> bool {
        validate_digest_reference(&self.tag)
    }
}

impl fmt::Display for ImageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let separator = if self.is_digest_reference() { '@' } else { ':' };
        write!(
            f,
            "{}/{}{}{}",
            self.registry, self.repository, separator, self.tag
        )
    }
}

fn split_repository_tag(reference: &str) -> Option<(&str, &str)> {
    let tag_separator = reference.rfind(':')?;
    let last_path_separator = reference.rfind('/').unwrap_or(0);
    if tag_separator <= last_path_separator {
        return None;
    }
    Some((&reference[..tag_separator], &reference[tag_separator + 1..]))
}

fn valid_registry(registry: &str) -> bool {
    !registry.is_empty()
        && !registry.starts_with('.')
        && !registry.ends_with('.')
        && registry.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b':' | b'[' | b']')
        })
}

fn valid_repository(repository: &str) -> bool {
    !repository.is_empty()
        && !repository.starts_with('/')
        && !repository.ends_with('/')
        && repository.split('/').all(valid_repository_component)
}

fn valid_repository_component(component: &str) -> bool {
    !component.is_empty()
        && component.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 128
        && tag
            .bytes()
            .next()
            .map(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            .unwrap_or(false)
        && tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}
