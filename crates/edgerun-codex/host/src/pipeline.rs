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
const MAX_REVEALED_CONTEXT_CHARS: usize = 18_000;
const MAX_REVEAL_ROUNDS: usize = 2;
const MAX_REVEALS_PER_ROUND: usize = 8;
const MAX_REPO_EDITS_PER_STAGE: usize = 4;

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

pub trait RepoTools {
    fn reveal_file(&self, path: &str) -> Option<String>;
    fn reveal_definition(&self, name: &str) -> Option<String>;
    fn edit_file(&mut self, path: &str, replacement: String) -> Result<(), String>;
    fn diff(&self) -> String;
    fn changed_files(&self) -> Vec<String>;
    fn write_back(&mut self, path: &str) -> Result<(), String>;
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

    fn repo_revealed(&mut self, _request: &str, _found: bool) -> Result<(), BoxError> {
        Ok(())
    }

    fn repo_edited(&mut self, _path: &str, _ok: bool) -> Result<(), BoxError> {
        Ok(())
    }

    fn repo_written(&mut self, _path: &str, _ok: bool) -> Result<(), BoxError> {
        Ok(())
    }
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
- Choose repo_reveal_file, repo_reveal_definition, repo_edit, repo_diff, repo_writeback, and validation commands.
- Do not use host_shell for repository search, read, edit, patch, diff, or build planning.
- host_shell is only for non-repo OS operations when the user explicitly asks for it.
- Avoid exploratory thrashing.
- Make the Executor's work deterministic.

Output exactly these sections:
RepoReads:
RepoDefinitions:
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
- Produce exact in-memory repo edits from the ToolPlan.
- Use repo_edit(path) followed immediately by one fenced replacement body for full-file replacement edits.
- Prefer one focused edit over many speculative edits.
- Keep output actionable and minimal.
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
- Review the in-memory repo diff before it is written back.
- Find contradictions, duplicate systems, broken interfaces, unsafe assumptions, and likely compile failures.
- Confirm the plan respects the in-memory repo boundary.
- Flag any host_shell use for repo work as a blocking issue.
- Prefer corrections over vague criticism.
- Return Verdict: accept only when the diff should be written back to disk.

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
    repo: &mut dyn RepoTools,
    observer: &mut dyn PipelineObserver,
) -> Result<PipelineOutput, BoxError> {
    let mut stage_outputs = Vec::with_capacity(STAGES.len());

    let router_output = run_stage(
        runtime,
        client,
        user_request,
        prior_history,
        repo_context,
        repo,
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
        let mut output = run_stage(
            runtime,
            client,
            user_request,
            prior_history,
            repo_context,
            repo,
            &stage_outputs,
            *stage_index,
            position + 1,
            total,
            observer,
        )?;

        if output.stage.kind == PipelineKind::Executor {
            output.text = apply_repo_edits_from_executor(&output.text, repo, observer)?;
        } else if output.stage.kind == PipelineKind::Reviewer && reviewer_accepts(&output.text) {
            let written = write_back_changed_files(repo, observer)?;
            if !written.is_empty() {
                output.text.push_str("\n\nRepoWriteback:\n");
                output.text.push_str(&written);
            }
        }

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
    repo: &mut dyn RepoTools,
    stage_outputs: &[StageOutput],
    stage_index: usize,
    position: usize,
    total: usize,
    observer: &mut dyn PipelineObserver,
) -> Result<StageOutput, BoxError> {
    let stage = STAGES[stage_index];
    let mut revealed_context = String::new();
    let mut text = String::new();

    for round in 0..=MAX_REVEAL_ROUNDS {
        observer.stage_started(&stage, position, total)?;
        let handoff = pipeline_handoff(
            user_request,
            prior_history,
            repo_context,
            &revealed_context,
            stage_outputs,
            &stage,
        );
        let output = runtime.block_on(client.collect_turn(turn_request(vec![
            message_item("system", stage.prompt),
            message_item("user", &handoff),
        ])))?;
        text = normalize_output(output.output_text);

        let requests = reveal_requests(&text);
        if requests.is_empty() || round == MAX_REVEAL_ROUNDS {
            break;
        }

        let mut added = false;
        for request in requests.into_iter().take(MAX_REVEALS_PER_ROUND) {
            let label = request.label();
            let revealed = match &request {
                RevealRequest::File(path) => repo.reveal_file(path),
                RevealRequest::Definition(name) => repo.reveal_definition(name),
            };
            observer.repo_revealed(&label, revealed.is_some())?;
            if let Some(body) = revealed {
                append_revealed_context(&mut revealed_context, &label, &body);
                added = true;
            }
        }
        if !added {
            break;
        }
    }

    let bounded = truncate_chars(&text, MAX_STAGE_OUTPUT_CHARS);
    observer.stage_finished(&stage, position, total, &bounded)?;
    Ok(StageOutput {
        stage,
        text: bounded,
    })
}

fn apply_repo_edits_from_executor(
    output: &str,
    repo: &mut dyn RepoTools,
    observer: &mut dyn PipelineObserver,
) -> Result<String, BoxError> {
    let mut text = output.to_string();
    let edits = repo_edits(output);
    if edits.is_empty() {
        return Ok(text);
    }

    text.push_str("\n\nRepoEditResults:\n");
    for edit in edits.into_iter().take(MAX_REPO_EDITS_PER_STAGE) {
        let ok = repo.edit_file(&edit.path, edit.replacement).is_ok();
        observer.repo_edited(&edit.path, ok)?;
        text.push_str("- ");
        text.push_str(&edit.path);
        text.push_str(if ok { " applied in memory\n" } else { " failed\n" });
    }
    text.push_str("\nRepoDiff:\n");
    text.push_str(&repo.diff());
    Ok(truncate_chars(&text, MAX_STAGE_OUTPUT_CHARS))
}

fn write_back_changed_files(
    repo: &mut dyn RepoTools,
    observer: &mut dyn PipelineObserver,
) -> Result<String, BoxError> {
    let mut out = String::new();
    for path in repo.changed_files() {
        let ok = repo.write_back(&path).is_ok();
        observer.repo_written(&path, ok)?;
        out.push_str("- ");
        out.push_str(&path);
        out.push_str(if ok { " written\n" } else { " write failed\n" });
    }
    Ok(out)
}

fn reviewer_accepts(output: &str) -> bool {
    output
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("Verdict: accept") || line.trim().eq_ignore_ascii_case("Verdict: accepted"))
}

