use super::*;

#[test]
fn definition_scope_follows_the_kind() {
    let here = Path::new("infra/app/main.tf");
    assert!(in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/app/vars.tf")
    ));
    assert!(!in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/vpc/vars.tf")
    ));
    assert!(!in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/app/notes.md")
    ));
    let compose = Path::new("compose.yml");
    assert!(in_def_scope(Kind::Yaml, compose, compose));
    assert!(!in_def_scope(Kind::Yaml, compose, Path::new("other.yml")));
    assert!(in_def_scope(
        Kind::Make,
        Path::new("Makefile"),
        Path::new("tests/rules.mk")
    ));
    assert!(!in_def_scope(
        Kind::Make,
        Path::new("Makefile"),
        Path::new("a.py")
    ));
    assert!(
        in_def_scope(
            Kind::TsJs,
            Path::new("ui/app.tsx"),
            Path::new("lib/types.ts")
        ),
        "`.tsx` finds its types in `.ts`"
    );
    assert!(
        in_def_scope(Kind::TsJs, Path::new("ui/app.tsx"), Path::new("e.js")),
        "and its helpers in `.js`"
    );
    assert!(!in_def_scope(
        Kind::Rust,
        Path::new("src/main.rs"),
        Path::new("build.py")
    ));
    assert!(
        in_def_scope(
            Kind::Jvm,
            Path::new("app/src/App.kt"),
            Path::new("lib/src/Invoice.java")
        ),
        "a Kotlin file finds the Java class it calls"
    );
    assert!(!in_def_scope(
        Kind::Jvm,
        Path::new("app/src/App.kt"),
        Path::new("lib/src/invoice.py")
    ));
    assert!(
        in_def_scope(
            Kind::Ruby,
            Path::new("Rakefile"),
            Path::new("lib/invoice.rb")
        ),
        "a Rakefile finds the class it drives in the library it loads"
    );
    assert!(
        in_def_scope(
            Kind::Sql,
            Path::new("migrations/002.sql"),
            Path::new("schema.ddl")
        ),
        "a migration finds the table it alters in whatever file created it"
    );
    assert!(!in_def_scope(
        Kind::Sql,
        Path::new("migrations/002.sql"),
        Path::new("notes.md")
    ));
    for (kind, here, other) in [
        (Kind::Markdown, "README.md", "docs/guide.md"),
        (Kind::Html, "index.html", "about.html"),
        (Kind::Docker, "Dockerfile", "web/Dockerfile"),
    ] {
        assert!(in_def_scope(kind, Path::new(here), Path::new(here)));
        assert!(
            !in_def_scope(kind, Path::new(here), Path::new(other)),
            "a {kind:?} file holds its own anchors and stages"
        );
    }
}

