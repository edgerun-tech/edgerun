//! Encoding types — from WHATWG Encoding Living Standard.
//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform.py

/// All encoding types defined by the Encoding Standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// UTF-8 (labels: unicode-1-1-utf-8, utf-8, utf8)
    UTF8,
    /// UTF-16LE (labels: utf-16le)
    UTF16LE,
    /// UTF-16BE (labels: utf-16be)
    UTF16BE,
    /// IBM866 (labels: 866, cp866, csibm866...)
    IBM866,
    /// ISO-8859-2 (labels: csisolatin2, iso-8859-2, iso-ir-101...)
    ISO88592,
    /// ISO-8859-3 (labels: csisolatin3, iso-8859-3, iso-ir-109...)
    ISO88593,
    /// ISO-8859-4 (labels: csisolatin4, iso-8859-4, iso-ir-110...)
    ISO88594,
    /// ISO-8859-5 (labels: csisolatincyrillic, cyrillic, iso-8859-5...)
    ISO88595,
    /// ISO-8859-6 (labels: arabic, asmo-708, csiso88596e...)
    ISO88596,
    /// ISO-8859-7 (labels: csisolatingreek, ecma-118, elot_928...)
    ISO88597,
    /// ISO-8859-8 (labels: csiso88598e, csisolatinhebrew, hebrew...)
    ISO88598,
    /// ISO-8859-8-I (labels: csiso88598i, logical, iso-8859-8-i...)
    ISO88598I,
    /// ISO-8859-10 (labels: csisolatin6, iso-8859-10, iso-ir-157...)
    ISO885910,
    /// ISO-8859-13 (labels: iso-8859-13, iso8859-13, iso885913)
    ISO885913,
    /// ISO-8859-14 (labels: iso-8859-14, iso8859-14, iso885914)
    ISO885914,
    /// ISO-8859-15 (labels: csisolatin9, iso-8859-15, iso8859-15...)
    ISO885915,
    /// ISO-8859-16 (labels: iso-8859-16)
    ISO885916,
    /// KOI8-R (labels: cskoi8r, koi, koi8...)
    KOI8R,
    /// KOI8-U (labels: koi8-u)
    KOI8U,
    /// macintosh (labels: csmacintosh, mac, macintosh...)
    Macintosh,
    /// windows-874 (labels: dos-874, iso-8859-11, iso8859-11...)
    Windows874,
    /// windows-1250 (labels: cp1250, windows-1250, x-cp1250)
    Windows1250,
    /// windows-1251 (labels: cp1251, windows-1251, x-cp1251)
    Windows1251,
    /// windows-1252 (labels: ansi_x3.4-1968, ascii, cp1252...)
    Windows1252,
    /// windows-1253 (labels: cp1253, windows-1253, x-cp1253)
    Windows1253,
    /// windows-1254 (labels: csisolatin5, iso-8859-9, iso-ir-148...)
    Windows1254,
    /// windows-1255 (labels: cp1255, windows-1255, x-cp1255)
    Windows1255,
    /// windows-1256 (labels: cp1256, windows-1256, x-cp1256)
    Windows1256,
    /// windows-1257 (labels: cp1257, windows-1257, x-cp1257)
    Windows1257,
    /// windows-1258 (labels: cp1258, windows-1258, x-cp1258)
    Windows1258,
    /// x-mac-cyrillic (labels: x-mac-cyrillic, x-mac-ukrainian)
    XMacCyrillic,
    /// GBK (labels: chinese, csgb2312, csiso58gb231280...)
    GBK,
    /// gb18030 (labels: gb18030)
    Gb18030,
    /// Big5 (labels: big5, big5-hkscs, cn-big5...)
    Big5,
    /// EUC-JP (labels: cseucpkdfmtjapanese, euc-jp, x-euc-jp)
    EUCJP,
    /// ISO-2022-JP (labels: csiso2022jp, iso-2022-jp)
    ISO2022JP,
    /// Shift_JIS (labels: csshiftjis, ms932, ms_kanji...)
    ShiftJIS,
    /// EUC-KR (labels: cseuckr, csksc56011987, euc-kr...)
    EUCKR,
    /// replacement (labels: csiso2022kr, hz-gb-2312, iso-2022-cn...)
    Replacement,
}

