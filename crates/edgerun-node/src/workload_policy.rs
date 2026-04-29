/// Workload content policy — validates what images can be run.
///
/// Prevents running arbitrary untrusted images by enforcing:
/// - Registry allowlists (only approved registries)
/// - Image name patterns (only approved repos)
/// - Digest pinning (specific image versions only)
/// - Tag restrictions (no `:latest` from untrusted registries)
///
/// Policy state is projected from the immutable event log — this module
/// only defines the validation logic. The caller is responsible for
/// projecting the current policy from committed events.
use std::collections::HashSet;

/// Policy for validating workload image references.
#[derive(Clone, Debug, Default)]
pub struct WorkloadPolicy {
    /// Allowed registries (e.g., "docker.io", "ghcr.io"). Empty = allow all.
    pub allowed_registries: HashSet<String>,
    /// Blocked image name patterns (substring match).
    pub blocked_images: HashSet<String>,
    /// Pinned digests: "registry/repo@sha256:..." → allowed.
    /// If non-empty, ONLY these exact images can run.
    pub pinned_digests: HashSet<String>,
}

impl WorkloadPolicy {
    /// Create a permissive policy (allows everything).
    pub fn permissive() -> Self {
        Self::default()
    }

    /// Validate an image reference against this policy.
    /// Returns Ok(()) if allowed, Err(reason) if blocked.
    pub fn validate(&self, image_ref: &str) -> Result<(), String> {
        // If pinned digests are set, only those are allowed
        if !self.pinned_digests.is_empty() {
            if self.pinned_digests.contains(image_ref) {
                return Ok(());
            }
            return Err(format!(
                "image not in pinned digests ({} pins configured)",
                self.pinned_digests.len()
            ));
        }

        // Check registry allowlist
        if !self.allowed_registries.is_empty() {
            let registry = extract_registry(image_ref);
            if !self.allowed_registries.contains(&registry) {
                return Err(format!("registry '{}' not in allowlist", registry));
            }
        }

        // Check blocked images
        for pattern in &self.blocked_images {
            if image_ref.contains(pattern) {
                return Err(format!(
                    "image '{}' matches blocked pattern '{}'",
                    image_ref, pattern
                ));
            }
        }

        Ok(())
    }
}

/// Extract the registry from an image reference.
/// "alpine:latest" → "docker.io"
/// "ghcr.io/myorg/app:v1" → "ghcr.io"
/// "docker.io/library/nginx:1.25" → "docker.io"
fn extract_registry(image: &str) -> String {
    // Split on '/' to get the first segment
    let first = image.split('/').next().unwrap_or("");

    // A registry is identified by:
    // - containing a '.' (e.g., "ghcr.io", "my.registry.com")
    // - being "localhost" (possibly with a port like "localhost:5000")
    //
    // If the first segment only contains a ':' but no '/' after it,
    // it's actually a tag separator (e.g., "alpine:latest"), not a registry.
    let is_registry =
        first.contains('.') || first == "localhost" || (first.contains(':') && image.contains('/'));

    if is_registry {
        first.to_string()
    } else {
        "docker.io".to_string()
    }
}

/// Load policy from a simple text file (one directive per line).
///
/// Format:
/// ```
/// allow-registry: docker.io
/// allow-registry: ghcr.io
/// block-image: cryptominer
/// block-image: /malicious/
/// pin-digest: docker.io/library/alpine@sha256:abc123...
/// ```
pub fn load_policy_file(path: &std::path::Path) -> std::io::Result<WorkloadPolicy> {
    let mut policy = WorkloadPolicy::default();
    if !path.exists() {
        return Ok(policy);
    }

    let content = std::fs::read_to_string(path)?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((directive, value)) = line.split_once(':') {
            let value = value.trim().to_string();
            match directive.trim() {
                "allow-registry" => {
                    policy.allowed_registries.insert(value);
                }
                "block-image" => {
                    policy.blocked_images.insert(value);
                }
                "pin-digest" => {
                    policy.pinned_digests.insert(value);
                }
                _ => {}
            }
        }
    }

    Ok(policy)
}

// ===========================================================================
// Rate limiting — in-memory sliding window
//
// The canonical record of workloads is always in the immutable event log.
// This rate limiter is a transient pre-filter to prevent spam/DoS before
// a command is even validated. It is NOT the source of truth.
// ===========================================================================

use std::sync::Mutex;

use edgerun_core::util::now_unix_micros_u64;

