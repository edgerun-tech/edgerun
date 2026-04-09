//! D-Bus wire protocol — read/write values, encode/decode messages.

use std::collections::HashMap;
use std::io;
use crate::dbus_types::*;

// Consume one complete type from a signature string.
pub fn consume_one(sig: &str) -> io::Result<(String, &str)> {
    if sig.is_empty() { return Err(io::Error::new(io::ErrorKind::InvalidData, "empty sig")); }
    match sig.as_bytes()[0] {
        b'a' => {
            let el = &sig[1..];
            if el.starts_with('{') {
                let mut d = 0; let mut end = 0;
                for (i, &b) in el.as_bytes().iter().enumerate() {
                    if b == b'{' { d += 1; }
                    if b == b'}' { d -= 1; if d == 0 { end = i + 1; break; } }
                }
                if end == 0 { return Err(io::Error::new(io::ErrorKind::InvalidData, "bad dict")); }
                Ok((format!("a{}", &el[..end]), &el[end..]))
            } else {
                let (es, r) = consume_one(el)?;
                Ok((format!("a{}", es), r))
            }
        }
        b'(' => {
            let mut d = 1; let mut end = 0;
            for (i, &b) in sig[1..].as_bytes().iter().enumerate() {
                if b == b'(' { d += 1; }
                if b == b')' { d -= 1; if d == 0 { end = i + 2; break; } }
            }
            if end == 0 { return Err(io::Error::new(io::ErrorKind::InvalidData, "bad struct")); }
            Ok((sig[..end].into(), &sig[end..]))
        }
        b'v' => Ok(("v".into(), &sig[1..])),
        b'y'|b'b'|b'q'|b'i'|b'u'|b't'|b's'|b'o'|b'g' => {
            Ok((sig[..1].into(), &sig[1..]))
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "bad type")),
    }
}

pub fn vsig(v: &Val) -> String {
    match v {
        Val::Y(_) => "y".into(), Val::B(_) => "b".into(), Val::Q(_) => "q".into(),
        Val::I(_) => "i".into(), Val::U(_) => "u".into(), Val::T(_) => "t".into(),
        Val::S(_) => "s".into(), Val::O(_) => "o".into(), Val::G(s) => format!("g{}", s),
        Val::Arr(items) => { if items.is_empty() { "av".into() } else { format!("a{}", vsig(&items[0])) } }
        Val::Str(fields) => { let mut i = String::new(); for f in fields { i.push_str(&vsig(f)); } format!("({})", i) }
        Val::Dict(entries) => { if entries.is_empty() { "a{sv}".into() } else { format!("a{{{}{}}}", vsig(&entries[0].0), vsig(&entries[0].1)) } }
        Val::Var(inner) => format!("v{}", vsig(inner)),
    }
}

// ===========================================================================
// Reader
// ===========================================================================

pub struct Rdr { d: Vec<u8>, p: usize }

