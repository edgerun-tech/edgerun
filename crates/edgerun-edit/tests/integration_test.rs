use std::fs;
use std::path::PathBuf;

use edgerun_edit::edit_ops::{
    add_derive, add_fn, add_use, find_fn, incoming_refs, list_file, new_file, parse_file,
    remove_fn, remove_file, rename_type_in_file, replace_fn_body, write_file,
};
use edgerun_edit::git::{commit, diff, find_git_root, GitSafety};
use edgerun_edit::project::{walk_rs, Project};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn temp_dir() -> PathBuf {
    // Each call gets a unique directory under /tmp
    let uuid = uuid::Uuid::new_v4().to_string();
    let dir = std::env::temp_dir().join(format!("edgerun_edit_test_{uuid}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &PathBuf) {
    let _ = fs::remove_dir_all(dir);
}

fn write_rs(dir: &PathBuf, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, content).unwrap();
    path
}

fn read_rs(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

// ── parse_file / write_file ─────────────────────────────────────────────────

#[test]
fn test_parse_and_write_roundtrip() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub fn hello() { println!(\"hi\"); }");

    let parsed = parse_file(&path).expect("parse should succeed");
    write_file(&path, &parsed).expect("write should succeed");

    let content = read_rs(&path);
    assert!(content.contains("fn hello"));
    cleanup(&dir);
}

#[test]
fn test_parse_invalid_rust() {
    let dir = temp_dir();
    let path = write_rs(&dir, "bad.rs", "this is not rust {{{");

    let result = parse_file(&path);
    assert!(result.is_err(), "should fail to parse invalid rust");
    cleanup(&dir);
}

#[test]
fn test_parse_missing_file() {
    let result = parse_file(&PathBuf::from("/nonexistent/path.rs"));
    assert!(result.is_err());
}

// ── replace_fn_body ─────────────────────────────────────────────────────────

#[test]
fn test_replace_fn_body_found() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub fn compute(x: u32) -> u32 { x + 1 }",
    );

    let mut file = parse_file(&path).unwrap();
    let found = replace_fn_body(&mut file, "compute", "x * 2").unwrap();
    assert!(found, "should find the function");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("x * 2"));
    assert!(!content.contains("x + 1"));
    cleanup(&dir);
}

#[test]
fn test_replace_fn_body_not_found() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub fn other() {}");

    let mut file = parse_file(&path).unwrap();
    let found = replace_fn_body(&mut file, "nonexistent", "1").unwrap();
    assert!(!found, "should not find nonexistent function");
    cleanup(&dir);
}

#[test]
fn test_replace_fn_body_invalid_body() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub fn foo() {}");

    let mut file = parse_file(&path).unwrap();
    // "not valid rust" is not a valid block expression
    let result = replace_fn_body(&mut file, "foo", "not valid rust @@@");
    assert!(result.is_err(), "should reject invalid body");
    cleanup(&dir);
}

#[test]
fn test_replace_fn_body_multiline() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub fn foo() { println!(\"old\"); }",
    );

    let mut file = parse_file(&path).unwrap();
    let found = replace_fn_body(
        &mut file,
        "foo",
        "let x = 1;\n    let y = 2;\n    x + y",
    )
    .unwrap();
    assert!(found);
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("let x = 1"));
    assert!(content.contains("x + y"));
    cleanup(&dir);
}

// ── add_fn ───────────────────────────────────────────────────────────────────

#[test]
fn test_add_fn_basic() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "// existing");

    let mut file = parse_file(&path).unwrap();
    add_fn(&mut file, "double", "x: u32", "u32", "{ x * 2 }").unwrap();
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("pub fn double(x: u32) -> u32"));
    assert!(content.contains("x * 2"));
    cleanup(&dir);
}

#[test]
fn test_add_fn_no_args_no_ret() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "");

    let mut file = parse_file(&path).unwrap();
    add_fn(&mut file, "greet", "", "", r#"{ println!("hi"); }"#).unwrap();
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("pub fn greet()"));
    cleanup(&dir);
}

