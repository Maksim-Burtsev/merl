//! Project-wide grep (ripgrep's own crates) and the regex rules behind "go to definition",
//! the symbol list and the word under the cursor.
//!
//! There is no language server here: a definition is whatever a per-kind line pattern says it
//! is, searched in the project first and then in the standard library and the installed
//! dependencies the toolchain on this machine knows about.
//!
//! The rules live in the modules below and are re-exported here, so a caller names what
//! it wants (`search::def_patterns`) and not which module happens to hold it.

mod bindings;
mod c;
mod component;
mod csharp;
mod csproj;
mod css;
mod dart;
mod defs;
mod fields;
mod grep;
mod imports;
mod jsdoc;
mod jvm;
mod kind;
mod labels;
mod links;
mod make;
mod objc;
mod php;
mod powershell;
mod python;
mod ruby;
mod rust;
mod scope;
mod shell;
mod swift;
mod symbols;
mod syntax;
mod types;
mod words;

pub use bindings::*;
pub use c::*;
pub use component::*;
pub use csharp::*;
pub use csproj::*;
pub use css::*;
pub use dart::*;
pub use defs::*;
pub use fields::*;
pub use grep::*;
pub use imports::*;
pub use jsdoc::*;
pub use jvm::*;
pub use kind::*;
pub use labels::*;
pub use links::*;
pub use make::*;
pub use objc::*;
pub use php::*;
pub use powershell::*;
pub use python::*;
pub use ruby::*;
pub use rust::*;
pub use scope::*;
pub use shell::*;
pub use swift::*;
pub use symbols::*;
pub use syntax::*;
pub use types::*;
pub use words::*;

#[cfg(test)]
mod tests;