impl Encoding {
    /// Look up an encoding by label (case-insensitive).
    pub fn from_label(label: &str) -> Option<Self> {
        match label.to_lowercase().as_str() {
            "unicode-1-1-utf-8" => Some(Encoding::UTF8),
            "utf-8" => Some(Encoding::UTF8),
            "utf8" => Some(Encoding::UTF8),
            "utf-16le" => Some(Encoding::UTF16LE),
            "utf-16be" => Some(Encoding::UTF16BE),
            "866" => Some(Encoding::IBM866),
            "cp866" => Some(Encoding::IBM866),
            "csibm866" => Some(Encoding::IBM866),
            "ibm866" => Some(Encoding::IBM866),
            "csisolatin2" => Some(Encoding::ISO88592),
            "iso-8859-2" => Some(Encoding::ISO88592),
            "iso-ir-101" => Some(Encoding::ISO88592),
            "iso8859-2" => Some(Encoding::ISO88592),
            "iso88592" => Some(Encoding::ISO88592),
            "iso_8859-2" => Some(Encoding::ISO88592),
            "iso_8859-2:1987" => Some(Encoding::ISO88592),
            "l2" => Some(Encoding::ISO88592),
            "latin2" => Some(Encoding::ISO88592),
            "csisolatin3" => Some(Encoding::ISO88593),
            "iso-8859-3" => Some(Encoding::ISO88593),
            "iso-ir-109" => Some(Encoding::ISO88593),
            "iso8859-3" => Some(Encoding::ISO88593),
            "iso88593" => Some(Encoding::ISO88593),
            "iso_8859-3" => Some(Encoding::ISO88593),
            "iso_8859-3:1988" => Some(Encoding::ISO88593),
            "l3" => Some(Encoding::ISO88593),
            "latin3" => Some(Encoding::ISO88593),
            "csisolatin4" => Some(Encoding::ISO88594),
            "iso-8859-4" => Some(Encoding::ISO88594),
            "iso-ir-110" => Some(Encoding::ISO88594),
            "iso8859-4" => Some(Encoding::ISO88594),
            "iso88594" => Some(Encoding::ISO88594),
            "iso_8859-4" => Some(Encoding::ISO88594),
            "iso_8859-4:1988" => Some(Encoding::ISO88594),
            "l4" => Some(Encoding::ISO88594),
            "latin4" => Some(Encoding::ISO88594),
            "csisolatincyrillic" => Some(Encoding::ISO88595),
            "cyrillic" => Some(Encoding::ISO88595),
            "iso-8859-5" => Some(Encoding::ISO88595),
            "iso-ir-144" => Some(Encoding::ISO88595),
            "iso8859-5" => Some(Encoding::ISO88595),
            "iso88595" => Some(Encoding::ISO88595),
            "iso_8859-5" => Some(Encoding::ISO88595),
            "iso_8859-5:1988" => Some(Encoding::ISO88595),
            "arabic" => Some(Encoding::ISO88596),
            "asmo-708" => Some(Encoding::ISO88596),
            "csiso88596e" => Some(Encoding::ISO88596),
            "csiso88596i" => Some(Encoding::ISO88596),
            "csisolatinarabic" => Some(Encoding::ISO88596),
            "ecma-114" => Some(Encoding::ISO88596),
            "iso-8859-6" => Some(Encoding::ISO88596),
            "iso-8859-6-e" => Some(Encoding::ISO88596),
            "iso-8859-6-i" => Some(Encoding::ISO88596),
            "iso-ir-127" => Some(Encoding::ISO88596),
            "iso8859-6" => Some(Encoding::ISO88596),
            "iso88596" => Some(Encoding::ISO88596),
            "iso_8859-6" => Some(Encoding::ISO88596),
            "iso_8859-6:1987" => Some(Encoding::ISO88596),
            "csisolatingreek" => Some(Encoding::ISO88597),
            "ecma-118" => Some(Encoding::ISO88597),
            "elot_928" => Some(Encoding::ISO88597),
            "greek" => Some(Encoding::ISO88597),
            "greek8" => Some(Encoding::ISO88597),
            "iso-8859-7" => Some(Encoding::ISO88597),
            "iso-ir-126" => Some(Encoding::ISO88597),
            "iso8859-7" => Some(Encoding::ISO88597),
            "iso88597" => Some(Encoding::ISO88597),
            "iso_8859-7" => Some(Encoding::ISO88597),
            "iso_8859-7:1987" => Some(Encoding::ISO88597),
            "sun_eu_greek" => Some(Encoding::ISO88597),
            "csiso88598e" => Some(Encoding::ISO88598),
            "csisolatinhebrew" => Some(Encoding::ISO88598),
            "hebrew" => Some(Encoding::ISO88598),
            "iso-8859-8" => Some(Encoding::ISO88598),
            "iso-8859-8-e" => Some(Encoding::ISO88598),
            "iso-ir-138" => Some(Encoding::ISO88598),
            "iso8859-8" => Some(Encoding::ISO88598),
            "iso88598" => Some(Encoding::ISO88598),
            "iso_8859-8" => Some(Encoding::ISO88598),
            "iso_8859-8:1988" => Some(Encoding::ISO88598),
            "visual" => Some(Encoding::ISO88598),
            "csiso88598i" => Some(Encoding::ISO88598I),
            "logical" => Some(Encoding::ISO88598I),
            "iso-8859-8-i" => Some(Encoding::ISO88598I),
            "iso8859-8-i" => Some(Encoding::ISO88598I),
            "csisolatin6" => Some(Encoding::ISO885910),
            "iso-8859-10" => Some(Encoding::ISO885910),
            "iso-ir-157" => Some(Encoding::ISO885910),
            "iso8859-10" => Some(Encoding::ISO885910),
            "iso885910" => Some(Encoding::ISO885910),
            "iso_8859-10" => Some(Encoding::ISO885910),
            "iso_8859-10:1992" => Some(Encoding::ISO885910),
            "l6" => Some(Encoding::ISO885910),
            "latin6" => Some(Encoding::ISO885910),
            "iso-8859-13" => Some(Encoding::ISO885913),
            "iso8859-13" => Some(Encoding::ISO885913),
            "iso885913" => Some(Encoding::ISO885913),
            "iso-8859-14" => Some(Encoding::ISO885914),
            "iso8859-14" => Some(Encoding::ISO885914),
            "iso885914" => Some(Encoding::ISO885914),
            "csisolatin9" => Some(Encoding::ISO885915),
            "iso-8859-15" => Some(Encoding::ISO885915),
            "iso8859-15" => Some(Encoding::ISO885915),
            "iso885915" => Some(Encoding::ISO885915),
            "iso_8859-15" => Some(Encoding::ISO885915),
            "l9" => Some(Encoding::ISO885915),
            "iso-8859-16" => Some(Encoding::ISO885916),
            "cskoi8r" => Some(Encoding::KOI8R),
            "koi" => Some(Encoding::KOI8R),
            "koi8" => Some(Encoding::KOI8R),
            "koi8-r" => Some(Encoding::KOI8R),
            "koi8_r" => Some(Encoding::KOI8R),
            "koi8-u" => Some(Encoding::KOI8U),
            "csmacintosh" => Some(Encoding::Macintosh),
            "mac" => Some(Encoding::Macintosh),
            "macintosh" => Some(Encoding::Macintosh),
            "x-mac-roman" => Some(Encoding::Macintosh),
            "dos-874" => Some(Encoding::Windows874),
            "iso-8859-11" => Some(Encoding::Windows874),
            "iso8859-11" => Some(Encoding::Windows874),
            "iso885911" => Some(Encoding::Windows874),
            "tis-620" => Some(Encoding::Windows874),
            "windows-874" => Some(Encoding::Windows874),
            "cp1250" => Some(Encoding::Windows1250),
            "windows-1250" => Some(Encoding::Windows1250),
            "x-cp1250" => Some(Encoding::Windows1250),
            "cp1251" => Some(Encoding::Windows1251),
            "windows-1251" => Some(Encoding::Windows1251),
            "x-cp1251" => Some(Encoding::Windows1251),
            "ansi_x3.4-1968" => Some(Encoding::Windows1252),
            "ascii" => Some(Encoding::Windows1252),
            "cp1252" => Some(Encoding::Windows1252),
            "cp819" => Some(Encoding::Windows1252),
            "csisolatin1" => Some(Encoding::Windows1252),
            "ibm819" => Some(Encoding::Windows1252),
            "iso-8859-1" => Some(Encoding::Windows1252),
            "iso-ir-100" => Some(Encoding::Windows1252),
            "iso8859-1" => Some(Encoding::Windows1252),
            "iso88591" => Some(Encoding::Windows1252),
            "iso_8859-1" => Some(Encoding::Windows1252),
            "iso_8859-1:1987" => Some(Encoding::Windows1252),
            "l1" => Some(Encoding::Windows1252),
            "latin1" => Some(Encoding::Windows1252),
            "us-ascii" => Some(Encoding::Windows1252),
            "windows-1252" => Some(Encoding::Windows1252),
            "x-cp1252" => Some(Encoding::Windows1252),
            "cp1253" => Some(Encoding::Windows1253),
            "windows-1253" => Some(Encoding::Windows1253),
            "x-cp1253" => Some(Encoding::Windows1253),
            "csisolatin5" => Some(Encoding::Windows1254),
            "iso-8859-9" => Some(Encoding::Windows1254),
            "iso-ir-148" => Some(Encoding::Windows1254),
            "iso8859-9" => Some(Encoding::Windows1254),
            "iso88599" => Some(Encoding::Windows1254),
            "iso_8859-9" => Some(Encoding::Windows1254),
            "iso_8859-9:1989" => Some(Encoding::Windows1254),
            "l5" => Some(Encoding::Windows1254),
            "latin5" => Some(Encoding::Windows1254),
            "windows-1254" => Some(Encoding::Windows1254),
            "x-cp1254" => Some(Encoding::Windows1254),
            "cp1255" => Some(Encoding::Windows1255),
            "windows-1255" => Some(Encoding::Windows1255),
            "x-cp1255" => Some(Encoding::Windows1255),
            "cp1256" => Some(Encoding::Windows1256),
            "windows-1256" => Some(Encoding::Windows1256),
            "x-cp1256" => Some(Encoding::Windows1256),
            "cp1257" => Some(Encoding::Windows1257),
            "windows-1257" => Some(Encoding::Windows1257),
            "x-cp1257" => Some(Encoding::Windows1257),
            "cp1258" => Some(Encoding::Windows1258),
            "windows-1258" => Some(Encoding::Windows1258),
            "x-cp1258" => Some(Encoding::Windows1258),
            "x-mac-cyrillic" => Some(Encoding::XMacCyrillic),
            "x-mac-ukrainian" => Some(Encoding::XMacCyrillic),
            "chinese" => Some(Encoding::GBK),
            "csgb2312" => Some(Encoding::GBK),
            "csiso58gb231280" => Some(Encoding::GBK),
            "gb2312" => Some(Encoding::GBK),
            "gb_2312" => Some(Encoding::GBK),
            "gb_2312-80" => Some(Encoding::GBK),
            "gbk" => Some(Encoding::GBK),
            "iso-ir-58" => Some(Encoding::GBK),
            "x-gbk" => Some(Encoding::GBK),
            "gb18030" => Some(Encoding::Gb18030),
            "big5" => Some(Encoding::Big5),
            "big5-hkscs" => Some(Encoding::Big5),
            "cn-big5" => Some(Encoding::Big5),
            "csbig5" => Some(Encoding::Big5),
            "x-x-big5" => Some(Encoding::Big5),
            "cseucpkdfmtjapanese" => Some(Encoding::EUCJP),
            "euc-jp" => Some(Encoding::EUCJP),
            "x-euc-jp" => Some(Encoding::EUCJP),
            "csiso2022jp" => Some(Encoding::ISO2022JP),
            "iso-2022-jp" => Some(Encoding::ISO2022JP),
            "csshiftjis" => Some(Encoding::ShiftJIS),
            "ms932" => Some(Encoding::ShiftJIS),
            "ms_kanji" => Some(Encoding::ShiftJIS),
            "shift-jis" => Some(Encoding::ShiftJIS),
            "shift_jis" => Some(Encoding::ShiftJIS),
            "sjis" => Some(Encoding::ShiftJIS),
            "windows-31j" => Some(Encoding::ShiftJIS),
            "x-sjis" => Some(Encoding::ShiftJIS),
            "cseuckr" => Some(Encoding::EUCKR),
            "csksc56011987" => Some(Encoding::EUCKR),
            "euc-kr" => Some(Encoding::EUCKR),
            "iso-ir-149" => Some(Encoding::EUCKR),
            "korean" => Some(Encoding::EUCKR),
            "ks_c_5601-1987" => Some(Encoding::EUCKR),
            "ks_c_5601-1989" => Some(Encoding::EUCKR),
            "ksc5601" => Some(Encoding::EUCKR),
            "ksc_5601" => Some(Encoding::EUCKR),
            "windows-949" => Some(Encoding::EUCKR),
            "csiso2022kr" => Some(Encoding::Replacement),
            "hz-gb-2312" => Some(Encoding::Replacement),
            "iso-2022-cn" => Some(Encoding::Replacement),
            "iso-2022-cn-ext" => Some(Encoding::Replacement),
            "iso-2022-kr" => Some(Encoding::Replacement),
            "replacement" => Some(Encoding::Replacement),
            _ => None,
        }
    }
}

/// BOM (Byte Order Mark) table.
pub const BOM_TABLE: &[(Encoding, &[u8])] = &[
    (Encoding::UTF8, &[0xEF, 0xBB, 0xBF]),
    (Encoding::UTF16LE, &[0xFF, 0xFE]),
    (Encoding::UTF16BE, &[0xFE, 0xFF]),
];

/// Check if the given bytes start with a BOM.
pub fn detect_bom(bytes: &[u8]) -> Option<(Encoding, usize)> {
    for &(enc, bom) in BOM_TABLE {
        if bytes.starts_with(bom) {
            return Some((enc, bom.len()));
        }
    }
    None
}