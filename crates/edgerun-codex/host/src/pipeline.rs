use std::error::Error;

use codex_core::protocol::models::ContentItem;
use codex_core::protocol::models::ResponseItem;
use codex_core::ModelClient;
use codex_core::Prompt;
use codex_core::TurnRequest;

pub type BoxError = Box<dyn Error>;

const ROUTER_STAGE_INDEX: usize = 0;
const SUMMARIZER_STAGE_INDEX: usize = 6;
const MAX_HISTORY_CHARS: usize = 2400;
const MAX_STAGE_OUTPUT_CHARS: usize = 2400;
const MAX_PREVIOUS_OUTPUT_CHARS: usize = 9000;
const MAX_REPO_CONTEXT_CHARS: usize = 12_000;

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

Boundary rules:
- Repository work happens in the in-memory repo workspace.
- Searching, reading, editing, diffing, and patch planning for repo files must assume memory-backed repo tools, not shell.
- host_shell is only for non-repo operations and only when the user explicitly asks for shell/OS work.

Routing rules:
- Explanation-only tasks should usually use: Router, Codebase, Summarizer.
- Build/test/debug failures should usually use: Router, Codebase, Toolsmith, Executor, Reviewer, Summarizer.
- Code edits should usually use: Router, Codebase, Architect, Toolsmith, Executor, Reviewer, Summarizer.
- Architecture-only tasks should usually use: Router, Codebase, Architect, Reviewer, Summarizer.

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
- Use the provided in-memory repo workspace context as the source of truth.
- Identify the exact files/modules/concepts that likely matter.
- Prefer existing APIs and callers over adding compatibility shims.
- Prefer small relevant context over broad search.
- Produce a CodebaseContextPlan that another agent can execute using memory-backed repo tools.
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
- Assume repository reads/searches/edits happen in the in-memory workspace.
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
- Translate the ChangePlan into exact memory-backed repo tool actions.
- Choose repo_search, repo_read, repo_edit, repo_diff, repo_writeback, and validation commands.
- Do not use host_shell for repository search, read, edit, patch, diff, or build planning.
- host_shell is only for non-repo OS operations when the user explicitly asks for it.
- Avoid exploratory thrashing.
- Make the Executor's work deterministic.

