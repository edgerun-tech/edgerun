#!/usr/bin/env python3
"""Extract CSS Flexbox spec: container/child properties, alignment enums."""
import json, re, sys

def main():
    # Flex container properties
    container_props = [
        ("display", "flex, inline-flex", "Establishes flex formatting context"),
        ("flex-direction", "row, row-reverse, column, column-reverse", "Main axis direction"),
        ("flex-wrap", "nowrap, wrap, wrap-reverse", "Whether items wrap"),
        ("flex-flow", "flex-direction flex-wrap", "Shorthand for direction + wrap"),
        ("justify-content", "flex-start, flex-end, center, space-between, space-around, space-evenly, start, end, left, right", "Main axis alignment"),
        ("align-content", "flex-start, flex-end, center, space-between, space-around, space-evenly, stretch, baseline, first baseline, last baseline, start, end, left, right", "Cross axis alignment of lines"),
        ("align-items", "flex-start, flex-end, center, baseline, stretch, start, end, self-start, self-end, first baseline, last baseline", "Cross axis alignment of items"),
        ("gap", "<length-percentage>{1,2}", "Gaps between flex items"),
        ("row-gap", "<length-percentage>", "Row gap"),
        ("column-gap", "<length-percentage>", "Column gap"),
    ]
    # Flex child properties
    child_props = [
        ("order", "<integer>", "Paint/order reordering"),
        ("flex-grow", "<number>", "Growth factor"),
        ("flex-shrink", "<number>", "Shrinkage factor"),
        ("flex-basis", "content | <width>", "Initial main size"),
        ("flex", "none | flex-grow [flex-shrink flex-basis]", "Shorthand for grow + shrink + basis"),
        ("align-self", "auto, flex-start, flex-end, center, baseline, stretch, start, end, self-start, self-end", "Individual cross axis alignment"),
    ]

    catalog = {
        "spec": "CSS Flexible Box Layout",
        "spec_date": "2026-04-10",
        "container_properties": [{"name": n, "values": v, "description": d} for n, v, d in container_props],
        "child_properties": [{"name": n, "values": v, "description": d} for n, v, d in child_props],
    }
    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
