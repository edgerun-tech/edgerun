//! D-Bus types and constants. No I/O here.

use crate::prelude::v1::*;
use alloc::collections::BTreeMap as HashMap;

pub const BYTE_ORDER: u8 = b'l';
pub const F_PATH: u8 = 1;
pub const F_IFACE: u8 = 2;
pub const F_MEMBER: u8 = 3;
pub const F_ERRNAME: u8 = 4;
pub const F_REPLY: u8 = 5;
pub const F_DEST: u8 = 6;
pub const F_SENDER: u8 = 7;
pub const F_SIG: u8 = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MType {
    Call = 1,
    Return = 2,
    Err = 3,
    Signal = 4,
}

#[derive(Clone, Debug)]
pub enum Val {
    Y(u8),
    B(bool),
    Q(u16),
    I(i32),
    U(u32),
    T(u64),
    S(String),
    O(String),
    G(String),
    Arr(Vec<Val>),
    Str(Vec<Val>),
    Dict(Vec<(Val, Val)>),
    Var(Box<Val>),
}

impl Val {
    pub fn s(&self) -> Option<&str> {
        match self {
            Val::S(s) => Some(s),
            _ => None,
        }
    }
    pub fn o(&self) -> Option<&str> {
        match self {
            Val::O(s) => Some(s),
            _ => None,
        }
    }
    pub fn b(&self) -> Option<bool> {
        match self {
            Val::B(x) => Some(*x),
            _ => None,
        }
    }
    pub fn u32(&self) -> Option<u32> {
        match self {
            Val::U(x) => Some(*x),
            _ => None,
        }
    }
    pub fn stru(&self) -> Option<&[Val]> {
        match self {
            Val::Str(f) => Some(f),
            _ => None,
        }
    }

    pub fn ao(&self) -> Option<Vec<String>> {
        if let Val::Arr(items) = self {
            let mut r = Vec::new();
            for i in items {
                if let Val::O(p) = i {
                    r.push(p.clone());
                } else {
                    return None;
                }
            }
            Some(r)
        } else {
            None
        }
    }

    pub fn dict_ss(&self) -> Option<Vec<(String, String)>> {
        if let Val::Dict(entries) = self {
            let mut r = Vec::new();
            for (k, v) in entries {
                if let (Val::S(ks), Val::S(vs)) = (k, v) {
                    r.push((ks.clone(), vs.clone()));
                } else {
                    return None;
                }
            }
            Some(r)
        } else {
            None
        }
    }

    pub fn into_struct(self) -> Option<Vec<Val>> {
        match self {
            Val::Str(f) => Some(f),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Msg {
    pub mt: MType,
    pub fl: u8,
    pub ser: u32,
    pub fields: HashMap<u8, Val>,
    pub body: Vec<Val>,
    pub bsig: String,
}

impl Msg {
    pub fn call(path: &str, iface: &str, member: &str, dest: &str) -> Self {
        let mut f = HashMap::new();
        f.insert(F_PATH, Val::O(path.into()));
        f.insert(F_IFACE, Val::S(iface.into()));
        f.insert(F_MEMBER, Val::S(member.into()));
        f.insert(F_DEST, Val::S(dest.into()));
        Self {
            mt: MType::Call,
            fl: 0,
            ser: 0,
            fields: f,
            body: vec![],
            bsig: String::new(),
        }
    }

    pub fn ret(ser: u32, sender: &str) -> Self {
        let mut f = HashMap::new();
        f.insert(F_REPLY, Val::U(ser));
        f.insert(F_DEST, Val::S(sender.into()));
        Self {
            mt: MType::Return,
            fl: 0,
            ser: 0,
            fields: f,
            body: vec![],
            bsig: String::new(),
        }
    }

    pub fn err(ser: u32, sender: &str, name: &str, msg: &str) -> Self {
        let mut f = HashMap::new();
        f.insert(F_REPLY, Val::U(ser));
        f.insert(F_DEST, Val::S(sender.into()));
        f.insert(F_ERRNAME, Val::S(name.into()));
        Self {
            mt: MType::Err,
            fl: 0,
            ser: 0,
            fields: f,
            body: vec![Val::S(msg.into())],
            bsig: "s".into(),
        }
    }

    pub fn sig(path: &str, iface: &str, member: &str) -> Self {
        let mut f = HashMap::new();
        f.insert(F_PATH, Val::O(path.into()));
        f.insert(F_IFACE, Val::S(iface.into()));
        f.insert(F_MEMBER, Val::S(member.into()));
        Self {
            mt: MType::Signal,
            fl: 0,
            ser: 0,
            fields: f,
            body: vec![],
            bsig: String::new(),
        }
    }

    pub fn body(mut self, vals: Vec<Val>, sig: &str) -> Self {
        self.body = vals;
        self.bsig = sig.into();
        self
    }

    pub fn path(&self) -> Option<&str> {
        self.fields.get(&F_PATH).and_then(|v| v.o())
    }
    pub fn iface(&self) -> Option<&str> {
        self.fields.get(&F_IFACE).and_then(|v| v.s())
    }
    pub fn member(&self) -> Option<&str> {
        self.fields.get(&F_MEMBER).and_then(|v| v.s())
    }
    pub fn sender(&self) -> Option<&str> {
        self.fields.get(&F_SENDER).and_then(|v| v.s())
    }
}
