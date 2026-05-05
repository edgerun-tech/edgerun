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
        Short: "Generate reports",
    }

    cmd.AddCommand(&cobra.Command{
        Use:   "api-parity",
        Short: "Generate edgerun-json public API report",
        RunE:  reportAPIParity,
    })

    cmd.AddCommand(&cobra.Command{
        Use:   "audit-rkyv",
        Short: "Report the active rkyv protocol boundary",
        RunE:  reportAuditRkyv,
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

    srcDir := filepath.Join(root, "crates/utility/edgerun-json/src")
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

func reportAuditRkyv(cmd *cobra.Command, args []string) error {
    root, err := findRepoRoot()
    if err != nil {
        return err
    }
    boundary := filepath.Join(root, "crates/edgerun-wire/src/lib.rs")
    fmt.Println("Rkyv Protocol Boundary Report")
    fmt.Println("=============================")
    if _, err := os.Stat(boundary); err != nil {
        return fmt.Errorf("rkyv boundary not found: %w", err)
    }
    fmt.Println("Internal wire protocol: rkyv")
    fmt.Println("Boundary:", boundary)
    return nil
}
