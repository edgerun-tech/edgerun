use edgerun_error::Error;
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
struct InnerError(&'static str);

impl fmt::Display for InnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl StdError for InnerError {}

const HINT: &str = "check the path";

#[derive(Debug, Error)]
enum ExampleError {
    #[error("unit failed")]
    Unit,
    #[error("tuple failed: {0}")]
    Tuple(String),
    #[error("tuple field hidden")]
    TupleHidden(String),
    #[error("named `{path}` failed: {source}")]
    Named {
        path: String,
        #[source]
        source: InnerError,
    },
    #[error(transparent)]
    Transparent(#[from] InnerError),
    #[error("path `{path}` failed")]
    Path { path: PathBuf },
    #[error("explicit {} failed: {source}. {hint}", path.display(), hint = HINT)]
    ExplicitArgs { path: PathBuf, source: InnerError },
    #[error("nested value: {}", .inner.value)]
    DotShorthand { inner: NestedValue },
    #[error("triple: {}, {}, {}", .inner.value, .inner.value, .inner.value)]
    TripleDot { inner: NestedValue },
}

#[derive(Debug)]
struct NestedValue {
    value: u8,
}

#[derive(Debug, Error)]
#[error("{message}")]
struct StructError {
    message: String,
    source: Option<InnerError>,
}

#[test]
fn formats_unit_tuple_and_named_variants() {
    assert_eq!(ExampleError::Unit.to_string(), "unit failed");
    assert_eq!(
        ExampleError::Tuple("bad".to_string()).to_string(),
        "tuple failed: bad"
    );
    assert_eq!(
        ExampleError::TupleHidden("bad".to_string()).to_string(),
        "tuple field hidden"
    );
    assert_eq!(
        ExampleError::Named {
            path: "config.json".to_string(),
            source: InnerError("nope"),
        }
        .to_string(),
        "named `config.json` failed: nope"
    );
    assert_eq!(
        ExampleError::Path {
            path: PathBuf::from("config.json"),
        }
        .to_string(),
        "path `config.json` failed"
    );
    assert_eq!(
        ExampleError::ExplicitArgs {
            path: PathBuf::from("config.json"),
            source: InnerError("nope"),
        }
        .to_string(),
        "explicit config.json failed: nope. check the path"
    );
    assert_eq!(
        ExampleError::DotShorthand {
            inner: NestedValue { value: 7 },
        }
        .to_string(),
        "nested value: 7"
    );
    assert_eq!(
        ExampleError::TripleDot {
            inner: NestedValue { value: 7 },
        }
        .to_string(),
        "triple: 7, 7, 7"
    );
    assert_eq!(
        StructError {
            message: "structured".to_string(),
            source: None,
        }
        .to_string(),
        "structured"
    );
}

#[test]
fn exposes_sources_and_from_conversions() {
    let named = ExampleError::Named {
        path: "config.json".to_string(),
        source: InnerError("nope"),
    };
    assert_eq!(
        named.source().map(ToString::to_string).as_deref(),
        Some("nope")
    );

    let transparent = ExampleError::from(InnerError("wrapped"));
    assert_eq!(transparent.to_string(), "wrapped");
    assert_eq!(
        transparent.source().map(ToString::to_string).as_deref(),
        Some("wrapped")
    );
}