#[test]
fn test_add_fn_multiple_args() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "");

    let mut file = parse_file(&path).unwrap();
    add_fn(&mut file, "add", "a: i32, b: i32", "i32", "{ a + b }").unwrap();
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("pub fn add(a: i32, b: i32) -> i32"));
    cleanup(&dir);
}

#[test]
fn test_add_fn_invalid_arg() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "");

    let mut file = parse_file(&path).unwrap();
    let result = add_fn(&mut file, "bad", "not valid rust", "", "{}");
    assert!(result.is_err());
    cleanup(&dir);
}

#[test]
fn test_add_fn_invalid_ret() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "");

    let mut file = parse_file(&path).unwrap();
    let result = add_fn(&mut file, "bad", "", "not a type", "{}");
    assert!(result.is_err());
    cleanup(&dir);
}

// ── remove_fn ────────────────────────────────────────────────────────────────

#[test]
fn test_remove_fn_found() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub fn keep() {}\npub fn remove() {}",
    );

    let mut file = parse_file(&path).unwrap();
    let removed = remove_fn(&mut file, "remove");
    assert!(removed, "should remove existing function");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("fn keep"));
    assert!(!content.contains("fn remove"));
    cleanup(&dir);
}

#[test]
fn test_remove_fn_not_found() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub fn foo() {}");

    let mut file = parse_file(&path).unwrap();
    let removed = remove_fn(&mut file, "bar");
    assert!(!removed, "should not remove nonexistent function");
    cleanup(&dir);
}

// ── add_use ──────────────────────────────────────────────────────────────────

#[test]
fn test_add_use_basic() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "fn main() {}");

    let mut file = parse_file(&path).unwrap();
    add_use(&mut file, "std::collections::HashMap").unwrap();
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("pub use std::collections::HashMap"));
    // use should be at the top
    let use_pos = content.find("pub use").unwrap();
    let fn_pos = content.find("fn main").unwrap();
    assert!(use_pos < fn_pos, "use should appear before fn");
    cleanup(&dir);
}

#[test]
fn test_add_use_invalid() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "");

    let mut file = parse_file(&path).unwrap();
    let result = add_use(&mut file, "{{{ not a use }}}");
    assert!(result.is_err());
    cleanup(&dir);
}

// ── add_derive ───────────────────────────────────────────────────────────────

#[test]
fn test_add_derive_to_struct() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub struct Foo { pub x: u32 }");

    let mut file = parse_file(&path).unwrap();
    let added = add_derive(&mut file, "Foo", "Debug, Clone").unwrap();
    assert!(added, "should find struct");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("#[derive(Debug, Clone)]"));
    cleanup(&dir);
}

#[test]
fn test_add_derive_to_enum() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub enum Bar { A, B }");

    let mut file = parse_file(&path).unwrap();
    let added = add_derive(&mut file, "Bar", "PartialEq").unwrap();
    assert!(added, "should find enum");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("#[derive(PartialEq)]"));
    cleanup(&dir);
}

#[test]
fn test_add_derive_merge() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "#[derive(Debug)]\npub struct Foo { pub x: u32 }");

    let mut file = parse_file(&path).unwrap();
    let added = add_derive(&mut file, "Foo", "Clone").unwrap();
    assert!(added);
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("Debug"));
    assert!(content.contains("Clone"));
    cleanup(&dir);
}

#[test]
fn test_add_derive_not_found() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub struct Foo { pub x: u32 }");

    let mut file = parse_file(&path).unwrap();
    let added = add_derive(&mut file, "Nonexistent", "Debug").unwrap();
    assert!(!added, "should not find nonexistent type");
    cleanup(&dir);
}

#[test]
fn test_add_derive_invalid() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub struct Foo { pub x: u32 }");

    let mut file = parse_file(&path).unwrap();
    let result = add_derive(&mut file, "Foo", "{{{ not a derive }}}");
    assert!(result.is_err());
    cleanup(&dir);
}

// ── rename_type_in_file ─────────────────────────────────────────────────────

