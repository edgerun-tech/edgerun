#!/usr/bin/env python3
"""
Generate a complete browser engine crate from spec catalogs.
Outputs: crates/edgerun-browser/
"""
import json, os, re, sys

def load(path):
    with open(path) as f:
        return json.load(f)

def ri(name):
    return name.replace("-", "_")

def rv(name):
    return "".join(p.capitalize() for p in name.replace("-", "_").split("_"))

def classify_cm(raw):
    if not raw: return "Custom"
    l = raw.lower()
    if l in ("nothing", "nothing.", "void"): return "Nothing"
    if "transparent" in l: return "Transparent"
    if l.startswith("text"): return "Text"
    if "metadata content" in l: return "Metadata"
    if "heading" in l: return "Heading"
    if "sectioning" in l: return "Sectioning"
    if "flow content" in l: return "Flow"
    if "phrasing content" in l or "text that is not" in l: return "Phrasing"
    if "embedded content" in l: return "Embedded"
    if "interactive content" in l: return "Interactive"
    if "palatable content" in l: return "Palatable"
    if "script-supporting" in l: return "ScriptSupporting"
    return "Custom"

def w(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(content)

# ===========================================================================
def gen_element_registry(html):
    L = []
    W = L.append
    W("//! HTML Element Registry \u2014 generated from WHATWG HTML Living Standard.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum ContentModel {")
    for cm in ["Flow","Phrasing","Metadata","Heading","Sectioning","Embedded",
               "Interactive","Palatable","ScriptSupporting","Nothing",
               "Transparent","Text","Custom"]:
        W(f"    {cm},")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub struct AttrDef {")
    W("    pub name: &'static str,")
    W("    pub description: &'static str,")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub struct ElementDef {")
    W("    pub tag: &'static str,")
    W("    pub content_model: ContentModel,")
    W("    pub has_global_attributes: bool,")
    W("    pub dom_interface: &'static str,")
    W("    pub specific_attributes: &'static [AttrDef],")
    W("}")
    W("")
    W("pub struct ElementRegistry;")
    W("")
    W("impl ElementRegistry {")
    W("    pub fn by_tag(tag: &str) -> Option<&'static ElementDef> {")
    W("        match tag {")
    for el in html["elements"]:
        t = el["tag_name"]
        W(f'            "{t}" => Some(&E_{ri(t).upper()}),')
    W("            _ => None,")
    W("        }")
    W("    }")
    W("    pub fn all() -> impl Iterator<Item = &'static ElementDef> {")
    W("        ALL.iter().copied()")
    W("    }")
    W("}")
    W("")
    for el in html["elements"]:
        t = el["tag_name"]
        cm = classify_cm(el["content_model"])
        iface = el["dom_interface"] or "HTMLElement"
        ga = str(el.get("has_global_attributes", False)).lower()
        attrs = el.get("element_specific_attributes", [])
        ap = []
        for a in attrs:
            desc = a.get("description", "").replace("\\", "\\\\").replace('"', '\\"')
            ap.append(f'AttrDef {{ name: "{a["name"]}", description: "{desc}" }}')
        alist = ", ".join(ap)
        W(f"pub const E_{ri(t).upper()}: ElementDef = ElementDef {{")
        W(f'    tag: "{t}",')
        W(f"    content_model: ContentModel::{cm},")
        W(f"    has_global_attributes: {ga},")
        W(f'    dom_interface: "{iface}",')
        W(f"    specific_attributes: &[{alist}],")
        W("};")
        W("")
    W("const ALL: &[&ElementDef] = &[")
    for el in html["elements"]:
        t = el["tag_name"]
        W(f"    &E_{ri(t).upper()},")
    W("];")
    return "\n".join(L)

# ===========================================================================
def gen_tokenizer_states():
    states = [
        "Data","RcData","Rawtext","ScriptData","Plaintext","TagOpen",
        "EndTagOpen","TagName","RcDataLessthanSign","RcDataEndTagOpen",
        "RcDataEndTagName","RawtextLessthanSign","RawtextEndTagOpen",
        "RawtextEndTagName","ScriptDataLessthanSign","ScriptDataEndTagOpen",
        "ScriptDataEndTagName","ScriptDataEscapeStart","ScriptDataEscapeStartDash",
        "ScriptDataEscaped","ScriptDataEscapedDash","ScriptDataEscapedDashDash",
        "ScriptDataEscapedLessthanSign","ScriptDataEscapedEndTagOpen",
        "ScriptDataEscapedEndTagName","ScriptDataDoubleEscapeStart",
        "ScriptDataDoubleEscaped","ScriptDataDoubleEscapedDash",
        "ScriptDataDoubleEscapedDashDash","ScriptDataDoubleEscapedLessthanSign",
        "ScriptDataDoubleEscapeEnd","BeforeAttributeName","AttributeName",
        "AfterAttributeName","BeforeAttributeValue","AttributeValueDoubleQuoted",
        "AttributeValueSingleQuoted","AttributeValueUnquoted",
        "AfterAttributeValueQuoted","SelfClosingStartTag","BogusComment",
        "MarkupDeclarationOpen","CommentStart","CommentStartDash","Comment",
        "CommentLessThanSign","CommentLessThanSignBang",
        "CommentLessThanSignBangDash","CommentLessThanSignBangDashDash",
        "CommentEndDash","CommentEnd","CommentEndBang","Doctype",
        "BeforeDoctypeName","DoctypeName","AfterDoctypeName",
        "AfterDoctypePublicKeyword","BeforeDoctypePublicIdentifier",
        "DoctypePublicIdentifierDoubleQuoted","DoctypePublicIdentifierSingleQuoted",
        "AfterDoctypePublicIdentifier","BetweenDoctypePublicAndSystemIdentifiers",
        "AfterDoctypeSystemKeyword","BeforeDoctypeSystemIdentifier",
        "DoctypeSystemIdentifierDoubleQuoted","DoctypeSystemIdentifierSingleQuoted",
        "AfterDoctypeSystemIdentifier","BogusDoctype","CdataSection",
        "CdataSectionBracket","CdataSectionEnd",
        "CharacterReferenceInAttributeValue","NamedCharacterReference",
        "NamedCharacterReferenceStart","NamedCharacterReferenceNoSemicolon",
        "NumericCharacterReference","NumericCharacterReferenceStart",
        "NumericCharacterReferenceEnd","HexademicalCharacterReferenceStart",
        "AmbiguousAmpersand","Eof",
    ]
    L = []
    W = L.append
    W("//! HTML Tokenizer States \u2014 WHATWG \u00a713.2.5.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("extern crate alloc;")
    W("use alloc::{string::String, vec::Vec};")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum TokenizerState {")
    for s in states:
        W(f"    {s},")
    W("}")
    W("")
    W("impl TokenizerState {")
    W("    pub fn initial() -> Self { Self::Data }")
    W("}")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum TokenKind {")
    W("    DoctypeTag,")
    W("    StartTag,")
    W("    EndTag,")
    W("    Comment,")
    W("    Character(u32),")
    W("    Eof,")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub struct Token {")
    W("    pub kind: TokenKind,")
    W("    pub tag_name: String,")
    W("    pub attributes: Vec<(String, String)>,")
    W("    pub self_closing: bool,")
    W("}")
    return "\n".join(L)

# ===========================================================================
def gen_tree_builder():
    modes = [
        "Initial","BeforeHtml","BeforeHead","InHead","InHeadNoscript",
        "AfterHead","InBody","Text","InTable","InTableText",
        "InCaption","InRow","InCell","InColgroup","InTableBody",
        "AfterBody","InFrameset","AfterFrameset","AfterAfterBody",
        "AfterAfterFrameset",
    ]
    L = []
    W = L.append
    W("extern crate alloc;")
    W("use alloc::vec::Vec;")
    W("")
    W = L.append
    W("//! HTML Tree Builder Insertion Modes \u2014 WHATWG \u00a713.2.6.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum InsertionMode {")
    for m in modes:
        W(f"    {m},")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub enum InsertionAction {")
    W("    InsertNode,")
    W("    Ignore,")
    W("    FosterParent,")
    W("    Reprocess(InsertionMode),")
    W("    ChangeMode(InsertionMode),")
    W("    InScope { tag: &'static str, result: bool },")
    W("}")
    W("")
    W("pub struct TreeBuilderState {")
    W("    pub insertion_mode: InsertionMode,")
    W("    pub original_insertion_mode: Option<InsertionMode>,")
    W("    pub frameset_ok: bool,")
    W("    pub head_element: Option<usize>, // index into open_elements")
    W("    pub form_element: Option<usize>,")
    W("    pub template_insertion_modes: Vec<InsertionMode>,")
    W("}")
    W("")
    W("impl TreeBuilderState {")
    W("    pub fn new() -> Self {")
    W("        Self {")
    W("            insertion_mode: InsertionMode::Initial,")
    W("            original_insertion_mode: None,")
    W("            frameset_ok: true,")
    W("            head_element: None,")
    W("            form_element: None,")
    W("            template_insertion_modes: Vec::new(),")
    W("        }")
    W("    }")
    W("}")
    return "\n".join(L)

# ===========================================================================
def gen_property_registry(css):
    props = css["properties"]
    L = []
    W = L.append
    W("//! CSS Property Registry \u2014 generated from W3C CSS specifications.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum CssPropertyId {")
    W("    Unspecified = 0,")
    for i, p in enumerate(props):
        W(f"    {rv(p['name'])} = {i + 1},")
    W("}")
    W("")
    W("impl CssPropertyId {")
    W("    pub fn from_name(name: &str) -> Option<Self> {")
    W("        match name {")
    for p in props:
        W(f'            "{p["name"]}" => Some(CssPropertyId::{rv(p["name"])}),')
    W("            _ => None,")
    W("        }")
    W("    }")
    W("    pub fn name(&self) -> &'static str {")
    W("        match self {")
    for p in props:
        W(f'            CssPropertyId::{rv(p["name"])} => "{p["name"]}",')
    W("        }")
    W("    }")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub struct PropertyDef {")
    W("    pub id: CssPropertyId,")
    W("    pub name: &'static str,")
    W("    pub value_syntax: &'static str,")
    W("    pub initial_value: &'static str,")
    W("    pub inherited: bool,")
    W("    pub animates: bool,")
    W("}")
    W("")
    W("pub struct PropertyRegistry;")
    W("")
    W("impl PropertyRegistry {")
    W("    pub fn by_name(name: &str) -> Option<&'static PropertyDef> {")
    W("        CssPropertyId::from_name(name).and_then(Self::by_id)")
    W("    }")
    W("    pub fn by_id(id: CssPropertyId) -> Option<&'static PropertyDef> {")
    W("        match id {")
    for p in props:
        W(f'            CssPropertyId::{rv(p["name"])} => Some(&P_{ri(p["name"]).upper()}),')
    W("            CssPropertyId::Unspecified => None,")
    W("        }")
    W("    }")
    W("}")
    W("")
    for p in props:
        n = p["name"]
        inh = p.get("inherited", "").lower().startswith("yes")
        init = p.get("initial", "").replace('"', r'\"')
        syn = p.get("value", "").replace('"', r'\"').replace("\\", "\\\\")
        L.append(f'pub const P_{ri(n).upper()}: PropertyDef = PropertyDef {{')
        L.append(f'    id: CssPropertyId::{rv(n)},')
        L.append(f'    name: "{n}",')
        L.append(f'    value_syntax: "{syn}",')
        L.append(f'    initial_value: "{init}",')
        L.append(f'    inherited: {str(inh).lower()},')
        L.append(f'    animates: false,')
        L.append("};")
        L.append("")
    return "\n".join(L)

# ===========================================================================
def gen_value_types(css):
    L = []
    W = L.append
    W("extern crate alloc;")
    W("use alloc::string::String;")
    W("use alloc::vec::Vec;")
    W("")
    W = L.append
    W("//! CSS Value Types \u2014 generated from W3C CSS specifications.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum CssValueType {")
    W("    Keyword, Dimension, Number, Percentage, String, Url, Function, Color,")
    W("}")
    W("")
    W("#[derive(Debug, Clone, PartialEq)]")
    W("pub enum CssValue {")
    W('    Keyword(&\'static str),')
    W('    Dimension(f64, &' + "static str), // (value, unit)")
    W("    Number(f64),")
    W("    Percentage(f64),")
    W("    StringValue(String),")
    W("    Url(String),")
    W("    Function { name: String, args: Vec<CssValue> },")
    W("    Color { r: f64, g: f64, b: f64, a: f64 },")
    W("    List(Vec<CssValue>),")
    W("}")
    W("")
    W("impl CssValue {")
    W('    pub fn value_type(&self) -> CssValueType {')
    W("        match self {")
    W("            Self::Keyword(_) => CssValueType::Keyword,")
    W("            Self::Dimension(_, _) => CssValueType::Dimension,")
    W("            Self::Number(_) => CssValueType::Number,")
    W("            Self::Percentage(_) => CssValueType::Percentage,")
    W("            Self::StringValue(_) => CssValueType::String,")
    W("            Self::Url(_) => CssValueType::Url,")
    W("            Self::Function { .. } => CssValueType::Function,")
    W("            Self::Color { .. } => CssValueType::Color,")
    W("            Self::List(_) => CssValueType::Keyword,")
    W("        }")
    W("    }")
    W('    pub fn is_inherit(&self) -> bool { matches!(self, Self::Keyword("inherit")) }')
    W('    pub fn is_initial(&self) -> bool { matches!(self, Self::Keyword("initial")) }')
    W('    pub fn is_unset(&self) -> bool { matches!(self, Self::Keyword("unset")) }')
    W('    pub fn is_auto(&self) -> bool { matches!(self, Self::Keyword("auto")) }')
    W('    pub fn is_none(&self) -> bool { matches!(self, Self::Keyword("none")) }')
    W("}")
    return "\n".join(L)

# ===========================================================================
def gen_at_rule_registry(css):
    rules = css.get("at_rules", [])
    L = []
    W = L.append
    W("use crate::property_registry::CssPropertyId;")
    W("extern crate alloc;")
    W("use alloc::vec::Vec;")
    W("")
    W = L.append
    W("//! CSS At-Rule Registry \u2014 generated from W3C CSS specifications.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum AtRuleId {")
    W("    Unspecified = 0,")
    for i, r in enumerate(rules):
        W(f"    {rv(r['name'])} = {i + 1},")
    W("}")
    W("")
    W("impl AtRuleId {")
    W("    pub fn from_name(name: &str) -> Option<Self> {")
    W("        match name {")
    for r in rules:
        W(f'            "{r["name"]}" => Some(AtRuleId::{rv(r["name"])}),')
    W("            _ => None,")
    W("        }")
    W("    }")
    W("    pub fn name(&self) -> &'static str {")
    W("        match self {")
    for r in rules:
        W(f'            AtRuleId::{rv(r["name"])} => "@{r["name"]}",')
    W("        }")
    W("    }")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub enum AtRulePrelude {")
    for r in rules:
        W(f"    {rv(r['name'])},")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub struct AtRuleBlock {")
    W("    pub at_rule: AtRuleId,")
    W("    pub declarations: Vec<(CssPropertyId, crate::value_types::CssValue)>,")
    W("}")
    return "\n".join(L)

# ===========================================================================
def gen_js_object_registry(js):
    objs = [o for o in js["built_in_objects"]
            if o["prototype_methods"] or o["static_properties"]]
    header = []
    consts = []
    match_arms = []
    def W(line):
        header.append(line)
    W("//! ECMAScript Built-in Object Registry \u2014 generated from ECMA-262.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum JsObjectId {")
    W("    Unspecified = 0,")
    for i, o in enumerate(objs):
        W(f"    {rv(o['name'])} = {i + 1},")
    W("}")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum JsMemberKind { Method, Getter, Setter, Data }")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub struct JsMemberDef {")
    W("    pub name: &'static str,")
    W("    pub kind: JsMemberKind,")
    W("    pub param_count: u8,")
    W("    pub is_prototype: bool,")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub struct JsObjectDef {")
    W("    pub id: JsObjectId,")
    W("    pub name: &'static str,")
    W("    pub members: &'static [JsMemberDef],")
    W("}")
    W("")
    W("pub struct JsObjectRegistry;")

    for o in objs:
        n = o["name"]
        methods = o.get("prototype_methods", [])
        props = o.get("static_properties", [])
        proto_props = o.get("prototype_properties", [])
        members = []
        for m in methods:
            pc = len(m.get("parameters", []))
            members.append(f'JsMemberDef {{ name: "{m["name"]}", kind: JsMemberKind::Method, param_count: {pc}, is_prototype: true }}')
        for p in props:
            members.append(f'JsMemberDef {{ name: "{p["name"]}", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }}')
        for p in proto_props:
            members.append(f'JsMemberDef {{ name: "{p["name"]}", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }}')
        member_str = ", ".join(members)
        match_arms.append(f'            "{n}" => Some(&JSO_{ri(n).upper()}),')
        consts.append(f"pub const JSO_{ri(n).upper()}: JsObjectDef = JsObjectDef {{")
        consts.append(f"    id: JsObjectId::{rv(n)},")
        consts.append(f'    name: "{n}",')
        consts.append(f"    members: &[{member_str}],")
        consts.append("};")
        consts.append("")

    impl = [
        "",
        "impl JsObjectRegistry {",
        "    pub fn by_name(name: &str) -> Option<&'static JsObjectDef> {",
        "        match name {",
    ]
    impl.extend(match_arms)
    impl.extend([
        '            _ => None,',
        '        }',
        '    }',
        '}',
    ])

    return "\n".join(header + consts + impl)

# ===========================================================================
def gen_js_abstract_ops(js):
    ops = js["abstract_operations"]
    header = []
    consts = []
    match_arms = []
    def W(line):
        header.append(line)
    W("//! ECMAScript Abstract Operations \u2014 generated from ECMA-262.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum AbstractOpId {")
    W("    Unspecified = 0,")
    for i, op in enumerate(ops):
        W(f"    {rv(op['name'])} = {i + 1},")
    W("}")
    W("")
    W("#[derive(Debug, Clone)]")
    W("pub struct AbstractOpDef {")
    W("    pub id: AbstractOpId,")
    W("    pub name: &'static str,")
    W("    pub params: &'static [&'static str],")
    W("}")
    W("")
    W("pub struct AbstractOpRegistry;")

    for op in ops:
        n = op["name"]
        ps = op.get("parameters", [])
        # Escape backslashes and brackets for Rust string
        clean_ps = []
        for p in ps:
            p = p.replace("\\", "\\\\").replace("[", "[").replace("]", "]")
            clean_ps.append(f'"{p}"')
        plist = ", ".join(clean_ps)
        match_arms.append(f'            "{n}" => Some(&AOP_{ri(n).upper()}),')
        consts.append(f"pub const AOP_{ri(n).upper()}: AbstractOpDef = AbstractOpDef {{")
        consts.append(f"    id: AbstractOpId::{rv(n)},")
        consts.append(f'    name: "{n}",')
        consts.append(f"    params: &[{plist}],")
        consts.append("};")
        consts.append("")

    impl = [
        "",
        "impl AbstractOpRegistry {",
        "    pub fn by_name(name: &str) -> Option<&'static AbstractOpDef> {",
        "        match name {",
    ]
    impl.extend(match_arms)
    impl.extend([
        '            _ => None,',
        '        }',
        '    }',
        '}',
    ])

    return "\n".join(header + consts + impl)

# ===========================================================================
def gen_js_globals(js):
    # Deduplicate symbols
    seen = set()
    syms = []
    for s in js["well_known_symbols"]:
        key = s.lower()
        if key not in seen:
            seen.add(key)
            syms.append(s)

    L = []
    W = L.append
    W("//! ECMAScript Well-Known Symbols and Intrinsics \u2014 ECMA-262.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum WellKnownSymbol {")
    for s in syms:
        W(f"    {rv(s)},")
    W("}")
    W("")
    W("impl WellKnownSymbol {")
    W("    pub fn description(&self) -> &'static str {")
    W("        match self {")
    for s in syms:
        W(f'            WellKnownSymbol::{rv(s)} => "Symbol.{s}",')
    W("        }")
    W("    }")
    W("}")
    W("")
    # Deduplicate intrinsics, skip numeric-starting ones
    intrinsics = []
    seen_intr = set()
    for intr in js["well_known_intrinsics"]:
        name = intr.replace(".", "_")
        rn = rv(name)
        if rn not in seen_intr and rn and rn[0].isalpha():
            seen_intr.add(rn)
            intrinsics.append(intr)

    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum IntrinsicId {")
    for i, intr in enumerate(intrinsics[:60]):
        name = intr.replace(".", "_")
        W(f"    {rv(name)} = {i + 1},")
    W("}")
    W("")
    W("impl IntrinsicId {")
    W("    pub fn name(&self) -> &'static str {")
    W("        match self {")
    for intr in intrinsics[:60]:
        name = intr.replace(".", "_")
        W(f'            IntrinsicId::{rv(name)} => "%{intr}%",')
    W("        }")
    W("    }")
    W("}")
    return "\n".join(L)

# ===========================================================================
def gen_dom_hierarchy(html):
    elements = html["elements"]
    ifaces = {}
    for el in elements:
        iface = el["dom_interface"] or "HTMLElement"
        ifaces.setdefault(iface, []).append(el["tag_name"])

    L = []
    W = L.append
    W("extern crate alloc;")
    W("use alloc::string::String;")
    W("use alloc::vec::Vec;")
    W("")
    W = L.append
    W("//! DOM Node Trait Hierarchy \u2014 generated from WHATWG DOM + Web IDL.")
    W("//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py")
    W("")
    W("use crate::element_registry::ElementDef;")
    W("")
    W("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    W("pub enum NodeType {")
    W("    Element = 1, Text = 3, Comment = 8, Document = 9,")
    W("    DocumentType = 10, DocumentFragment = 11,")
    W("}")
    W("")
    W("pub trait Node {")
    W("    fn node_type(&self) -> NodeType;")
    W("    fn node_name(&self) -> &str;")
    W("    fn parent_node(&self) -> Option<&dyn Node>;")
    W("    fn first_child(&self) -> Option<&dyn Node>;")
    W("    fn last_child(&self) -> Option<&dyn Node>;")
    W("    fn previous_sibling(&self) -> Option<&dyn Node>;")
    W("    fn next_sibling(&self) -> Option<&dyn Node>;")
    W("    fn child_nodes(&self) -> Vec<&dyn Node>;")
    W("}")
    W("")
    W("pub trait Element: Node {")
    W("    fn tag_name(&self) -> &str;")
    W("    fn element_def(&self) -> &'static ElementDef;")
    W("    fn get_attribute(&self, name: &str) -> Option<&str>;")
    W("    fn set_attribute(&mut self, name: &str, value: &str);")
    W("    fn remove_attribute(&mut self, name: &str);")
    W("    fn has_attribute(&self, name: &str) -> bool;")
    W("    fn id(&self) -> &str;")
    W("    fn class_list(&self) -> &[String];")
    W("}")
    W("")
    W("pub trait HTMLElement: Element {")
    W("    fn title(&self) -> &str;")
    W("    fn lang(&self) -> &str;")
    W("    fn hidden(&self) -> bool;")
    W("}")
    W("")
    W("pub trait FormAssociated: HTMLElement {")
    W("    fn form(&self) -> Option<&dyn Element>;")
    W("    fn disabled(&self) -> bool;")
    W("}")
    W("")
    W("pub trait Labelable: FormAssociated {}")
    W("")
    W("pub trait MediaElement: HTMLElement {")
    W("    fn src(&self) -> &str;")
    W("    fn autoplay(&self) -> bool;")
    W("    fn controls(&self) -> bool;")
    W("    fn muted(&self) -> bool;")
    W("}")
    W("")
    # Generate specific interface traits
    for iface, tags in sorted(ifaces.items()):
        if iface in ("HTMLElement", "Element", "Node") or len(tags) < 2:
            continue
        if len(L) > 200:  # cap output
            break
        tag_list = ", ".join(f"<{t}>" for t in tags)
        W(f"/// {iface}: {tag_list}")
        W(f"pub trait {iface}: HTMLElement {{")
        W("}")
        W("")
    return "\n".join(L)

# ===========================================================================
def main():
    if len(sys.argv) < 5:
        print(f"Usage: {sys.argv[0]} <html.json> <css.json> <ecma.json> <outdir>", file=sys.stderr)
        sys.exit(1)

    html = load(sys.argv[1])
    css = load(sys.argv[2])
    js = load(sys.argv[3])
    out = sys.argv[4]

    files = {
        "Cargo.toml": '[package]\nname = "edgerun-browser"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "Browser engine core types from WHATWG/W3C/ECMA-262"\n\n[dependencies]\n',
        "src/lib.rs": '''//! edgerun-browser \u2014 Browser engine core types from web specs.
//!
//! Generated from WHATWG HTML, W3C CSS, and ECMA-262 specifications.
#![cfg_attr(not(test), no_std)]

pub mod element_registry;
pub mod tokenizer_states;
pub mod tree_builder;
pub mod property_registry;
pub mod value_types;
pub mod at_rule_registry;
pub mod js_object_registry;
pub mod js_abstract_ops;
pub mod js_globals;
pub mod dom_hierarchy;
''',
        "src/element_registry.rs": gen_element_registry(html),
        "src/tokenizer_states.rs": gen_tokenizer_states(),
        "src/tree_builder.rs": gen_tree_builder(),
        "src/property_registry.rs": gen_property_registry(css),
        "src/value_types.rs": gen_value_types(css),
        "src/at_rule_registry.rs": gen_at_rule_registry(css),
        "src/js_object_registry.rs": gen_js_object_registry(js),
        "src/js_abstract_ops.rs": gen_js_abstract_ops(js),
        "src/js_globals.rs": gen_js_globals(js),
        "src/dom_hierarchy.rs": gen_dom_hierarchy(html),
    }

    total = 0
    for rel, content in files.items():
        path = os.path.join(out, rel)
        w(path, content)
        n = content.count("\n")
        total += n
        print(f"  {rel}: {n} lines")

    print(f"\nDone. {len(files)} files, {total} total lines.")

if __name__ == "__main__":
    main()
