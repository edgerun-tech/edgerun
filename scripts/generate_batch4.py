#!/usr/bin/env python3
"""Generate edgerun-images, edgerun-fonts, edgerun-media-queries, edgerun-css-syntax."""
import json, os

def load(p):
    with open(p) as f: return json.load(f)

def rv(n):
    s = n.replace('-','_').replace(' ','_')
    return ''.join(p.capitalize() for p in s.split('_'))

def w(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f: f.write(content)

def gen_toml(name):
    return f'[package]\nname = "edgerun-{name}"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "CSS {name} types generated from W3C spec"\n\n[dependencies]\n'

# ===========================================================================
# CSS Images
# ===========================================================================
images = load('scripts/css_images_catalog.json')
L = ['//! CSS Images types \u2014 from CSS Images Level 4.',
     '//! DO NOT EDIT. Regenerate with: scripts/generate_batch4.py',
     '#![cfg_attr(not(test), no_std)]', '',
     'extern crate alloc;',
     'use alloc::{string::String, vec::Vec};', '']

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum GradientType {')
for g in images['gradient_types']:
    L.append(f'    /// {g[2]}')
    L.append(f'    {rv(g[0])},')
L.extend(['}', ''])

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum ShapeType {')
for s in images['shape_types']:
    L.append(f'    {rv(s[0])},')
L.extend(['}', ''])

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum SizeKeyword {')
for s in images['size_keywords']:
    L.append(f'    {rv(s[0])},')
L.extend(['}', ''])

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum SideOrCorner {')
for s in images['side_keywords']:
    L.append(f'    {rv(s[0].replace(" ", "_"))},')
L.extend(['}', ''])

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum ImageType {')
for i in images['image_types']:
    L.append(f'    /// {i[1]}')
    L.append(f'    {rv(i[0])},')
L.extend(['}', ''])

L.extend([
    '/// A CSS image value.',
    '#[derive(Debug, Clone, PartialEq)]',
    'pub enum CssImage {',
    '    Url(String),',
    '    LinearGradient { angle: Option<String>, stops: Vec<CssColorStop> },',
    '    RadialGradient { shape: Option<ShapeType>, size: Option<SizeKeyword>, position: Option<String>, stops: Vec<CssColorStop> },',
    '    ConicGradient { from_angle: Option<String>, position: Option<String>, stops: Vec<CssColorStop> },',
    '    Image(Vec<String>),',
    '    ImageSet(Vec<ImageSetEntry>),',
    '    CrossFade(Vec<CssImage>),',
    '    Paint(String),',
    '}',
    '',
    '#[derive(Debug, Clone, PartialEq)]',
    'pub struct CssColorStop { pub color: String, pub position: Option<String> }',
    '',
    '#[derive(Debug, Clone, PartialEq)]',
    'pub struct ImageSetEntry { pub url: String, pub resolution: Option<String>, pub media: Option<String> }',
])
w('crates/edgerun-images/src/lib.rs', '\n'.join(L))
w('crates/edgerun-images/Cargo.toml', gen_toml('images'))
print(f'edgerun-images: {len(images["gradient_types"])} gradients, {len(images["image_types"])} image types')

# ===========================================================================
# CSS Fonts
# ===========================================================================
fonts = load('scripts/css_fonts_catalog.json')
L = ['//! CSS Fonts types \u2014 from CSS Fonts Level 4.',
     '//! DO NOT EDIT. Regenerate with: scripts/generate_batch4.py',
     '#![cfg_attr(not(test), no_std)]', '',
     'extern crate alloc;',
     'use alloc::{string::String, vec::Vec};', '']

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum FontDisplay {')
for fd in fonts['font_display_values']:
    L.append(f'    /// {fd[1]}')
    L.append(f'    {rv(fd[0])},')
L.extend(['}', ''])

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum FontVariantLigatures {')
for v in fonts['font_variant_ligatures']:
    L.append(f'    {rv(v)},')
L.extend(['}', ''])

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum FontVariantCaps {')
for v in fonts['font_variant_caps']:
    L.append(f'    {rv(v)},')
L.extend(['}', ''])

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum FontWeight {')
for fw in fonts['font_weight_keywords']:
    L.append(f'    /// {fw[1]}')
    L.append(f'    {rv(fw[0])},')
L.extend(['}', ''])

L.extend([
    '#[derive(Debug, Clone)]',
    'pub struct FontFaceDescriptor {',
    '    pub src: Vec<String>,',
    '    pub font_family: String,',
    '    pub font_style: Option<String>,',
    '    pub font_weight: Option<String>,',
    '    pub font_stretch: Option<String>,',
    '    pub font_display: Option<FontDisplay>,',
    '    pub unicode_range: Option<String>,',
    '    pub font_variant: Option<String>,',
    '    pub font_feature_settings: Option<String>,',
    '}',
])
w('crates/edgerun-fonts/src/lib.rs', '\n'.join(L))
w('crates/edgerun-fonts/Cargo.toml', gen_toml('fonts'))
print(f'edgerun-fonts: {len(fonts["font_face_descriptors"])} descriptors, {len(fonts["font_display_values"])} display values')

# ===========================================================================
# Media Queries
# ===========================================================================
mq = load('scripts/css_media_catalog.json')
L = ['//! CSS Media Queries types \u2014 from Media Queries Level 4.',
     '//! DO NOT EDIT. Regenerate with: scripts/generate_batch4.py',
     '#![cfg_attr(not(test), no_std)]', '',
     'extern crate alloc;',
     'use alloc::{string::String, vec::Vec, boxed::Box};', '']

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum MediaType {')
for t in mq['media_types']:
    L.append(f'    {rv(t)},')
L.extend(['}', ''])

L.append('#[derive(Debug, Clone, Copy, PartialEq, Eq)]')
L.append('pub enum MediaFeature {')
for f in mq['media_features']:
    L.append(f'    /// {f[2]}')
    L.append(f'    {rv(f[0])},')
L.extend(['}', ''])

L.append('impl MediaFeature {')
L.append('    pub fn name(&self) -> &\'static str {')
L.append('        match self {')
for f in mq['media_features']:
    L.append(f'            MediaFeature::{rv(f[0])} => "{f[0]}",')
L.extend(['        }', '    }', '}', ''])

L.extend([
    '#[derive(Debug, Clone, PartialEq)]',
    'pub enum MediaCondition {',
    '    Feature(MediaFeature, Option<String>),',
    '    Not(Box<MediaCondition>),',
    '    And(Vec<MediaCondition>),',
    '    Or(Vec<MediaCondition>),',
    '}',
    '',
    '#[derive(Debug, Clone)]',
    'pub struct MediaQuery {',
    '    pub media_type: Option<MediaType>,',
    '    pub condition: Option<MediaCondition>,',
    '}',
])
w('crates/edgerun-media-queries/src/lib.rs', '\n'.join(L))
w('crates/edgerun-media-queries/Cargo.toml', gen_toml('media-queries'))
print(f'edgerun-media-queries: {len(mq["media_types"])} types, {len(mq["media_features"])} features')

# ===========================================================================
# CSS Syntax
# ===========================================================================
st = load('scripts/css_syntax_catalog.json')
L = ['//! CSS Syntax types \u2014 from CSS Syntax Level 3.',
     '//! DO NOT EDIT. Regenerate with: scripts/generate_batch4.py',
     '#![cfg_attr(not(test), no_std)]', '',
     'extern crate alloc;',
     'use alloc::{string::String, vec::Vec};', '']

# Token types with data-carrying ones
data_tokens = {'Ident', 'Function', 'AtKeyword', 'Hash', 'String', 'Url', 'Dimension', 'Number', 'Percentage', 'UnicodeRange', 'WhiteSpace', 'Delim'}
simple_tokens = {'OpenParen', 'CloseParen', 'OpenSquare', 'CloseSquare', 'OpenCurly', 'CloseCurly', 'Colon', 'Semicolon', 'Comma'}

L.append('#[derive(Debug, Clone, PartialEq)]')
L.append('pub enum CssToken {')
for t in st['token_types']:
    L.append(f'    /// {t[1]}')
    if t[0] in data_tokens:
        L.append(f'    {t[0]}(String),')
    else:
        L.append(f'    {t[0]},')
L.extend(['}', ''])

L.extend([
    '/// CSS Tokenizer state machine states.',
    '#[derive(Debug, Clone, Copy, PartialEq, Eq)]',
    'pub enum TokenizerState {',
    '    Data, Comment, String, StringStartingEscape,',
    '    Url, UrlBad, Number, NumberStartingDash, NumberStartingDot,',
    '    Ident, AtKeyword, Hash, HashAlphanumeric,',
    '    Cdo, Cdc, HtmlCommentOpen, HtmlCommentClose,',
    '    Percentage, Dimension, UnicodeRange, Eof,',
    '}',
    '',
    '/// Parsed unicode range.',
    '#[derive(Debug, Clone, Copy, PartialEq, Eq)]',
    'pub struct UnicodeRange { pub start: u32, pub end: u32 }',
    '',
    'impl UnicodeRange {',
    '    pub fn parse(s: &str) -> Option<Self> {',
    '        let s = s.trim_start_matches("U+").trim_start_matches("u+");',
    '        if s.contains("?") {',
    '            let prefix = s.trim_end_matches("?");',
    '            let start = u32::from_str_radix(prefix, 16).ok()?;',
    '            let end = start | ((1 << (4 * (s.len() - prefix.len()))) - 1);',
    '            Some(Self { start, end })',
    '        } else if let Some((a, b)) = s.split_once("-") {',
    '            let start = u32::from_str_radix(a, 16).ok()?;',
    '            let end = u32::from_str_radix(b, 16).ok()?;',
    '            Some(Self { start, end })',
    '        } else {',
    '            let cp = u32::from_str_radix(s, 16).ok()?;',
    '            Some(Self { start: cp, end: cp })',
    '        }',
    '    }',
    '}',
])
w('crates/edgerun-css-syntax/src/lib.rs', '\n'.join(L))
w('crates/edgerun-css-syntax/Cargo.toml', gen_toml('css-syntax'))
print(f'edgerun-css-syntax: {len(st["token_types"])} token types, {len(st["range_syntax"])} range patterns')

print('\nDone.')