#[test]
fn ruby_roots_are_the_locked_gems_of_the_ruby_the_project_names() {
    let tmp = std::env::temp_dir().join(format!("merl-ruby-roots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (root, home) = (tmp.join("app"), tmp.join("home"));
    let write = |path: &Path, text: &str| {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        &root.join("Gemfile.lock"),
        "GIT\n  remote: https://github.com/heartcombo/devise.git\n  revision: 0123456789abcdef0123\n  specs:\n    devise (4.9.4)\n      bcrypt (~> 3.0)\n\nPATH\n  remote: engines/local\n  specs:\n    local (0.1.0)\n\nGEM\n  remote: https://rubygems.org/\n  specs:\n    rack (3.1.0)\n    rack-attack (6.7.0)\n      rack (>= 1.0, < 4)\n    nokogiri (1.16.0-arm64-darwin)\n    missing (1.0.0)\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  rack-attack\n",
    );
    write(&root.join(".ruby-version"), "ruby-3.3.0\n");
    write(
        &root.join(".bundle/config"),
        "---\nBUNDLE_PATH: \"vendor/bundle\"\n",
    );
    let bundle = root.join("vendor/bundle/ruby/3.3.0");
    let ruby = home.join(".rbenv/versions/3.3.0/lib/ruby");
    let gems = ruby.join("gems/3.3.0");
    for dir in [
        bundle.join("gems/rack-attack-6.7.0"),
        gems.join("gems/rack-attack-6.6.0"),
        gems.join("gems/rack-attack-6.7.0"),
        gems.join("gems/rack-3.1.0/lib"),
        gems.join("gems/nokogiri-1.16.0-arm64-darwin"),
        gems.join("bundler/gems/devise-0123456789ab"),
        gems.join("gems/rbs-3.9.1/core"),
        gems.join("gems/rbs-3.10.0/core"),
        ruby.join("3.3.0"),
    ] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let asked = std::cell::Cell::new(false);
    let ask = || {
        asked.set(true);
        None
    };
    assert_eq!(
        ruby_roots(&root, &home, &[], ask),
        vec![
            gems.join("gems/rbs-3.10.0/core"),
            ruby.join("3.3.0"),
            gems.join("bundler/gems/devise-0123456789ab"),
            gems.join("gems/rack-3.1.0/lib"),
            bundle.join("gems/rack-attack-6.7.0"),
            gems.join("gems/nokogiri-1.16.0-arm64-darwin"),
        ],
        "the newest `rbs` gem's `core/`, the standard library of the Ruby `.ruby-version` names, \
         a `GIT` gem's checkout, then the gems at the versions the lockfile locks: a gem's `lib` \
         when it has one, `BUNDLE_PATH` before the Ruby's own gems, no other version"
    );
    assert!(!asked.get(), "a version manager's Ruby needs no `ruby` run");
    std::fs::remove_dir_all(home.join(".rbenv")).unwrap();
    let path = tmp.join("gem-path");
    let said = format!("{}\n{}\n", tmp.join("lib").display(), path.display());
    std::fs::create_dir_all(path.join("gems/rack-3.1.0")).unwrap();
    let roots = ruby_roots(&root, &home, &[], || Some(said));
    assert_eq!(
        roots,
        vec![
            tmp.join("lib"),
            path.join("gems/rack-3.1.0"),
            bundle.join("gems/rack-attack-6.7.0"),
        ],
        "without that Ruby the one on the PATH is asked: its standard library, then its gem path"
    );
    let env = tmp.join("env-gems");
    std::fs::create_dir_all(env.join("gems/rack-3.1.0")).unwrap();
    let roots = ruby_roots(&root, &home, std::slice::from_ref(&env), || {
        Some(format!(
            "{}\n{}\n",
            tmp.join("lib").display(),
            path.display()
        ))
    });
    assert_eq!(
        roots[1],
        env.join("gems/rack-3.1.0"),
        "`GEM_HOME` and `GEM_PATH` come after `BUNDLE_PATH`, before the Ruby's own gem path"
    );
    write(
        &root.join("Gemfile.lock"),
        "GEM\n  specs:\n    missing (1.0.0)\n",
    );
    std::fs::create_dir_all(path.join("gems/rbs-3.9.1/core")).unwrap();
    let said = format!("{}\n{}\n", tmp.join("lib").display(), path.display());
    assert!(
        ruby_roots(&root, &home, &[], || Some(said)).is_empty(),
        "with none of the locked gems installed there are no roots, whatever Ruby, standard \
         library and core signatures are there"
    );
    write(
        &root.join("Gemfile.lock"),
        "GEM\n  specs:\n    rack-attack (6.7.0)\n",
    );
    let beside = tmp.join("bundle/ruby/3.3.0/gems/rack-attack-6.7.0");
    std::fs::create_dir_all(&beside).unwrap();
    write(&root.join(".bundle/config"), "BUNDLE_PATH: '../bundle'\n");
    assert_eq!(
        ruby_roots(&root, &home, &[], || None).last(),
        Some(&beside),
        "a `BUNDLE_PATH` beside the project is read without its `..`"
    );
    std::fs::remove_file(root.join("Gemfile.lock")).unwrap();
    assert!(
        ruby_roots(&root, &home, &[], || panic!("asked")).is_empty(),
        "no lockfile: nothing outside"
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn a_toolchain_runs_from_the_root_of_the_disk_with_the_go_installed() {
    let command = toolchain("go", &["env", "GOROOT"]);
    assert_eq!(
        command.get_current_dir(),
        Some(Path::new("/")),
        "no `rust-toolchain.toml` or `go.mod` of the project picks the toolchain (#183)"
    );
    assert!(
        command
            .get_envs()
            .any(|(k, v)| k == "GOTOOLCHAIN" && v == Some("local".as_ref())),
        "nor does Go download one"
    );
}

#[test]
fn roots_are_directories_kept_once_in_order_and_never_the_project() {
    let (dir, _) = scratch("existing-once", &[("b/x", ""), ("a/x", ""), ("file", "")]);
    let (a, b) = (dir.join("a"), dir.join("b"));
    assert_eq!(
        existing_once(
            vec![
                b.clone(),
                PathBuf::new(),
                dir.clone(),
                a.clone(),
                dir.join("gone"),
                dir.join("file"),
                b.clone(),
            ],
            &dir
        ),
        [b, a],
        "`sys.path` can list a directory twice, far apart: the first stays where it is"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rust_roots_are_the_locked_crates_in_every_registry_index() {
    let (dir, _) = scratch(
        "locked-crates",
        &[("index-a/.keep", ""), ("index-b/.keep", "")],
    );
    let lock = "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.200\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\ndependencies = [\n \"serde_derive\",\n]\n\n[[package]]\nname = \"merl\"\nversion = \"0.8.3\"\n";
    let mut dirs = locked_crates(&dir, lock);
    dirs.sort();
    assert_eq!(
        dirs,
        [
            dir.join("index-a/merl-0.8.3"),
            dir.join("index-a/serde-1.0.200"),
            dir.join("index-b/merl-0.8.3"),
            dir.join("index-b/serde-1.0.200"),
        ],
        "a `[[package]]` table's `name` and `version` are the directory `name-version`"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn go_roots_are_the_required_modules_as_the_cache_spells_them() {
    let gomod = "module example.com/me\n\ngo 1.22\n\nrequire github.com/BurntSushi/toml v1.3.2\n\nrequire (\n\tgolang.org/x/sys v0.20.0 // indirect\n)\n\nreplace example.com/x => ../x\n";
    let cache = Path::new("/cache");
    assert_eq!(
        required_modules(cache, gomod),
        [
            cache.join("github.com/!burnt!sushi/toml@v1.3.2"),
            cache.join("golang.org/x/sys@v0.20.0"),
        ],
        "a `require` line or a line of its block; upper case is `!` and the letter"
    );
}

#[test]
fn c_roots_are_the_system_headers_the_sdk_frameworks_and_pods() {
    let (sdk, _) = scratch(
        "c-roots",
        &[
            (
                "System/Library/Frameworks/Accelerate.framework/Frameworks/vImage.framework/x",
                "",
            ),
            (
                "System/Library/Frameworks/Foundation.framework/Headers/x",
                "",
            ),
            (
                "System/iOSSupport/System/Library/Frameworks/UIKit.framework/Headers/x",
                "",
            ),
        ],
    );
    let root = Path::new("/project");
    let system = [
        "/usr/include",
        "/usr/local/include",
        "/opt/homebrew/include",
    ]
    .map(PathBuf::from);
    assert_eq!(
        c_roots(None, root),
        [&system[..], &[root.join("Pods")]].concat()
    );
    let frameworks = sdk.join("System/Library/Frameworks");
    assert_eq!(
        c_roots(Some(sdk.clone()), root),
        [
            system[0].clone(),
            sdk.join("usr/include"),
            system[1].clone(),
            system[2].clone(),
            frameworks.clone(),
            sdk.join("System/iOSSupport/System/Library/Frameworks"),
            frameworks.join("Accelerate.framework/Frameworks"),
            root.join("Pods"),
        ],
        "the SDK's headers, its frameworks, UIKit's under `iOSSupport`, those an umbrella \
         framework holds, and CocoaPods' `Pods/` (#417)"
    );
    std::fs::remove_dir_all(&sdk).unwrap();
}

#[test]
#[cfg(unix)]
fn a_framework_is_read_through_its_headers_link_once() {
    let (sdk, _) = scratch(
        "frameworks",
        &[
            (
                "System/Library/Frameworks/Foo.framework/Versions/A/Headers/foo.h",
                "",
            ),
            (
                "System/Library/Frameworks/Foo.framework/Modules/module.h",
                "",
            ),
        ],
    );
    let foo = sdk.join("System/Library/Frameworks/Foo.framework");
    std::os::unix::fs::symlink("A", foo.join("Versions/Current")).unwrap();
    std::os::unix::fs::symlink("Versions/Current/Headers", foo.join("Headers")).unwrap();
    assert_eq!(
        external_files(Kind::C, &[sdk.join("System/Library/Frameworks")]),
        [foo.join("Headers/foo.h")]
    );
    std::fs::remove_dir_all(&sdk).unwrap();
}

#[test]
fn cpp_dirs_are_the_versions_under_cpp() {
    let (dir, _) = scratch(
        "cpp-dirs",
        &[
            ("c++/v1/vector", ""),
            ("c++/13/vector", ""),
            ("c++/README", ""),
        ],
    );
    let mut dirs = cpp_dirs(std::slice::from_ref(&dir));
    dirs.sort();
    assert_eq!(dirs, [dir.join("c++/13"), dir.join("c++/v1")]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn only_the_rbs_gems_core_signatures_are_ruby_files() {
    let (dir, _) = scratch(
        "core-rbs",
        &[
            ("gems/rbs-3.9.1/core/string.rbs", ""),
            ("gems/rbs-3.9.1/stdlib/json/json.rbs", ""),
            ("app/core/user.rbs", ""),
            ("app/core/user.rb", ""),
        ],
    );
    let mut files = external_files(Kind::Ruby, std::slice::from_ref(&dir));
    files.sort();
    assert_eq!(
        files,
        [
            dir.join("app/core/user.rb"),
            dir.join("gems/rbs-3.9.1/core/string.rbs"),
        ],
        "a `.rbs` elsewhere repeats the `.rb` beside it (#369)"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_exports_map_is_a_top_level_key_that_is_not_null() {
    assert!(exports_map(r#"{"exports": {".": "./i.js"}}"#));
    assert!(exports_map(r#"{"exports": "./i.js"}"#));
    assert!(!exports_map(r#"{"exports": null}"#));
    assert!(!exports_map(r#"{"a": {"exports": "./i.js"}}"#));
    assert!(!exports_map(r#"{"main": "exports"}"#));
    assert!(
        exports_map(r#"{"note": "say \"", "exports": "./i.js"}"#),
        "a string is read whole, an escaped quote and all"
    );
}
