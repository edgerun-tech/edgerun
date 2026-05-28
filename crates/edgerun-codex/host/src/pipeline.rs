use std::error::Error;

use codex_core::protocol::models::ContentItem;
use codex_core::protocol::models::ResponseItem;
use codex_core::ModelClient;
use codex_core::Prompt;
use codex_core::TurnRequest;

pub type BoxError = Box<dyn Error>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipelineKind {
    Router,
    Codebase,
    Architect,
    Toolsmith,
    Executor,
    Reviewer,
    Summarizer,
}

#[derive(Clone, Copy, Debug)]
pub struct PipelineStage {
    pub kind: PipelineKind,
    pub name: &'static str,
    pub output_name: &'static str,
    pub prompt: &'static str,
}

#[derive(Clone, Debug)]
pub struct StageOutput {
    pub stage: PipelineStage,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct PipelineOutput {
    pub final_reply: String,
    pub durable_summary: String,
    pub stages: Vec<StageOutput>,
}

pub trait PipelineObserver {
    fn stage_started(
        &mut self,
        stage: &PipelineStage,
        index: usize,
        total: usize,
    ) -> Result<(), BoxError>;

    fn stage_finished(
        &mut self,
        stage: &PipelineStage,
        index: usize,
        total: usize,
        output: &str,
    ) -> Result<(), BoxError>;
}

pub const STAGES: &[PipelineStage] = &[
    PipelineStage {
        kind: PipelineKind::Router,
        name: "Router",
        output_name: "TaskClassification",
        prompt: r#"You are the Router stage in a local code-editing agent pipeline.

Your job:
- Classify the user's task.
- Decide which downstream stages matter.
- Identify whether this is code edit, build/test failure, architecture planning, repo inspection, documentation, debugging, or operations.
- Do not solve the task.
- Do not produce patches.
- Output a compact TaskClassification handoff.

Output exactly these sections:
TaskType:
Risk:
RequiredContext:
RequiredTools:
DownstreamPlan:
StopConditions:"#,
    },
    PipelineStage {
        kind: PipelineKind::Codebase,
        name: "Codebase",
        output_name: "CodebaseContextPlan",
        prompt: r#"You are the Codebase stage.

Your job:
- Assume you keep a stable mental map of the repository.
- Given the user task and Router classification, identify the exact files/modules/concepts that likely matter.
- Prefer small relevant context over broad search.
- Produce a CodebaseContextPlan that another agent can execute with minimal repo reads.
- Do not write code yet.

Output exactly these sections:
RelevantFiles:
ExistingAPIs:
LikelyConflictWithExistingSystems:
ContextToLoad:
ContextToAvoid:
QuestionsResolvedByInspection:"#,
    },
    PipelineStage {
        kind: PipelineKind::Architect,
        name: "Architect",
        output_name: "ChangePlan",
        prompt: r#"You are the Architect stage.

Your job:
- Convert the task and CodebaseContextPlan into the smallest coherent change.
- Prefer adapting existing architecture over creating parallel systems.
- Be aggressive about avoiding duplicate abstractions.
- Produce a concise ChangePlan for an Executor.
- Do not output code unless a tiny signature or data shape is necessary.

Output exactly these sections:
DesignDecision:
FilesToChange:
DataFlow:
StateFlow:
CompatibilityRules:
PatchPlan:
ValidationPlan:"#,
    },
    PipelineStage {
        kind: PipelineKind::Toolsmith,
        name: "Toolsmith",
        output_name: "ToolPlan",
        prompt: r#"You are the Toolsmith stage.

Your job:
- Translate the ChangePlan into exact tools/actions.
- Choose shell commands, file reads, patch strategy, tests, and verification commands.
- Avoid exploratory thrashing.
- Make the Executor's work deterministic.

Output exactly these sections:
ReadCommands:
EditStrategy:
PatchBoundaries:
BuildCommands:
TestCommands:
RollbackPlan:
ExpectedArtifacts:"#,
    },
    PipelineStage {
        kind: PipelineKind::Executor,
        name: "Executor",
        output_name: "PatchExecution",
        prompt: r#"You are the Executor stage.

Your job:
- Produce the exact implementation plan or patch instructions from the ToolPlan.
- Prefer one focused patch over many speculative edits.
- Keep output actionable and minimal.
- When code is required, output concrete patch chunks or replacement blocks.
- Do not add new systems when existing callers/components should be updated.

Output exactly these sections:
PatchSummary:
FilesModified:
ExactChanges:
CommandsToRun:
ExpectedResult:"#,
    },
    PipelineStage {
        kind: PipelineKind::Reviewer,
        name: "Reviewer",
        output_name: "ReviewResult",
        prompt: r#"You are the Reviewer stage.

Your job:
- Review the proposed PatchExecution before it is applied or finalized.
- Find contradictions, duplicate systems, broken interfaces, unsafe assumptions, and likely compile failures.
- Prefer corrections over vague criticism.

Output exactly these sections:
Verdict:
BlockingIssues:
NonBlockingIssues:
InterfaceCompatibility:
MissingValidation:
CorrectedPatchGuidance:"#,
    },
    PipelineStage {
        kind: PipelineKind::Summarizer,
        name: "Summarizer",
        output_name: "DurableSummary",
        prompt: r#"You are the Summarizer stage.

Your job:
- Produce the final user-facing answer and durable memory summary.
- Keep it compact.
- Include what changed, what remains, and exact next command.
- Preserve critical decisions for future agent context.

Output exactly these sections:
FinalAnswer:
DurableMemory:
NextCommand:
OpenRisks:"#,
    },
];

pub fn run_pipeline(
    runtime: &edgerun_tokio::runtime::Runtime,
    client: &ModelClient,
    user_request: &str,
    prior_history: &[ResponseItem],
    observer: &mut dyn PipelineObserver,
) -> Result<PipelineOutput, BoxError> {
    let mut stage_outputs = Vec::with_capacity(STAGES.len());
    let total = STAGES.len();

    for (index, stage) in STAGES.iter().enumerate() {
        observer.stage_started(stage, index, total)?;
        let handoff = pipeline_handoff(user_request, prior_history, &stage_outputs, stage);
        let output = runtime.block_on(client.collect_turn(turn_request(vec![
            message_item("system", stage.prompt),
            message_item("user", &handoff),
        ])))?;

        let text = normalize_output(output.output_text);
        observer.stage_finished(stage, index, total, &text)?;
        stage_outputs.push(StageOutput { stage: *stage, text });
    }

    let durable_summary = stage_outputs
        .last()
        .map(|stage| stage.text.clone())
        .unwrap_or_default();
    let final_reply = final_answer_from_summary(&durable_summary)
        .unwrap_or_else(|| durable_summary.clone());

    Ok(PipelineOutput {
        final_reply,
        durable_summary,
        stages: stage_outputs,
    })
}

pub fn run_mock_pipeline(
    user_request: &str,
    observer: &mut dyn PipelineObserver,
) -> Result<PipelineOutput, BoxError> {
    let mut stages = Vec::with_capacity(STAGES.len());
    let total = STAGES.len();

    for (index, stage) in STAGES.iter().enumerate() {
        observer.stage_started(stage, index, total)?;
        let text = format!(
            "{}:\nmock handoff for request: {}\n",
            stage.output_name, user_request
        );
        observer.stage_finished(stage, index, total, &text)?;
        stages.push(StageOutput { stage: *stage, text });
    }

    let final_reply = format!(
        "Pipeline completed in mock mode. Request classified, planned, routed through tools, reviewed, and summarized.\n\nRequest: {}",
        user_request
    );

    Ok(PipelineOutput {
        durable_summary: final_reply.clone(),
        final_reply,
        stages,
    })
}

fn pipeline_handoff(
    user_request: &str,
    prior_history: &[ResponseItem],
    stage_outputs: &[StageOutput],
    stage: &PipelineStage,
) -> String {
    let mut out = String::new();
    out.push_str("USER_REQUEST:\n");
    out.push_str(user_request);
    out.push_str("\n\nCURRENT_STAGE:\n");
    out.push_str(stage.name);
    out.push_str("\n\nEXPECTED_OUTPUT:\n");
    out.push_str(stage.output_name);
    out.push_str("\n\nRECENT_TRANSCRIPT_SUMMARY:\n");
    out.push_str(&compact_history(prior_history));
    out.push_str("\n\nPREVIOUS_STAGE_OUTPUTS:\n");

    if stage_outputs.is_empty() {
        out.push_str("(none)\n");
    } else {
        for item in stage_outputs {
            out.push_str("\n--- ");
            out.push_str(item.stage.output_name);
            out.push_str(" / ");
            out.push_str(item.stage.name);
            out.push_str(" ---\n");
            out.push_str(&item.text);
            out.push('\n');
        }
    }

    out.push_str("\nCONSTRAINTS:\n");
    out.push_str("- Keep output compact.\n");
    out.push_str("- Prefer existing repo systems and interfaces.\n");
    out.push_str("- Do not create parallel systems unless explicitly justified.\n");
    out.push_str("- Optimize for fixed-prompt cache locality and 32k context.\n");
    out
}

fn compact_history(history: &[ResponseItem]) -> String {
    let mut out = String::new();
    let keep_from = history.len().saturating_sub(6);
    for item in &history[keep_from..] {
        if let ResponseItem::Message { role, content, .. } = item {
            out.push_str(role);
            out.push_str(": ");
            for part in content {
                match part {
                    ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                        out.push_str(&truncate_chars(text, 600));
                    }
                    _ => {}
                }
            }
            out.push('\n');
        }
    }
    if out.is_empty() {
        out.push_str("(none)\n");
    }
    out
}

fn final_answer_from_summary(summary: &str) -> Option<String> {
    let marker = "FinalAnswer:";
    let start = summary.find(marker)? + marker.len();
    let rest = summary[start..].trim_start();
    let end = rest.find("\nDurableMemory:").unwrap_or(rest.len());
    Some(rest[..end].trim().to_string()).filter(|value| !value.is_empty())
}

fn normalize_output(value: String) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "(empty stage output)".to_string()
    } else {
        trimmed.to_string()
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max_chars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn turn_request(input: Vec<ResponseItem>) -> TurnRequest {
    TurnRequest {
        prompt: Prompt {
            input,
            ..Prompt::default()
        },
        store: false,
        ..TurnRequest::default()
    }
}

fn message_item(role: &str, text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: role.to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_has_cache_friendly_fixed_stage_order() {
        assert_eq!(STAGES.len(), 7);
        assert_eq!(STAGES[0].name, "Router");
        assert_eq!(STAGES[6].output_name, "DurableSummary");
    }

    #[test]
    fn final_answer_is_extracted_from_summary() {
        let summary = "FinalAnswer:\nDone.\nDurableMemory:\nRemember this.";
        assert_eq!(final_answer_from_summary(summary).unwrap(), "Done.");
    }
}
