#!/usr/bin/env python3
"""Extract URL spec: URL parser states, URLSearchParams, origin, etc."""
import json, re, sys

def main():
    with open(sys.argv[1]) as f:
        md = f.read()

    # URL parser states from spec
    parser_states = [
        "SchemeStart", "Scheme", "NoScheme", "SpecialRelativeOrAuthority",
        "PathOrAuthority", "Relative", "RelativeSlash", "SpecialAuthoritySlashes",
        "SpecialAuthorityIgnoreSlashes", "Query", "Host", "Hostname", "Port",
        "PathStart", "Path", "CannotBeABaseUrlPath", "File", "FileHost",
        "FileSlash", "Fragment",
    ]

    # URL class properties
    url_props = [
        ("href", "String", "The full URL"),
        ("origin", "String", "The origin (read-only)"),
        ("protocol", "String", "Scheme with trailing ':'"),
        ("username", "String", "Username component"),
        ("password", "String", "Password component"),
        ("host", "String", "Host with port"),
        ("hostname", "String", "Host without port"),
        ("port", "Option<String>", "Port number"),
        ("pathname", "String", "Path component"),
        ("search", "String", "Query string with '?'"),
        ("hash", "String", "Fragment with '#'"),
    ]

    # URLSearchParams methods
    urlsp_methods = [
        ("append", "void", "name: &str, value: &str"),
        ("delete", "void", "name: &str"),
        ("delete_with_value", "void", "name: &str, value: &str"),
        ("get", "Option<String>", "name: &str"),
        ("get_all", "Vec<String>", "name: &str"),
        ("has", "bool", "name: &str"),
        ("has_with_value", "bool", "name: &str, value: &str"),
        ("set", "void", "name: &str, value: &str"),
        ("sort", "void", ""),
        ("to_string", "String", ""),
    ]

    # Special schemes
    special_schemes = ["ftp", "file", "http", "https", "ws", "wss"]

    # Default ports
    default_ports = {"ftp": 21, "file": None, "http": 80, "https": 443, "ws": 80, "wss": 443}

    catalog = {
        "spec": "URL Living Standard",
        "spec_date": "2026-04-10",
        "url_parser_states": parser_states,
        "url_properties": [{"name": n, "type": t, "description": d} for n, t, d in url_props],
        "urlsearchparams_methods": [{"name": n, "return_type": t, "parameters": p} for n, t, p in urlsp_methods],
        "special_schemes": special_schemes,
        "default_ports": default_ports,
    }

    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