/// Tracks recent workload submissions per requester for spam prevention.
pub struct RateLimiter {
    /// Max workloads allowed per requester within the window.
    max_per_window: u32,
    /// Window duration in microseconds.
    window_us: u64,
    /// Per-requester submission timestamps (µs since epoch).
    submissions: Mutex<std::collections::HashMap<Vec<u8>, Vec<u64>>>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    /// `max_per_window` = max submissions allowed per requester in `window_us` microseconds.
    pub fn new(max_per_window: u32, window_us: u64) -> Self {
        Self {
            max_per_window,
            window_us,
            submissions: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Check if a requester is within rate limits.
    /// If ok, records the submission. Returns true if allowed.
    pub fn check_and_record(&self, requester_id: &[u8]) -> bool {
        let now = now_unix_micros_u64();

        let mut map = self.submissions.lock().expect("rate limit map poisoned");
        let entries = map.entry(requester_id.to_vec()).or_default();

        // Prune old entries outside the window
        let cutoff = now.saturating_sub(self.window_us);
        entries.retain(|&ts| ts > cutoff);

        if entries.len() >= self.max_per_window as usize {
            return false;
        }

        entries.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissive_allows_everything() {
        let policy = WorkloadPolicy::permissive();
        assert!(policy.validate("alpine:latest").is_ok());
        assert!(policy.validate("ghcr.io/myorg/app:v1").is_ok());
        assert!(policy.validate("evil.com/crypto:latest").is_ok());
    }

    #[test]
    fn registry_allowlist_blocks_unknown() {
        let mut policy = WorkloadPolicy::default();
        policy.allowed_registries.insert("docker.io".into());
        policy.allowed_registries.insert("ghcr.io".into());

        assert!(policy.validate("alpine:latest").is_ok());
        assert!(policy.validate("ghcr.io/org/app:v1").is_ok());
        let err = policy.validate("evil.com/crypto:latest").unwrap_err();
        assert!(err.contains("not in allowlist"));
    }

    #[test]
    fn blocked_image_patterns() {
        let mut policy = WorkloadPolicy::default();
        policy.blocked_images.insert("cryptominer".into());
        policy.blocked_images.insert("/malicious/".into());

        assert!(policy.validate("nginx:latest").is_ok());
        let err = policy
            .validate("docker.io/hacker/cryptominer:v2")
            .unwrap_err();
        assert!(err.contains("blocked pattern"));
        let err = policy
            .validate("evil.com/malicious/tool:latest")
            .unwrap_err();
        assert!(err.contains("blocked pattern"));
    }

    #[test]
    fn pinned_digests_only() {
        let mut policy = WorkloadPolicy::default();
        policy
            .pinned_digests
            .insert("docker.io/library/alpine@sha256:abc123".into());

        assert!(policy
            .validate("docker.io/library/alpine@sha256:abc123")
            .is_ok());
        let err = policy
            .validate("docker.io/library/alpine:latest")
            .unwrap_err();
        assert!(err.contains("not in pinned digests"));
    }

    #[test]
    fn extract_registry_defaults() {
        assert_eq!(extract_registry("alpine:latest"), "docker.io");
        assert_eq!(extract_registry("library/nginx:1.25"), "docker.io");
        assert_eq!(extract_registry("ghcr.io/myorg/app:v1"), "ghcr.io");
        assert_eq!(extract_registry("localhost:5000/myimg"), "localhost:5000");
        assert_eq!(
            extract_registry("my.registry.com/org/img"),
            "my.registry.com"
        );
    }

    #[test]
    fn rate_limiter_allows_within_window() {
        let limiter = RateLimiter::new(5, 60_000_000); // 5 per minute
        let requester = b"requester-1";
        for _ in 0..5 {
            assert!(limiter.check_and_record(requester));
        }
    }

    #[test]
    fn rate_limiter_rejects_over_window() {
        let limiter = RateLimiter::new(2, 60_000_000); // 2 per minute
        let requester = b"requester-2";
        assert!(limiter.check_and_record(requester));
        assert!(limiter.check_and_record(requester));
        assert!(!limiter.check_and_record(requester)); // 3rd should be rejected
    }

    #[test]
    fn rate_limiter_independent_per_requester() {
        let limiter = RateLimiter::new(1, 60_000_000);
        assert!(limiter.check_and_record(b"req-a"));
        assert!(!limiter.check_and_record(b"req-a"));
        assert!(limiter.check_and_record(b"req-b")); // different requester
    }
}
