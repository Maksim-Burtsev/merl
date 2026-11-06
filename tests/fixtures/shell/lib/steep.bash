# #1335: `split_into NAME TEXT` fills the array NAME, as bats's `run` fills `lines`; `ask`
# reads its first argument as a prompt and `taste` only reads it, so neither names a variable.
split_into() {
  local -r into="$1"
  IFS=$'\n' read -d '' -r -a "$into" <<<"$2" || true
}

ask() {
  read -r -p "$1" answer
}

taste() {
  echo "${!1}"
}

decant() {
  split_into cups "$1"
  ask whistle
  taste sediment
  printf -v "$2" '%s' "$1"
}

# A one-line function just above a writer writes nothing: `top_up` fills no `beaker`.
top_up() { echo "$1"; }
brim() {
  printf -v "$1" '%s' full
}

brew_tray() {
  top_up beaker
  brim flask
}
