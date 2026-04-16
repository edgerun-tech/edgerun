//! Prompt templates for the coding agent.

pub struct Prompts;

impl Prompts {
    pub fn system() -> &'static str {
        r#"You are an expert coding assistant. You have shell access to run commands.

When you need to run a command, prefix it with $ like this:
$ cargo check
$ grep -r "pattern" src/

You can run multiple commands in sequence. After seeing the output, provide your analysis and answer.

Always prefer running commands when you need information about the codebase rather than guessing.
Be concise and direct in your responses."#
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

    pub fn to_prompt(&self) -> String {
        match self.task_type {
            TaskType::Generate => {
                format!("{}\n\n{}", Prompts::system(), self.description)
            }
            _ => {
                format!("{}\n\n{}", Prompts::system(), self.description)
            }
        }
    }
}
