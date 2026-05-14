use std::any::TypeId;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt;
use std::fs;
use std::io;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::num::{
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};
use std::ops::{Range, RangeInclusive};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use edgerun_ts_rs_derive::TS;

pub trait TS {
    type WithoutGenerics: TS + ?Sized;
    type OptionInnerType: ?Sized;

    const IS_OPTION: bool = false;

    fn docs() -> Option<String> {
        None
    }

    fn ident() -> String {
        let name = Self::name();
        name.split('<').next().unwrap_or(&name).to_string()
    }

    fn decl() -> String;

    fn decl_concrete() -> String {
        Self::decl()
    }

    fn name() -> String;

    fn inline() -> String;

    fn inline_flattened() -> String {
        Self::inline()
    }

    fn visit_dependencies(_: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
    }

    fn visit_generics(_: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
    }

    fn dependencies() -> Vec<Dependency>
    where
        Self: 'static,
    {
        let mut deps = Vec::new();
        struct Visit<'a>(&'a mut Vec<Dependency>);
        impl TypeVisitor for Visit<'_> {
            fn visit<T: TS + 'static + ?Sized>(&mut self) {
                if let Some(dep) = Dependency::from_ty::<T>() {
                    self.0.push(dep);
                }
            }
        }
        Self::visit_dependencies(&mut Visit(&mut deps));
        deps.sort();
        deps.dedup_by_key(|dep| dep.type_id);
        deps
    }

    fn export() -> Result<(), ExportError>
    where
        Self: 'static,
    {
        let path = Self::default_output_path()
            .ok_or_else(|| ExportError::CannotBeExported(std::any::type_name::<Self>()))?;
        export_to_path::<Self>(&path)
    }

    fn export_all() -> Result<(), ExportError>
    where
        Self: 'static,
    {
        Self::export_all_to(default_out_dir())
    }

    fn export_all_to(out_dir: impl AsRef<Path>) -> Result<(), ExportError>
    where
        Self: 'static,
    {
        let out_dir = out_dir.as_ref();
        let mut seen = HashSet::new();
        export_recursive::<Self>(out_dir, &mut seen)
    }

    fn export_to_string() -> Result<String, ExportError>
    where
        Self: 'static,
    {
        Ok(format!("export {}\n", Self::decl()))
    }

    fn output_path() -> Option<PathBuf> {
        None
    }

    fn default_output_path() -> Option<PathBuf> {
        Some(default_out_dir().join(Self::output_path()?))
    }
}

pub trait TypeVisitor: Sized {
    fn visit<T: TS + 'static + ?Sized>(&mut self);
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Dependency {
    pub type_id: TypeId,
    pub ts_name: String,
    pub output_path: PathBuf,
}

impl Dependency {
    pub fn from_ty<T: TS + 'static + ?Sized>() -> Option<Self> {
        Some(Self {
            type_id: TypeId::of::<T>(),
            ts_name: T::ident(),
            output_path: T::output_path()?,
        })
    }
}

#[derive(Debug)]
pub enum ExportError {
    CannotBeExported(&'static str),
    Io(io::Error),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::CannotBeExported(ty) => write!(f, "{ty} cannot be exported"),
            ExportError::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<io::Error> for ExportError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ExportError> for edgerun_error::Error {
    fn from(value: ExportError) -> Self {
        edgerun_error::Error::from_boxed(Box::new(value))
    }
}

#[doc(hidden)]
pub trait IsOption {}

impl<T> IsOption for Option<T> {}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Dummy;

impl TS for Dummy {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name() -> String {
        "unknown".to_string()
    }

    fn inline() -> String {
        Self::name()
    }

    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }
}

fn export_recursive<T: TS + 'static + ?Sized>(
    out_dir: &Path,
    seen: &mut HashSet<TypeId>,
) -> Result<(), ExportError> {
    if !seen.insert(TypeId::of::<T>()) {
        return Ok(());
    }
    if let Some(path) = T::output_path() {
        export_to_path::<T>(&out_dir.join(path))?;
    }
    struct Exporter<'a> {
        out_dir: &'a Path,
        seen: &'a mut HashSet<TypeId>,
        error: Option<ExportError>,
    }
    impl TypeVisitor for Exporter<'_> {
        fn visit<U: TS + 'static + ?Sized>(&mut self) {
            if self.error.is_none()
                && let Err(error) = export_recursive::<U>(self.out_dir, self.seen)
            {
                self.error = Some(error);
            }
        }
    }
    let mut exporter = Exporter {
        out_dir,
        seen,
        error: None,
    };
    T::visit_dependencies(&mut exporter);
    if let Some(error) = exporter.error {
        return Err(error);
    }
    Ok(())
}

