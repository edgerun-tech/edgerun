#!/usr/bin/env python3
"""Extract Selectors Level 4: pseudo-classes, pseudo-elements, combinators, attr selectors."""
import json, re, sys

def _strip(t):
    t = re.sub(r'<[^>]+>', '', t)
    return re.sub(r'\s+', ' ', t.replace('&amp;','&').replace('&lt;','<').replace('&gt;','>').replace('&#39;',"'")).strip()

def main():
    with open(sys.argv[1]) as f:
        md = f.read()

    # Pseudo-classes: extract from spec + supplement with known full list
    pcs = []
    seen = set()
    for m in re.finditer(r'\[§\s*[\d.]+\s+(:[\w-]+)(?:\([^)]*\))?\s*pseudo-class\]', md):
        name = m.group(1)
        if name in seen: continue
        seen.add(name)
        ctx = md[m.start():m.start()+500]
        args = ""
        am = re.search(r'`[^`]*' + re.escape(name) + r'\(([^)]+)\)', ctx)
        if am: args = am.group(1).strip()
        pcs.append({"name": name, "arguments": args, "description": ""})

    # Supplement with full CSS Selectors 4 pseudo-class list
    known_pcs = [
        (":is", "forgiving selector list"),
        (":where", "forgiving selector list"),
        (":not", "forgiving selector list"),
        (":has", "forgiving selector list"),
        (":hover", ""),
        (":active", ""),
        (":focus", ""),
        (":focus-within", ""),
        (":focus-visible", ""),
        (":visited", ""),
        (":link", ""),
        (":target", ""),
        (":target-within", ""),
        (":lang", "language list"),
        (":dir", "ltr or rtl"),
        (":any-link", ""),
        (":local-link", ""),
        (":scope", ""),
        (":defined", ""),
        (":checked", ""),
        (":indeterminate", ""),
        (":default", ""),
        (":valid", ""),
        (":invalid", ""),
        (":in-range", ""),
        (":out-of-range", ""),
        (":required", ""),
        (":optional", ""),
        (":blank", ""),
        (":placeholder-shown", ""),
        (":read-only", ""),
        (":read-write", ""),
        (":disabled", ""),
        (":enabled", ""),
        (":playing", ""),
        (":paused", ""),
        (":current", "selector list"),
        (":past", "selector list"),
        (":future", "selector list"),
        (":host", "selector list"),
        (":host-context", "selector list"),
        (":popover-open", ""),
        (":modal", ""),
        (":fullscreen", ""),
        (":picture-in-picture", ""),
        (":autofill", ""),
        (":user-invalid", ""),
    ]
    for name, args in known_pcs:
        if name not in seen:
            seen.add(name)
            pcs.append({"name": name, "arguments": args, "description": ""})

    # Pseudo-elements: from spec + known full list
    pe_names = set()
    for m in re.finditer(r'::\w[\w-]*', md):
        pe_names.add(m.group(0))
    known_pes = [
        "::before", "::after", "::first-line", "::first-letter",
        "::marker", "::placeholder", "::selection",
        "::backdrop", "::file-selector-button",
        "::column", "::page",
        "::slotted", "::shadow", "::lang", "::part",
    ]
    pes = []
    seen_pe = set()
    for name in list(pe_names) + known_pes:
        if name not in seen_pe:
            seen_pe.add(name)
            pes.append({"name": name})

    # Combinators (from spec text)
    combs = [
        {"name": "descendant", "syntax": " ", "char": "", "description": "E F matches F descendant of E"},
        {"name": "child", "syntax": ">", "char": ">", "description": "E > F matches F direct child of E"},
        {"name": "next_sibling", "syntax": "+", "char": "+", "description": "E + F matches F next sibling of E"},
        {"name": "subsequent_sibling", "syntax": "~", "char": "~", "description": "E ~ F matches F subsequent sibling of E"},
        {"name": "column", "syntax": "||", "char": "||", "description": "E || F matches F in column E"},
    ]

    # Attribute selector operators
    attr_ops = [
        {"operator": "", "name": "exists", "description": "[attr] — element has attribute"},
        {"operator": "=", "name": "equals", "description": '[attr="val"] — exact match'},
        {"operator": "~=", "name": "contains_word", "description": '[attr~="val"] — whitespace-separated word'},
        {"operator": "|=", "name": "prefix_hyphen", "description": '[attr|="val"] — equals or prefix with hyphen'},
        {"operator": "^=", "name": "prefix", "description": '[attr^="val"] — starts with'},
        {"operator": "$=", "name": "suffix", "description": '[attr$="val"] — ends with'},
        {"operator": "*=", "name": "contains", "description": '[attr*="val"] — contains substring'},
        {"modifier": "i", "name": "case_insensitive", "description": 'Case-insensitive match'},
        {"modifier": "s", "name": "case_sensitive", "description": 'Case-sensitive match'},
    ]

    # Specificity
    specificity = [
        {"component": "A", "name": "id_selectors", "description": "Count of ID selectors"},
        {"component": "B", "name": "class_attr_pseudo", "description": "Classes, attributes, pseudo-classes"},
        {"component": "C", "name": "type_pseudo_element", "description": "Type selectors and pseudo-elements"},
    ]

    catalog = {
        "spec": "Selectors Level 4",
        "spec_date": "2026-04-10",
        "total_pseudo_classes": len(pcs),
        "total_pseudo_elements": len(pes),
        "total_combinators": len(combs),
        "total_attr_ops": len(attr_ops),
        "pseudo_classes": pcs,
        "pseudo_elements": pes,
        "combinators": combs,
        "attribute_selector_operators": attr_ops,
        "specificity": specificity,
    }
    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
