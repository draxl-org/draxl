#![allow(dead_code)]

use draxl_ast::File;
use draxl_parser::parse_file;
use draxl_printer::canonicalize_file;
use draxl_validate::validate_file;
use std::fs;
use std::path::PathBuf;

pub struct ExampleCase {
    pub slug: &'static str,
    pub path: &'static str,
}

pub fn examples() -> &'static [ExampleCase] {
    &[
        ExampleCase {
            slug: "01_add",
            path: "examples/01_add.rs.dx",
        },
        ExampleCase {
            slug: "02_shapes",
            path: "examples/02_shapes.rs.dx",
        },
        ExampleCase {
            slug: "03_match",
            path: "examples/03_match.rs.dx",
        },
        ExampleCase {
            slug: "04_ranks",
            path: "examples/04_ranks.rs.dx",
        },
        ExampleCase {
            slug: "05_use_tree_group",
            path: "examples/05_use_tree_group.rs.dx",
        },
    ]
}

pub fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

pub fn read(relative: &str) -> String {
    fs::read_to_string(repo_path(relative))
        .unwrap_or_else(|err| panic!("failed to read {relative}: {err}"))
}

pub fn parse_and_validate(relative: &str) -> File {
    let source = read(relative);
    let file =
        parse_file(&source).unwrap_or_else(|err| panic!("failed to parse {relative}: {err}"));
    validate_file(&file)
        .unwrap_or_else(|errors| panic!("failed to validate {relative}: {errors:?}"));
    file
}

pub fn canonical_without_spans(file: &File) -> File {
    canonicalize_file(&file.without_spans())
}
