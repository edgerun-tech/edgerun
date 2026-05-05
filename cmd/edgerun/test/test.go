package test

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"
)

func Cmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "test",
		Short: "Run test suites (conformance, e2e, mesh, h2spec, miri, fuzz, podman, docker)",
	}

	cmd.AddCommand(&cobra.Command{
		Use:   "conformance",
		Short: "Run v0 conformance corpus tests",
		RunE:  testConformance,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "e2e [--all] [--hardware] [--mesh]",
		Short: "Run end-to-end tests",
		RunE:  testE2E,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "mesh",
		Short: "Run mesh integration test with network namespaces",
		RunE:  testMesh,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "h2spec",
		Short: "Run h2spec HTTP/2 conformance tests",
		RunE:  testH2Spec,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "oci",
		Short: "Run OCI runtime conformance tests",
		RunE:  testOCIRuntime,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "oci-runc <runtime-bin> [pattern-file]",
		Short: "Run runc integration tests against custom runtime",
		Args:  cobra.MinimumNArgs(1),
		RunE:  testOCIRunc,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "oci-rootless <runtime-bin>",
		Short: "Run rootless podman tests",
		Args:  cobra.ExactArgs(1),
		RunE:  testOCIRootless,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "oci-dind",
		Short: "Run Docker-in-Docker test",
		RunE:  testOCIDind,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "miri",
		Short: "Run Miri memory safety checks",
		RunE:  testMiri,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "fuzz [target] [--duration 10s]",
		Short: "Run fuzz targets",
		RunE:  testFuzz,
	})

	cmd.Flags().Bool("all", false, "Run all e2e tests including hardware and mesh")
	cmd.Flags().Bool("hardware", false, "Include hardware capability tests")
	cmd.Flags().Bool("mesh", false, "Include mesh integration tests")
	return cmd
}

func runCmd(dir, name string, args ...string) error {
	cmd := exec.Command(name, args...)
	cmd.Dir = dir
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func runCmdSilent(dir, name string, args ...string) ([]byte, error) {
	cmd := exec.Command(name, args...)
	cmd.Dir = dir
	return cmd.CombinedOutput()
}

func testConformance(cmd *cobra.Command, args []string) error {
	fmt.Println("Running conformance corpus tests...")
	root, err := findRepoRoot()
	if err != nil {
		return err
	}
	return runCmd(root, "cargo", "test", "-p", "edgerun-core", "--", "conformance", "--ignored", "--nocapture")
}

func testE2E(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}

	all, _ := cmd.Flags().GetBool("all")
	hardware, _ := cmd.Flags().GetBool("hardware")
	mesh, _ := cmd.Flags().GetBool("mesh")

	fmt.Println("Building edgerun-node...")
	if err := runCmd(root, "cargo", "build", "-p", "edgerun-node", "--release"); err != nil {
		return err
	}

	fmt.Println("Running edgerun-e2e tests...")
	if err := runCmd(root, "cargo", "test", "-p", "edgerun-e2e", "--", "--ignored"); err != nil {
		return err
	}

	if all || hardware {
		fmt.Println("Running hardware tests...")
		if err := runCmd(root, "cargo", "test", "-p", "edgerun-e2e", "--features", "hardware", "--", "hardware", "--ignored"); err != nil {
			return err
		}
	}

	if all || mesh {
		fmt.Println("Running mesh integration test...")
		return testMesh(cmd, args)
	}

	return nil
}

func testMesh(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}
	script := root + "/scripts/mesh-integration-test.sh"
	if _, err := os.Stat(script); err == nil {
		fmt.Println("Running mesh integration test...")
		return runCmd(root, "bash", script)
	}
	// If script doesn't exist, run the mesh test directly
	fmt.Println("Running mesh namespace tests...")
	return runCmd(root, "cargo", "test", "-p", "edgerun-mesh", "--", "mesh", "--ignored", "--nocapture")
}

