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
}

#[test]
fn ruby_classes_are_read_by_their_nesting() {
    let text = "module Shop\n  class Basket < Base\n    include Taxed\n    prepend Logged\n    extend Finder\n    has_many :lines\n    class << self\n      def build\n      end\n    end\n    def total\n      helper do\n        include Nope\n      end\n    end\n  end\nend\nclass Shop::Order\n  def lines\n  end\nend\n";
    assert_eq!(ruby_class_path(text, 6), "Shop::Basket");
    assert_eq!(
        ruby_class_path(text, 8),
        "Shop::Basket",
        "`class << self` is the class around it"
    );
    assert_eq!(ruby_class_path(text, 19), "Shop::Order");
    assert_eq!(ruby_class_path(text, 1), "");
    assert_eq!(ruby_declared_path(text, 2).as_deref(), Some("Shop::Basket"));
    assert_eq!(ruby_declared_path(text, 18).as_deref(), Some("Shop::Order"));
    assert_eq!(
        ruby_declared_path(text, 7),
        None,
        "`class << self` declares nothing"
    );
    assert_eq!(ruby_declared_path(text, 11), None);
    let RubyParents {
        superclasses,
        includes,
        extends,
    } = ruby_class_parents(text, 2);
    assert_eq!(superclasses, ["Base"]);
    assert_eq!(
        includes,
        ["Taxed", "Logged"],
        "a `prepend` is an `include`, one in a block of a method is not the class's"
    );
    assert_eq!(extends, ["Finder"]);
    assert!(
        ruby_self_is_class(text, 6),
        "the class body calls on the class"
    );
    assert!(ruby_self_is_class(text, 8), "`class << self`");
    assert!(!ruby_self_is_class(text, 12), "an instance method");
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
    let env = tmp.join("env-too");
    std::fs::create_dir_all(env.join("gems/rack-attack-6.7.0")).unwrap();
    assert_eq!(
        ruby_roots(&root, &home, std::slice::from_ref(&env), || None),
        std::slice::from_ref(&beside),
        "`BUNDLE_PATH` before `GEM_HOME` for a gem both hold"
    );
    for manager in [
        ".local/share/mise/installs/ruby/3.3.0",
        ".asdf/installs/ruby/3.3.0",
        ".rubies/ruby-3.3.0",
    ] {
        let stdlib = home.join(manager).join("lib/ruby/3.3.0");
        std::fs::create_dir_all(&stdlib).unwrap();
        assert_eq!(
            ruby_roots(&root, &home, &[], || panic!("asked")),
            [stdlib, beside.clone()],
            "the Ruby of {manager}"
        );
        std::fs::remove_dir_all(home.join(manager.split('/').next().unwrap())).unwrap();
    }
    write(
        &root.join("Gemfile.lock"),
        "PATH\n  remote: engines/local\n  specs:\n    local (0.1.0)\n\nGEM\n  specs:\n    rack-attack (6.7.0)\n",
    );
    std::fs::create_dir_all(tmp.join("bundle/ruby/3.3.0/gems/local-0.1.0")).unwrap();
    assert_eq!(
        ruby_roots(&root, &home, &[], || None),
        std::slice::from_ref(&beside),
        "a `PATH` gem is in the project already"
    );
    std::fs::remove_file(root.join("Gemfile.lock")).unwrap();
    assert!(
        ruby_roots(&root, &home, &[], || panic!("asked")).is_empty(),
        "no lockfile: nothing outside"
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
