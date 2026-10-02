use super::*;

fn declares(line: &str, word: &str) -> bool {
    Regex::new(&r_patterns(word).join("|"))
        .unwrap()
        .is_match(line)
}

#[test]
fn r_declaration_forms() {
    for (line, word) in [
        ("format_price <- function(cents) {", "format_price"),
        ("format_price <<- function(cents) {", "format_price"),
        ("square <- \\(x) x^2", "square"),
        ("    label = function() format_price(self$total),", "label"),
        (
            "handlers <- list(on_paid = function(o) o, on_lost = \\(o) NULL)",
            "on_lost",
        ),
        ("  limit <- 10", "limit"),
        ("limit = 10", "limit"),
        ("limit =", "limit"),
        ("`%+%` <- function(a, b) paste(a, b)", "%+%"),
        (
            "`names<-.invoice` <- function(x, value) x",
            "names<-.invoice",
        ),
        ("print.invoice <- function(x, ...) x", "print.invoice"),
        (".onLoad <- function(libname, pkgname) NULL", ".onLoad"),
        (
            r#"setClass("Person", representation(name = "character"))"#,
            "Person",
        ),
        (
            r#"setGeneric("area", function(shape) standardGeneric("area"))"#,
            "area",
        ),
        (r#"setMethod("area", "Circle", function(shape) 1)"#, "area"),
        (
            r#"methods::setRefClass('Account', fields = list())"#,
            "Account",
        ),
        (r#"setClass(Class = "Person")"#, "Person"),
        (r#"Cart <- R6::R6Class("Cart","#, "Cart"),
    ] {
        assert!(declares(line, word), "{line}: {word}");
    }
}

#[test]
fn r_refusals() {
    for (line, word) in [
        ("format_price(10)", "format_price"),
        ("f(limit = 10)", "limit"),
        ("  limit = 10,", "limit"),
        ("  limit = 10", "limit"),
        ("limit == 10", "limit"),
        ("x$limit <- 10", "limit"),
        (r#"x[["limit"]] <- 10"#, "limit"),
        ("self$total <- 0", "total"),
        (r#"names(x) <- c("a", "b")"#, "names"),
        ("fit <- lm(y ~ x)", "y"),
        ("10 -> limit", "limit"),
        ("for (i in xs) print(i)", "i"),
        ("    total = 0,", "total"),
        ("limits <- 10", "limit"),
        ("print.invoice <- function(x) x", "invoice"),
        ("print.invoice <- function(x) x", "print"),
        (
            r#"setMethod("area", "Circle", function(shape) 1)"#,
            "Circle",
        ),
        ("# limit <- 10", "limit"),
    ] {
        assert!(!declares(line, word), "{line}: {word}");
    }
}

#[test]
fn r_symbols_are_functions_and_classes() {
    let r = |line| one(Kind::R, line);
    for (line, name) in [
        ("format_price <- function(cents) {", "format_price"),
        ("  label = function() 1,", "label"),
        ("  on_lost = \\(o) NULL", "on_lost"),
        ("print.invoice <- function(x, ...) x", "print.invoice"),
        ("`%+%` <- function(a, b) a", "`%+%`"),
        (r#"setClass("Person", representation())"#, "Person"),
        (
            r#"setGeneric("area", function(shape) standardGeneric("area"))"#,
            "area",
        ),
        (r#"Account <- setRefClass("Account")"#, "Account"),
        (r#"Cart <- R6::R6Class("Cart","#, "Cart"),
        ("Cart <- R6Class(", "Cart"),
    ] {
        assert_eq!(r(line).as_deref(), Some(name), "{line}");
    }
    for line in [
        "limit <- 10",
        "limit = 10",
        r#"setMethod("area", "Circle", function(shape) 1)"#,
        "  total = 0,",
        "format_price(10)",
    ] {
        assert_eq!(r(line), None, "{line}");
    }
}

#[test]
fn r_literals_hide_declarations() {
    let text = r#"a <- "
b <- 1
\" c <- 2
"
d <- 'it\'s
e <- 3'
f <- r"(
g <- 4 )-" ]"
)"
h <- R'---[
i <- 5 ]--'
]---'
# j <- "
k <- `odd "name` + 1
l <- 6
"#;
    let hidden: Vec<usize> = literal_lines(Kind::R, text)
        .iter()
        .enumerate()
        .filter(|(_, h)| **h)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(hidden, [2, 3, 4, 6, 8, 9, 11, 12]);
}

#[test]
fn an_r_name_holds_its_dots_and_backticks() {
    fn word(line: &str, col: usize) -> Option<&str> {
        definition_word(Some(Kind::R), line, col).map(|(_, w)| w)
    }
    let line = "x <- print.invoice(.data) + base::nchar(a$b)";
    assert_eq!(word(line, 12), Some("print.invoice"));
    assert_eq!(word(line, 6), Some("print.invoice"));
    assert_eq!(word(line, 21), Some(".data"));
    assert_eq!(word(line, 36), Some("nchar"));
    assert_eq!(word(line, 42), Some("b"));
    let quoted = "`names<-.invoice`(x) <- a %+% b";
    assert_eq!(word(quoted, 0), Some("names<-.invoice"));
    assert_eq!(word(quoted, 5), Some("names<-.invoice"));
    assert_eq!(word(quoted, 27), Some("%+%"));
    assert_eq!(word(quoted, 18), Some("x"));
}

#[test]
fn r_sources_a_file_from_the_root_then_from_its_directory() {
    let files: Vec<PathBuf> = ["R/money.R", "R/cart.R", "scripts/helpers.R"]
        .map(PathBuf::from)
        .to_vec();
    let line = r#"source("R/money.R")"#;
    assert_eq!(r_source(line, 9).as_deref(), Some("R/money.R"));
    assert_eq!(r_source(line, 7).as_deref(), Some("R/money.R"));
    assert_eq!(r_source(line, 2), None);
    assert_eq!(
        r_source(r#"sys.source(file = 'a.R')"#, 20).as_deref(),
        Some("a.R")
    );
    assert_eq!(
        r_files(Path::new("R"), "R/money.R", &files),
        [PathBuf::from("R/money.R")]
    );
    assert_eq!(
        r_files(Path::new("scripts"), "helpers.R", &files),
        [PathBuf::from("scripts/helpers.R")]
    );
    assert!(r_files(Path::new("R"), "missing.R", &files).is_empty());
}

#[test]
fn a_package_qualifier_outside_the_project_is_foreign() {
    let dir = std::env::temp_dir().join(format!("merl-r-pkg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let line = "x <- dplyr::filter(a) + shop:::helper(b) + filter(c)";
    let at = |w: &str| line.find(w).unwrap();
    assert!(r_foreign_package(line, at("filter(a)"), &dir));
    assert!(r_foreign_package(line, at("helper"), &dir));
    std::fs::write(dir.join("DESCRIPTION"), "Package: shop\nVersion: 1.0\n").unwrap();
    assert!(r_foreign_package(line, at("filter(a)"), &dir));
    assert!(!r_foreign_package(line, at("helper"), &dir));
    assert!(!r_foreign_package(line, at("filter(c)"), &dir));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn r_is_told_by_its_extension_and_its_profile() {
    assert_eq!(kind_of(Path::new("R/money.R")), Some(Kind::R));
    assert_eq!(kind_of(Path::new("R/money.r")), Some(Kind::R));
    assert_eq!(kind_of(Path::new(".Rprofile")), Some(Kind::R));
    assert_eq!(kind_of(Path::new("report.Rmd")), None);
    assert_eq!(word_chars(Some(Kind::R), false), ".");
}