func testH2Spec(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}

	fmt.Println("Building h2spec-server...")
	if err := runCmd(root, "cargo", "build", "-p", "edgerun-http", "--features", "tls", "--bin", "h2spec-server"); err != nil {
		return fmt.Errorf("build failed: %w", err)
	}

	// Check h2spec is available
	if _, err := exec.LookPath("h2spec"); err != nil {
		return fmt.Errorf("h2spec not found. Install: go install github.com/summerwind/h2spec/cmd/h2spec@latest")
	}

	fmt.Println("Starting h2spec-server on port 8443...")
	server := exec.Command(filepath.Join(root, "target/debug/h2spec-server"))
	server.Stdout = os.Stdout
	server.Stderr = os.Stderr
	if err := server.Start(); err != nil {
		return err
	}
	defer server.Process.Kill()

	fmt.Println("Running h2spec...")
	return runCmd(root, "h2spec", "-p", "8443", "-t", "--json-path=h2spec-report.json")
}

func testOCIRuntime(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}
	fmt.Println("Running OCI runtime conformance tests...")
	return runCmd(root, "cargo", "test", "-p", "edgerun-oci", "--test", "conformance", "--", "--test-threads=1")
}

func testOCIRunc(cmd *cobra.Command, args []string) error {
	runtimeBin := args[0]
	patternFile := args[1]
	if len(args) < 2 {
		patternFile = "tests/runc/runc_test_pattern"
	}
	root, err := findRepoRoot()
	if err != nil {
		return err
	}
	fmt.Printf("Running runc integration tests with runtime: %s\n", runtimeBin)
	script := filepath.Join(root, "tests/runc/runc_integration_test.sh")
	return runCmd(root, "bash", script, runtimeBin, patternFile)
}

func testOCIRootless(cmd *cobra.Command, args []string) error {
	runtimeBin := args[0]
	root, err := findRepoRoot()
	if err != nil {
		return err
	}
	fmt.Printf("Running rootless podman tests with runtime: %s\n", runtimeBin)
	return runCmd(root, "bash", "tests/rootless-tests/run.sh", runtimeBin)
}

func testOCIDind(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}
	fmt.Println("Running Docker-in-Docker test...")
	return runCmd(root, "bash", "tests/dind/run.sh")
}

func testMiri(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}
	fmt.Println("Installing Miri component...")
	if err := runCmd(root, "rustup", "component", "add", "miri"); err != nil {
		return err
	}
	fmt.Println("Running Miri checks on edgerun-json...")
	os.Setenv("MIRIFLAGS", "-Zmiri-disable-isolation")
	tests := []string{"--lib", "--test", "json_test_suite", "--test", "unicode", "--test", "behavioral_parity"}
	for _, t := range tests {
		fmt.Printf("  miri %s\n", t)
		if err := runCmd(root, "cargo", "miri", "test", t); err != nil {
			fmt.Fprintf(os.Stderr, "WARNING: miri %s failed\n", t)
		}
	}
	return nil
}

func testFuzz(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}

	// Check cargo-fuzz
	if _, err := exec.LookPath("cargo-fuzz"); err != nil {
		fmt.Println("Installing cargo-fuzz...")
		if err := runCmd(root, "cargo", "install", "cargo-fuzz"); err != nil {
			return err
		}
	}

	target := ""
	duration := "10s"
	if len(args) > 0 {
		target = args[0]
	}
	if d := cmd.Flag("duration"); d != nil {
		duration = d.Value.String()
	}

	if target == "" {
		fmt.Println("Running all fuzz targets...")
		targets := []string{"parse", "roundtrip", "tape"}
		for _, t := range targets {
			fmt.Printf("  fuzz %s for %s\n", t, duration)
			if err := runCmd(root, "timeout", duration, "cargo", "fuzz", "run", t); err != nil {
				fmt.Fprintf(os.Stderr, "WARNING: fuzz %s exited\n", t)
			}
		}
	} else {
		fmt.Printf("Running fuzz target %s for %s\n", target, duration)
		return runCmd(root, "timeout", duration, "cargo", "fuzz", "run", target)
	}
	return nil
}

func findRepoRoot() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "Cargo.toml")); err == nil {
			return dir, nil
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}
	return "", fmt.Errorf("could not find repo root (no Cargo.toml)")
}