#[test]
fn test_rename_type_struct_def() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub struct OldName { pub x: u32 }");

    let mut file = parse_file(&path).unwrap();
    rename_type_in_file(&mut file, "OldName", "NewName");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("struct NewName"));
    assert!(!content.contains("struct OldName"));
    cleanup(&dir);
}

#[test]
fn test_rename_type_in_fn_signature() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub struct Foo { pub x: u32 }\npub fn process(f: Foo) -> Foo { f }",
    );

    let mut file = parse_file(&path).unwrap();
    rename_type_in_file(&mut file, "Foo", "Bar");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("struct Bar"));
    assert!(content.contains("f: Bar"));
    assert!(content.contains("-> Bar"));
    assert!(!content.contains("Foo"));
    cleanup(&dir);
}

#[test]
fn test_rename_type_in_generics() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub struct Foo {}\npub fn process(items: Vec<Foo>) -> Option<Foo> { None }",
    );

    let mut file = parse_file(&path).unwrap();
    rename_type_in_file(&mut file, "Foo", "Bar");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("Vec<Bar>"));
    assert!(content.contains("Option<Bar>"));
    cleanup(&dir);
}

#[test]
fn test_rename_type_in_struct_field() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub struct Inner { pub val: u32 }\npub struct Outer { pub inner: Inner }",
    );

    let mut file = parse_file(&path).unwrap();
    rename_type_in_file(&mut file, "Inner", "Data");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("struct Data"));
    assert!(content.contains("inner: Data"));
    assert!(!content.contains("struct Inner"));
    cleanup(&dir);
}

#[test]
fn test_rename_type_in_enum_variant_field() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub struct Foo { pub x: u32 }\npub enum E { A { f: Foo } }",
    );

    let mut file = parse_file(&path).unwrap();
    rename_type_in_file(&mut file, "Foo", "Bar");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("f: Bar"));
    cleanup(&dir);
}

#[test]
fn test_rename_type_in_impl() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub struct Foo { pub x: u32 }\nimpl Foo {\n    pub fn new() -> Foo { Foo { x: 0 } }\n}",
    );

    let mut file = parse_file(&path).unwrap();
    rename_type_in_file(&mut file, "Foo", "Bar");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("impl Bar"));
    assert!(content.contains("-> Bar"));
    assert!(!content.contains("Foo"));
    cleanup(&dir);
}

#[test]
fn test_rename_type_in_use_tree() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub use crate::Foo;");

    let mut file = parse_file(&path).unwrap();
    rename_type_in_file(&mut file, "Foo", "Bar");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("pub use crate::Bar"));
    cleanup(&dir);
}

#[test]
fn test_rename_type_no_match() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub struct Foo { pub x: u32 }");

    let mut file = parse_file(&path).unwrap();
    let before = read_rs(&path);
    rename_type_in_file(&mut file, "Nonexistent", "Bar");
    write_file(&path, &file).unwrap();

    // Should still be valid Rust (just no renames happened)
    let after = read_rs(&path);
    assert!(after.contains("struct Foo"));
    cleanup(&dir);
}

// ── new_file / remove_file ──────────────────────────────────────────────────

#[test]
fn test_new_file_valid_rust() {
    let dir = temp_dir();
    let path = dir.join("src").join("lib.rs");

    new_file(&path, "pub fn hello() {}").unwrap();
    assert!(path.exists());

    let content = read_rs(&path);
    assert!(content.contains("fn hello"));
    cleanup(&dir);
}

#[test]
fn test_new_file_empty() {
    let dir = temp_dir();
    let path = dir.join("empty.rs");

    new_file(&path, "").unwrap();
    assert!(path.exists());
    assert!(read_rs(&path).is_empty());
    cleanup(&dir);
}

#[test]
fn test_new_file_already_exists() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "");

    let result = new_file(&path, "fn foo() {}");
    assert!(result.is_err());
    cleanup(&dir);
}

#[test]
fn test_new_file_invalid_rust() {
    let dir = temp_dir();
    let path = dir.join("bad.rs");

    let result = new_file(&path, "not rust {{{");
    assert!(result.is_err());
    assert!(!path.exists());
    cleanup(&dir);
}

