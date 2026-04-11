package main

import (
	"os"

	"edgerunrefcore/cmd/edgerun/gen"
	"edgerunrefcore/cmd/edgerun/report"
	"edgerunrefcore/cmd/edgerun/spec"
	"edgerunrefcore/cmd/edgerun/test"
	"edgerunrefcore/cmd/edgerun/tools"
	"github.com/spf13/cobra"
)

func main() {
	root := &cobra.Command{
		Use:   "edgerun",
		Short: "Edgerun development CLI — spec extraction, code generation, testing, and tooling",
	}

	// edgerun spec extract <domain> [flags]
	specCmd := spec.Cmd()
	root.AddCommand(specCmd)

	// edgerun gen <target> [flags]
	genCmd := gen.Cmd()
	root.AddCommand(genCmd)

	// edgerun test <suite> [flags]
	testCmd := test.Cmd()
	root.AddCommand(testCmd)

	// edgerun report <type> [flags]
	reportCmd := report.Cmd()
	root.AddCommand(reportCmd)

	// edgerun tools <action> [flags]
	toolsCmd := tools.Cmd()
	root.AddCommand(toolsCmd)

	if err := root.Execute(); err != nil {
		os.Exit(1)
	}
}
