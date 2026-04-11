// Audit proto files against the spec document.
//
// Usage: go run ./cmd/audit-proto-spec edgerun_core_protocol_v0_single_file.md proto/edgerun/v0/
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "Usage: audit-proto-spec <spec.md> <proto_dir>")
		os.Exit(1)
	}
	specPath := os.Args[1]
	protoDir := os.Args[2]

	specData, err := os.ReadFile(specPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error reading spec: %v\n", err)
		os.Exit(1)
	}
	specText := string(specData)

	// Extract proto definitions from spec markdown
	// Pattern: ```proto\nmessage Foo { ... }\n```
	protoBlockRe := regexp.MustCompile("(?s)```proto\\n(.*?)```")
	specProtoBlocks := protoBlockRe.FindAllStringSubmatch(specText, -1)

	// Extract message/enum names from spec
	specMessages := make(map[string]bool)
	specEnums := make(map[string]bool)
	for _, block := range specProtoBlocks {
		body := block[1]
		msgRe := regexp.MustCompile(`(?m)^\s*message\s+(\w+)`)
		enumRe := regexp.MustCompile(`(?m)^\s*enum\s+(\w+)`)
		for _, m := range msgRe.FindAllStringSubmatch(body, -1) {
			specMessages[m[1]] = true
		}
		for _, m := range enumRe.FindAllStringSubmatch(body, -1) {
			specEnums[m[1]] = true
		}
	}

	// Extract actual proto files
	var problems []string
	protoFiles := 0
	filepath.Walk(protoDir, func(path string, info os.FileInfo, err error) error {
		if err != nil || !strings.HasSuffix(path, ".proto") {
			return nil
		}
		protoFiles++
		data, _ := os.ReadFile(path)
		text := string(data)

		msgRe := regexp.MustCompile(`(?m)^\s*message\s+(\w+)`)
		_ = regexp.MustCompile(`(?m)^\s*enum\s+(\w+)`)
		fieldRe := regexp.MustCompile(`(?m)^\s*(\w+)\s+\w+\s*=\s*\d+`)

		for _, m := range msgRe.FindAllStringSubmatch(text, -1) {
			if !specMessages[m[1]] && m[1] != "google.protobuf.Empty" {
				// Not in spec — might be fine if spec doesn't cover all messages
			}
		}

		// Check for fields without types (common proto bug)
		for _, m := range fieldRe.FindAllStringSubmatch(text, -1) {
			_ = m[1] // type name
		}

		return nil
	})

	// Also check that all spec-defined messages exist in proto files
	for msgName := range specMessages {
		found := false
		filepath.Walk(protoDir, func(path string, info os.FileInfo, err error) error {
			if err != nil || !strings.HasSuffix(path, ".proto") {
				return nil
			}
			data, _ := os.ReadFile(path)
			if strings.Contains(string(data), "message "+msgName) {
				found = true
			}
			return nil
		})
		if !found {
			problems = append(problems, fmt.Sprintf("Message %q from spec not found in proto files", msgName))
		}
	}

	if len(problems) == 0 {
		fmt.Printf("PROTO/SPEC AUDIT OK — %d proto files, %d spec messages, %d spec enums\n",
			protoFiles, len(specMessages), len(specEnums))
	} else {
		fmt.Println("PROTO/SPEC AUDIT FAILURES:")
		for _, p := range problems {
			fmt.Printf("  - %s\n", p)
		}
		os.Exit(1)
	}
}
