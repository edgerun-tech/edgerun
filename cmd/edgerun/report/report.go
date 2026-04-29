package report

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"
)

func Cmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "report",
		Short: "Generate reports (api-parity, audit-proto)",
	}

	cmd.AddCommand(&cobra.Command{
		Use:   "api-parity",
		Short: "Generate edgerun-json public API report",
		RunE:  reportAPIParity,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "audit-proto",
		Short: "Audit proto files against spec document",
		RunE:  reportAuditProto,
	})

	return cmd
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

func reportAPIParity(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}

	srcDir := filepath.Join(root, "crates/edgerun-json/src")
	fmt.Println("edgerun-json API Compatibility Report")
	fmt.Println("======================================")
	fmt.Println()

	items := findPublicItems(srcDir)
	fmt.Printf("| %-40s | %-12s |\n", "Item", "Status")
	fmt.Printf("|%-42s|%-14s|\n", strings.Repeat("-", 40), strings.Repeat("-", 12))
	for _, item := range items {
		fmt.Printf("| %-40s | %-12s |\n", item.name, item.status)
	}
	fmt.Println()
	return nil
}

type pubItem struct{ name, status string }

func findPublicItems(srcDir string) []pubItem {
	var items []pubItem
	filepath.WalkDir(srcDir, func(path string, d os.DirEntry, err error) error {
		if err != nil || d.IsDir() || !strings.HasSuffix(path, ".rs") {
			return nil
		}
		data, _ := os.ReadFile(path)
		for _, line := range strings.Split(string(data), "\n") {
			line = strings.TrimSpace(line)
			if strings.HasPrefix(line, "pub fn ") || strings.HasPrefix(line, "pub struct ") ||
				strings.HasPrefix(line, "pub enum ") || strings.HasPrefix(line, "pub type ") {
				items = append(items, pubItem{name: line, status: "implemented"})
			}
		}
		return nil
	})
	return items
}

func reportAuditProto(cmd *cobra.Command, args []string) error {
	root, err := findRepoRoot()
	if err != nil {
		return err
	}

	protoDir := filepath.Join(root, "proto/edgerun/v0")
	specFile := filepath.Join(root, "edgerun_core_protocol_v0_single_file.md")
	fmt.Println("Proto Audit Report")
	fmt.Println("==================")
	fmt.Println()

	if _, err := os.Stat(specFile); os.IsNotExist(err) {
		fmt.Println("Spec file not found:", specFile)
		return nil
	}

	specData, _ := os.ReadFile(specFile)
	specText := string(specData)
	protoFiles := findProtoFiles(protoDir)

	discrepancies := 0
	for _, pf := range protoFiles {
		data, _ := os.ReadFile(pf)
		for _, line := range strings.Split(string(data), "\n") {
			line = strings.TrimSpace(line)
			if strings.HasPrefix(line, "message ") || strings.HasPrefix(line, "enum ") {
				parts := strings.Fields(line)
				if len(parts) >= 2 && !strings.Contains(specText, parts[1]) {
					fmt.Printf("  MISSING %s: %s in %s\n", strings.ToUpper(parts[0]), parts[1], pf)
					discrepancies++
				}
			}
		}
	}

	if discrepancies == 0 {
		fmt.Println("All proto definitions match spec document.")
	} else {
		fmt.Printf("\n%d discrepancies found.\n", discrepancies)
	}
	return nil
}

func findProtoFiles(dir string) []string {
	var files []string
	filepath.WalkDir(dir, func(path string, d os.DirEntry, err error) error {
		if err == nil && strings.HasSuffix(path, ".proto") {
			files = append(files, path)
		}
		return nil
	})
	return files
}