#[test]
fn test_remove_file_exists() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "");

    remove_file(&path).unwrap();
    assert!(!path.exists());
    cleanup(&dir);
}

#[test]
fn test_remove_file_missing() {
    let result = remove_file(&PathBuf::from("/nonexistent/file.rs"));
    assert!(result.is_err());
}

// ── list_file ────────────────────────────────────────────────────────────────

#[test]
fn test_list_file_all_types() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        r#"
extern crate alloc;
use std::collections::HashMap;
pub struct Foo { pub x: u32 }
pub enum Bar { A, B }
pub trait MyTrait {}
pub type Alias = u32;
pub const MAX: u32 = 100;
pub static COUNT: u32 = 0;
pub fn hello() {}
pub mod inner {}
pub union MyUnion { pub x: u32 }
"#,
    );

    let items = list_file(&path).unwrap();
    let names: Vec<&str> = items.iter().map(|s| s.as_str()).collect();

    assert!(names.iter().any(|s| s.contains("fn hello")));
    assert!(names.iter().any(|s| s.contains("struct Foo")));
    assert!(names.iter().any(|s| s.contains("enum Bar")));
    assert!(names.iter().any(|s| s.contains("trait MyTrait")));
    assert!(names.iter().any(|s| s.contains("type Alias")));
    assert!(names.iter().any(|s| s.contains("const MAX")));
    assert!(names.iter().any(|s| s.contains("static COUNT")));
    assert!(names.iter().any(|s| s.contains("mod inner")));
    assert!(names.iter().any(|s| s.contains("union MyUnion")));
    assert!(names.iter().any(|s| s.contains("extern crate")));
    assert!(names.iter().any(|s| s.contains("use")));
    cleanup(&dir);
}

#[test]
fn test_list_file_empty() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "");

    let items = list_file(&path).unwrap();
    assert!(items.is_empty());
    cleanup(&dir);
}

#[test]
fn test_list_file_impl() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub struct Foo {}\nimpl Foo { pub fn bar() {} }",
    );

    let items = list_file(&path).unwrap();
    assert!(items.iter().any(|s| s.contains("impl Foo")));
    cleanup(&dir);
}

// ── find_fn ──────────────────────────────────────────────────────────────────

#[test]
fn test_find_fn_exists() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub fn compute(x: u32) -> u32 { x * 2 }",
    );

    let file = parse_file(&path).unwrap();
    let result = find_fn(&file, "compute");
    assert!(result.is_some());
    let src = result.unwrap();
    assert!(src.contains("fn compute"));
    assert!(src.contains("x * 2"));
    cleanup(&dir);
}

#[test]
fn test_find_fn_not_found() {
    let dir = temp_dir();
    let path = write_rs(&dir, "lib.rs", "pub fn other() {}");

    let file = parse_file(&path).unwrap();
    let result = find_fn(&file, "compute");
    assert!(result.is_none());
    cleanup(&dir);
}

#[test]
fn test_find_fn_pretty_printed() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        "pub fn ugly(  x:u32,  y:u32  )->u32{x+y}",
    );

    let file = parse_file(&path).unwrap();
    let result = find_fn(&file, "ugly");
    assert!(result.is_some());
    // prettyplease should format it nicely
    let src = result.unwrap();
    assert!(src.contains("fn ugly"));
    assert!(src.contains("x: u32"));
    assert!(src.contains("y: u32"));
    cleanup(&dir);
}

// ── incoming_refs ────────────────────────────────────────────────────────────

#[test]
fn test_incoming_refs_cross_file() {
    let dir = temp_dir();
    let lib = write_rs(
        &dir,
        "src/lib.rs",
        "use crate::utils::Helper;\npub fn get_helper() -> Helper { Helper }",
    );
    let utils = write_rs(
        &dir,
        "src/utils.rs",
        "pub struct Helper;",
    );

    let files = vec![lib.clone(), utils.clone()];
    let count = incoming_refs(&files, &utils);
    assert_eq!(count, 2, "lib has 2 refs to Helper (use + return type)");
    cleanup(&dir);
}

