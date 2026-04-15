//! Prompt templates for different agent tasks.
//!
//! Optimized for devstral-small-2:24b with 32k context.
//! Keep prompts concise but informative.

pub struct Prompts;

impl Prompts {
    pub fn system() -> &'static str {
        r#"You are an expert Rust programmer and code analysis assistant. Your role is to:
- Generate clean, idiomatic Rust code
- Debug and fix code issues
- Write comprehensive tests
- Explain code clearly

Always use AST-safe editing operations. Never use string replacement on Rust code.
When generating code, prefer:
- Proper error handling with Result types
- Idiomatic Rust patterns (iterators, closures, etc.)
- Documentation comments where helpful
- Tests that cover edge cases"#
    }

    pub fn code_generation() -> &'static str {
        r#"Generate a Rust function with the following requirements:
- Function name: {name}
- Arguments: {args}
- Return type: {ret}
- Body: {body}

Provide only the function implementation, no extra explanation."#
    }

    pub fn debug() -> &'static str {
        r#"Debug the following Rust code. The error is:
{error}

Code:
```{language}
{code}
```

Provide:
1. Root cause analysis
2. Fixed code using AST-safe edit operations
3. Brief explanation"#
    }

    pub fn test_generation() -> &'static str {
        r#"Generate unit tests for the following Rust code:

```{language}
{code}
```

Requirements:
- Use #[cfg(test)] and #[test] attributes
- Test edge cases and error conditions
- Follow Rust testing conventions
- Use proper assertions"#
    }

    pub fn refactor() -> &'static str {
        r#"Refactor the following Rust code to improve:
{goals}

Original code:
```{language}
{code}
```

Provide improved code with brief explanation of changes."#
    }

    pub fn code_review() -> &'static str {
        r#"Review the following Rust code for:
- Potential bugs
- Performance issues
- Style inconsistencies
- Missing error handling

```{language}
{code}
```"#
    }

    pub fn edit_operations() -> &'static str {
        r#"When making edits to Rust code, use these operations:
- add_fn: Add a new function
- replace_fn_body: Replace function body
- rename_type: Rename struct/enum
- add_use: Add import
- add_derive: Add derive macro

Format your response as JSON:
{"op": "operation_name", "file": "path.rs", "params": {...}}"#
    }

    pub fn full_context_prompt(message: &str, code_context: &str, history: &str) -> String {
        let mut parts = Vec::new();

        parts.push(Self::system().to_string());

        if !code_context.is_empty() {
            parts.push(format!("## Code Context\n{}", code_context));
        }

        if !history.is_empty() {
            parts.push(format!("## Conversation History\n{}", history));
        }

        parts.push(format!("## Task\n{}", message));

        parts.join("\n\n")
    }
}

pub struct AgentTask {
    pub task_type: TaskType,
    pub description: String,
    pub code: Option<String>,
    pub file: Option<String>,
    pub language: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TaskType {
    Generate,
    Debug,
    Test,
    Refactor,
    Review,
    Explain,
}

impl AgentTask {
    pub fn generate(description: &str) -> Self {
        Self {
            task_type: TaskType::Generate,
            description: description.to_string(),
            code: None,
            file: None,
            language: Some("rust".to_string()),
            error: None,
        }
    }

    pub fn debug(description: &str, code: &str, error: &str) -> Self {
        Self {
            task_type: TaskType::Debug,
            description: description.to_string(),
            code: Some(code.to_string()),
            file: None,
            language: Some("rust".to_string()),
            error: Some(error.to_string()),
        }
    }

    pub fn test(code: &str) -> Self {
        Self {
            task_type: TaskType::Test,
            description: "Generate unit tests".to_string(),
            code: Some(code.to_string()),
            file: None,
            language: Some("rust".to_string()),
            error: None,
        }
    }

    pub fn to_prompt(&self) -> String {
        match self.task_type {
            TaskType::Generate => {
                format!("{}\n\n{}", Prompts::system(), self.description)
            }
            TaskType::Debug => {
                let code = self.code.as_deref().unwrap_or("");
                let error = self.error.as_deref().unwrap_or("");
                let lang = self.language.as_deref().unwrap_or("rust");
                format!(
                    "Debug the following Rust code. The error is:\n{}\n\n```{}\n{}\n```\n\nProvide:\n1. Root cause analysis\n2. Fixed code\n3. Brief explanation",
                    error, lang, code
                )
            }
            TaskType::Test => {
                let code = self.code.as_deref().unwrap_or("");
                let lang = self.language.as_deref().unwrap_or("rust");
                format!(
                    "Generate unit tests for the following Rust code:\n\n```{}\n{}\n```\n\nRequirements:\n- Use #[cfg(test)] and #[test] attributes\n- Test edge cases and error conditions\n- Follow Rust testing conventions",
                    lang, code
                )
            }
            TaskType::Refactor => {
                let code = self.code.as_deref().unwrap_or("");
                let lang = self.language.as_deref().unwrap_or("rust");
                format!(
                    "Refactor the following Rust code to improve:\n{}\n\n```{}\n{}\n```\n\nProvide improved code with brief explanation.",
                    self.description, lang, code
                )
            }
            TaskType::Review => {
                let code = self.code.as_deref().unwrap_or("");
                let lang = self.language.as_deref().unwrap_or("rust");
                format!(
                    "Review the following Rust code for:\n- Potential bugs\n- Performance issues\n- Style inconsistencies\n- Missing error handling\n\n```{}\n{}\n```",
                    lang, code
                )
            }
            TaskType::Explain => {
                format!(
                    "{}\n\nExplain this code:\n\n{}",
                    Prompts::system(),
                    self.description
                )
            }
        }
    }
}

// Benchmark comment