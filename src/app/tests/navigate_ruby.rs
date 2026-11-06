#![expect(clippy::too_many_lines, reason = "test tables")]
use super::*;

#[test]
fn a_call_no_project_declaration_answers_goes_into_the_locked_gems() {
    let gem = "vendor/bundle/ruby/3.3.0/gems/rack-attack-6.7.0/lib/rack/attack.rb";
    let (dir, mut a) = project_app(
        "ruby-gems",
        &[
            (".gitignore", "/vendor/bundle\n"),
            (".bundle/config", "---\nBUNDLE_PATH: \"vendor/bundle\"\n"),
            (
                "Gemfile.lock",
                "GEM\n  remote: https://rubygems.org/\n  specs:\n    rack-attack (6.7.0)\n      rack (>= 1.0, < 4)\n",
            ),
            (
                gem,
                "module Rack\n  class Attack\n    class << self\n      def throttle(name, options, &block)\n        throttles[name] = Throttle.new(name, options, &block)\n      end\n    end\n  end\nend\n",
            ),
            (
                "vendor/bundle/ruby/3.3.0/gems/rack-attack-6.6.0/lib/rack/attack.rb",
                "module Rack\n  class Attack\n    def self.throttle(name)\n    end\n  end\nend\n",
            ),
            (
                "config/initializers/rack_attack.rb",
                "class Rack::Attack\n  throttle('throttle_sign_up_attempts/ip', limit: 25, period: 300) do |req|\n    req.ip\n  end\nend\n",
            ),
        ],
    );
    let roots = search::ruby_roots(&dir, &dir.join("no-home"), &[], || None);
    use_roots(&mut a, Kind::Ruby, &roots);
    d_on(&mut a, "config/initializers/rack_attack.rb", "  throttle");
    assert_eq!(
        shown(&mut a),
        jump(
            "throttle \u{2192} Rack.Attack.throttle (by name, 1 match)",
            &format!("{gem}:4")
        ),
        "a class method a gem declares, called bare in the project's reopening of the gem's class, is that gem's, at the version the lockfile names; a version it does not name is not read"
    );
    assert_eq!(
        a.buf.readonly,
        Some("outside the project"),
        "a gem's file is opened read-only"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_core_signatures_and_the_standard_library_come_first() {
    let (dir, mut a) = project_app(
        "ruby-core",
        &[
            (
                "app/models/settings.rb",
                "class Settings\n  def fetch(key)\n    @values[key]\n  end\nend\n",
            ),
            (
                "app/models/account.rb",
                "class Account < ApplicationRecord\nend\n",
            ),
            (
                "lib/my_gem.rb",
                "module MyGem\n  mattr_accessor :api_key\nend\n",
            ),
            (
                "app/lib/run.rb",
                "class Run\n  def call(options)\n    Account.find(1)\n    options.fetch(:limit)\n    Logger.new($stdout).info('x')\n    config.send_email_changed_notification\n    options.name\n    MyGem.api_key\n    puts 'x'\n  rescue Errno::ENOENT\n    nil\n  end\nend\n",
            ),
        ],
    );
    let root = external_root(
        "ruby-core",
        &[
            (
                "rbs-3.4.0/core/hash.rbs",
                "class Hash[unchecked out K, unchecked out V] < Object\n  def fetch: (K arg0) -> V\n           | [X] (K arg0) { (K arg0) -> X } -> (V | X)\nend\n",
            ),
            (
                "rbs-3.4.0/core/kernel.rbs",
                "module Kernel\n  def self?.puts: (*untyped) -> nil\nend\n",
            ),
            (
                "rbs-3.4.0/core/errno.rbs",
                "module Errno\n  class ENOENT < SystemCallError\n  end\nend\n",
            ),
            (
                "3.3.0/logger.rb",
                "class Logger\n  def info(progname = nil, &block)\n  end\nend\n",
            ),
            (
                "gems/semantic_logger-4.15.0/lib/semantic_logger/base.rb",
                "module SemanticLogger\n  class Base\n    def info(message = nil)\n    end\n  end\nend\n",
            ),
            (
                "gems/activerecord-8.0.0/lib/active_record/core.rb",
                "module ActiveRecord\n  module Core\n    module ClassMethods\n      def find(*ids)\n      end\n    end\n  end\nend\n",
            ),
            (
                "gems/devise-4.9.4/lib/devise.rb",
                "module Devise\n  mattr_accessor :send_email_changed_notification, :api_key\n  @@send_email_changed_notification = false\n\n  def self.setup\n    name = 'devise'\n  end\nend\n",
            ),
        ],
    );
    let roots = [
        "rbs-3.4.0/core",
        "3.3.0",
        "gems/semantic_logger-4.15.0/lib",
        "gems/devise-4.9.4",
        "gems/activerecord-8.0.0",
    ]
    .map(|r| root.join(r));
    use_roots(&mut a, Kind::Ruby, &roots);
    let at = |p: &str| format!("{}", root.join(p).display());
    d_on(&mut a, "app/lib/run.rb", "options.fetch");
    assert_eq!(
        shown(&mut a),
        picker(
            "fetch: by name, 2 declarations",
            &[
                ("Settings.fetch", "app/models/settings.rb:2"),
                ("fetch", "hash.rbs:2"),
            ]
        ),
        "the core's `def name: …` in a `class` is read from the `rbs` gem's signatures; a member found by name in the project gets the ones outside beside it, so the project's one namesake is offered, never jumped to"
    );
    d_on(&mut a, "app/lib/run.rb", ".info");
    assert_eq!(
        shown(&mut a),
        picker(
            "info: by name, 2 declarations",
            &[
                ("Logger.info", "logger.rb:2"),
                (
                    "SemanticLogger.Base.info",
                    "semantic_logger-4.15.0/lib/semantic_logger/base.rb:3"
                ),
            ]
        ),
        "the standard library's hits come before the gems'"
    );
    d_on(&mut a, "app/lib/run.rb", "Errno::ENOENT");
    assert_eq!(
        shown(&mut a),
        picker("ENOENT: by name, 1 match", &[("ENOENT", "errno.rbs:2")]),
        "A constant's path names no file: `Errno` is no directory of the core, and the name is looked for by name, offered as any `::` path no declaration of the project spells"
    );
    d_on(&mut a, "app/lib/run.rb", "Account.find");
    assert_eq!(
        shown(&mut a),
        jump(
            "find \u{2192} ActiveRecord.Core.ClassMethods.find (by name, 1 match)",
            &at("gems/activerecord-8.0.0/lib/active_record/core.rb:4")
        ),
        "A class method the project's class does not declare: ActiveRecord's, by name"
    );
    d_on(&mut a, "app/lib/run.rb", ".send_email_changed_notification");
    assert_eq!(
        shown(&mut a),
        picker(
            "send_email_changed_notification: by name, 1 match",
            &[(
                "Devise.send_email_changed_notification",
                "devise-4.9.4/lib/devise.rb:2"
            )]
        ),
        "`mattr_accessor` declares its name as `attr_accessor` does. On a value of no known type the one row is offered, as without the gems (#390)"
    );
    d_on(&mut a, "app/lib/run.rb", "options.name");
    assert_eq!(
        shown(&mut a),
        jump("no definition for name", "app/lib/run.rb:7"),
        "A local of a gem's method declares nothing by name (#383): `name = 'devise'` is no answer"
    );
    d_on(&mut a, "app/lib/run.rb", "MyGem.api_key");
    assert_eq!(
        shown(&mut a),
        jump(
            "api_key \u{2192} MyGem.api_key (via MyGem)",
            "lib/my_gem.rb:2"
        ),
        "A class-level accessor is the class's own: the project's `MyGem.api_key`, never Devise's"
    );
    d_on(&mut a, "app/lib/run.rb", "  puts");
    assert_eq!(
        shown(&mut a),
        jump(
            "puts \u{2192} Kernel.puts (by name, 1 match)",
            &at("rbs-3.4.0/core/kernel.rbs:2")
        ),
        "`def self?.puts:` declares Kernel's `puts`"
    );
    assert_eq!(
        a.rel_path_of(&root.join("rbs-3.4.0/core/kernel.rbs")),
        "kernel.rbs",
        "Opened, a core signature is named from its root and read as Ruby"
    );
    assert_eq!(a.kind(), Some(Kind::Ruby));
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_ruby_setter_lands_on_its_bare_name_in_attr_writer() {
    assert_eq!(word_col("  attr_writer :name", "name=", ""), 15);
}

#[test]
fn new_on_a_class_with_no_initialize_of_its_own_is_class_new() {
    let (dir, mut a) = project_app(
        "ruby-new",
        &[("app/plain.rb", "class Plain\nend\n\nPlain.new\n")],
    );
    let root = external_root(
        "ruby-new",
        &[(
            "rbs-3.4.0/core/class.rbs",
            "class Class < Module\n  def new: (*untyped) -> untyped\nend\n",
        )],
    );
    use_roots(&mut a, Kind::Ruby, &[root.join("rbs-3.4.0/core")]);
    d_on(&mut a, "app/plain.rb", "Plain.new");
    assert_eq!(
        shown(&mut a),
        jump("no definition for new", "app/plain.rb:4"),
        "asked for as `new`, not `initialize`"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_module_extended_lends_its_instance_methods_to_the_class_side() {
    let (dir, mut a) = project_app(
        "ruby-extend",
        &[
            (
                "app/greeting.rb",
                "module Greeting\n  def hello\n  end\n\n  def self.hello\n  end\nend\n",
            ),
            (
                "app/host.rb",
                "class Host\n  extend Greeting\n\n  def self.run\n    hello\n  end\nend\n",
            ),
        ],
    );
    a.no_external();
    d_on(&mut a, "app/host.rb", "    hello");
    assert_eq!(
        shown(&mut a),
        jump(
            "hello \u{2192} Greeting.hello (via Greeting)",
            "app/greeting.rb:2"
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_ivar_at_the_top_of_a_file_is_that_files_own() {
    let (dir, mut a) = project_app(
        "ruby-top-ivar",
        &[
            (
                "app/a.rb",
                "@x = 1\n\nclass K\n  def m\n    @x = 2\n  end\nend\n\nputs @x\n",
            ),
            ("app/b.rb", "@x = 3\n"),
        ],
    );
    a.no_external();
    d_on(&mut a, "app/a.rb", "puts @x");
    assert_eq!(shown(&mut a), jump("@x: in this file", "app/a.rb:1"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_member_on_new_is_the_class_s_unless_the_class_defines_new_or_is_not_declared() {
    let (dir, mut a) = project_app(
        "ruby-on-new",
        &[
            (
                "app/pots.rb",
                "class Pot\n  def self.new(n)\n    Cup.allocate\n  end\n\n  def fill\n  end\nend\n\nclass Cup\n  def fill\n  end\nend\n",
            ),
            (
                "app/run.rb",
                "Cup.new(1).fill\nPot.new(1).fill\nJar.new(1).fill\n",
            ),
        ],
    );
    a.no_external();
    d_on(&mut a, "app/run.rb", "Cup.new(1).|fill");
    assert_eq!(
        shown(&mut a),
        jump("fill \u{2192} Cup.fill (via Cup)", "app/pots.rb:11")
    );
    let both = [
        ("Pot.fill", "app/pots.rb:6"),
        ("Cup.fill", "app/pots.rb:11"),
    ];
    for code in ["Pot.new(1).|fill", "Jar.new(1).|fill"] {
        d_on(&mut a, "app/run.rb", code);
        assert_eq!(
            shown(&mut a),
            picker("fill: by name, 2 declarations", &both),
            "{code}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn one_press_searches_a_ruby_class_name_once() {
    let (dir, mut a) = project_app(
        "ruby-once",
        &[
            (
                "app/models/account.rb",
                "class Account < Base\n  def go\n    ping\n  end\nend\n",
            ),
            ("app/models/base.rb", "class Base\nend\n"),
            (
                "app/models/other.rb",
                "class Other\n  def ping\n  end\nend\n",
            ),
        ],
    );
    d_on(&mut a, "app/models/account.rb", "    ping");
    assert_eq!(
        shown(&mut a),
        picker(
            "ping: by name, 1 match",
            &[("Other.ping", "app/models/other.rb:2")]
        ),
        "a bare call only an unrelated class declares is offered (#1093)"
    );
    std::fs::write(
        dir.join("app/models/base_ext.rb"),
        "class Base < Other\nend\n",
    )
    .unwrap();
    a.files.push("app/models/base_ext.rb".into());
    let text = std::fs::read_to_string(dir.join("app/models/account.rb")).unwrap();
    assert!(
        a.ruby_self_methods(Path::new("app/models/account.rb"), &text, "ping")
            .is_empty(),
        "the ancestor walks of one press reuse its search for `Base`, so a declaration that appeared after it is not read"
    );
    d_on(&mut a, "app/models/account.rb", "    ping");
    assert_eq!(
        shown(&mut a),
        jump(
            "ping \u{2192} Other.ping (via Other)",
            "app/models/other.rb:2"
        ),
        "the next press searches again"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
