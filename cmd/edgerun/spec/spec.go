package spec

import (
    "fmt"

    "github.com/spf13/cobra"
)

func Cmd() *cobra.Command {
    return &cobra.Command{
        Use:   "spec",
        Short: "removed legacy catalog extractor",
        Long:  "The legacy schema catalog extractor was removed. Internal Edgerun payloads use rkyv only.",
        RunE: func(cmd *cobra.Command, args []string) error {
            return fmt.Errorf("legacy catalog extraction was removed; use the rkyv protocol boundary")
        },
    }
}
