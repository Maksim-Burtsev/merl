# A small ledger package: a derivation, its options and a test.
{ lib
, stdenv
, fetchFromGitHub
, python3
, writeShellScript
, ...
}@args:

let
  inherit (lib) mkOption types optionalString;
  version = "1.4.2";
  ratio = 1.5;
  count = 42;
  enabled = true;
  nothing = null;
  home = ./src;
  config = <nixpkgs/config.nix>;
  url = https://example.org/ledger;
  name = "ledger-${version}";
  escaped = "a \"quoted\" \${not} \n line";
  long = ''
    first line
    second ''${kept} and ${version}
  '';
  entries = [ 1 2 (count + 1) "four" ];
  total = builtins.foldl' (a: b: a + b) 0 [ 1 2 3 ];
  pick = { amount ? 0, name, ... }: if amount > 0 && name != "" then amount else -amount;
  nested = rec {
    a = 1;
    b = a * 2;
    "quoted key" = b // { c = 3; };
  };
  hasA = nested ? a;
  orElse = nested.missing or "fallback";
  check = assert count >= 1; !enabled || count < 100;
  report = writeShellScript "report" ''
    echo "total: $1" >&2
  '';
in
stdenv.mkDerivation (finalAttrs: {
  pname = "ledger";
  inherit version;

  src = fetchFromGitHub {
    owner = "example";
    repo = "ledger";
    rev = "v${finalAttrs.version}";
    hash = lib.fakeHash;
  };

  nativeBuildInputs = with python3.pkgs; [ setuptools wheel ];

  buildPhase = ''
    make -j"$NIX_BUILD_CORES" PREFIX=$out
  '';

  postInstall = ''
    for f in $out/bin/*; do
      wrapProgram "$f" --set LEDGER_HOME ${home}
    done
  '' + optionalString stdenv.isDarwin ''
    rm -rf $out/lib/*.a
  '';

  /* python */
  passthru.script = ''
    print("ledger", ${toString count})
  '';

  meta = with lib; {
    description = "A small ledger";
    license = licenses.mit;
    platforms = platforms.unix;
    broken = stdenv.hostPlatform.isWindows;
  };
})
