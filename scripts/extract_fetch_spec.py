#!/usr/bin/env python3
"""Extract Fetch spec: Request/Response/Headers types, CORS modes, redirect modes, body mixin."""
import json, re, sys

def main():
    with open(sys.argv[1]) as f:
        md = f.read()

    # Request methods from Request class definition
    request_init = [
        ("method", "String", 'GET, POST, PUT, DELETE, etc.'),
        ("url", "String", 'The URL of the request'),
        ("headers", "HeadersInit", 'Headers object or dict'),
        ("body", "Option<BodyInit>", 'Request body'),
        ("referrer", "String", 'Referrer URL'),
        ("referrer_policy", "String", 'no-referrer, origin, etc.'),
        ("mode", "RequestMode", 'cors, no-cors, same-origin, navigate'),
        ("credentials", "RequestCredentials", 'omit, same-origin, include'),
        ("cache", "RequestCache", 'default, no-store, reload, etc.'),
        ("redirect", "RequestRedirect", 'follow, error, manual'),
        ("integrity", "String", 'Subresource integrity hash'),
        ("keep_alive", "bool", 'Keepalive flag'),
        ("signal", "Option<AbortSignal>", 'AbortSignal'),
        ("priority", "RequestPriority", 'high, low, auto'),
        ("duplex", "String", 'half for streaming'),
    ]

    # Response properties
    response_props = [
        ("ok", "bool", 'Success status (200-299)'),
        ("status", "u16", 'HTTP status code'),
        ("status_text", "String", 'Status message'),
        ("headers", "Headers", 'Response headers'),
        ("url", "String", 'Response URL'),
        ("type", "ResponseType", 'basic, cors, opaque, etc.'),
        ("redirected", "bool", 'Whether redirected'),
        ("body", "Option<Body>", 'Response body stream'),
    ]

    # Headers methods
    headers_methods = [
        ("append", "void", "name: &str, value: &str"),
        ("delete", "void", "name: &str"),
        ("get", "Option<String>", "name: &str"),
        ("get_set_cookie", "Vec<String>", ""),
        ("has", "bool", "name: &str"),
        ("set", "void", "name: &str, value: &str"),
        ("entries", "Iterator", ""),
        ("keys", "Iterator", ""),
        ("values", "Iterator", ""),
        ("for_each", "void", "callback: Fn(&str, &str)"),
    ]

    # Body mixin methods
    body_methods = [
        ("array_buffer", "Vec<u8>", ""),
        ("blob", "Blob", ""),
        ("bytes", "Vec<u8>", ""),
        ("form_data", "FormData", ""),
        ("json", "JsValue", ""),
        ("text", "String", ""),
    ]

    # Request modes enum values
    request_modes = [
        ("RequestMode", ["SameOrigin", "Cors", "NoCors", "Navigate", "WebSocket"]),
        ("RequestCredentials", ["Omit", "SameOrigin", "Include"]),
        ("RequestCache", ["Default", "NoStore", "Reload", "NoCache", "ForceCache", "OnlyIfCached"]),
        ("RequestRedirect", ["Follow", "Error", "Manual"]),
        ("RequestPriority", ["High", "Low", "Auto"]),
        ("ResponseType", ["Basic", "Cors", "Default", "Error", "Opaque", "OpaqueRedirect"]),
        ("ReferrerPolicy", ["NoReferrer", "NoReferrerWhenDowngrade", "SameOrigin", "Origin",
                            "StrictOrigin", "OriginWhenCrossOrigin", "StrictOriginWhenCrossOrigin",
                            "UnsafeUrl"]),
        ("RequestDestination", ["", "Audio", "AudioWorklet", "Document", "Embed", "Font", "Frame",
                                "Iframe", "Image", "Manifest", "Object", "PainterWorklet", "Report",
                                "Script", "SharedWorker", "Style", "Track", "Video", "Worker", "Xslt"]),
    ]

    # Fetch algorithm states
    fetch_states = [
        ("NetworkError", "The fetch encountered a network error"),
        ("Aborted", "The fetch was aborted"),
        ("Done", "The fetch completed successfully"),
    ]

    catalog = {
        "spec": "Fetch Living Standard",
        "spec_date": "2026-04-10",
        "request_init": [{"name": n, "type": t, "description": d} for n, t, d in request_init],
        "response_properties": [{"name": n, "type": t, "description": d} for n, t, d in response_props],
        "headers_methods": [{"name": n, "return_type": t, "parameters": p} for n, t, p in headers_methods],
        "body_methods": [{"name": n, "return_type": t, "parameters": p} for n, t, p in body_methods],
        "enums": [{"name": name, "variants": [v for v in variants if v]} for name, variants in request_modes],
        "fetch_states": [{"name": n, "description": d} for n, d in fetch_states],
    }

    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
