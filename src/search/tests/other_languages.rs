//! Lua, Elixir, Zig, and the shell, SQL, Make, Terraform, Docker and YAML files.

use super::*;

const LUA: &str = r#"local uv = vim.uv

local M = {}
local cache, hits = {}, 0

function M.setup(opts)
  local defaults = { limit = 10 }
  cache = defaults
  return M.normalise(opts)
end

function M:render(row)
  return row
end

function normalise(opts)
  return opts
end

local function trim(s)
  return s
end

M.format = function(row)
  return trim(row)
end

local handlers = {
  open = function(id)
    return id
  end,
  limit = 10,
}

--[[
function M.ghost(x)
  return x
end
]]

M.setup({ limit = 1 })
return M

local pat = [=[
^\s*\%(\[[A-Za-z]\+\]\)* ]-] x
function M.ghosted(x)
end
]=]

function M.after(x)
  return x
end

local sql = [[
function M.ghost2(x)
end
]]

function M.last() end
"#;

#[test]
fn lua_def_patterns_find_functions_and_locals() {
    let (dir, files) = scratch("lua", &[("init.lua", LUA)]);
    let d = |w| defs(&dir, &files, Kind::Lua, w);
    assert_eq!(d("setup"), [6], "the declaration, not the call on line 41");
    assert_eq!(d("render"), [12], "the `M:name` form");
    assert_eq!(d("normalise"), [16], "not the `M.normalise(opts)` call");
    assert_eq!(d("trim"), [20], "`local function`");
    assert_eq!(d("format"), [24], "`M.name = function`");
    assert_eq!(d("open"), [29], "a function in a table of handlers");
    assert_eq!(d("M"), [3]);
    assert_eq!(d("uv"), [1]);
    // `local a, b = …` declares both, and a later bare `cache = …` is an assignment to the
    // local already declared, not a declaration of its own.
    assert_eq!(d("cache"), [4]);
    assert_eq!(d("hits"), [4]);
    assert_eq!(d("defaults"), [7], "a local inside a body");
    assert_eq!(
        d("limit"),
        Vec::<usize>::new(),
        "a table field holding a value has no rule: the line is also an assignment"
    );
    assert_eq!(d("opts"), Vec::<usize>::new(), "a parameter");
    assert_eq!(d("row"), Vec::<usize>::new());
    assert_eq!(
        d("vim"),
        Vec::<usize>::new(),
        "the right-hand side of a local"
    );
    // A `[=[ … ]=]` long string closes on the `=` it was opened with, so neither the
    // `\[[` of the Vim regex inside it nor the `]-]` closes it, and what follows the
    // string is still read as code.
    assert_eq!(d("pat"), [44]);
    assert_eq!(d("after"), [50]);
    assert_eq!(d("last"), [59]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn lua_long_brackets_hide_what_they_hold() {
    // The `--[[ … ]]` block comment on lines 35-39, the `[=[ … ]=]` string on 44-48 and
    // the `[[ … ]]` one on 54-57: the functions inside them declare nothing, the way a
    // Python docstring's example does not.
    let lit = literal_lines(Kind::Lua, LUA);
    assert_eq!(
        lit.iter()
            .enumerate()
            .filter(|(_, l)| **l)
            .map(|(i, _)| i + 1)
            .collect::<Vec<_>>(),
        [36, 37, 38, 39, 45, 46, 47, 48, 55, 56, 57]
    );
    // A `--` line comment is still one line, whatever quote it holds.
    assert!(
        literal_lines(Kind::Lua, "-- don't\nlocal x = 1\n")[1..]
            .iter()
            .all(|l| !l)
    );
}

#[test]
fn lua_scope_roots_and_names() {
    let here = Path::new("lua/config/init.lua");
    assert!(in_def_scope(
        Kind::Lua,
        here,
        Path::new("lua/plugins/ui.lua")
    ));
    assert!(!in_def_scope(Kind::Lua, here, Path::new("main.c")));
    // `require "x"` binds a name, but there is no root to resolve it against and Lua's own
    // `package.path` is the embedding interpreter's, so nothing is bound and nothing is
    // searched outside the project.
    assert!(imports(Kind::Lua, LUA).is_empty());
    assert!(external_roots(Kind::Lua, Path::new("/")).is_empty());
    assert!(member_patterns(Kind::Lua, "setup").is_none());
    // A function nested in another is named under it, as in every kind, and a `--`
    // comment in between is a comment, not a declaration that names nothing.
    assert_eq!(
        qualified(
            Kind::Lua,
            "function M.setup()\n-- a note\n  local function inner() end\nend\n",
            3,
            "inner"
        )
        .as_deref(),
        Some("setup.inner")
    );
}

#[test]
fn lua_symbol_names() {
    let lua = |line| one(Kind::Lua, line);
    for (line, name) in [
        ("function setup(opts)", Some("setup")),
        ("function M.setup(opts)", Some("setup")),
        ("function M:render(row)", Some("render")),
        ("function vim.lsp.util.clamp(x)", Some("clamp")),
        ("local function trim(s)", Some("trim")),
        ("  local function inner()", Some("inner")),
        ("M.format = function(row)", Some("format")),
        ("local format = function(row)", Some("format")),
        ("  open = function(id)", Some("open")),
        // Not a declaration: a call, a field holding a value, a local, a return.
        ("M.setup({ limit = 1 })", None),
        ("  limit = 10,", None),
        ("local M = {}", None),
        ("local cache, hits = {}, 0", None),
        ("  return M.normalise(opts)", None),
        ("  end,", None),
        ("-- function ghost(x)", None),
    ] {
        assert_eq!(lua(line).as_deref(), name, "{line}");
    }
}

const EX: &str = r#"defmodule MyApp.Ledger do
  @moduledoc """
  Examples:

      def ghost(x), do: x
  """

  @timeout 5_000
  @derive {Jason.Encoder, only: [:id]}

  defstruct [:id, :total, currency: "EUR"]

  @type t :: %__MODULE__{}

  @spec parse(String.t()) :: t
  def parse(nil), do: nil

  def parse(raw) when is_binary(raw) do
    %__MODULE__{id: raw}
  end

  defp normalise(raw) do
    String.trim(raw)
  end

  defmacro with_total(do: block) do
    block
  end

  defguard is_positive(n) when n > 0

  defdelegate encode(value), to: Jason

  def timeout, do: @timeout
end

defprotocol Renderable do
  def render(value)
end

defimpl Renderable, for: MyApp.Ledger do
  def render(ledger), do: ledger.id
end

defmodule MyApp.LedgerTest do
  @moduletag :slow
  @tag :external

  defmacrop guard!(x), do: x
  defguardp is_even(n) when rem(n, 2) == 0

  def empty?(rows), do: rows == []
  def put!(row), do: row
end
"#;

#[test]
fn elixir_def_patterns_find_every_def_form() {
    let (dir, files) = scratch("ex", &[("ledger.ex", EX)]);
    let d = |w| defs(&dir, &files, Kind::Elixir, w);
    assert_eq!(
        d("Ledger"),
        [1],
        "the last part of `defmodule MyApp.Ledger`"
    );
    assert_eq!(d("Renderable"), [37], "not the `defimpl` that uses it");
    // Two clauses of one function are two declarations, so both are offered; the `@spec`
    // above them is a promise about `parse`, not its definition.
    assert_eq!(d("parse"), [16, 18]);
    assert_eq!(d("normalise"), [22], "`defp`");
    assert_eq!(d("with_total"), [26], "`defmacro`");
    assert_eq!(d("is_positive"), [30], "`defguard`");
    assert_eq!(d("encode"), [32], "`defdelegate`");
    assert_eq!(d("render"), [38, 42], "the protocol and its implementation");
    // The attribute and the function of the same name are both declarations, of different
    // things, so `d` offers both rather than guessing.
    assert_eq!(d("timeout"), [8, 34]);
    assert_eq!(d("id"), [11], "a struct field, atom list form");
    assert_eq!(d("currency"), [11], "the keyword form of the same line");
    // The attributes the language owns, and the names they talk about.
    assert_eq!(d("t"), Vec::<usize>::new(), "`@type t ::` declares no `t`");
    assert_eq!(d("spec"), Vec::<usize>::new());
    assert_eq!(d("type"), Vec::<usize>::new());
    assert_eq!(d("moduledoc"), Vec::<usize>::new());
    assert_eq!(d("derive"), Vec::<usize>::new());
    assert_eq!(d("MyApp"), Vec::<usize>::new(), "a namespace, not a module");
    assert_eq!(d("raw"), Vec::<usize>::new(), "a parameter");
    assert_eq!(d("block"), Vec::<usize>::new());
    assert_eq!(d("Jason"), Vec::<usize>::new());
    assert_eq!(d("guard"), [49], "`defmacrop`, past the trailing `!`");
    assert_eq!(d("is_even"), [50], "`defguardp`");
    // A name Elixir spells with a trailing `?` or `!` is found from the bare word, as
    // Ruby's is: the cursor on `empty` in `empty?(rows)` reaches `def empty?`.
    assert_eq!(d("empty"), [52]);
    assert_eq!(d("put"), [53]);
    // ExUnit's and Mix's attributes are directives too, so `d` on one has nothing to find
    // rather than a picker of every place the directive is written.
    assert_eq!(d("tag"), Vec::<usize>::new());
    assert_eq!(d("moduletag"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn elixir_heredocs_hide_what_they_hold() {
    // `@moduledoc """ … """` on lines 2-6: the `def ghost(x)` of its example declares
    // nothing, as a Python docstring's does not.
    let lit = literal_lines(Kind::Elixir, EX);
    assert_eq!(
        lit.iter()
            .enumerate()
            .filter(|(_, l)| **l)
            .map(|(i, _)| i + 1)
            .collect::<Vec<_>>(),
        [3, 4, 5, 6]
    );
}

#[test]
fn elixir_scope_roots_and_names() {
    let here = Path::new("lib/my_app/ledger.ex");
    assert!(in_def_scope(
        Kind::Elixir,
        here,
        Path::new("test/ledger_test.exs")
    ));
    assert!(!in_def_scope(Kind::Elixir, here, Path::new("mix.lock")));
    // `alias` and `import` bind names, but `mix` puts the dependencies in `deps/` inside the
    // project, so they are project files already and there is no root to leave for.
    assert!(imports(Kind::Elixir, EX).is_empty());
    assert!(external_roots(Kind::Elixir, Path::new("/")).is_empty());
    assert!(member_patterns(Kind::Elixir, "parse").is_none());
    // A function is named under the module it is written in, as in every kind.
    assert_eq!(
        qualified(Kind::Elixir, EX, 22, "normalise").as_deref(),
        Some("Ledger.normalise")
    );
    assert_eq!(qualified(Kind::Elixir, EX, 1, "Ledger"), None);
}

#[test]
fn elixir_symbol_names() {
    let ex = |line| one(Kind::Elixir, line);
    for (line, name) in [
        ("defmodule MyApp.Ledger do", Some("Ledger")),
        ("defmodule Ledger do", Some("Ledger")),
        ("defprotocol Renderable do", Some("Renderable")),
        ("  def parse(nil), do: nil", Some("parse")),
        ("  def timeout, do: @timeout", Some("timeout")),
        ("  defp normalise(raw) do", Some("normalise")),
        ("  def empty?(rows), do: rows == []", Some("empty?")),
        ("  def put!(row), do: row", Some("put!")),
        ("  defmacro with_total(do: block) do", Some("with_total")),
        ("  defmacrop guard!(x), do: x", Some("guard!")),
        ("  defguard is_positive(n) when n > 0", Some("is_positive")),
        (
            "  defguardp is_even(n) when rem(n, 2) == 0",
            Some("is_even"),
        ),
        ("  defdelegate encode(value), to: Jason", Some("encode")),
        // A `defimpl` names the module `Protocol.Type`, and neither half is its own name;
        // `defstruct` declares every field on one line; an attribute belongs to the language.
        ("defimpl Renderable, for: MyApp.Ledger do", None),
        ("  defstruct [:id, :total]", None),
        ("  @spec parse(String.t()) :: t", None),
        ("  @type t :: %__MODULE__{}", None),
        ("  @moduledoc \"\"\"", None),
        ("  @timeout 5_000", None),
        // The shared pattern called this a declaration of `x`.
        ("    Enum.map(rows, fn x -> x.id end)", None),
        ("    String.trim(raw)", None),
        ("  end", None),
    ] {
        assert_eq!(ex(line).as_deref(), name, "{line}");
    }
}

const ZIG: &str = r#"const std = @import("std");
const Allocator = std.mem.Allocator;

pub const Error = error{OutOfRange};

pub const Ledger = struct {
    total: u32,
    rows: []const Row,

    const empty: Ledger = .{ .total = 0, .rows = &.{} };

    pub fn init(allocator: Allocator) Ledger {
        var self = Ledger{ .total = 0, .rows = &.{} };
        return self;
    }

    pub inline fn isEmpty(self: Ledger) bool {
        return self.rows.len == 0;
    }

    fn compute(self: Ledger) u32 {
        return self.total;
    }
};

pub const Row = struct { id: u32 };

const Status = enum { open, closed };

const Value = union(enum) { n: u32, s: []const u8 };

pub var counter: u32 = 0;
threadlocal var scratch: [16]u8 = undefined;

export fn ledger_total(l: *Ledger) u32 {
    return l.total;
}

pub extern "c" fn strlen(s: [*:0]const u8) usize;

noinline fn slow(x: u32) u32 {
    return x;
}

test "a ledger starts empty" {
    const l = Ledger.init(std.testing.allocator);
    try std.testing.expect(l.isEmpty());
}

const first, const second = .{ 1, 2 };

extern fn puts(s: [*:0]const u8) c_int;

export inline fn fast(x: u32) u32 {
    comptime var seen: u32 = 0;
    seen += x;
    return seen;
}

const help =
    \\```zig
    \\const x = 1;
    \\```
;

pub fn after() void {}
"#;

#[test]
fn zig_def_patterns_find_functions_types_and_constants() {
    let (dir, files) = scratch("zig", &[("ledger.zig", ZIG)]);
    let d = |w| defs(&dir, &files, Kind::Zig, w);
    assert_eq!(d("std"), [1]);
    assert_eq!(d("Allocator"), [2]);
    assert_eq!(d("Error"), [4]);
    assert_eq!(
        d("Ledger"),
        [6],
        "not the literal on line 13 or the call on line 46"
    );
    assert_eq!(d("empty"), [10], "a constant in a struct body");
    assert_eq!(d("init"), [12], "not the `Ledger.init(…)` call on line 46");
    assert_eq!(d("isEmpty"), [17], "`pub inline fn`");
    assert_eq!(d("compute"), [21]);
    assert_eq!(
        d("Row"),
        [26],
        "not the `rows: []const Row` field that uses it"
    );
    assert_eq!(d("Status"), [28], "`const X = enum`");
    assert_eq!(d("Value"), [30], "`const X = union(enum)`");
    assert_eq!(d("counter"), [32], "`pub var`");
    assert_eq!(d("scratch"), [33], "`threadlocal var`");
    assert_eq!(d("ledger_total"), [35], "`export fn`");
    assert_eq!(d("strlen"), [39], r#"`pub extern "c" fn`"#);
    assert_eq!(d("slow"), [41], "`noinline fn`");
    assert_eq!(
        d("self"),
        [13],
        "a local; the parameters of lines 17 and 21 are not"
    );
    assert_eq!(d("l"), [46]);
    assert_eq!(
        d("total"),
        Vec::<usize>::new(),
        "a struct field has no rule"
    );
    assert_eq!(d("id"), Vec::<usize>::new());
    assert_eq!(
        d("ledger"),
        Vec::<usize>::new(),
        "a word inside a test description declares nothing"
    );
    assert_eq!(d("open"), Vec::<usize>::new(), "an enum field");
    // A destructuring declares both names, but only the first one starts the line, and every
    // rule here is anchored there.
    assert_eq!(d("first"), [50]);
    assert_eq!(d("second"), Vec::<usize>::new());
    assert_eq!(d("puts"), [52], "`extern fn`, with no calling convention");
    assert_eq!(d("fast"), [54], "`export inline fn`");
    assert_eq!(d("seen"), [55], "`comptime var`");
    // Zig has no literal that runs over lines: a `\\` string ends with its line, so the
    // markdown fences on 61-63 open nothing and the declaration below them is still found.
    assert_eq!(d("after"), [66]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn zig_has_no_literal_that_runs_over_lines() {
    // A `\\` string holding markdown is idiomatic in Zig, and its ``` fences are not a
    // TypeScript template: reading Zig with the C family's rules would hide every line
    // after the first fence, and `d` would say `no definition` over code it can see.
    assert!(
        literal_lines(Kind::Zig, ZIG).iter().all(|l| !l),
        "a Zig line was taken for the inside of a literal"
    );
}

#[test]
fn zig_scope_roots_and_names() {
    let here = Path::new("src/main.zig");
    assert!(in_def_scope(Kind::Zig, here, Path::new("src/ledger.zig")));
    assert!(!in_def_scope(Kind::Zig, here, Path::new("build.zig.zon")));
    assert!(imports(Kind::Zig, ZIG).is_empty());
    assert!(member_patterns(Kind::Zig, "init").is_none());
    // The standard library `zig env` reports, on a machine that has a `zig`; nothing at all
    // on one that does not, as for every kind whose toolchain is not installed.
    assert!(
        external_roots(Kind::Zig, Path::new("/"))
            .iter()
            .all(|r| r.is_dir() && r.ends_with("std")),
        "a Zig root that is not an existing `std` directory"
    );
    // A method is named under the type it is declared in, as in every kind.
    assert_eq!(
        qualified(Kind::Zig, ZIG, 12, "init").as_deref(),
        Some("Ledger.init")
    );
    assert_eq!(qualified(Kind::Zig, ZIG, 6, "Ledger"), None);
}

#[test]
fn zig_std_comes_from_zig_env() {
    // What `zig env` prints: JSON on some versions, ZON on others.
    let json = "{\n \"zig_exe\": \"/opt/homebrew/bin/zig\",\n \"lib_dir\": \"/opt/lib/zig\",\n \"std_dir\": \"/opt/lib/zig/std\"\n}\n";
    assert_eq!(zig_roots(json), [PathBuf::from("/opt/lib/zig/std")]);
    let zon = ".{ .zig_exe = \"/usr/bin/zig\", .lib_dir = \"/usr/lib/zig\", .std_dir = \"/usr/lib/zig/std\" }\n";
    assert_eq!(zig_roots(zon), [PathBuf::from("/usr/lib/zig/std")]);
    // A version that reports only the library directory the standard library sits in.
    assert_eq!(
        zig_roots("{\"lib_dir\": \"/usr/lib/zig\"}"),
        [PathBuf::from("/usr/lib/zig/std")]
    );
    // No `zig` on this machine: nothing to search outside the project.
    assert!(zig_roots("").is_empty());
}

#[test]
fn zig_symbol_names() {
    let zig = |line| one(Kind::Zig, line);
    for (line, name) in [
        // The shared pattern reads these; the rows of this kind must not list them again.
        ("pub const Ledger = struct {", Some("Ledger")),
        ("const Status = enum { open, closed };", Some("Status")),
        ("const Value = union(enum) { n: u32 };", Some("Value")),
        (
            "    pub fn init(allocator: Allocator) Ledger {",
            Some("init"),
        ),
        ("    fn compute(self: Ledger) u32 {", Some("compute")),
        (
            "export fn ledger_total(l: *Ledger) u32 {",
            Some("ledger_total"),
        ),
        (
            "pub extern \"c\" fn strlen(s: [*:0]const u8) usize;",
            Some("strlen"),
        ),
        // These it has no word for.
        (
            "    pub inline fn isEmpty(self: Ledger) bool {",
            Some("isEmpty"),
        ),
        ("noinline fn slow(x: u32) u32 {", Some("slow")),
        (
            "export inline fn ledger_total(l: *Ledger) u32 {",
            Some("ledger_total"),
        ),
        (
            "pub extern \"c\" inline fn strlen(s: [*:0]const u8) usize;",
            Some("strlen"),
        ),
        (
            "test \"a ledger starts empty\" {",
            Some("a ledger starts empty"),
        ),
        // A global, a local and a field stay off the list, as in every other kind.
        ("pub var counter: u32 = 0;", None),
        ("threadlocal var scratch: [16]u8 = undefined;", None),
        ("        var self = Ledger{ .total = 0 };", None),
        ("    const empty: Ledger = .{ .total = 0 };", None),
        ("    total: u32,", None),
        ("    return self.total;", None),
        ("    try std.testing.expect(l.isEmpty());", None),
    ] {
        assert_eq!(zig(line).as_deref(), name, "{line}");
    }
}

const SH: &str = "#!/usr/bin/env bash\nset -eu\n\nexport ROOT=/srv\nlocal -i tries=3\ndeclare -r -x LIMIT=10\nreadonly NAME=app\nPATH+=:/opt/bin\nalias ll='ls -l'\n\nbuild() {\n  echo \"$ROOT\"\n}\n\nfunction deploy {\n  build\n}\n\nfunction check() {\n  [ \"$NAME\" = app ]\n}\n\nbuild \"$ROOT\"\n";

#[test]
fn shell_def_patterns_find_functions_assignments_and_aliases() {
    let (dir, files) = scratch("sh", &[("run.sh", SH)]);
    let d = |w| defs(&dir, &files, Kind::Shell, w);
    // The definition, not the `build` call on line 16 or line 23.
    assert_eq!(d("build"), [11]);
    assert_eq!(d("deploy"), [15]);
    assert_eq!(d("check"), [19], "`function name()` counts once");
    assert_eq!(d("ROOT"), [4], "not the `\"$ROOT\"` uses");
    assert_eq!(d("tries"), [5]);
    assert_eq!(d("LIMIT"), [6], "behind `declare` and its flags");
    // The `readonly` assignment, not the `[ \"$NAME\" = app ]` test.
    assert_eq!(d("NAME"), [7]);
    assert_eq!(d("PATH"), [8], "`+=` appends to a variable");
    assert_eq!(d("ll"), [9]);
    assert_eq!(d("echo"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

const SQL: &str = r#"CREATE TABLE public.orders (
  id serial PRIMARY KEY,
  customer_id int REFERENCES customers(id)
);

CREATE OR REPLACE FUNCTION total(o int) RETURNS int AS $$ SELECT 0 $$ LANGUAGE sql;

CREATE UNIQUE INDEX orders_id_idx ON public.orders (id);

create materialized view daily_totals as select 1;

CREATE TYPE mood AS ENUM ('ok', 'bad');

CREATE TABLE IF NOT EXISTS billing.invoices (id int);

CREATE TABLE "user" (id int);

WITH recent AS (
  SELECT * FROM public.orders
), older AS (
  SELECT * FROM archive
)
SELECT * FROM recent JOIN older ON true;

SELECT * FROM customers;
JOIN customers ON true
INSERT INTO customers VALUES (1);
ALTER TABLE customers ADD COLUMN x int;
DROP TABLE customers;
"#;

#[test]
fn sql_def_patterns_find_create_statements_and_ctes() {
    let (dir, files) = scratch("sql", &[("schema.sql", SQL)]);
    let d = |w| defs(&dir, &files, Kind::Sql, w);
    // The bare name finds the schema-qualified `CREATE TABLE`.
    assert_eq!(d("orders"), [1]);
    assert_eq!(d("total"), [6]);
    assert_eq!(d("orders_id_idx"), [8]);
    // Lower-case keywords read the same.
    assert_eq!(d("daily_totals"), [10]);
    assert_eq!(d("mood"), [12]);
    assert_eq!(d("invoices"), [14]);
    assert_eq!(d("user"), [16], "a quoted name");
    // The `WITH` and the `,` continuation both open a CTE.
    assert_eq!(d("recent"), [18]);
    assert_eq!(d("older"), [20]);
    // `customers` is only ever used, never created: no definition.
    assert_eq!(d("customers"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

const MAKE: &str = ".PHONY: build test\nCC ?= gcc\nexport CFLAGS := -O2\nbuild test-all: deps\n\t$(CC) -o app\ndeps::\n\t@echo deps\n%.o: %.c\n";

#[test]
fn make_def_patterns_find_targets_and_variables() {
    let (dir, files) = scratch("make", &[("Makefile", MAKE)]);
    let d = |w| defs(&dir, &files, Kind::Make, w);
    assert_eq!(d("build"), [4]);
    assert_eq!(d("test-all"), [4], "one of several targets");
    // The `deps::` rule, not the prerequisite on line 4.
    assert_eq!(d("deps"), [6]);
    assert_eq!(d("CC"), [2]);
    assert_eq!(d("CFLAGS"), [3]);
    assert_eq!(d("gcc"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

const TF: &str = r#"variable "region" {
  default = "eu"
}
locals {
  name = "logs-${var.region}"
  tags = {
    name = "x"
  }
}
resource "aws_s3_bucket" "logs" {
  bucket = local.name
}
data "aws_ami" "logs" {
  most_recent = true
}
module "vpc" {
  source = "./vpc"
}
output "bucket" {
  value = aws_s3_bucket.logs.id
}
"#;

#[test]
fn terraform_def_patterns_resolve_the_address() {
    let (dir, files) = scratch("tf", &[("main.tf", TF)]);
    let d = |w| defs(&dir, &files, Kind::Terraform, w);
    assert_eq!(d("var.region"), [1]);
    assert_eq!(d("aws_s3_bucket.logs.id"), [10]);
    assert_eq!(d("data.aws_ami.logs"), [13]);
    assert_eq!(d("module.vpc.cidr"), [16]);
    // A bare name is any block with that label, as from a `.tfvars` file.
    assert_eq!(d("logs"), [10, 13]);
    assert_eq!(d("region"), [1]);
    // `local.name` matches every `name =`; only the one directly in `locals` survives.
    assert_eq!(d("local.name"), [5, 7]);
    assert_eq!(def_block(Kind::Terraform, "local.name"), Some("locals"));
    assert_eq!(def_block(Kind::Terraform, "var.name"), None);
    assert!(directly_inside(TF, 5, "locals"));
    assert!(!directly_inside(TF, 7, "locals"), "nested in `tags`");
    assert!(!directly_inside(TF, 11, "locals"));
    assert!(!directly_inside(TF, 1, "locals"));
    assert!(!directly_inside(TF, 0, "locals"));
    assert!(def_patterns(Kind::Terraform, "each.key").is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn docker_and_yaml_def_patterns() {
    let docker = "FROM rust:1.80 AS build\nRUN cargo build\nFROM --platform=$BUILDPLATFORM debian AS Runtime\nCOPY --from=build /x /y\n";
    let yaml = "x-common: &common\n  restart: always\nservices:\n  web:\n    <<: *common\n    depends_on: [db-main]\n  db-main:  # the database\n    image: postgres\n.base:\n  script: make\n";
    let (dir, files) = scratch("dy", &[("Dockerfile", docker), ("compose.yml", yaml)]);
    let (docker, yaml) = (files[..1].to_vec(), files[1..].to_vec());
    assert_eq!(defs(&dir, &docker, Kind::Docker, "build"), [1]);
    assert_eq!(defs(&dir, &docker, Kind::Docker, "runtime"), [3]);
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "common"), [1]);
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "db-main"), [7]);
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "web"), [4]);
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "base"), [9]);
    // A key with a value on its line is data, not a definition.
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "image"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}