impl Rdr {
    pub fn new(d: Vec<u8>) -> Self { Self { d, p: 0 } }
    fn rem(&self) -> usize { self.d.len() - self.p }
    fn al(&mut self, a: usize) { let o = self.p % a; if o != 0 { self.p += a - o; } }
    fn by(&mut self, n: usize) -> io::Result<&[u8]> {
        if self.p + n > self.d.len() { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "trunc")); }
        let s = self.p; self.p += n; Ok(&self.d[s..s + n])
    }
    fn u8(&mut self) -> io::Result<u8> { Ok(self.by(1)?[0]) }
    fn u16(&mut self) -> io::Result<u16> { let b = self.by(2)?; Ok(u16::from_le_bytes([b[0], b[1]])) }
    fn u32(&mut self) -> io::Result<u32> { let b = self.by(4)?; Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]])) }
    fn i32(&mut self) -> io::Result<i32> { let b = self.by(4)?; Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]])) }
    fn u64(&mut self) -> io::Result<u64> { let b = self.by(8)?; Ok(u64::from_le_bytes(b.try_into().unwrap())) }
    fn sv(&mut self) -> io::Result<String> {
        let l = self.u32()? as usize;
        let b: Vec<u8> = self.by(l)?.to_vec();
        if self.rem() > 0 && self.d[self.p] == 0 { self.p += 1; }
        String::from_utf8(b).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }
    fn sg(&mut self) -> io::Result<String> {
        let l = self.u8()? as usize;
        let b: Vec<u8> = self.by(l)?.to_vec();
        if self.rem() > 0 && self.d[self.p] == 0 { self.p += 1; }
        String::from_utf8(b).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    pub fn val(&mut self, sig: &str) -> io::Result<Val> {
        if sig.is_empty() { return Err(io::Error::new(io::ErrorKind::InvalidData, "empty")); }
        match sig.as_bytes()[0] {
            b'y' => Ok(Val::Y(self.u8()?)),
            b'b' => { self.al(4); Ok(Val::B(self.u32()? != 0)) }
            b'q' => { self.al(2); Ok(Val::Q(self.u16()?)) }
            b'i' => { self.al(4); Ok(Val::I(self.i32()?)) }
            b'u' => { self.al(4); Ok(Val::U(self.u32()?)) }
            b't' => { self.al(8); Ok(Val::T(self.u64()?)) }
            b's' => Ok(Val::S(self.sv()?)),
            b'o' => Ok(Val::O(self.sv()?)),
            b'g' => Ok(Val::G(self.sg()?)),
            b'a' => {
                let el = &sig[1..];
                if el.starts_with('{') && el.ends_with('}') {
                    let inner = &el[1..el.len() - 1];
                    if inner.len() < 2 { return Ok(Val::Dict(vec![])); }
                    let kt = &inner[..1]; let vt = &inner[1..];
                    self.al(4); let al = self.u32()? as usize; let end = self.p + al;
                    let mut ent = Vec::new();
                    while self.p < end {
                        // Each dict entry is 8-byte aligned (like a struct)
                        self.al(8);
                        if self.p >= end { break; }
                        ent.push((self.val(kt)?, self.val(vt)?));
                    }
                    Ok(Val::Dict(ent))
                } else if el == "y" {
                    self.al(4); let al = self.u32()? as usize;
                    Ok(Val::Arr(self.by(al)?.iter().map(|&x| Val::Y(x)).collect()))
                } else {
                    self.al(4); let al = self.u32()? as usize; let end = self.p + al;
                    let mut it = Vec::new(); while self.p < end { it.push(self.val(el)?); }
                    self.p = end; Ok(Val::Arr(it))
                }
            }
            b'r' | b'(' => {
                let inner = if sig.starts_with('(') && sig.ends_with(')') { &sig[1..sig.len() - 1] } else { sig };
                self.al(8); let mut f = Vec::new(); let mut rem = inner;
                while !rem.is_empty() { let (ts, r) = consume_one(rem)?; f.push(self.val(&ts)?); rem = r; }
                Ok(Val::Str(f))
            }
            b'v' => { let is = self.sg()?; Ok(Val::Var(Box::new(self.val(&is)?))) }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "bad type")),
        }
    }
}

// ===========================================================================
// Writer
// ===========================================================================

pub struct Wtr { d: Vec<u8> }

impl Wtr {
    pub fn new() -> Self { Self { d: Vec::with_capacity(256) } }
    pub fn finish(self) -> Vec<u8> { self.d }

    fn pd(&mut self, a: usize) { let o = self.d.len() % a; for _ in 0..(a - o) % a { self.d.push(0); } }
    fn y(&mut self, v: u8) { self.d.push(v); }
    fn u16(&mut self, v: u16) { self.d.extend_from_slice(&v.to_le_bytes()); }
    fn i32(&mut self, v: i32) { self.d.extend_from_slice(&v.to_le_bytes()); }
    fn u32(&mut self, v: u32) { self.d.extend_from_slice(&v.to_le_bytes()); }
    fn u64(&mut self, v: u64) { self.d.extend_from_slice(&v.to_le_bytes()); }
    fn ss(&mut self, s: &str) { let b = s.as_bytes(); self.u32(b.len() as u32); self.d.extend_from_slice(b); self.d.push(0); }
    fn gs(&mut self, s: &str) { let b = s.as_bytes(); self.y(b.len() as u8); self.d.extend_from_slice(b); self.d.push(0); }

