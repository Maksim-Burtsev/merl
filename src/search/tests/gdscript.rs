use super::*;

const SCRIPT: &str = "class_name Player extends CharacterBody2D
signal died
const SPEED := 200.0
@export var health := 100
enum State { IDLE, RUN }
enum {
\tLEFT,
\tRIGHT = 2,
}
class Stats:
\tvar speed := 1.0
\tfunc create(a: int,
\t\t\tb := Vector2(1, 2)) -> Stats:
\t\tvar local := a
\t\tif a > 0:
\t\t\tvar inner := 1
\t\tfor item in [b]:
\t\t\tprint(item, inner)
\t\tvar f = func(x): return x
\t\treturn Stats.new()
\"\"\"
var speed := 0
\"\"\"
func _ready():
\tspeed = 1
";

fn declared(word: &str) -> Vec<usize> {
    let re = Regex::new(&gdscript_patterns(word).join("|")).unwrap();
    let lines: Vec<&str> = SCRIPT.lines().collect();
    let literal = literal_lines(Kind::Gdscript, SCRIPT);
    (lines.iter().enumerate())
        .filter(|&(i, l)| {
            !literal[i]
                && re.is_match(l)
                && declares_where(Kind::Gdscript, Path::new("a.gd"), word, i + 1, l, || &lines)
        })
        .map(|(i, _)| i + 1)
        .collect()
}

#[test]
fn gdscript_declares_its_forms_and_no_local() {
    for (word, lines) in [
        ("Player", vec![1]),
        ("CharacterBody2D", vec![]),
        ("died", vec![2]),
        ("SPEED", vec![3]),
        ("health", vec![4]),
        ("State", vec![5]),
        ("RUN", vec![5]),
        ("LEFT", vec![7]),
        ("RIGHT", vec![8]),
        ("Stats", vec![10]),
        ("speed", vec![11]),
        ("create", vec![12]),
        ("local", vec![]),
        ("inner", vec![]),
        ("item", vec![]),
        ("_ready", vec![24]),
    ] {
        assert_eq!(declared(word), lines, "{word}");
    }
}

fn bound(line: usize, name: &str) -> Vec<usize> {
    bindings(Kind::Gdscript, SCRIPT, line, name)
        .iter()
        .map(|b| b.line1)
        .collect()
}

#[test]
fn gdscript_locals_stay_in_their_function() {
    assert_eq!(bound(14, "a"), [12]);
    assert_eq!(bound(16, "b"), [13]);
    assert_eq!(bound(18, "item"), [17]);
    assert_eq!(bound(18, "local"), [14]);
    assert_eq!(bound(18, "inner"), Vec::<usize>::new());
    assert_eq!(bound(16, "inner"), [16]);
    assert_eq!(bound(19, "x"), [19]);
    assert_eq!(bound(25, "speed"), Vec::<usize>::new());
    assert_eq!(bound(11, "speed"), Vec::<usize>::new());
    assert_eq!(bound(25, "local"), Vec::<usize>::new());
}

#[test]
fn gdscript_symbols_list_types_functions_and_signals_only() {
    let re = Regex::new(GDSCRIPT_SYMBOL).unwrap();
    let names: Vec<String> = SCRIPT.lines().filter_map(|l| symbol_name(&re, l)).collect();
    assert_eq!(
        names,
        ["Player", "died", "State", "Stats", "create", "_ready"]
    );
    assert!(!shared_symbols(Some(Kind::Gdscript)));
}

#[test]
fn gdscript_paths_autoloads_and_words_that_name_nothing() {
    let line = r#"const COIN = preload("res://actors/coin.tscn")"#;
    assert_eq!(
        gdscript_path(line, 25).as_deref(),
        Some("res://actors/coin.tscn")
    );
    assert_eq!(gdscript_path(line, 6), None);
    let files = [
        PathBuf::from("game/project.godot"),
        PathBuf::from("game/actors/coin.tscn"),
    ];
    assert_eq!(
        gdscript_files(
            Path::new("game/actors/player.gd"),
            "res://actors/coin.tscn",
            &files
        ),
        [PathBuf::from("game/actors/coin.tscn")]
    );
    let project = "[application]\nGameState=\"*res://x.gd\"\n[autoload]\nGameState=\"*res://globals/game_state.gd\"\nHud=\"res://ui/hud.gd\"\n[input]\nA=\"*res://a.gd\"\n";
    assert_eq!(
        gdscript_autoloads(project),
        [(
            "GameState".to_owned(),
            "res://globals/game_state.gd".to_owned()
        )]
    );
    for (line, word, nothing) in [
        ("\t$Sprite2D.play()", "Sprite2D", true),
        ("\t$UI/HealthBar.value = 1", "HealthBar", true),
        ("\t%HealthBar.value = 1", "HealthBar", true),
        ("\tvar bag = {\"speed\": 1}", "speed", true),
        ("\temit_signal(\"died\")", "died", false),
        ("\tdied.emit() # \"died\"", "died", false),
        ("\tvar s = 'it\\'s' + speed", "speed", false),
    ] {
        let at = line.rfind(word).unwrap();
        assert_eq!(gdscript_names_nothing(line, at), nothing, "{line}");
    }
}

#[test]
fn gdscript_strings_and_comments_over_lines() {
    let text = "\"\"\"\nfunc a():\n\"\"\"\n'''\nvar b\n'''\n## doc \"\"\"\nvar c = &\"x\"\nvar d = ^\"y\"\nfunc e():\n";
    let hidden: Vec<usize> = (literal_lines(Kind::Gdscript, text).iter().enumerate())
        .filter(|(_, l)| **l)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(hidden, [2, 3, 5, 6]);
}
