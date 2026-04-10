#!/usr/bin/env python3
"""Generate edgerun-fetch, edgerun-url, edgerun-encoding, edgerun-uievents from catalogs."""
import json, os, re, sys

def load(p):
    with open(p) as f: return json.load(f)

def ri(n): return n.replace("-","_")
def rv(n): return "".join(p.capitalize() for p in re.sub(r'(?<!^)(?=[A-Z])','_',n).replace("-","_").split("_"))

def w(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f: f.write(content)

# ===========================================================================
# FETCH
# ===========================================================================
def gen_fetch(cat):
    files = {}
    files["Cargo.toml"] = '[package]\nname = "edgerun-fetch"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "Fetch Living Standard types"\n\n[dependencies]\n'

    init = cat["request_init"]
    enums = cat["enums"]
    headers = cat["headers_methods"]
    body = cat["body_methods"]
    response = cat["response_properties"]

    L = ["//! Fetch types \u2014 from WHATWG Fetch Living Standard.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform.py", "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
         "pub enum RequestMode {"]
    for v in enums[0]["variants"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum RequestCredentials {")
    for v in enums[1]["variants"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum RequestCache {")
    for v in enums[2]["variants"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum RequestRedirect {")
    for v in enums[3]["variants"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum ResponseType {")
    for v in enums[5]["variants"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    # RequestInit struct
    L.append("#[derive(Debug, Clone, Default)]")
    L.append("pub struct RequestInit {")
    for f in init:
        ft = f["type"]
        L.append(f"    pub {ri(f['name'])}: {ft},")
    L.extend(["}", ""])

    # Response struct
    L.append("#[derive(Debug, Clone)]")
    L.append("pub struct Response {")
    for f in response:
        ft = f["type"]
        L.append(f"    pub {ri(f['name'])}: {ft},")
    L.extend(["}", ""])

    # Headers methods trait
    L.append("pub trait HeadersOps {")
    for m in headers:
        params = ", ".join(m.get("parameters", "").split(", ")) if m.get("parameters") else ""
        rt = m.get("return_type", "void")
        if rt == "void":
            L.append(f"    fn {ri(m['name'])}(&mut self{', ' + params if params else ''});")
        else:
            L.append(f"    fn {ri(m['name'])}(&self{', ' + params if params else ''}) -> {rt};")
    L.extend(["}", ""])

    # Body trait
    L.append("/// Body mixin methods.")
    L.append("pub trait BodyOps {")
    for m in body:
        rt = m.get("return_type", "Vec<u8>")
        L.append(f"    fn {ri(m['name'])}(&self) -> {rt};")
    L.extend(["}", ""])

    files["src/lib.rs"] = "\n".join(L)
    return files

# ===========================================================================
# URL
# ===========================================================================
def gen_url(cat):
    files = {}
    files["Cargo.toml"] = '[package]\nname = "edgerun-url"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "URL Living Standard types"\n\n[dependencies]\n'

    states = cat["url_parser_states"]
    props = cat["url_properties"]
    sp_methods = cat["urlsearchparams_methods"]
    special = cat["special_schemes"]
    dports = cat["default_ports"]

    L = ["//! URL types \u2014 from WHATWG URL Living Standard.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform.py", "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
         "pub enum UrlParserState {"]
    for s in states:
        L.append(f"    {rv(s)},")
    L.extend(["}", ""])

    L.append("#[derive(Debug, Clone)]")
    L.append("pub struct Url {")
    for p in props:
        L.append(f"    pub {ri(p['name'])}: {p['type']},")
    L.extend(["}", ""])

    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum SpecialScheme {")
    for s in special:
        L.append(f"    {rv(s)},")
    L.extend(["}", ""])

    L.append("impl SpecialScheme {")
    L.append("    pub fn default_port(&self) -> Option<u16> {")
    L.append("        match self {")
    for s in special:
        port = dports.get(s)
        L.append(f'            SpecialScheme::{rv(s)} => {port if port else "None"},')
    L.extend(["        }", "    }", "}", ""])

    L.append("/// URLSearchParams operations.")
    L.append("pub trait URLSearchParamsOps {")
    for m in sp_methods:
        params = m.get("parameters", "")
        rt = m.get("return_type", "void")
        if rt == "void":
            L.append(f"    fn {ri(m['name'])}(&mut self{', ' + params if params else ''});")
        else:
            L.append(f"    fn {ri(m['name'])}(&self{', ' + params if params else ''}) -> {rt};")
    L.extend(["}", ""])

    files["src/lib.rs"] = "\n".join(L)
    return files

# ===========================================================================
# ENCODING
# ===========================================================================
def gen_encoding(cat):
    files = {}
    files["Cargo.toml"] = '[package]\nname = "edgerun-encoding"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "Encoding Living Standard types"\n\n[dependencies]\n'

    encodings = cat["encodings"]
    boms = cat["bom_table"]

    L = ["//! Encoding types \u2014 from WHATWG Encoding Living Standard.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform.py", "",
         "/// All encoding types defined by the Encoding Standard.",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
         "pub enum Encoding {"]
    for enc in encodings:
        L.append(f"    /// {enc['name']}" + (f" (labels: {', '.join(enc['labels'][:3])}...)" if len(enc["labels"]) > 3 else f" (labels: {', '.join(enc['labels'])})"))
        L.append(f"    {rv(enc['name'])},")
    L.extend(["}", ""])

    L.append("impl Encoding {")
    L.append("    /// Look up an encoding by label (case-insensitive).")
    L.append("    pub fn from_label(label: &str) -> Option<Self> {")
    L.append("        match label.to_lowercase().as_str() {")
    for enc in encodings:
        for label in enc["labels"]:
            L.append(f'            "{label}" => Some(Encoding::{rv(enc["name"])}),')
    L.extend(['            _ => None,', '        }', '    }', '}', ''])

    L.append("/// BOM (Byte Order Mark) table.")
    L.append("pub const BOM_TABLE: &[(Encoding, &[u8])] = &[")
    for bom in boms:
        bytes_hex = ", ".join(f"0x{b:02X}" for b in bom["bytes"])
        L.append(f"    (Encoding::{rv(bom['encoding'])}, &[{bytes_hex}]),")
    L.extend(["];", ""])

    L.append("/// Check if the given bytes start with a BOM.")
    L.append("pub fn detect_bom(bytes: &[u8]) -> Option<(Encoding, usize)> {")
    L.append("    for &(enc, bom) in BOM_TABLE {")
    L.append("        if bytes.starts_with(bom) {")
    L.append("            return Some((enc, bom.len()));")
    L.append("        }")
    L.append("    }")
    L.append("    None")
    L.append("}")

    files["src/lib.rs"] = "\n".join(L)
    return files

# ===========================================================================
# UI EVENTS
# ===========================================================================
def gen_uievents(cat):
    files = {}
    files["Cargo.toml"] = '[package]\nname = "edgerun-uievents"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "UI Events types from W3C UI Events spec"\n\n[dependencies]\n'

    keys = cat["keyboard_keys"]
    buttons = cat["mouse_buttons"]
    deltas = cat["wheel_delta_modes"]
    modifiers = cat["modifier_keys"]

    L = ["//! UI Events types \u2014 from W3C UI Events specification.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform.py", "",
         "/// KeyboardEvent key values.",
         "#[derive(Debug, Clone, PartialEq, Eq, Hash)]",
         "pub enum Key {"]
    for k in keys:
        if k.isascii() and len(k) == 1 and k.isprintable():
            L.append(f"    /// Key `{k}`")
            if k in ('"', '\\'):
                L.append(f"    Char('{k}'),")
            else:
                L.append(f"    Char('{k}'),")
        else:
            L.append(f"    /// {k}")
            L.append(f"    {rv(k)},")
    L.extend(["}", ""])

    L.append("/// Mouse button values.")
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum MouseButton {")
    for b in buttons:
        L.append(f"    /// {b['value']}: {b['description']}")
        L.append(f"    {rv(b['name'])} = {b['value']},")
    L.extend(["}", ""])

    L.append("/// WheelEvent delta modes.")
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum DeltaMode {")
    for d in deltas:
        L.append(f"    /// {d['value']}: {d['description']}")
        L.append(f"    {rv(d['name'])} = {d['value']},")
    L.extend(["}", ""])

    L.append("/// Modifier key names.")
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    L.append("pub enum ModifierKey {")
    for m in modifiers:
        L.append(f"    {rv(m)},")
    L.extend(["}", ""])

    files["src/lib.rs"] = "\n".join(L)
    return files

# ===========================================================================
def main():
    if len(sys.argv) < 6:
        print(f"Usage: {sys.argv[0]} <fetch.json> <url.json> <encoding.json> <uievents.json> <out_dir>", file=sys.stderr)
        sys.exit(1)

    fetch_cat = load(sys.argv[1])
    url_cat = load(sys.argv[2])
    encoding_cat = load(sys.argv[3])
    uievents_cat = load(sys.argv[4])
    out = sys.argv[5]

    for rel, content in gen_fetch(fetch_cat).items():
        p = os.path.join(out, "edgerun-fetch", rel)
        w(p, content)
        print(f"  edgerun-fetch/{rel}: {content.count(chr(10))} lines")

    for rel, content in gen_url(url_cat).items():
        p = os.path.join(out, "edgerun-url", rel)
        w(p, content)
        print(f"  edgerun-url/{rel}: {content.count(chr(10))} lines")

    for rel, content in gen_encoding(encoding_cat).items():
        p = os.path.join(out, "edgerun-encoding", rel)
        w(p, content)
        print(f"  edgerun-encoding/{rel}: {content.count(chr(10))} lines")

    for rel, content in gen_uievents(uievents_cat).items():
        p = os.path.join(out, "edgerun-uievents", rel)
        w(p, content)
        print(f"  edgerun-uievents/{rel}: {content.count(chr(10))} lines")

    print("\nDone.")

if __name__ == "__main__":
    main()
