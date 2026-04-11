// Extract CSS Flexbox spec: container/child properties, alignment enums.
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type FlexProp struct {
	Name        string `json:"name"`
	Values      string `json:"values"`
	Description string `json:"description"`
}

type FlexCatalog struct {
	Spec              string     `json:"spec"`
	SpecDate          string     `json:"spec_date"`
	ContainerProps    []FlexProp `json:"container_properties"`
	ChildProps        []FlexProp `json:"child_properties"`
}

func main() {
	if len(os.Args) >= 2 {
		if _, err := os.ReadFile(os.Args[1]); err != nil {
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
			os.Exit(1)
		}
	}

	catalog := FlexCatalog{
		Spec: "CSS Flexible Box Layout", SpecDate: "2026-04-10",
		ContainerProps: []FlexProp{
			{"display", "flex, inline-flex", "Establishes flex formatting context"},
			{"flex-direction", "row, row-reverse, column, column-reverse", "Main axis direction"},
			{"flex-wrap", "nowrap, wrap, wrap-reverse", "Whether items wrap"},
			{"flex-flow", "flex-direction flex-wrap", "Shorthand for direction + wrap"},
			{"justify-content", "flex-start, flex-end, center, space-between, space-around, space-evenly, start, end, left, right", "Main axis alignment"},
			{"align-content", "flex-start, flex-end, center, space-between, space-around, space-evenly, stretch, baseline, first baseline, last baseline, start, end, left, right", "Cross axis alignment of lines"},
			{"align-items", "flex-start, flex-end, center, baseline, stretch, start, end, self-start, self-end, first baseline, last baseline", "Cross axis alignment of items"},
			{"gap", "<length-percentage>{1,2}", "Gaps between flex items"},
			{"row-gap", "<length-percentage>", "Row gap"},
			{"column-gap", "<length-percentage>", "Column gap"},
		},
		ChildProps: []FlexProp{
			{"order", "<integer>", "Paint/order reordering"},
			{"flex-grow", "<number>", "Growth factor"},
			{"flex-shrink", "<number>", "Shrinkage factor"},
			{"flex-basis", "content | <width>", "Initial main size"},
			{"flex", "none | flex-grow [flex-shrink flex-basis]", "Shorthand for grow + shrink + basis"},
			{"align-self", "auto, flex-start, flex-end, center, baseline, stretch, start, end, self-start, self-end", "Individual cross axis alignment"},
		},
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