fn export_to_path<T: TS + 'static + ?Sized>(path: &Path) -> Result<(), ExportError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, T::export_to_string()?)?;
    Ok(())
}

fn default_out_dir() -> PathBuf {
    std::env::var_os("TS_RS_EXPORT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("bindings"))
}

macro_rules! primitive_ts {
    ($($ty:ty => $ts:literal),* $(,)?) => {
        $(
            impl TS for $ty {
                type WithoutGenerics = Self;
                type OptionInnerType = Self;
                fn name() -> String { $ts.to_string() }
                fn inline() -> String { Self::name() }
                fn decl() -> String { panic!("{} cannot be declared", Self::name()) }
            }
        )*
    };
}

primitive_ts!(
    u8 => "number",
    u16 => "number",
    u32 => "number",
    usize => "number",
    i8 => "number",
    i16 => "number",
    i32 => "number",
    isize => "number",
    f32 => "number",
    f64 => "number",
    NonZeroU8 => "number",
    NonZeroU16 => "number",
    NonZeroU32 => "number",
    NonZeroUsize => "number",
    NonZeroI8 => "number",
    NonZeroI16 => "number",
    NonZeroI32 => "number",
    NonZeroIsize => "number",
    u64 => "bigint",
    i64 => "bigint",
    u128 => "bigint",
    i128 => "bigint",
    NonZeroU64 => "bigint",
    NonZeroI64 => "bigint",
    NonZeroU128 => "bigint",
    NonZeroI128 => "bigint",
    bool => "boolean",
    char => "string",
    str => "string",
    String => "string",
    Path => "string",
    PathBuf => "string",
    Ipv4Addr => "string",
    Ipv6Addr => "string",
    IpAddr => "string",
    SocketAddrV4 => "string",
    SocketAddrV6 => "string",
    SocketAddr => "string",
    () => "null",
    Duration => "number",
);

impl<T: TS> TS for Option<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = T;
    const IS_OPTION: bool = true;

    fn name() -> String {
        format!("{} | null", T::name())
    }

    fn inline() -> String {
        format!("{} | null", T::inline())
    }

    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }

    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        v.visit::<T>();
        T::visit_dependencies(v);
    }
}

impl<T: TS> TS for Vec<T> {
    type WithoutGenerics = Vec<Dummy>;
    type OptionInnerType = Self;

    fn ident() -> String {
        "Array".to_string()
    }

    fn name() -> String {
        format!("Array<{}>", T::name())
    }

    fn inline() -> String {
        format!("Array<{}>", T::inline())
    }

    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }

    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        v.visit::<T>();
        T::visit_dependencies(v);
    }
}

impl<T: TS, const N: usize> TS for [T; N] {
    type WithoutGenerics = [Dummy; N];
    type OptionInnerType = Self;

    fn name() -> String {
        format!("Array<{}>", T::name())
    }

    fn inline() -> String {
        format!("Array<{}>", T::inline())
    }

    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }

    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        v.visit::<T>();
        T::visit_dependencies(v);
    }
}

impl<K: TS, V: TS, H> TS for HashMap<K, V, H> {
    type WithoutGenerics = HashMap<Dummy, Dummy>;
    type OptionInnerType = Self;

    fn name() -> String {
        format!("{{ [key: string]: {} }}", V::name())
    }

    fn inline() -> String {
        format!("{{ [key: string]: {} }}", V::inline())
    }

    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }

    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        v.visit::<V>();
        V::visit_dependencies(v);
    }
}

impl<T: TS, E: TS> TS for Result<T, E> {
    type WithoutGenerics = Result<Dummy, Dummy>;
    type OptionInnerType = Self;

    fn name() -> String {
        format!("{{ Ok: {} }} | {{ Err: {} }}", T::name(), E::name())
    }

    fn inline() -> String {
        format!("{{ Ok: {} }} | {{ Err: {} }}", T::inline(), E::inline())
    }

    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }

    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        v.visit::<T>();
        T::visit_dependencies(v);
        v.visit::<E>();
        E::visit_dependencies(v);
    }
}

