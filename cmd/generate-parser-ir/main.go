// generate-parser-ir writes WHATWG HTML spec data as proto IR files.
//
// Usage: go run ./cmd/generate-parser-ir [data-dir]
package main

import (
	"fmt"
	"os"

	"edgerunrefcore/pkg/parserir"
)

func main() {
	dir := "data"
	if len(os.Args) > 1 {
		dir = os.Args[1]
	}
	os.MkdirAll(dir, 0755)

	parserir.Generate(dir)
	fmt.Println("Parser IR generated successfully.")
}
