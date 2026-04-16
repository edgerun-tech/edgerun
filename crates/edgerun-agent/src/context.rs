//! Context management — 32K optimized.

use std::collections::VecDeque;

pub fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    let mut end = max_chars;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
}

pub struct ConversationHistory {
    messages: VecDeque<Message>,
    max_messages: usize,
}

impl ConversationHistory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_messages,
        }
    }

    pub fn add(&mut self, role: MessageRole, content: String) {
        self.messages.push_back(Message { role, content });
        while self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }

    pub fn to_string(&self) -> String {
        self.messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                };
                format!("{}: {}", role, m.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_ascii() {
        let s = "abcdefghij".repeat(100);
        let truncated = truncate_str(&s, 500);
        assert!(truncated.len() <= 510);
        assert!(truncated.starts_with("abcdefghij"));
        assert!(truncated.len() < s.len());
    }

    #[test]
    fn test_truncate_str_utf8() {
        let s: String = "日本語".repeat(1000);
        let truncated = truncate_str(&s, 500);
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }

    #[test]
    fn test_conversation_history() {
        let mut history = ConversationHistory::new(3);
        history.add(MessageRole::User, "hello".to_string());
        history.add(MessageRole::Assistant, "hi".to_string());
        history.add(MessageRole::User, "how are you".to_string());
        history.add(MessageRole::Assistant, "fine".to_string());

        assert_eq!(history.messages.len(), 3);
        assert!(history.to_string().contains("how are you"));
    }
}
