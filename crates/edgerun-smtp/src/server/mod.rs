pub mod dsn_generator;
pub mod handler;
pub mod maildir;
pub mod rate_limit;
pub mod session;

pub use dsn_generator::{DeliveryStatus, DsnAction, DsnBounce};
pub use handler::{MailHandler, MemoryMailStore};
pub use maildir::{MaildirMessage, MaildirState, MaildirStore, MailboxStats};
pub use rate_limit::{RateLimitConfig, RateLimiter};
pub use session::{SmtpServer, SmtpServerConfig};
