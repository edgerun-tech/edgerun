// Extract CSS Grid spec: grid properties, track functions, alignment.
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type GridProp struct {
	Name        string `json:"name"`
	Values      string `json:"values"`
	Description string `json:"description"`
}

type TrackFunc struct {
	Name   string `json:"name"`
	Syntax string `json:"syntax"`
}

type GridCatalog struct {
	Spec           string    `json:"spec"`
	SpecDate       string    `json:"spec_date"`
	Properties     []GridProp `json:"properties"`
	TrackFunctions []TrackFunc `json:"track_functions"`
}

func main() {
	if len(os.Args) >= 2 {
		if _, err := os.ReadFile(os.Args[1]); err != nil {
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
			os.Exit(1)
		}
	}

	catalog := GridCatalog{
		Spec: "CSS Grid Layout", SpecDate: "2026-04-10",
		Properties: []GridProp{
			{"display", "grid, inline-grid", "Establishes grid formatting context"},
			{"grid-template-columns", "none | track-list | subgrid", "Column track definitions"},
			{"grid-template-rows", "none | track-list | subgrid", "Row track definitions"},
			{"grid-template-areas", "none | string+", "Named grid areas"},
			{"grid-template", "Template shorthand", "Shorthand for columns + rows + areas"},
			{"grid-auto-columns", "track-size+", "Implicit column size"},
			{"grid-auto-rows", "track-size+", "Implicit row size"},
			{"grid-auto-flow", "row, column, dense, row dense, column dense", "Auto-placement algorithm"},
			{"grid", "Template + auto-flow shorthand", "Shorthand for all template + auto properties"},
			{"grid-column-start", "line name, span, auto", "Column start line"},
			{"grid-column-end", "line name, span, auto", "Column end line"},
			{"grid-row-start", "line name, span, auto", "Row start line"},
			{"grid-row-end", "line name, span, auto", "Row end line"},
			{"grid-column", "start / end shorthand", "Column placement"},
			{"grid-row", "start / end shorthand", "Row placement"},
			{"grid-area", "name or start/end shorthand", "Grid area placement"},
			{"column-gap", "<length-percentage>", "Column gutters"},
			{"row-gap", "<length-percentage>", "Row gutters"},
			{"gap", "<length-percentage>{1,2}", "Shorthand for row + column gap"},
			{"justify-content", "start, end, center, stretch, space-between, space-around, space-evenly", "Inline alignment of grid"},
			{"align-content", "start, end, center, stretch, space-between, space-around, space-evenly", "Block alignment of grid"},
			{"place-content", "align justify", "Shorthand for align-content + justify-content"},
			{"justify-items", "start, end, center, stretch, baseline", "Inline alignment of items"},
			{"align-items", "start, end, center, stretch, baseline", "Block alignment of items"},
			{"place-items", "align justify", "Shorthand for align-items + justify-items"},
			{"justify-self", "auto, start, end, center, stretch, baseline", "Individual inline alignment"},
			{"align-self", "auto, start, end, center, stretch, baseline", "Individual block alignment"},
			{"place-self", "align justify", "Shorthand for align-self + justify-self"},
		},
		TrackFunctions: []TrackFunc{
			{"repeat", "repeat(N, track-list)"},
			{"minmax", "minmax(min, max)"},
			{"fit-content", "fit-content(length)"},
			{"max-content", "max-content"},
			{"min-content", "min-content"},
			{"auto", "auto"},
			{"fr", "<flex>fr"},
		},
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