    pub fn wval(&mut self, sig: &str, v: &Val) {
        if sig.is_empty() { return; }
        match sig.as_bytes()[0] {
            b'y' => { if let Val::Y(x) = v { self.d.push(*x); } }
            b'b' => { self.pd(4); if let Val::B(x) = v { self.u32(if *x { 1 } else { 0 }); } }
            b'q' => { self.pd(2); if let Val::Q(x) = v { self.u16(*x); } }
            b'i' => { self.pd(4); if let Val::I(x) = v { self.i32(*x); } }
            b'u' => { self.pd(4); if let Val::U(x) = v { self.u32(*x); } }
            b't' => { self.pd(8); if let Val::T(x) = v { self.u64(*x); } }
            b's' | b'o' => {
                if let Val::S(s) = v { self.ss(s); }
                if let Val::O(s) = v { self.ss(s); }
            }
            b'g' => { if let Val::G(s) = v { self.gs(s); } }
            b'a' => {
                let el = &sig[1..];
                if el.starts_with('{') && el.ends_with('}') {
                    let inner = &el[1..el.len() - 1];
                    let kt = &inner[..1]; let vt = &inner[1..];
                    let mut tmp = Wtr::new();
                    if let Val::Dict(entries) = v {
                        for (k, vv) in entries {
                            tmp.pd(8);
                            tmp.wval(kt, k);
                            tmp.wval(vt, vv);
                        }
                    }
                    let td = tmp.finish();
                    self.pd(4); self.u32(td.len() as u32);
                    self.d.extend_from_slice(&td);
                } else if el == "y" {
                    if let Val::Arr(items) = v {
                        let bytes: Vec<u8> = items.iter().filter_map(|x| if let Val::Y(b) = x { Some(*b) } else { None }).collect();
                        self.pd(4); self.u32(bytes.len() as u32); self.d.extend_from_slice(&bytes);
                    }
                } else {
                    let mut tmp = Wtr::new();
                    if let Val::Arr(items) = v { for it in items { tmp.wval(el, it); } }
                    let td = tmp.finish(); self.pd(4); self.u32(td.len() as u32); self.d.extend_from_slice(&td);
                }
            }
            b'r' | b'(' => {
                let inner = if sig.starts_with('(') && sig.ends_with(')') { &sig[1..sig.len() - 1] } else { sig };
                self.pd(8);
                if let Val::Str(fields) = v {
                    let mut rem = inner;
                    for f in fields {
                        let (ts, r) = match consume_one(rem) { Ok(r) => r, Err(_) => break };
                        self.wval(&ts, f); rem = r;
                    }
                }
            }
            b'v' => {
                if let Val::Var(inner) = v {
                    let is = vsig(inner);
                    self.gs(&is); self.wval(&is, inner);
                }
            }
            _ => {}
        }
    }
}

// ===========================================================================
// Message encode / decode
// ===========================================================================

pub fn encode_msg(msg: &Msg) -> Vec<u8> {
    // Write body
    let mut bw = Wtr::new();
    if !msg.bsig.is_empty() && !msg.body.is_empty() {
        let mut rem = msg.bsig.as_str();
        for v in &msg.body {
            let (ts, r) = match consume_one(rem) { Ok(r) => r, Err(_) => break };
            bw.wval(&ts, v); rem = r;
        }
    }
    let bdata = bw.finish();

    // Write header fields
    let mut hf = Wtr::new();
    if !msg.bsig.is_empty() {
        hf.y(F_SIG); hf.y(b'v'); hf.gs("g"); hf.wval("g", &Val::G(msg.bsig.clone()));
    }
    for (&c, val) in &msg.fields {
        let sg = match val { Val::O(_) => "o".into(), Val::S(_) => "s".into(), Val::U(_) => "u".into(), _ => vsig(val) };
        hf.y(c); hf.y(b'v'); hf.gs(&sg); hf.wval(&sg, val);
    }
    let hfd = hf.finish();
    let hl = 16 + hfd.len();
    let ah = (hl + 7) & !7;
    let pad = ah - hl;

    let mut out = Vec::with_capacity(ah + bdata.len());
    out.push(BYTE_ORDER); out.push(msg.mt as u8); out.push(msg.fl); out.push(1);
    out.extend_from_slice(&(bdata.len() as u32).to_le_bytes());
    out.extend_from_slice(&msg.ser.to_le_bytes());
    out.extend_from_slice(&(hfd.len() as u32).to_le_bytes());
    out.extend_from_slice(&hfd);
    for _ in 0..pad { out.push(0); }
    out.extend_from_slice(&bdata);
    out
}

