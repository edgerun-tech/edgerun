package tools

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/spf13/cobra"
)

func Cmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "tools",
		Short: "Development tooling (setup, release, demo)",
	}

	cmd.AddCommand(&cobra.Command{
		Use:   "setup",
		Short: "Set up development environment (git hooks, check tools)",
		RunE:  toolsSetup,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "release <version>",
		Short: "Release a crate (update version, changelog, tag, push)",
		Args:  cobra.ExactArgs(1),
		RunE:  toolsRelease,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "demo capability",
		Short: "Run TCP capability demo (server + client)",
		RunE:  toolsDemoCapability,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "check-deps",
		Short: "Check for recommended dev tools",
		RunE:  toolsCheckDeps,
	})

	return cmd
}

func toolsSetup(cmd *cobra.Command, args []string) error {
	root := findRepoRoot()
	fmt.Println("Setting up development environment...")

	hooksDir := filepath.Join(root, ".githooks")
	if _, err := os.Stat(hooksDir); err == nil {
		exec.Command("git", "config", "core.hooksPath", ".githooks").Run()
		fmt.Println("  Git hooks configured: .githooks/")
	}

	out, _ := exec.Command("rustc", "--version").CombinedOutput()
	fmt.Printf("  Rust: %s", out)
	return toolsCheckDeps(cmd, args)
}

func toolsRelease(cmd *cobra.Command, args []string) error {
	version := args[0]
	root := findRepoRoot()
	fmt.Printf("Releasing version %s\n", version)

	out, _ := exec.Command("git", "status", "--porcelain").CombinedOutput()
	if len(out) > 0 {
		return fmt.Errorf("working directory is not clean")
	}

	fmt.Println("  Running tests...")
	if err := runCmd(root, "cargo", "test"); err != nil {
		return fmt.Errorf("tests failed: %w", err)
	}

	fmt.Println("  Building release...")
	if err := runCmd(root, "cargo", "build", "--release"); err != nil {
		return fmt.Errorf("build failed: %w", err)
	}

	fmt.Println("  Committing and tagging...")
	exec.Command("git", "add", "-A").Run()
	exec.Command("git", "commit", "-m", fmt.Sprintf("release: v%s", version)).Run()
	exec.Command("git", "tag", "-a", fmt.Sprintf("v%s", version), "-m", fmt.Sprintf("Version %s", version)).Run()

	fmt.Printf("\nRelease v%s created. Run: git push && git push --tags\n", version)
	return nil
}

func toolsDemoCapability(cmd *cobra.Command, args []string) error {
	addr := "127.0.0.1:8080"
	fmt.Printf("Starting capability demo server on %s\n", addr)

	serverLog, _ := os.Create("/tmp/edgerun-capability-server.log")
	defer serverLog.Close()

	server := exec.Command("cargo", "run", "-p", "edgerun-remote-capability", "--bin", "capability-demo-server", "--", addr)
	server.Stdout = serverLog
	server.Stderr = serverLog
	if err := server.Start(); err != nil {
		return err
	}
	defer server.Process.Kill()

	fmt.Println("  Running client...")
	clientLog, _ := os.Create("/tmp/edgerun-capability-client.log")
	defer clientLog.Close()
	client := exec.Command("cargo", "run", "-p", "edgerun-remote-capability", "--bin", "capability-demo-client", "--", addr)
	client.Stdout = clientLog
	client.Stderr = clientLog
	client.Run()

	fmt.Printf("\nServer log: /tmp/edgerun-capability-server.log\nClient log: /tmp/edgerun-capability-client.log\n")
	return nil
}

func toolsCheckDeps(cmd *cobra.Command, args []string) error {
	fmt.Println("\nChecking development dependencies...")
	tools := map[string]string{
		"cargo-fuzz":  "cargo install cargo-fuzz",
		"cargo-miri":  "rustup component add miri",
		"cargo-audit": "cargo install cargo-audit",
	}
	for tool, install := range tools {
		if _, err := exec.LookPath(tool); err == nil {
			fmt.Printf("  ✓ %s\n", tool)
		} else {
			fmt.Printf("  ✗ %s (install: %s)\n", tool, install)
		}
	}
	return nil
}

func findRepoRoot() string {
	dir, _ := os.Getwd()
	for {
		if _, err := os.Stat(filepath.Join(dir, "Cargo.toml")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}
	return "."
}

func runCmd(dir, name string, args ...string) error {
	cmd := exec.Command(name, args...)
	cmd.Dir = dir
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}