#[test]
fn test_incoming_refs_no_refs() {
    let dir = temp_dir();
    let lib = write_rs(&dir, "src/lib.rs", "pub mod utils;");
    let utils = write_rs(&dir, "src/utils.rs", "pub struct Helper;");

    // Only utils file, no other file references it
    let files = vec![utils.clone()];
    let count = incoming_refs(&files, &utils);
    assert_eq!(count, 0);
    cleanup(&dir);
}

#[test]
fn test_incoming_refs_type_usage() {
    let dir = temp_dir();
    let lib = write_rs(
        &dir,
        "src/lib.rs",
        "use crate::models::User;\npub fn get_user() -> User { todo!() }",
    );
    let models = write_rs(
        &dir,
        "src/models.rs",
        "pub struct User { pub name: String }",
    );

    let files = vec![lib.clone(), models.clone()];
    let count = incoming_refs(&files, &models);
    // use crate::models::User -> counts User in use tree (1)
    // -> User -> counts in return type (1)
    // Total = 2 (use path segments + return type)
    assert!(count >= 2, "lib has at least 2 refs to User (use + return type), got {count}");
    cleanup(&dir);
}

// ── project.rs tests ────────────────────────────────────────────────────────

#[test]
fn test_project_single_file() {
    let dir = temp_dir();
    let path = write_rs(&dir, "main.rs", "fn main() {}");

    let project = Project::single_file(&path);
    assert_eq!(project.source_files.len(), 1);
    assert_eq!(project.source_files[0], path);
    cleanup(&dir);
}

#[test]
fn test_walk_rs_finds_all_files() {
    let dir = temp_dir();
    let src = dir.join("src");
    fs::create_dir_all(src.join("sub")).unwrap();

    write_rs(&dir, "src/lib.rs", "");
    write_rs(&dir, "src/main.rs", "");
    write_rs(&dir, "src/sub/mod.rs", "");
    write_rs(&dir, "src/sub/nested.rs", "");

    let mut files = std::collections::HashSet::new();
    walk_rs(&src, &mut files);

    assert_eq!(files.len(), 4, "should find all 4 .rs files");
    cleanup(&dir);
}

#[test]
fn test_walk_rs_skips_non_rs() {
    let dir = temp_dir();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(src.join("lib.rs"), "").unwrap();
    fs::write(src.join("readme.md"), "").unwrap();
    fs::write(src.join("Cargo.toml"), "").unwrap();

    let mut files = std::collections::HashSet::new();
    walk_rs(&src, &mut files);

    assert_eq!(files.len(), 1);
    assert!(files.iter().any(|p| p.file_name().unwrap() == "lib.rs"));
    cleanup(&dir);
}

// ── git.rs tests ─────────────────────────────────────────────────────────────

#[test]
fn test_find_git_root_from_subdir() {
    let dir = temp_dir();
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();
    fs::create_dir_all(dir.join("src").join("sub")).unwrap();

    let root = find_git_root(&dir.join("src").join("sub"));
    assert!(root.is_some());
    assert_eq!(root.unwrap(), dir);
    cleanup(&dir);
}

#[test]
fn test_find_git_root_not_a_repo() {
    let dir = temp_dir();
    let root = find_git_root(&dir);
    assert!(root.is_none(), "should not find git root in non-repo");
    cleanup(&dir);
}

#[test]
fn test_git_safety_new_in_repo() {
    let dir = temp_dir();
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    let safety = GitSafety::new(&dir);
    assert!(safety.is_some());
    cleanup(&dir);
}

#[test]
fn test_git_safety_new_not_in_repo() {
    let dir = temp_dir();
    let safety = GitSafety::new(&dir);
    assert!(safety.is_none());
    cleanup(&dir);
}

#[test]
fn test_git_safety_track() {
    let dir = temp_dir();
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    let mut safety = GitSafety::new(&dir).unwrap();
    let file_path = dir.join("src").join("lib.rs");
    safety.track(&file_path);

    // Track uses relative paths from git root
    assert!(!safety.modified.is_empty());
    cleanup(&dir);
}