pub fn decode_msg(data: &[u8]) -> io::Result<Msg> {
    if data.len() < 16 { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "short")); }
    if data[0] != BYTE_ORDER { return Err(io::Error::new(io::ErrorKind::InvalidData, "order")); }
    let mt = match data[1] {
        1 => MType::Call, 2 => MType::Return, 3 => MType::Err, 4 => MType::Signal,
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "type")),
    };
    let fl = data[2];
    let bl = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let ser = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let hfl = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
    let th = 16 + hfl as usize;
    let ah = (th + 7) & !7;
    if ah + bl > data.len() { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "trunc")); }

    let mut rdr = Rdr::new(data[16..16 + hfl as usize].to_vec());
    let mut fields = HashMap::new();
    while rdr.rem() > 0 {
        let fc = rdr.u8()?;
        if rdr.u8()? != b'v' { return Err(io::Error::new(io::ErrorKind::InvalidData, "not var")); }
        let sg = rdr.sg()?;
        fields.insert(fc, rdr.val(&sg)?);
    }

    let bsig = fields.get(&F_SIG).and_then(|v| v.s()).unwrap_or("").to_string();
    let bdata = &data[ah..ah + bl];
    let mut br = Rdr::new(bdata.to_vec());
    let mut body = Vec::new();
    let mut rem = bsig.as_str();
    while !rem.is_empty() {
        let (ts, r) = consume_one(rem)?;
        body.push(br.val(&ts)?);
        rem = r;
    }

    Ok(Msg { mt, fl, ser, fields, body, bsig })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_string() {
        let mut w = Wtr::new();
        w.wval("s", &Val::S("hello".into()));
        let mut r = Rdr::new(w.finish());
        let v = r.val("s").unwrap();
        assert_eq!(v.s(), Some("hello"));
    }

    #[test]
    fn roundtrip_dict_ss() {
        let mut w = Wtr::new();
        let d = Val::Dict(vec![
            (Val::S("k1".into()), Val::S("v1".into())),
            (Val::S("k2".into()), Val::S("v2".into())),
        ]);
        w.wval("a{ss}", &d);
        let mut r = Rdr::new(w.finish());
        let v = r.val("a{ss}").unwrap();
        let pairs = v.dict_ss().unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("k1".into(), "v1".into()));
    }

    #[test]
    fn roundtrip_byte_array() {
        let mut w = Wtr::new();
        let bytes: Vec<Val> = b"secret".iter().map(|&b| Val::Y(b)).collect();
        w.wval("ay", &Val::Arr(bytes));
        let mut r = Rdr::new(w.finish());
        let v = r.val("ay").unwrap();
        if let Val::Arr(items) = v {
            let raw: Vec<u8> = items.iter().filter_map(|x| if let Val::Y(b) = x { Some(*b) } else { None }).collect();
            assert_eq!(raw, b"secret");
        } else { panic!("not array"); }
    }

    #[test]
    fn roundtrip_struct_ss() {
        let sig = "(ss)";
        let secret = Val::Str(vec![
            Val::S("hello".into()),
            Val::S("world".into()),
        ]);
        let mut w = Wtr::new();
        w.wval(sig, &secret);
        let bytes = w.finish();
        eprintln!("struct bytes: {:02x?}", bytes);
        let mut r = Rdr::new(bytes);
        let v = r.val(sig).unwrap();
        let fields = v.into_struct().unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].s(), Some("hello"));
        assert_eq!(fields[1].s(), Some("world"));
    }

    #[test]
    fn roundtrip_struct_oayss() {
        let sig = "(oa{sv}ays)";
        let secret = Val::Str(vec![
            Val::O("/org/freedesktop/secrets/session/s1".into()),
            Val::Dict(vec![]),
            Val::Arr(b"my-password".iter().map(|&b| Val::Y(b)).collect()),
            Val::S("text/plain".into()),
        ]);
        let mut w = Wtr::new();
        w.wval(sig, &secret);
        let bytes = w.finish();
        eprintln!("wrote {} bytes: {:02x?}", bytes.len(), bytes);
        let mut r = Rdr::new(bytes);
        let v = r.val(sig).unwrap();
        let fields = v.into_struct().unwrap();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0].o(), Some("/org/freedesktop/secrets/session/s1"));
        if let Val::Arr(arr) = &fields[2] { assert_eq!(arr.len(), 11); } else { panic!("field 2 not array"); }
    }

    #[test]
    fn encode_decode_method_call() {
        let msg = Msg::call("/org/freedesktop/secrets", "org.freedesktop.Secret.Service", "OpenSession", ":1.42")
            .body(vec![Val::S("plain".into()), Val::Var(Box::new(Val::S("".into())))], "sv");
        let msg = Msg { ser: 1, ..msg };
        let data = encode_msg(&msg);
        let decoded = decode_msg(&data).unwrap();
        assert_eq!(decoded.mt, MType::Call);
        assert_eq!(decoded.ser, 1);
        assert_eq!(decoded.path(), Some("/org/freedesktop/secrets"));
        assert_eq!(decoded.member(), Some("OpenSession"));
        assert_eq!(decoded.body.len(), 2);
    }

    #[test]
    fn encode_decode_return() {
        let msg = Msg::ret(42, ":1.42").body(vec![Val::S("ok".into())], "s");
        let data = encode_msg(&msg);
        let decoded = decode_msg(&data).unwrap();
        assert_eq!(decoded.mt, MType::Return);
    }

    #[test]
    fn vsig_all() {
        assert_eq!(vsig(&Val::Y(1)), "y");
        assert_eq!(vsig(&Val::B(true)), "b");
        assert_eq!(vsig(&Val::S("x".into())), "s");
        assert_eq!(vsig(&Val::O("/x".into())), "o");
        assert_eq!(vsig(&Val::Var(Box::new(Val::S("x".into())))), "vs");
    }
}