impl<K: TS, V: TS> TS for BTreeMap<K, V> {
    type WithoutGenerics = HashMap<Dummy, Dummy>;
    type OptionInnerType = Self;
    fn name() -> String {
        <HashMap<K, V> as TS>::name()
    }
    fn inline() -> String {
        <HashMap<K, V> as TS>::inline()
    }
    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <HashMap<K, V> as TS>::visit_dependencies(v);
    }
}

impl<T: TS, H> TS for HashSet<T, H> {
    type WithoutGenerics = Vec<Dummy>;
    type OptionInnerType = Self;
    fn name() -> String {
        <Vec<T> as TS>::name()
    }
    fn inline() -> String {
        <Vec<T> as TS>::inline()
    }
    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <Vec<T> as TS>::visit_dependencies(v);
    }
}

macro_rules! vec_like_ts {
    ($ty:ty) => {
        impl<T: TS> TS for $ty {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;
            fn name() -> String {
                <Vec<T> as TS>::name()
            }
            fn inline() -> String {
                <Vec<T> as TS>::inline()
            }
            fn decl() -> String {
                panic!("{} cannot be declared", Self::name())
            }
            fn visit_dependencies(v: &mut impl TypeVisitor)
            where
                Self: 'static,
            {
                <Vec<T> as TS>::visit_dependencies(v);
            }
        }
    };
}

vec_like_ts!(BTreeSet<T>);
vec_like_ts!(VecDeque<T>);
vec_like_ts!(Range<T>);
vec_like_ts!(RangeInclusive<T>);

macro_rules! transparent_ts {
    ($ty:ty) => {
        impl<T: TS> TS for $ty {
            type WithoutGenerics = Self;
            type OptionInnerType = T;
            fn ident() -> String {
                T::ident()
            }
            fn name() -> String {
                T::name()
            }
            fn inline() -> String {
                T::inline()
            }
            fn inline_flattened() -> String {
                T::inline_flattened()
            }
            fn decl() -> String {
                T::decl()
            }
            fn output_path() -> Option<PathBuf> {
                T::output_path()
            }
            fn visit_dependencies(v: &mut impl TypeVisitor)
            where
                Self: 'static,
            {
                T::visit_dependencies(v);
            }
        }
    };
}

transparent_ts!(Box<T>);
transparent_ts!(std::sync::Arc<T>);
transparent_ts!(std::rc::Rc<T>);
transparent_ts!(std::cell::Cell<T>);
transparent_ts!(std::cell::RefCell<T>);
transparent_ts!(std::sync::Mutex<T>);
transparent_ts!(std::sync::RwLock<T>);
transparent_ts!(PhantomData<T>);

impl<T: TS + ?Sized> TS for &T {
    type WithoutGenerics = Self;
    type OptionInnerType = T;
    fn ident() -> String {
        T::ident()
    }
    fn name() -> String {
        T::name()
    }
    fn inline() -> String {
        T::inline()
    }
    fn decl() -> String {
        T::decl()
    }
    fn output_path() -> Option<PathBuf> {
        T::output_path()
    }
}

impl<'a, T: TS + ToOwned + ?Sized> TS for std::borrow::Cow<'a, T> {
    type WithoutGenerics = Self;
    type OptionInnerType = T;
    fn ident() -> String {
        T::ident()
    }
    fn name() -> String {
        T::name()
    }
    fn inline() -> String {
        T::inline()
    }
    fn decl() -> String {
        T::decl()
    }
    fn output_path() -> Option<PathBuf> {
        T::output_path()
    }
}

impl<T: TS + ?Sized> TS for std::sync::Weak<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = T;
    fn ident() -> String {
        T::ident()
    }
    fn name() -> String {
        T::name()
    }
    fn inline() -> String {
        T::inline()
    }
    fn decl() -> String {
        T::decl()
    }
    fn output_path() -> Option<PathBuf> {
        T::output_path()
    }
}

impl<A: TS, B: TS> TS for (A, B) {
    type WithoutGenerics = (Dummy, Dummy);
    type OptionInnerType = Self;
    fn name() -> String {
        format!("[{}, {}]", A::name(), B::name())
    }
    fn inline() -> String {
        format!("[{}, {}]", A::inline(), B::inline())
    }
    fn decl() -> String {
        panic!("{} cannot be declared", Self::name())
    }
}