Output exactly these sections:
RepoReads:
RepoSearches:
RepoEdits:
ValidationCommands:
HostShellRequests:
RollbackPlan:
ExpectedArtifacts:"#,
    },
    PipelineStage {
        kind: PipelineKind::Executor,
        name: "Executor",
        output_name: "PatchExecution",
        prompt: r#"You are the Executor stage.

Your job:
- Produce the exact in-memory repo edit plan from the ToolPlan.
- Prefer one focused patch over many speculative edits.
- Keep output actionable and minimal.
- When code is required, output concrete replacement blocks or patch chunks for repo_edit.
- Do not add new systems when existing callers/components should be updated.
- Do not use shell for repo file operations.

Output exactly these sections:
PatchSummary:
FilesModified:
RepoEditOperations:
ValidationCommands:
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
- Confirm the plan respects the in-memory repo boundary.
- Flag any host_shell use for repo work as a blocking issue.
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
- Mention whether the repo work stayed inside the in-memory workspace boundary.

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
    repo_context: &str,
    observer: &mut dyn PipelineObserver,
) -> Result<PipelineOutput, BoxError> {
    let mut stage_outputs = Vec::with_capacity(STAGES.len());

    let router_output = run_stage(
        runtime,
        client,
        user_request,
        prior_history,
        repo_context,
        &stage_outputs,
        ROUTER_STAGE_INDEX,
        0,
        STAGES.len(),
        observer,
    )?;
    stage_outputs.push(router_output);

    let route = route_after_router(user_request, &stage_outputs[0].text);
    let total = route.len() + 1;

    for (position, stage_index) in route.iter().enumerate() {
        let output = run_stage(
            runtime,
            client,
            user_request,
            prior_history,
            repo_context,
            &stage_outputs,
            *stage_index,
            position + 1,
            total,
            observer,
        )?;
        stage_outputs.push(output);
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
    let route = fallback_route_for_request(user_request);
    let total = route.len();
    let mut stages = Vec::with_capacity(total);

    for (index, stage_index) in route.iter().enumerate() {
        let stage = STAGES[*stage_index];
        observer.stage_started(&stage, index, total)?;
        let text = format!(
            "{}:\nmock handoff for request: {}\nBoundary: repo operations stay in memory; host_shell only on explicit non-repo request.\n",
            stage.output_name, user_request
        );
        observer.stage_finished(&stage, index, total, &text)?;
        stages.push(StageOutput { stage, text });
    }

    let final_reply = format!(
        "Pipeline completed in mock mode using {} stages. Repository work is expected to stay inside the in-memory workspace; host_shell is reserved for explicit non-repo requests.\n\nRequest: {}",
        total, user_request
    );

    Ok(PipelineOutput {
        durable_summary: final_reply.clone(),
        final_reply,
        stages,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_stage(
    runtime: &edgerun_tokio::runtime::Runtime,
    client: &ModelClient,
    user_request: &str,
    prior_history: &[ResponseItem],
    repo_context: &str,
    stage_outputs: &[StageOutput],
    stage_index: usize,
    position: usize,
    total: usize,
    observer: &mut dyn PipelineObserver,
) -> Result<StageOutput, BoxError> {
    let stage = STAGES[stage_index];
    observer.stage_started(&stage, position, total)?;
    let handoff = pipeline_handoff(user_request, prior_history, repo_context, stage_outputs, &stage);
    let output = runtime.block_on(client.collect_turn(turn_request(vec![
        message_item("system", stage.prompt),
        message_item("user", &handoff),
    ])))?;

    let text = normalize_output(output.output_text);
    let bounded = truncate_chars(&text, MAX_STAGE_OUTPUT_CHARS);
    observer.stage_finished(&stage, position, total, &bounded)?;
    Ok(StageOutput {
        stage,
        text: bounded,
    })
}

fn route_after_router(user_request: &str, router_output: &str) -> Vec<usize> {
    let mut route = Vec::new();
    let lower = router_output.to_ascii_lowercase();
    for stage_index in 1..STAGES.len() {
        let stage = STAGES[stage_index];
        if lower.contains(&stage.name.to_ascii_lowercase())
            || lower.contains(&stage.output_name.to_ascii_lowercase())
        {
            push_unique(&mut route, stage_index);
        }
    }

    if route.is_empty() {
        route = fallback_route_for_request(user_request);
        route.retain(|stage| *stage != ROUTER_STAGE_INDEX);
    }

    if !route.contains(&SUMMARIZER_STAGE_INDEX) {
        route.push(SUMMARIZER_STAGE_INDEX);
    }
    route.retain(|stage| *stage != ROUTER_STAGE_INDEX);
    route
}

fn fallback_route_for_request(user_request: &str) -> Vec<usize> {
    let lower = user_request.to_ascii_lowercase();
    let mut route = vec![ROUTER_STAGE_INDEX, 1];

    if contains_any(&lower, &["fix", "edit", "patch", "implement", "make it", "compile", "build", "test", "error", "failed", "failure", "panic", "crash"])
    {
        route.extend_from_slice(&[2, 3, 4, 5]);
    } else if contains_any(&lower, &["architecture", "design", "plan", "improve", "refactor"])
    {
        route.extend_from_slice(&[2, 5]);
    }

    route.push(SUMMARIZER_STAGE_INDEX);
    dedupe_route(route)
}

fn push_unique(route: &mut Vec<usize>, stage: usize) {
    if !route.contains(&stage) {
        route.push(stage);
    }
}

fn dedupe_route(route: Vec<usize>) -> Vec<usize> {
    let mut out = Vec::with_capacity(route.len());
    for stage in route {
        push_unique(&mut out, stage);
    }
    out
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn pipeline_handoff(
    user_request: &str,
    prior_history: &[ResponseItem],
    repo_context: &str,
    stage_outputs: &[StageOutput],
    stage: &PipelineStage,
) -> String {
    let mut out = String::new();
    out.push_str("USER_REQUEST:\n");
    out.push_str(&truncate_chars(user_request, 1800));
    out.push_str("\n\nCURRENT_STAGE:\n");
    out.push_str(stage.name);
    out.push_str("\n\nEXPECTED_OUTPUT:\n");
    out.push_str(stage.output_name);
    out.push_str("\n\nREPO_WORKSPACE_CONTEXT:\n");
    out.push_str(&truncate_chars(repo_context, MAX_REPO_CONTEXT_CHARS));
    out.push_str("\n\nRECENT_TRANSCRIPT_SUMMARY:\n");
    out.push_str(&compact_history(prior_history));
    out.push_str("\n\nPREVIOUS_STAGE_OUTPUTS:\n");

    if stage_outputs.is_empty() {
        out.push_str("(none)\n");
    } else {
        append_previous_outputs(&mut out, stage_outputs);
    }

    out.push_str("\nBOUNDARY:\n");
    out.push_str("- Repository search/read/edit/diff/patch work happens in the in-memory repo workspace.\n");
    out.push_str("- host_shell is a separate non-repo tool and only available when the user explicitly asks for host/OS shell work.\n");
    out.push_str("- Do not solve repo work by shelling out.\n");
    out.push_str("\nCONSTRAINTS:\n");
    out.push_str("- Keep output compact.\n");
    out.push_str("- Prefer existing repo systems and interfaces.\n");
    out.push_str("- Do not create parallel systems unless explicitly justified.\n");
    out.push_str("- Optimize for fixed-prompt cache locality and 32k context.\n");
    out
}

fn append_previous_outputs(out: &mut String, stage_outputs: &[StageOutput]) {
    let mut remaining = MAX_PREVIOUS_OUTPUT_CHARS;
    for item in stage_outputs.iter().rev() {
        if remaining == 0 {
            break;
        }
        out.push_str("\n--- ");
        out.push_str(item.stage.output_name);
        out.push_str(" / ");
        out.push_str(item.stage.name);
        out.push_str(" ---\n");
        let chunk = truncate_chars(&item.text, remaining.min(MAX_STAGE_OUTPUT_CHARS));
        remaining = remaining.saturating_sub(chunk.len());
        out.push_str(&chunk);
        out.push('\n');
    }
}

fn compact_history(history: &[ResponseItem]) -> String {
    let mut out = String::new();
    let keep_from = history.len().saturating_sub(4);
    let mut remaining = MAX_HISTORY_CHARS;
    for item in &history[keep_from..] {
        if remaining == 0 {
            break;
        }
        if let ResponseItem::Message { role, content, .. } = item {
            out.push_str(role);
            out.push_str(": ");
            for part in content {
                match part {
                    ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                        let chunk = truncate_chars(text, remaining.min(600));
                        remaining = remaining.saturating_sub(chunk.len());
                        out.push_str(&chunk);
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

    #[test]
    fn simple_explanation_route_skips_executor_chain() {
        assert_eq!(fallback_route_for_request("explain how this works"), vec![0, 1, 6]);
    }

    #[test]
    fn build_failure_route_includes_execution_chain() {
        assert_eq!(fallback_route_for_request("fix the build error"), vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn router_output_can_select_route() {
        let route = route_after_router("hello", "DownstreamPlan: Codebase, Summarizer");
        assert_eq!(route, vec![1, 6]);
    }

    #[test]
    fn handoff_contains_repo_workspace_boundary() {
        let handoff = pipeline_handoff("fix thing", &[], "RepoWorkspace:\nLoadedFiles: 1", &[], &STAGES[1]);
        assert!(handoff.contains("REPO_WORKSPACE_CONTEXT"));
        assert!(handoff.contains("host_shell is a separate non-repo tool"));
    }
}