fn append_revealed_context(out: &mut String, label: &str, body: &str) {
    if out.len() >= MAX_REVEALED_CONTEXT_CHARS {
        return;
    }
    out.push_str("\n--- ");
    out.push_str(label);
    out.push_str(" ---\n");
    let remaining = MAX_REVEALED_CONTEXT_CHARS.saturating_sub(out.len());
    out.push_str(&truncate_chars(body, remaining));
    out.push('\n');
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RevealRequest {
    File(String),
    Definition(String),
}

impl RevealRequest {
    fn label(&self) -> String {
        match self {
            RevealRequest::File(path) => format!("repo_reveal_file({path})"),
            RevealRequest::Definition(name) => format!("repo_reveal_definition({name})"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepoEdit {
    path: String,
    replacement: String,
}

fn reveal_requests(text: &str) -> Vec<RevealRequest> {
    let mut out = Vec::new();
    collect_reveal_calls(text, "repo_reveal_file", |arg| RevealRequest::File(arg), &mut out);
    collect_reveal_calls(text, "repo_reveal_definition", |arg| RevealRequest::Definition(arg), &mut out);
    dedupe_reveals(out)
}

fn collect_reveal_calls(
    text: &str,
    function_name: &str,
    to_request: impl Fn(String) -> RevealRequest,
    out: &mut Vec<RevealRequest>,
) {
    let mut cursor = 0;
    let needle = format!("{function_name}(");
    while let Some(relative) = text[cursor..].find(&needle) {
        let start = cursor + relative + needle.len();
        let Some(end_relative) = text[start..].find(')') else {
            break;
        };
        let end = start + end_relative;
        if let Some(arg) = clean_tool_arg(&text[start..end]) {
            out.push(to_request(arg));
        }
        cursor = end + 1;
    }
}

fn repo_edits(text: &str) -> Vec<RepoEdit> {
    let mut out = Vec::new();
    let mut cursor = 0;
    let needle = "repo_edit(";
    while let Some(relative) = text[cursor..].find(needle) {
        let path_start = cursor + relative + needle.len();
        let Some(path_end_relative) = text[path_start..].find(')') else {
            break;
        };
        let path_end = path_start + path_end_relative;
        let Some(path) = clean_tool_arg(&text[path_start..path_end]) else {
            cursor = path_end + 1;
            continue;
        };
        let after_path = path_end + 1;
        let Some(fence_start_relative) = text[after_path..].find("```") else {
            cursor = after_path;
            continue;
        };
        let fence_start = after_path + fence_start_relative + 3;
        let Some(fence_end_relative) = text[fence_start..].find("```") else {
            break;
        };
        let fence_end = fence_start + fence_end_relative;
        let replacement = strip_fence_language(&text[fence_start..fence_end]).to_string();
        out.push(RepoEdit { path, replacement });
        cursor = fence_end + 3;
    }
    out
}

fn strip_fence_language(value: &str) -> &str {
    let value = value.trim_start_matches('\n');
    let Some(newline) = value.find('\n') else {
        return value;
    };
    let first = &value[..newline];
    if first.len() <= 32 && first.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '+') {
        &value[newline + 1..]
    } else {
        value
    }
}

fn clean_tool_arg(raw: &str) -> Option<String> {
    let cleaned = raw
        .trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();
    if cleaned.is_empty() || cleaned.len() > 256 || cleaned.contains('\n') {
        None
    } else {
        Some(cleaned)
    }
}

fn dedupe_reveals(requests: Vec<RevealRequest>) -> Vec<RevealRequest> {
    let mut out = Vec::new();
    for request in requests {
        if !out.contains(&request) {
            out.push(request);
        }
    }
    out
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
    revealed_context: &str,
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
    if !revealed_context.is_empty() {
        out.push_str("\n\nREVEALED_REPO_CONTEXT:\n");
        out.push_str(revealed_context);
    }
    out.push_str("\n\nRECENT_TRANSCRIPT_SUMMARY:\n");
    out.push_str(&compact_history(prior_history));
    out.push_str("\n\nPREVIOUS_STAGE_OUTPUTS:\n");

    if stage_outputs.is_empty() {
        out.push_str("(none)\n");
    } else {
        append_previous_outputs(&mut out, stage_outputs);
    }

    out.push_str("\nREVEAL_LOOP:\n");
    out.push_str("- If you need an exact loaded file body, output repo_reveal_file(path) and stop.\n");
    out.push_str("- If you need an exact known function/type/body, output repo_reveal_definition(name) and stop.\n");
    out.push_str("- The host will reveal from memory and rerun this same stage.\n");
    out.push_str("\nEDIT_LOOP:\n");
    out.push_str("- Executor may output repo_edit(path) followed by one fenced full-file replacement body.\n");
    out.push_str("- The host applies repo_edit only in memory and sends the resulting RepoDiff to Reviewer.\n");
    out.push_str("- Reviewer controls writeback. Only Verdict: accept writes changed files to disk.\n");
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

    struct EmptyRepo;

    impl RepoTools for EmptyRepo {
        fn reveal_file(&self, _: &str) -> Option<String> {
            None
        }

        fn reveal_definition(&self, _: &str) -> Option<String> {
            None
        }

        fn edit_file(&mut self, _: &str, _: String) -> Result<(), String> {
            Err("not found".to_string())
        }

        fn diff(&self) -> String {
            "(no changes)".to_string()
        }

        fn changed_files(&self) -> Vec<String> {
            Vec::new()
        }

        fn write_back(&mut self, _: &str) -> Result<(), String> {
            Err("not found".to_string())
        }
    }

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
        let handoff = pipeline_handoff("fix thing", &[], "RepoWorkspace:\nLoadedFiles: 1", "", &[], &STAGES[1]);
        assert!(handoff.contains("REPO_WORKSPACE_CONTEXT"));
        assert!(handoff.contains("host_shell is a separate non-repo tool"));
    }

    #[test]
    fn parses_reveal_requests() {
        let requests = reveal_requests("Need repo_reveal_file(src/main.rs) and repo_reveal_definition(handle_chat_envelope)");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], RevealRequest::File("src/main.rs".to_string()));
        assert_eq!(requests[1], RevealRequest::Definition("handle_chat_envelope".to_string()));
    }

    #[test]
    fn parses_repo_edit_with_fenced_replacement() {
        let edits = repo_edits("repo_edit(src/lib.rs)\n```rust\nfn main() {}\n```");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].path, "src/lib.rs");
        assert_eq!(edits[0].replacement, "fn main() {}\n");
    }

    #[test]
    fn reviewer_accepts_exact_verdict() {
        assert!(reviewer_accepts("Verdict: accept\nBlockingIssues:"));
        assert!(!reviewer_accepts("Verdict: reject"));
    }
}
