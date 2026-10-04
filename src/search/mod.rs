//! Project-wide grep (ripgrep's own crates) and the regex rules behind "go to definition",
//! the symbol list and the word under the cursor.
//!
//! There is no language server here: a definition is whatever a per-kind line pattern says it
//! is, searched in the project first and then in the standard library and the installed
//! dependencies the toolchain on this machine knows about.
//!
//! The rules live in the modules below and are re-exported here, so a caller names what
//! it wants (`search::def_patterns`) and not which module happens to hold it.

mod android;
mod bindings;
mod c;
mod cmake;
mod component;
mod csharp;
mod csproj;
mod css;
mod dart;
mod defs;
mod erlang;
mod fields;
mod gdscript;
mod grep;
mod groovy;
mod haskell;
mod imports;
mod jsdoc;
mod json_ref;
mod julia;
mod jvm;
mod kind;
mod labels;
mod links;
mod lisp;
mod make;
mod ml;
mod nix;
mod objc;
mod perl;
mod php;
mod powershell;
mod python;
mod r;
mod ruby;
mod rust;
mod scope;
mod shell;
mod solidity;
mod sql;
mod starlark;
mod swift;
mod symbols;
mod syntax;
mod types;
mod words;

pub use android::*;
pub use bindings::*;
pub use c::*;
pub use cmake::*;
pub use component::*;
pub use csharp::*;
pub use csproj::*;
pub use css::*;
pub use dart::*;
pub use defs::*;
pub use erlang::*;
pub use fields::*;
pub use gdscript::*;
pub use grep::*;
pub use groovy::*;
pub use haskell::*;
pub use imports::*;
pub use jsdoc::*;
pub use json_ref::*;
pub use julia::*;
pub use jvm::*;
pub use kind::*;
pub use labels::*;
pub use links::*;
pub use lisp::*;
pub use make::*;
pub use ml::*;
pub use nix::*;
pub use objc::*;
pub use perl::*;
pub use php::*;
pub use powershell::*;
pub use python::*;
pub use r::*;
pub use ruby::*;
pub use rust::*;
pub use scope::*;
pub use shell::*;
pub use solidity::*;
pub use sql::*;
pub use starlark::*;
pub use swift::*;
pub use symbols::*;
pub use syntax::*;
pub use types::*;
pub use words::*;

#[cfg(test)]
mod tests;
