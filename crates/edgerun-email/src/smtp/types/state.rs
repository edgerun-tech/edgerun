/// SMTP session states per RFC 5321.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpState {
    /// Initial state — after connection, before EHLO/HELO.
    Connected,
    /// After EHLO/HELO — ready for MAIL FROM.
    Ready,
    /// After MAIL FROM — ready for RCPT TO.
    MailSet,
    /// After one or more RCPT TO — ready for DATA.
    RcptSet,
    /// During DATA transfer — reading message content.
    Data,
    /// QUIT received — connection shutting down.
    Quit,
}