#[test]
fn test_git_safety_rollback_no_changes() {
    let dir = temp_dir();
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    let safety = GitSafety::new(&dir).unwrap();
    let (ok, msg) = safety.rollback();
    assert!(ok);
    assert!(msg.contains("nothing to rollback"));
    cleanup(&dir);
}

#[test]
fn test_git_safety_stage_all_no_changes() {
    let dir = temp_dir();
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    let safety = GitSafety::new(&dir).unwrap();
    let (ok, msg) = safety.stage_all();
    assert!(ok);
    assert!(msg.contains("no files modified"));
    cleanup(&dir);
}

#[test]
fn test_git_diff_empty_repo() {
    let dir = temp_dir();
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    let output = diff(&dir);
    // Should not panic, may be empty
    assert!(output.is_empty() || output.contains("0 files"));
    cleanup(&dir);
}

#[test]
fn test_git_commit_nothing_staged() {
    let dir = temp_dir();
    // Initialize a proper git repo with an initial commit
    fs::create_dir_all(dir.join(".git")).unwrap();
    write_rs(&dir, "file.txt", "hello");

    let (ok, _msg) = commit(&dir, "test commit");
    // Should fail because nothing is staged
    assert!(!ok);
    cleanup(&dir);
}

// ── Integration: full workflow ──────────────────────────────────────────────

#[test]
fn test_full_edit_workflow() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        r#"
pub struct Config {
    pub name: String,
    pub value: u32,
}

pub fn create_config(name: &str) -> Config {
    Config {
        name: name.to_string(),
        value: 0,
    }
}
"#,
    );

    // 1. Parse
    let mut file = parse_file(&path).unwrap();

    // 2. Add derive
    add_derive(&mut file, "Config", "Debug, Clone").unwrap();

    // 3. Replace body
    replace_fn_body(
        &mut file,
        "create_config",
        r#"Config {
        name: name.to_string(),
        value: 42,
    }"#,
    )
    .unwrap();

    // 4. Add a new function
    add_fn(
        &mut file,
        "default_config",
        "",
        "Config",
        r#"{ Config { name: "default".to_string(), value: 0 } }"#,
    )
    .unwrap();

    // 5. Add a use
    add_use(&mut file, "std::fmt::Display").unwrap();

    // 6. Write back
    write_file(&path, &file).unwrap();

    // 7. Verify
    let content = read_rs(&path);
    assert!(content.contains("#[derive(Debug, Clone)]"));
    assert!(content.contains("value: 42"));
    assert!(content.contains("pub fn default_config"));
    assert!(content.contains("pub use std::fmt::Display"));

    // 8. Re-parse to verify it's valid Rust
    let re_parsed = parse_file(&path).unwrap();

    // 9. Find the new function
    assert!(find_fn(&re_parsed, "default_config").is_some());
    assert!(find_fn(&re_parsed, "create_config").is_some());

    // 10. List should show all items
    let items = list_file(&path).unwrap();
    assert!(items.iter().any(|s| s.contains("struct Config")));
    assert!(items.iter().any(|s| s.contains("fn create_config")));
    assert!(items.iter().any(|s| s.contains("fn default_config")));

    cleanup(&dir);
}

#[test]
fn test_rename_then_verify() {
    let dir = temp_dir();
    let path = write_rs(
        &dir,
        "lib.rs",
        r#"
pub struct OldConfig {
    pub name: String,
}

pub fn make() -> OldConfig {
    OldConfig { name: "test".to_string() }
}
"#,
    );

    let mut file = parse_file(&path).unwrap();
    rename_type_in_file(&mut file, "OldConfig", "NewConfig");
    write_file(&path, &file).unwrap();

    let content = read_rs(&path);
    assert!(content.contains("struct NewConfig"));
    assert!(content.contains("-> NewConfig"));
    assert!(content.contains("NewConfig {"));
    assert!(!content.contains("OldConfig"));

    // Re-parse to verify validity
    parse_file(&path).unwrap();
    cleanup(&dir);
}
