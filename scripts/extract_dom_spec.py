#!/usr/bin/env python3
"""
Extract DOM Living Standard definitions: interfaces, methods, attributes, events.

Usage: python3 scripts/extract_dom_spec.py docs/w3c_specs/dom.spec.whatwg.org_.md > scripts/dom_catalog.json
"""
import json, re, sys

def extract(markdown):
    interfaces = []
    events = []
    constants = []

    # ---- Find IDL interface blocks ----
    # Pattern: interface EventName : ParentName {
    #   ... attributes and methods ...
    # };
    iface_pattern = re.compile(
        r'(interface\s+(\w+)(?:\s*:\s*(\w+))?\s*\{)'
        r'(.*?)'
        r'\};',
        re.DOTALL
    )

    for m in iface_pattern.finditer(markdown):
        iface_name = m.group(2)
        parent = m.group(3) or ""
        body = m.group(4)

        # Extract attributes
        attrs = []
        for attr_m in re.finditer(r'(?:readonly\s+)?attribute\s+(\w+(?:<[^>]+>)?)\s+(\w+)', body):
            attr_type = attr_m.group(1)
            attr_name = attr_m.group(2)
            is_readonly = 'readonly' in body[max(0, attr_m.start()-20):attr_m.start()]
            attrs.append({"name": attr_name, "type": rust_type(attr_type), "readonly": is_readonly})

        # Extract methods
        methods = []
        for method_m in re.finditer(r'(\w+(?:<[^>]+>)?)\s+(\w+)\s*\(([^)]*)\)', body):
            ret_type = method_m.group(1)
            method_name = method_m.group(2)
            params = method_m.group(3).strip()
            # Skip common false positives
            if method_name in ('interface', 'callback', 'extends'):
                continue
            param_list = _parse_idl_params(params)
            methods.append({
                "name": method_name,
                "return_type": rust_type(ret_type),
                "parameters": param_list,
            })

        interfaces.append({
            "name": iface_name,
            "parent": parent,
            "attributes": attrs,
            "methods": methods,
        })

    # ---- Extract event types ----
    event_pattern = re.compile(r'^(event-type|event|eventname):\s*(\w+)', re.MULTILINE | re.IGNORECASE)
    seen_events = set()
    for m in event_pattern.finditer(markdown):
        evt = m.group(2)
        if evt not in seen_events and len(evt) > 2:
            seen_events.add(evt)
            events.append({"name": evt})

    # Add well-known events manually from the spec
    known_events = [
        "abort", "blur", "focus", "click", "dblclick", "mousedown", "mouseup",
        "mousemove", "mouseenter", "mouseleave", "keydown", "keyup", "keypress",
        "load", "unload", "beforeunload", "resize", "scroll", "error",
        "input", "change", "submit", "reset", "DOMContentLoaded", "readystatechange",
        "DOMContentLoaded", "hashchange", "popstate", "pageshow", "pagehide",
        "touchstart", "touchend", "touchmove", "touchcancel",
        "wheel", "contextmenu", "animationstart", "animationend", "animationiteration",
        "transitionend", "pointerdown", "pointerup", "pointermove",
        "drag", "dragstart", "dragend", "dragover", "drop",
        "copy", "cut", "paste", "message", "open", "close",
        "compositionstart", "compositionend", "compositionupdate",
        "beforeinput", "select", "selectstart", "selectionchange",
        "visibilitychange", "fullscreenchange", "fullscreenerror",
        "pointerenter", "pointerleave", "gotpointercapture", "lostpointercapture",
    ]
    for evt in known_events:
        if evt not in seen_events:
            seen_events.add(evt)
            events.append({"name": evt})

    # ---- WebIDL typedefs (useful type aliases) ----
    typedef_pattern = re.compile(r'typedef\s+(\w+)\s+(\w+)', re.MULTILINE)
    for m in typedef_pattern.finditer(markdown):
        constants.append({"kind": "typedef", "alias": m.group(2), "target": m.group(1)})

    return {
        "spec": "DOM Living Standard",
        "spec_date": "2026-04-10",
        "total_interfaces": len(interfaces),
        "total_events": len(events),
        "interfaces": interfaces,
        "events": events,
    }

def rust_type(idl_type):
    mapping = {
        'DOMString': 'String',
        'USVString': 'String',
        'ByteString': 'String',
        'boolean': 'bool',
        'unsigned short': 'u16',
        'unsigned long': 'u32',
        'unsigned long long': 'u64',
        'short': 'i16',
        'long': 'i32',
        'long long': 'i64',
        'float': 'f32',
        'double': 'f64',
        'any': 'JsValue',
        'Node': 'Node',
        'Element': 'Element',
        'Event': 'Event',
        'EventTarget': 'EventTarget',
        'Document': 'Document',
        'HTMLCollection': 'HtmlCollection',
        'NodeList': 'NodeList',
        'HTMLCollectionBase': 'HtmlCollection',
    }
    # Handle generic types like sequence<T>
    if idl_type.startswith('sequence<'):
        inner = idl_type[9:-1]
        return f"Vec<{rust_type(inner)}>"
    if idl_type.startswith('FrozenArray<'):
        inner = idl_type[12:-1]
        return f"Vec<{rust_type(inner)}>"
    return mapping.get(idl_type, idl_type)

def _parse_idl_params(param_str):
    if not param_str.strip():
        return []
    params = []
    for p in param_str.split(','):
        p = p.strip()
        if not p:
            continue
        # Remove optional markers and defaults
        p = p.split('=')[0].strip()
        p = p.replace('[', '').replace(']', '').strip()
        parts = p.split()
        if len(parts) >= 2:
            params.append({"type": rust_type(parts[-2]), "name": parts[-1]})
        elif len(parts) == 1:
            params.append({"type": "unknown", "name": parts[0]})
    return params

def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <dom-spec.md>", file=sys.stderr)
        sys.exit(1)
    with open(sys.argv[1]) as f:
        md = f.read()
    catalog = extract(md)
    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
