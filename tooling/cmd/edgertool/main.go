package main

import (
	"bufio"
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"net"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

func main() {
	if len(os.Args) < 2 {
		fatalf("usage: edgertool <nats-pub|agent-diff-proposed|agent-diff-accept|code-update-pub|agent-launch> [...]")
	}

	var err error
	switch os.Args[1] {
	case "nats-pub":
		err = cmdNatsPub(os.Args[2:])
	case "agent-diff-proposed":
		err = cmdAgentDiffProposed(os.Args[2:])
	case "agent-diff-accept":
		err = cmdAgentDiffAccept(os.Args[2:])
	case "code-update-pub":
		err = cmdCodeUpdatePub(os.Args[2:])
	case "agent-launch":
		err = cmdAgentLaunch(os.Args[2:])
	default:
		err = fmt.Errorf("unknown command: %s", os.Args[1])
	}
	if err != nil {
		fatalf("%v", err)
	}
}

type natsConfig struct {
	URL         string
	Retries     int
	RetryDelay  time.Duration
	ConnTimeout time.Duration
}

func defaultNATSURL() string {
	if v := strings.TrimSpace(os.Getenv("EDGERUN_EVENTBUS_NATS_URL")); v != "" {
		return v
	}
	return "nats://127.0.0.1:4222"
}

func cmdNatsPub(args []string) error {
	fs := flag.NewFlagSet("nats-pub", flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	var (
		subject     = fs.String("subject", "", "subject")
		payload     = fs.String("payload", "", "json payload")
		natsURL     = fs.String("nats-url", defaultNATSURL(), "nats://host:port")
		retries     = fs.Int("retries", envInt("NATS_PUB_RETRIES", 3), "publish retries")
		retryDelay  = fs.Duration("retry-delay", envDurationMs("NATS_PUB_RETRY_DELAY_MS", 200), "retry delay")
		connTimeout = fs.Duration("timeout", envDurationSec("NATS_PUB_TIMEOUT_S", 2), "connect timeout")
	)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *subject == "" || *payload == "" {
		return errors.New("nats-pub requires --subject and --payload")
	}
	cfg := natsConfig{URL: *natsURL, Retries: *retries, RetryDelay: *retryDelay, ConnTimeout: *connTimeout}
	return publishNATS(cfg, *subject, []byte(*payload))
}

func publishNATS(cfg natsConfig, subject string, payload []byte) error {
	host, port, err := parseNATSAddress(cfg.URL)
	if err != nil {
		return err
	}
	if cfg.Retries < 1 {
		cfg.Retries = 1
	}
	if cfg.RetryDelay < 0 {
		cfg.RetryDelay = 0
	}
	if cfg.ConnTimeout <= 0 {
		cfg.ConnTimeout = 2 * time.Second
	}

	addr := net.JoinHostPort(host, port)
	frame := bytes.NewBuffer(nil)
	fmt.Fprintf(frame, "CONNECT {\"verbose\":false,\"pedantic\":false}\r\n")
	fmt.Fprintf(frame, "PUB %s %d\r\n", subject, len(payload))
	frame.Write(payload)
	frame.WriteString("\r\nPING\r\n")

	var lastErr error
	for attempt := 1; attempt <= cfg.Retries; attempt++ {
		conn, err := net.DialTimeout("tcp", addr, cfg.ConnTimeout)
		if err != nil {
			lastErr = err
		} else {
			_ = conn.SetDeadline(time.Now().Add(cfg.ConnTimeout))
			_, err = conn.Write(frame.Bytes())
			_ = conn.Close()
			if err == nil {
				return nil
			}
			lastErr = err
		}
		if attempt < cfg.Retries {
			time.Sleep(cfg.RetryDelay)
		}
	}
	return fmt.Errorf("failed to publish to NATS after %d attempts: %s -> %s (%v)", cfg.Retries, subject, cfg.URL, lastErr)
}

func parseNATSAddress(raw string) (host, port string, err error) {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return "", "", errors.New("empty NATS url")
	}
	u, err := url.Parse(raw)
	if err != nil {
		return "", "", fmt.Errorf("invalid NATS url %q: %w", raw, err)
	}
	if u.Host == "" {
		return "", "", fmt.Errorf("invalid NATS url %q: missing host", raw)
	}
	host = u.Hostname()
	port = u.Port()
	if port == "" {
		port = "4222"
	}
	if host == "" {
		return "", "", fmt.Errorf("invalid NATS url %q: missing hostname", raw)
	}
	return host, port, nil
}

func cmdAgentDiffProposed(args []string) error {
	fs := flag.NewFlagSet("agent-diff-proposed", flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	var (
		runDir      = fs.String("run-dir", "", "run directory")
		repoRoot    = fs.String("repo-root", "", "repo root (for out/agents stream)")
		natsURL     = fs.String("nats-url", defaultNATSURL(), "nats url")
		retries     = fs.Int("retries", envInt("NATS_PUB_RETRIES", 3), "publish retries")
		retryDelay  = fs.Duration("retry-delay", envDurationMs("NATS_PUB_RETRY_DELAY_MS", 200), "retry delay")
		connTimeout = fs.Duration("timeout", envDurationSec("NATS_PUB_TIMEOUT_S", 2), "connect timeout")
	)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if strings.TrimSpace(*runDir) == "" {
		return errors.New("agent-diff-proposed requires --run-dir")
	}
	root, err := resolveRepoRoot(*repoRoot)
	if err != nil {
		return err
	}
	return emitDiffProposed(root, *runDir, natsConfig{URL: *natsURL, Retries: *retries, RetryDelay: *retryDelay, ConnTimeout: *connTimeout})
}

type proposedEvent struct {
	EventID      string `json:"event_id"`
	Kind         string `json:"kind"`
	TimeUTC      string `json:"time_utc"`
	AgentID      string `json:"agent_id"`
	RunID        string `json:"run_id"`
	PatchPath    string `json:"patch_path"`
	PatchSHA256  string `json:"patch_sha256"`
	LinesAdded   int    `json:"lines_added"`
	LinesDeleted int    `json:"lines_deleted"`
	Accepted     bool   `json:"accepted"`
}

func emitDiffProposed(repoRoot, runDir string, cfg natsConfig) error {
	baseDir := filepath.Join(runDir, "base")
	workDir := filepath.Join(runDir, "work")
	eventsDir := filepath.Join(runDir, "events")
	if err := os.MkdirAll(eventsDir, 0o755); err != nil {
		return err
	}
	if !isDir(baseDir) || !isDir(workDir) {
		return fmt.Errorf("base/work dirs missing in run dir: %s", runDir)
	}

	patchPath := filepath.Join(eventsDir, "proposal.patch")
	diffCmd := exec.Command("diff", "-ruN", "--exclude=.agent-meta", baseDir, workDir)
	var diffOut bytes.Buffer
	diffCmd.Stdout = &diffOut
	diffCmd.Stderr = os.Stderr
	err := diffCmd.Run()
	if err == nil {
		_ = os.Remove(patchPath)
		fmt.Println("no diff produced")
		return nil
	}
	var exitErr *exec.ExitError
	if !errors.As(err, &exitErr) || exitErr.ExitCode() != 1 {
		return fmt.Errorf("diff command failed: %w", err)
	}
	if err := os.WriteFile(patchPath, diffOut.Bytes(), 0o644); err != nil {
		return err
	}

	addCount, delCount, err := patchLineCounts(patchPath)
	if err != nil {
		return err
	}
	sha, err := fileSHA256Hex(patchPath)
	if err != nil {
		return err
	}
	runID := filepath.Base(filepath.Clean(runDir))
	agentID := sanitizeAgentID(strings.SplitN(runID, "-", 2)[0])
	if agentID == "" {
		agentID = "unknown"
	}
	now := time.Now().UTC().Format(time.RFC3339)
	event := proposedEvent{
		EventID:      fmt.Sprintf("evt-%s-%d", runID, time.Now().Unix()),
		Kind:         "agent.diff.proposed",
		TimeUTC:      now,
		AgentID:      agentID,
		RunID:        runID,
		PatchPath:    patchPath,
		PatchSHA256:  sha,
		LinesAdded:   addCount,
		LinesDeleted: delCount,
		Accepted:     false,
	}

	eventJSONPath := filepath.Join(eventsDir, "proposal.event.json")
	pretty, _ := json.MarshalIndent(event, "", "  ")
	pretty = append(pretty, '\n')
	if err := os.WriteFile(eventJSONPath, pretty, 0o644); err != nil {
		return err
	}
	streamPath := filepath.Join(repoRoot, "out", "agents", "events", "diff-events.ndjson")
	if err := os.MkdirAll(filepath.Dir(streamPath), 0o755); err != nil {
		return err
	}
	compact, _ := json.Marshal(event)
	f, err := os.OpenFile(streamPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	if err != nil {
		return err
	}
	_, _ = f.Write(compact)
	_, _ = f.Write([]byte("\n"))
	_ = f.Close()

	subject := fmt.Sprintf("edgerun.agents.%s.diff.proposed", agentID)
	if err := publishNATS(cfg, subject, compact); err != nil {
		return err
	}
	fmt.Printf("diff event emitted: %s\n", eventJSONPath)
	return nil
}

func cmdAgentDiffAccept(args []string) error {
	fs := flag.NewFlagSet("agent-diff-accept", flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	var (
		input       = fs.String("input", "", "run dir or patch path")
		apply       = fs.Bool("apply", false, "apply patch locally")
		repoRoot    = fs.String("repo-root", "", "repo root")
		natsURL     = fs.String("nats-url", defaultNATSURL(), "nats url")
		subject     = fs.String("subject", envOr("AGENT_DIFF_ACCEPTED_SUBJECT", "edgerun.agents.diff.accepted"), "accepted subject")
		retries     = fs.Int("retries", envInt("NATS_PUB_RETRIES", 3), "publish retries")
		retryDelay  = fs.Duration("retry-delay", envDurationMs("NATS_PUB_RETRY_DELAY_MS", 200), "retry delay")
		connTimeout = fs.Duration("timeout", envDurationSec("NATS_PUB_TIMEOUT_S", 2), "connect timeout")
	)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if strings.TrimSpace(*input) == "" {
		return errors.New("agent-diff-accept requires --input")
	}
	root, err := resolveRepoRoot(*repoRoot)
	if err != nil {
		return err
	}
	cfg := natsConfig{URL: *natsURL, Retries: *retries, RetryDelay: *retryDelay, ConnTimeout: *connTimeout}
	return acceptDiff(root, *input, *apply, *subject, cfg)
}

func cmdCodeUpdatePub(args []string) error {
	fs := flag.NewFlagSet("code-update-pub", flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	var (
		subject     = fs.String("subject", "edgerun.code.updated", "subject")
		revision    = fs.String("revision", "", "revision")
		runID       = fs.String("run-id", "", "run id")
		repoRoot    = fs.String("repo-root", "", "repo root")
		natsURL     = fs.String("nats-url", defaultNATSURL(), "nats url")
		retries     = fs.Int("retries", envInt("NATS_PUB_RETRIES", 3), "publish retries")
		retryDelay  = fs.Duration("retry-delay", envDurationMs("NATS_PUB_RETRY_DELAY_MS", 200), "retry delay")
		connTimeout = fs.Duration("timeout", envDurationSec("NATS_PUB_TIMEOUT_S", 2), "connect timeout")
	)
	if err := fs.Parse(args); err != nil {
		return err
	}
	root, err := resolveRepoRoot(*repoRoot)
	if err != nil {
		return err
	}
	rev := strings.TrimSpace(*revision)
	if rev == "" {
		rev = gitShortHead(root)
	}
	rid := strings.TrimSpace(*runID)
	if rid == "" {
		rid = fmt.Sprintf("manual-%d", time.Now().Unix())
	}
	ev := codeUpdatedEvent{
		EventType: "code_updated",
		Revision:  rev,
		RunID:     rid,
		TimeUTC:   time.Now().UTC().Format(time.RFC3339),
	}
	payload, _ := json.Marshal(ev)
	cfg := natsConfig{URL: *natsURL, Retries: *retries, RetryDelay: *retryDelay, ConnTimeout: *connTimeout}
	if err := publishNATS(cfg, *subject, payload); err != nil {
		return err
	}
	fmt.Printf("published %s: %s\n", *subject, rev)
	return nil
}

func cmdAgentLaunch(args []string) error {
	fs := flag.NewFlagSet("agent-launch", flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	var (
		agentID   = fs.String("agent-id", "", "agent id")
		prompt    = fs.String("prompt", "", "task prompt")
		repoRoot  = fs.String("repo-root", "", "repo root")
		runsRoot  = fs.String("runs-root", "", "runs root")
		mcpURL    = fs.String("mcp-syscall-url", envOr("MCP_SYSCALL_URL", "http://127.0.0.1:7047"), "mcp syscall url")
		codexBin  = fs.String("codex-bin", "/usr/lib/node_modules/@openai/codex/bin/codex.js", "codex js entrypoint")
		codexMod  = fs.String("codex-module", "/usr/lib/node_modules/@openai/codex", "codex module mount")
		nodeImage = fs.String("node-image", "node:22-bookworm", "docker node image")
		natsURL   = fs.String("nats-url", defaultNATSURL(), "nats url")
	)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if strings.TrimSpace(*agentID) == "" || strings.TrimSpace(*prompt) == "" {
		return errors.New("agent-launch requires --agent-id and --prompt")
	}
	root, err := resolveRepoRoot(*repoRoot)
	if err != nil {
		return err
	}
	safeAgentID := sanitizeAgentID(*agentID)
	if safeAgentID == "" {
		return errors.New("invalid agent id")
	}
	rRoot := strings.TrimSpace(*runsRoot)
	if rRoot == "" {
		if env := strings.TrimSpace(os.Getenv("EDGERUN_AGENT_RUNS_ROOT")); env != "" {
			rRoot = env
		} else {
			rRoot = filepath.Join(root, "out", "agents", "runs")
		}
	}
	tsCompact := time.Now().UTC().Format("20060102150405")
	runID := fmt.Sprintf("%s-%s", safeAgentID, tsCompact)
	runDir := filepath.Join(rRoot, runID)
	baseDir := filepath.Join(runDir, "base")
	workDir := filepath.Join(runDir, "work")
	eventsDir := filepath.Join(runDir, "events")
	if err := os.MkdirAll(eventsDir, 0o755); err != nil {
		return err
	}

	if err := buildVirtualView(root, baseDir); err != nil {
		return err
	}
	if err := copyDir(baseDir, workDir); err != nil {
		return err
	}

	home, _ := os.UserHomeDir()
	codexHome := filepath.Join(home, ".codex")
	prefix := strings.Join([]string{
		"You are operating on a virtualized workspace copy without git metadata.",
		"- Do not run git commands.",
		"- Gather context with /edgerun-agent-tools/mcp-context.sh.",
		"- Edit files in /workspace/virtual.",
		"- Keep changes minimal and coherent.",
	}, "\n")
	fullPrompt := fmt.Sprintf("%s\n\nTask:\n%s", prefix, *prompt)
	containerName := fmt.Sprintf("edgerun-agent-%s-%s", safeAgentID, tsCompact)

	dockerArgs := []string{
		"run", "--rm",
		"--name", containerName,
		"--network", "host",
		"-e", "HOME=/root",
		"-e", "MCP_SYSCALL_URL=" + *mcpURL,
		"-v", workDir + ":/workspace/virtual",
		"-v", baseDir + ":/workspace/base:ro",
		"-v", filepath.Join(root, "scripts", "agents") + ":/edgerun-agent-tools:ro",
		"-v", *codexMod + ":/usr/lib/node_modules/@openai/codex:ro",
		"-v", codexHome + ":/root/.codex",
		"-w", "/workspace/virtual",
		*nodeImage,
		"node", *codexBin, "exec", "--skip-git-repo-check", "-C", "/workspace/virtual", fullPrompt,
	}
	dockerCmd := exec.Command("docker", dockerArgs...)
	dockerCmd.Stdout = os.Stdout
	dockerCmd.Stderr = os.Stderr
	if err := dockerCmd.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "agent container failed (%v)\n", err)
		fmt.Fprintf(os.Stderr, "run dir kept at: %s\n", runDir)
		return err
	}

	cfg := natsConfig{
		URL:         *natsURL,
		Retries:     envInt("NATS_PUB_RETRIES", 3),
		RetryDelay:  envDurationMs("NATS_PUB_RETRY_DELAY_MS", 200),
		ConnTimeout: envDurationSec("NATS_PUB_TIMEOUT_S", 2),
	}
	if err := emitDiffProposed(root, runDir, cfg); err != nil {
		return err
	}

	fmt.Println("agent run complete")
	fmt.Printf("run_id: %s\n", runID)
	fmt.Printf("run_dir: %s\n", runDir)
	fmt.Printf("workspace: %s\n", workDir)
	fmt.Printf("proposed patch: %s\n", filepath.Join(eventsDir, "proposal.patch"))
	fmt.Printf("next test: %s/scripts/agents/test-executor.sh %s quick\n", root, workDir)
	fmt.Printf("next accept event: %s/scripts/agents/apply-accepted-diff.sh %s\n", root, runDir)
	fmt.Printf("next local apply (explicit): %s/scripts/agents/apply-accepted-diff.sh --apply %s\n", root, runDir)
	return nil
}

type acceptedEvent struct {
	EventType    string `json:"event_type"`
	RunID        string `json:"run_id"`
	AgentID      string `json:"agent_id"`
	PatchPath    string `json:"patch_path"`
	PatchSHA256  string `json:"patch_sha256"`
	LinesAdded   int    `json:"lines_added"`
	LinesDeleted int    `json:"lines_deleted"`
	TimeUTC      string `json:"time_utc"`
	ApplyLocal   bool   `json:"apply_local"`
}

type codeUpdatedEvent struct {
	EventType string `json:"event_type"`
	Revision  string `json:"revision"`
	RunID     string `json:"run_id"`
	TimeUTC   string `json:"time_utc"`
}

func acceptDiff(repoRoot, input string, apply bool, subject string, cfg natsConfig) error {
	patchPath, runID, agentID, err := resolvePatchInput(input)
	if err != nil {
		return err
	}

	sha, err := fileSHA256Hex(patchPath)
	if err != nil {
		return err
	}
	addCount, delCount, err := patchLineCounts(patchPath)
	if err != nil {
		return err
	}
	now := time.Now().UTC().Format(time.RFC3339)
	ev := acceptedEvent{
		EventType:    "agent_diff_accepted",
		RunID:        runID,
		AgentID:      agentID,
		PatchPath:    patchPath,
		PatchSHA256:  sha,
		LinesAdded:   addCount,
		LinesDeleted: delCount,
		TimeUTC:      now,
		ApplyLocal:   apply,
	}
	compact, _ := json.Marshal(ev)
	if err := publishNATS(cfg, subject, compact); err != nil {
		return err
	}
	fmt.Printf("accepted diff event published: %s sha=%s\n", subject, sha)

	if !apply {
		return nil
	}

	if err := ensureGitClean(repoRoot); err != nil {
		return err
	}
	tmpPatch, err := rewritePatchForRepo(patchPath, repoRoot)
	if err != nil {
		return err
	}
	defer os.Remove(tmpPatch)

	applyCmd := exec.Command("git", "apply", "--reject", "--whitespace=nowarn", tmpPatch)
	applyCmd.Dir = repoRoot
	applyCmd.Stdout = os.Stdout
	applyCmd.Stderr = os.Stderr
	if err := applyCmd.Run(); err != nil {
		return fmt.Errorf("git apply failed: %w", err)
	}
	fmt.Printf("accepted diff applied: %s\n", patchPath)

	codeEv := codeUpdatedEvent{
		EventType: "code_updated",
		Revision:  fmt.Sprintf("diff-%s", sha[:12]),
		RunID:     runID,
		TimeUTC:   time.Now().UTC().Format(time.RFC3339),
	}
	codePayload, _ := json.Marshal(codeEv)
	if err := publishNATS(cfg, "edgerun.code.updated", codePayload); err != nil {
		return err
	}
	fmt.Printf("published edgerun.code.updated: %s\n", codeEv.Revision)
	return nil
}

func resolvePatchInput(input string) (patchPath, runID, agentID string, err error) {
	st, err := os.Stat(input)
	if err != nil {
		return "", "", "", err
	}
	if st.IsDir() {
		patchPath = filepath.Join(input, "events", "proposal.patch")
		runID = filepath.Base(filepath.Clean(input))
		agentID = sanitizeAgentID(strings.SplitN(runID, "-", 2)[0])
		if agentID == "" {
			agentID = "unknown"
		}
	} else {
		patchPath = input
		runID = fmt.Sprintf("accepted-diff-%d", time.Now().Unix())
		agentID = "unknown"
	}
	if _, err := os.Stat(patchPath); err != nil {
		return "", "", "", fmt.Errorf("patch not found: %s", patchPath)
	}
	return patchPath, runID, agentID, nil
}

func patchLineCounts(patchPath string) (add, del int, err error) {
	f, err := os.Open(patchPath)
	if err != nil {
		return 0, 0, err
	}
	defer f.Close()
	s := bufio.NewScanner(f)
	for s.Scan() {
		line := s.Text()
		if strings.HasPrefix(line, "+++") || strings.HasPrefix(line, "---") {
			continue
		}
		if strings.HasPrefix(line, "+") {
			add++
		}
		if strings.HasPrefix(line, "-") {
			del++
		}
	}
	if err := s.Err(); err != nil {
		return 0, 0, err
	}
	return add, del, nil
}

func fileSHA256Hex(path string) (string, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:]), nil
}

func ensureGitClean(repoRoot string) error {
	cmd := exec.Command("git", "diff", "--quiet", "--ignore-submodules", "--")
	cmd.Dir = repoRoot
	if err := cmd.Run(); err != nil {
		return errors.New("working tree is dirty; commit/stash before applying accepted diff")
	}
	cmd = exec.Command("git", "diff", "--cached", "--quiet", "--ignore-submodules", "--")
	cmd.Dir = repoRoot
	if err := cmd.Run(); err != nil {
		return errors.New("working tree is dirty; commit/stash before applying accepted diff")
	}
	return nil
}

func rewritePatchForRepo(patchPath, repoRoot string) (string, error) {
	b, err := os.ReadFile(patchPath)
	if err != nil {
		return "", err
	}
	runWorkPrefix := filepath.ToSlash(filepath.Join(filepath.Dir(patchPath), "..", "work")) + "/"
	repoPrefix := filepath.ToSlash(repoRoot) + "/"
	rewritten := strings.ReplaceAll(string(b), runWorkPrefix, repoPrefix)
	tmp, err := os.CreateTemp("", "accepted-diff-*.patch")
	if err != nil {
		return "", err
	}
	defer tmp.Close()
	if _, err := tmp.WriteString(rewritten); err != nil {
		return "", err
	}
	return tmp.Name(), nil
}

func resolveRepoRoot(explicit string) (string, error) {
	if strings.TrimSpace(explicit) != "" {
		return explicit, nil
	}
	wd, err := os.Getwd()
	if err != nil {
		return "", err
	}
	cur := wd
	for {
		if fileExists(filepath.Join(cur, "Cargo.toml")) && isDir(filepath.Join(cur, "crates")) {
			return cur, nil
		}
		next := filepath.Dir(cur)
		if next == cur {
			break
		}
		cur = next
	}
	return "", errors.New("failed to resolve repo root (pass --repo-root)")
}

func gitShortHead(repoRoot string) string {
	cmd := exec.Command("git", "rev-parse", "--short", "HEAD")
	cmd.Dir = repoRoot
	out, err := cmd.Output()
	if err != nil {
		return "unknown"
	}
	rev := strings.TrimSpace(string(out))
	if rev == "" {
		return "unknown"
	}
	return rev
}

func sanitizeAgentID(v string) string {
	var b strings.Builder
	for _, r := range strings.ToLower(v) {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') || r == '_' || r == '-' {
			b.WriteRune(r)
		}
	}
	return b.String()
}

func buildVirtualView(repoRoot, destDir string) error {
	if _, err := exec.LookPath("rsync"); err != nil {
		return errors.New("rsync is required")
	}
	if err := os.MkdirAll(destDir, 0o755); err != nil {
		return err
	}
	args := []string{
		"-a", "--delete",
		"--exclude=.git/",
		"--exclude=.codex/",
		"--exclude=out/",
		"--exclude=target/",
		"--exclude=frontend/node_modules/",
		"--exclude=**/.DS_Store",
		repoRoot + "/",
		destDir + "/",
	}
	cmd := exec.Command("rsync", args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		return err
	}

	manifestDir := filepath.Join(destDir, ".agent-meta")
	if err := os.MkdirAll(manifestDir, 0o755); err != nil {
		return err
	}
	view := fmt.Sprintf("generated_at=%s\nsource_root=%s\n", time.Now().UTC().Format(time.RFC3339), repoRoot)
	if err := os.WriteFile(filepath.Join(manifestDir, "view.env"), []byte(view), 0o644); err != nil {
		return err
	}
	var files []string
	err := filepath.WalkDir(destDir, func(path string, d os.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if d.IsDir() {
			return nil
		}
		rel, err := filepath.Rel(destDir, path)
		if err != nil {
			return err
		}
		files = append(files, "./"+filepath.ToSlash(rel))
		return nil
	})
	if err != nil {
		return err
	}
	sort.Strings(files)
	content := strings.Join(files, "\n")
	if content != "" {
		content += "\n"
	}
	return os.WriteFile(filepath.Join(manifestDir, "files.txt"), []byte(content), 0o644)
}

func copyDir(src, dst string) error {
	if _, err := exec.LookPath("rsync"); err != nil {
		return errors.New("rsync is required")
	}
	if err := os.MkdirAll(dst, 0o755); err != nil {
		return err
	}
	cmd := exec.Command("rsync", "-a", src+"/", dst+"/")
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func fileExists(path string) bool {
	st, err := os.Stat(path)
	return err == nil && !st.IsDir()
}

func isDir(path string) bool {
	st, err := os.Stat(path)
	return err == nil && st.IsDir()
}

func envInt(name string, d int) int {
	if v := strings.TrimSpace(os.Getenv(name)); v != "" {
		var n int
		if _, err := fmt.Sscanf(v, "%d", &n); err == nil {
			return n
		}
	}
	return d
}

func envDurationMs(name string, d int) time.Duration {
	return time.Duration(envInt(name, d)) * time.Millisecond
}

func envDurationSec(name string, d int) time.Duration {
	return time.Duration(envInt(name, d)) * time.Second
}

func envOr(name, d string) string {
	if v := strings.TrimSpace(os.Getenv(name)); v != "" {
		return v
	}
	return d
}

func fatalf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
