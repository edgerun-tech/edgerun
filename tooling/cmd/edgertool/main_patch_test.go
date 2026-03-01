package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRewriteRunDiffToGitPatch(t *testing.T) {
	baseDir := "/tmp/run/base"
	workDir := "/tmp/run/work"
	raw := strings.Join([]string{
		"diff -ruN --no-dereference /tmp/run/base/docs/a.md /tmp/run/work/docs/a.md",
		"--- /tmp/run/base/docs/a.md\t2026-03-01 00:00:00.000000000 +0000",
		"+++ /tmp/run/work/docs/a.md\t2026-03-01 00:00:00.000000000 +0000",
		"@@ -1 +1,2 @@",
		" old",
		"+new",
	}, "\n")

	rewritten, err := rewriteRunDiffToGitPatch([]byte(raw), baseDir, workDir)
	if err != nil {
		t.Fatalf("rewriteRunDiffToGitPatch returned error: %v", err)
	}
	text := string(rewritten)
	if !strings.Contains(text, "diff --git a/docs/a.md b/docs/a.md") {
		t.Fatalf("missing git diff header in rewritten patch:\n%s", text)
	}
	if !strings.Contains(text, "--- a/docs/a.md") {
		t.Fatalf("missing rewritten old marker:\n%s", text)
	}
	if !strings.Contains(text, "+++ b/docs/a.md") {
		t.Fatalf("missing rewritten new marker:\n%s", text)
	}
}

func TestNormalizePatchForStorageCreatesTempPatch(t *testing.T) {
	runDir := t.TempDir()
	baseDir := filepath.Join(runDir, "base")
	workDir := filepath.Join(runDir, "work")
	eventsDir := filepath.Join(runDir, "events")
	if err := os.MkdirAll(baseDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(workDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(eventsDir, 0o755); err != nil {
		t.Fatal(err)
	}
	patchPath := filepath.Join(eventsDir, "proposal.patch")
	patch := strings.Join([]string{
		"diff -ruN --no-dereference " + filepath.ToSlash(filepath.Join(baseDir, "docs", "b.md")) + " " + filepath.ToSlash(filepath.Join(workDir, "docs", "b.md")),
		"--- " + filepath.ToSlash(filepath.Join(baseDir, "docs", "b.md")) + "\t2026-03-01 00:00:00.000000000 +0000",
		"+++ " + filepath.ToSlash(filepath.Join(workDir, "docs", "b.md")) + "\t2026-03-01 00:00:00.000000000 +0000",
		"@@ -0,0 +1 @@",
		"+ok",
		"",
	}, "\n")
	if err := os.WriteFile(patchPath, []byte(patch), 0o644); err != nil {
		t.Fatal(err)
	}

	normalizedPath, cleanup, err := normalizePatchForStorage(patchPath)
	if err != nil {
		t.Fatalf("normalizePatchForStorage returned error: %v", err)
	}
	if cleanup == nil {
		t.Fatalf("expected cleanup function for normalized temp patch")
	}
	defer cleanup()
	normalized, err := os.ReadFile(normalizedPath)
	if err != nil {
		t.Fatalf("read normalized patch: %v", err)
	}
	text := string(normalized)
	if !strings.Contains(text, "diff --git a/docs/b.md b/docs/b.md") {
		t.Fatalf("normalized patch missing git header:\n%s", text)
	}
	if strings.Contains(text, filepath.ToSlash(baseDir)) || strings.Contains(text, filepath.ToSlash(workDir)) {
		t.Fatalf("normalized patch still contains run absolute paths:\n%s", text)
	}
}
