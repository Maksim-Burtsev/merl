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
    // `.tsx` finds its types in `.ts` and its helpers in `.js`.
    assert!(in_def_scope(
        Kind::TsJs,
        Path::new("ui/app.tsx"),
        Path::new("lib/types.ts")
    ));
    assert!(in_def_scope(
        Kind::TsJs,
        Path::new("ui/app.tsx"),
        Path::new("e.js")
    ));
    assert!(!in_def_scope(
        Kind::Rust,
        Path::new("src/main.rs"),
        Path::new("build.py")
    ));
    // A Kotlin file finds the Java class it calls, and the other way round.
    assert!(in_def_scope(
        Kind::Jvm,
        Path::new("app/src/App.kt"),
        Path::new("lib/src/Invoice.java")
    ));
    assert!(!in_def_scope(
        Kind::Jvm,
        Path::new("app/src/App.kt"),
        Path::new("lib/src/invoice.py")
    ));
    // A Rakefile finds the class it drives in the library it loads.
    assert!(in_def_scope(
        Kind::Ruby,
        Path::new("Rakefile"),
        Path::new("lib/invoice.rb")
    ));
    // A migration finds the table it alters in whatever file created it.
    assert!(in_def_scope(
        Kind::Sql,
        Path::new("migrations/002.sql"),
        Path::new("schema.ddl")
    ));
    assert!(!in_def_scope(
        Kind::Sql,
        Path::new("migrations/002.sql"),
        Path::new("notes.md")
    ));
}

/// #369: Ruby's roots come from `Gemfile.lock`, `.bundle/config` and `.ruby-version`, read and
/// never run: the gems at the versions it locks, a `GIT` gem's checkout, the Ruby a version
/// manager in `HOME` installed, with its standard library and the newest `rbs` gem's `core/`.
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
        // A version the lockfile does not name is not read.
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
            // A gem is read from its `lib`, when it has one.
            gems.join("gems/rack-3.1.0/lib"),
            // `BUNDLE_PATH` comes before the Ruby's own gems.
            bundle.join("gems/rack-attack-6.7.0"),
            gems.join("gems/nokogiri-1.16.0-arm64-darwin"),
        ]
    );
    assert!(!asked.get(), "a version manager's Ruby needs no `ruby` run");
    // Without that Ruby the one on the PATH is asked: its standard library, then its gem path.
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
        ]
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
    // With none of the locked gems installed there are no roots at all, whatever Ruby, standard
    // library and core signatures are there: `d` stays in the project, as before #369.
    write(
        &root.join("Gemfile.lock"),
        "GEM\n  specs:\n    missing (1.0.0)\n",
    );
    std::fs::create_dir_all(path.join("gems/rbs-3.9.1/core")).unwrap();
    let said = format!("{}\n{}\n", tmp.join("lib").display(), path.display());
    assert!(ruby_roots(&root, &home, &[], || Some(said)).is_empty());
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
