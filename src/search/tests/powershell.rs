use super::*;

#[test]
fn a_powershell_name_holds_its_dashes() {
    let line = "$user = get-shopuser -Id $Id";
    let word = |col| definition_word(Some(Kind::PowerShell), line, col).map(|(_, w)| w);
    assert_eq!(word(12), Some("get-shopuser"));
    assert_eq!(
        word(23),
        Some("Id"),
        "a named argument's `-` is no part of its name"
    );
    assert_eq!(word(27), Some("Id"), "nor is the `$` of a variable");
}

#[test]
fn powershell_symbol_names() {
    let ps = |line| listed(Kind::PowerShell, line);
    assert_eq!(ps("function Get-ShopUser {"), ["Get-ShopUser"]);
    assert_eq!(ps("function global:Get-ShopUser($Id) {"), ["Get-ShopUser"]);
    assert_eq!(ps("  filter Select-Active {"), ["Select-Active"]);
    assert_eq!(ps("class Invoice : Base {"), ["Invoice"]);
    assert_eq!(ps("[Flags()] enum Status {"), ["Status"]);
    for none in [
        "$Config = @{",
        "  [string] $Name",
        "Set-Alias gu Get-ShopUser",
        "Get-X",
    ] {
        assert!(ps(none).is_empty(), "{none}");
    }
}

#[test]
fn powershell_literals_run_over_lines() {
    let text = "<#\n.EXAMPLE\nfunction A {}\n#>\n$h = @\"\nfunction B {\n\"@\n$q = @'\nfunction D {\n'@\n$x = 1 `\n  + 2 # <# no block\nfunction C {}\n";
    let literal: Vec<usize> = literal_lines(Kind::PowerShell, text)
        .iter()
        .enumerate()
        .filter(|(_, l)| **l)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(
        literal,
        [2, 3, 4, 6, 9],
        "the help block, the here-strings' bodies; a backtick continues a line and opens nothing, \
         and a `<#` in a comment opens nothing either"
    );
}

#[test]
fn a_backtick_continuation_keeps_every_line_below_in_place() {
    let text = "$x = Get-Item `\n    -Path .\n<#\nfunction A {}\n#>\n$t = @\"\nfunction B {\n\"@\n";
    let literal = literal_lines(Kind::PowerShell, text);
    assert_eq!(literal.len(), 9, "one flag per line, and one past the last");
    let at: Vec<usize> = (1..=8).filter(|&n| literal[n - 1]).collect();
    assert_eq!(at, [4, 5, 7]);
}

#[test]
fn a_named_argument_is_no_variable() {
    assert!(powershell_argument("Get-ShopUser -"));
    assert!(powershell_argument("Get-ShopUser($a, -"));
    assert!(!powershell_argument("$total-"));
    assert!(!powershell_argument("$"));
}

#[test]
fn u_marks_what_the_spelling_declares() {
    assert!(powershell_declares_here("class Tariff {", "Tariff"));
    assert!(!powershell_declares_here(
        "$tariff = [Tariff]::new(5)",
        "Tariff"
    ));
    assert!(powershell_declares_here("$Tariff = 1", "Tariff"));
    assert!(powershell_declares_here("$env:Path = 1", "Path"));
    let mut env = powershell_patterns("Path");
    powershell_sigil(&mut env, "$env:", "");
    let env = Regex::new(&env.join("|")).unwrap();
    assert!(env.is_match("$env:Path += ':/x'"));
    assert!(
        !env.is_match("$Path = 1"),
        "a plain `$Path` declares no environment variable"
    );
}

#[test]
fn d_follows_a_dot_sourced_or_imported_path() {
    let at = |line: &str, col| powershell_import(line, col);
    assert_eq!(at(". ./helpers.ps1", 4).as_deref(), Some("helpers.ps1"));
    assert_eq!(
        at(". $PSScriptRoot/lib/x.ps1", 20).as_deref(),
        Some("lib/x.ps1")
    );
    assert_eq!(
        at(r#". "$PSScriptRoot\lib\x.ps1""#, 20).as_deref(),
        Some("lib/x.ps1")
    );
    assert_eq!(
        at("Import-Module ./Shop/Users.psm1", 22).as_deref(),
        Some("Shop/Users.psm1")
    );
    assert_eq!(
        at("using module ./Shop.psm1", 16).as_deref(),
        Some("Shop.psm1")
    );
    assert_eq!(
        at("Import-Module ./Shop/Users.psm1", 3),
        None,
        "off the path"
    );
    assert_eq!(at("Import-Module Pester", 16), None, "a module by name");
    assert_eq!(at("Get-Item ./x.ps1", 11), None, "a call");
}

#[test]
fn powershell_module_directories() {
    let home = Path::new("/home/u");
    let env = Some(std::ffi::OsString::from("/a/Modules:/b/Modules"));
    assert_eq!(
        powershell_roots(env, home, None),
        [PathBuf::from("/a/Modules"), PathBuf::from("/b/Modules")]
    );
    assert_eq!(
        powershell_roots(None, home, None),
        [
            PathBuf::from("/home/u/.local/share/powershell/Modules"),
            PathBuf::from("/usr/local/share/powershell/Modules"),
        ]
    );
    let dir = std::env::temp_dir().join(format!("merl-pwsh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("opt/7")).unwrap();
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::write(dir.join("opt/7/pwsh"), "").unwrap();
    std::os::unix::fs::symlink(dir.join("opt/7/pwsh"), dir.join("bin/pwsh")).unwrap();
    let roots = powershell_roots(Some("".into()), home, Some(dir.join("bin/pwsh")));
    let real = std::fs::canonicalize(dir.join("opt/7"))
        .unwrap()
        .join("Modules");
    assert_eq!(roots.len(), 3, "an empty `PSModulePath` is none");
    assert_eq!(
        roots[2], real,
        "the `Modules` beside `pwsh` is the real file's"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_parameter_is_a_local_of_the_blocks_around_it() {
    let text = "param([string]$Root)\nfunction A {\n    param(\n        [string]$Id\n    )\n    $Id\n}\nfunction B {\n    $Id\n    $Root\n}\n";
    let at = |line, name| -> Vec<usize> {
        bindings(Kind::PowerShell, text, line, name)
            .iter()
            .map(|b| b.line1)
            .collect()
    };
    assert_eq!(at(6, "id"), [4], "names ignore case");
    assert!(
        at(9, "Id").is_empty(),
        "a function beside the cursor's is not around it"
    );
    assert_eq!(at(10, "Root"), [1], "the script's block is around it");
}
