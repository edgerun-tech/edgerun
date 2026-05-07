pub mod command;
pub mod dsn;
pub mod envelope;
pub mod headers;
pub mod limits;
pub mod response;
pub mod state;

pub use command::{
    extract_dsn_envid, extract_dsn_notify, extract_dsn_orcpt, extract_dsn_ret,
    parse_esmtp_parameters, SmtpCommand,
};
pub use dsn::{DsnNotify, DsnRet};
pub use envelope::MailEnvelope;
pub use headers::{get_date, get_from_address, get_header, get_subject, parse_headers};
pub use limits::ServerLimits;
pub use response::{
    parse_response_line, parse_response_lines, EnhancedStatusCode, SmtpResponse, SmtpResponseCode,
    SmtpResponseLine, SmtpResponseParseError,
};
pub use state::SmtpState;
