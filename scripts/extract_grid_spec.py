#!/usr/bin/env python3
"""Extract CSS Grid spec: grid properties, track functions, alignment."""
import json, re, sys

def main():
    grid_props = [
        ("display", "grid, inline-grid", "Establishes grid formatting context"),
        ("grid-template-columns", "none | track-list | subgrid", "Column track definitions"),
        ("grid-template-rows", "none | track-list | subgrid", "Row track definitions"),
        ("grid-template-areas", "none | string+", "Named grid areas"),
        ("grid-template", "Template shorthand", "Shorthand for columns + rows + areas"),
        ("grid-auto-columns", "track-size+", "Implicit column size"),
        ("grid-auto-rows", "track-size+", "Implicit row size"),
        ("grid-auto-flow", "row, column, dense, row dense, column dense", "Auto-placement algorithm"),
        ("grid", "Template + auto-flow shorthand", "Shorthand for all template + auto properties"),
        ("grid-column-start", "line name, span, auto", "Column start line"),
        ("grid-column-end", "line name, span, auto", "Column end line"),
        ("grid-row-start", "line name, span, auto", "Row start line"),
        ("grid-row-end", "line name, span, auto", "Row end line"),
        ("grid-column", "start / end shorthand", "Column placement"),
        ("grid-row", "start / end shorthand", "Row placement"),
        ("grid-area", "name or start/end shorthand", "Grid area placement"),
        ("column-gap", "<length-percentage>", "Column gutters"),
        ("row-gap", "<length-percentage>", "Row gutters"),
        ("gap", "<length-percentage>{1,2}", "Shorthand for row + column gap"),
        ("justify-content", "start, end, center, stretch, space-between, space-around, space-evenly", "Inline alignment of grid"),
        ("align-content", "start, end, center, stretch, space-between, space-around, space-evenly", "Block alignment of grid"),
        ("place-content", "align justify", "Shorthand for align-content + justify-content"),
        ("justify-items", "start, end, center, stretch, baseline", "Inline alignment of items"),
        ("align-items", "start, end, center, stretch, baseline", "Block alignment of items"),
        ("place-items", "align justify", "Shorthand for align-items + justify-items"),
        ("justify-self", "auto, start, end, center, stretch, baseline", "Individual inline alignment"),
        ("align-self", "auto, start, end, center, stretch, baseline", "Individual block alignment"),
        ("place-self", "align justify", "Shorthand for align-self + justify-self"),
    ]

    track_functions = [
        ("repeat", "repeat(N, track-list)"),
        ("minmax", "minmax(min, max)"),
        ("fit-content", "fit-content(length)"),
        ("max-content", "max-content"),
        ("min-content", "min-content"),
        ("auto", "auto"),
        ("fr", "<flex>fr"),
    ]

    catalog = {
        "spec": "CSS Grid Layout",
        "spec_date": "2026-04-10",
        "properties": [{"name": n, "values": v, "description": d} for n, v, d in grid_props],
        "track_functions": [{"name": n, "syntax": s} for n, s in track_functions],
    }
    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
