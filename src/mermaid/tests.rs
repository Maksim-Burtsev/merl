use super::*;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let pairs: Vec<(String, String)> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |k| pairs.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
}

#[test]
fn pictures_need_a_terminal_that_draws_them_and_no_multiplexer() {
    assert!(supported(&env(&[("TERM", "xterm-ghostty")])));
    assert!(supported(&env(&[("TERM", "xterm-kitty")])));
    assert!(supported(&env(&[("TERM_PROGRAM", "WezTerm")])));
    assert!(supported(&env(&[("TERM_PROGRAM", "ghostty")])));
    assert!(!supported(&env(&[
        ("TERM", "xterm-kitty"),
        ("ZELLIJ", "0")
    ])));
    assert!(supported(&env(&[("KITTY_WINDOW_ID", "1")])));
    assert!(!supported(&env(&[("TERM", "xterm-256color")])));
    assert!(!supported(&env(&[
        ("TERM", "xterm-ghostty"),
        ("TMUX", "/tmp/x")
    ])));
    assert!(!supported(&env(&[
        ("TERM_PROGRAM", "ghostty"),
        ("STY", "1")
    ])));
    assert!(!supported(&env(&[("TERM_PROGRAM", "Apple_Terminal")])));
}

#[test]
fn a_narrow_diagram_draws_at_the_size_of_the_text() {
    let cell = (10, 20);
    let (cols, rows) = cells((160, 80), cell, 100).unwrap();
    assert_eq!((cols, rows), (17, 5));
}

#[test]
fn a_wide_diagram_shrinks_to_the_pane_down_to_half_its_size() {
    let cell = (10, 20);
    let natural = cells((1000, 100), cell, 1000).unwrap();
    assert_eq!(natural.0, 107);
    let shrunk = cells((1000, 100), cell, 60).unwrap();
    assert_eq!(shrunk.0, 60);
    assert!(shrunk.1 < natural.1);
    assert_eq!(cells((1000, 100), cell, 50), None);
}

#[test]
fn colours_go_after_front_matter_and_before_everything_else() {
    let init = "%%{init: {}}%%";
    assert_eq!(
        with_colours("graph TD\n  A-->B", init),
        format!("{init}\ngraph TD\n  A-->B")
    );
    assert_eq!(
        with_colours("---\ntitle: T\n---\ngraph TD\n  A-->B", init),
        format!("---\ntitle: T\n---\n{init}\ngraph TD\n  A-->B")
    );
}

#[test]
fn a_diagram_draws_and_a_broken_one_does_not() {
    let job = |src: &str| Job {
        key: Key {
            src: hash(src),
            colours: 0,
        },
        src: Src::Diagram(src.into()),
        colours: String::new(),
    };
    let done = render(job("graph TD\n  A[Start] --> B[Done]"));
    let Ok(drawn) = done.drawn else {
        panic!("a flowchart draws")
    };
    assert!(drawn.natural.0 > 0 && drawn.natural.1 > 0);
    assert!(drawn.h >= drawn.natural.1);
    assert!(render(job("graph TD\n  A[Start --> ")).drawn.is_err());
}

fn drawn() -> Result<Drawn, String> {
    Ok(Drawn {
        frames: vec![(vec![1, 2, 3], 0)],
        natural: (20, 10),
        h: 20,
    })
}

fn flushed(d: &mut Diagrams) -> String {
    let mut out = Vec::new();
    d.flush(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn on() -> Diagrams {
    Diagrams::with_cell(Some((10, 20)))
}

#[test]
fn a_picture_is_sent_once_and_placed_again_only_when_it_moves() {
    let mut d = on();
    d.done(Done {
        key: d.key("graph TD"),
        drawn: drawn(),
    });
    let id = d.pic("graph TD").unwrap().id();
    let place = Place {
        id,
        placement: 1,
        x: 3,
        y: 4,
        cols: 2,
        rows: 1,
        crop_y: 0,
        crop_h: 20,
    };
    d.want = vec![place.clone()];
    let first = flushed(&mut d);
    assert!(first.contains(&format!("a=t,f=100,i={id},")));
    assert!(first.contains(&format!("\x1b[5;4H\x1b_Ga=p,i={id},p=1,")));
    assert_eq!(flushed(&mut d), "");
    d.want = vec![Place { y: 9, ..place }];
    let moved = flushed(&mut d);
    assert!(moved.contains("a=d,d=a") && moved.contains("a=p,") && !moved.contains("a=t"));
    d.want.clear();
    let gone = flushed(&mut d);
    assert!(gone.contains("a=d,d=a") && !gone.contains("a=p"));
    let mut out = Vec::new();
    d.clear(&mut out).unwrap();
    assert_eq!(out, b"\x1b_Ga=d,d=A,q=2\x1b\\");
}

#[test]
fn the_same_diagram_twice_is_placed_twice() {
    let mut d = on();
    d.done(Done {
        key: d.key("graph TD"),
        drawn: drawn(),
    });
    let id = d.pic("graph TD").unwrap().id();
    let place = |placement, y| Place {
        id,
        placement,
        x: 0,
        y,
        cols: 2,
        rows: 1,
        crop_y: 0,
        crop_h: 20,
    };
    d.want = vec![place(1, 0), place(2, 5)];
    let out = flushed(&mut d);
    assert!(out.contains(&format!("a=p,i={id},p=1,")));
    assert!(out.contains(&format!("a=p,i={id},p=2,")));
    assert_eq!(out.matches("a=t,").count(), 1);
}

#[test]
fn every_picture_gets_an_id_of_its_own() {
    let mut d = on();
    for src in ["graph TD", "graph LR", "graph BT"] {
        d.done(Done {
            key: d.key(src),
            drawn: drawn(),
        });
    }
    let ids: HashSet<u32> = ["graph TD", "graph LR", "graph BT"]
        .iter()
        .map(|s| d.pic(s).unwrap().id())
        .collect();
    assert_eq!(ids.len(), 3);
}

#[test]
fn another_theme_drops_the_old_pictures_and_frees_the_sent_ones() {
    let theme = |name| crate::theme::load(name).unwrap();
    let mut d = on();
    let dark = theme("tokyonight-moon");
    d.theme(&dark, dark.line_hl_dim);
    d.done(Done {
        key: d.key("graph TD"),
        drawn: drawn(),
    });
    let id = d.pic("graph TD").unwrap().id();
    d.want = vec![Place {
        id,
        placement: 1,
        x: 0,
        y: 0,
        cols: 2,
        rows: 1,
        crop_y: 0,
        crop_h: 20,
    }];
    flushed(&mut d);
    let light = theme("github-light");
    d.theme(&light, light.line_hl_dim);
    assert!(d.pic("graph TD").is_none());
    assert_eq!(d.jobs().len(), 1);
    d.want.clear();
    assert!(flushed(&mut d).contains(&format!("a=d,d=I,i={id},")));
    d.theme(&light, light.line_hl_dim);
    assert!(d.jobs().is_empty());
}

#[test]
fn at_most_two_diagrams_render_at_once() {
    let mut d = on();
    for src in ["graph TD", "graph LR", "graph BT", "graph RL"] {
        d.fit(src, 100);
    }
    let first = d.jobs();
    assert_eq!(first.len(), 2);
    assert!(d.jobs().is_empty());
    d.done(render(first.into_iter().next().unwrap()));
    assert_eq!(d.jobs().len(), 1);
}

#[test]
fn a_render_that_answers_for_old_colours_is_not_kept() {
    let theme = crate::theme::load("tokyonight-moon").unwrap();
    let mut d = on();
    let key = d.key("graph TD");
    d.theme(&theme, theme.line_hl_dim);
    d.done(Done {
        key,
        drawn: drawn(),
    });
    assert!(d.pic("graph TD").is_none());
    assert!(d.fit("graph TD", 100).is_some());
}

#[test]
fn a_diagram_the_rasterizer_has_to_shrink_stays_source() {
    let chain: Vec<String> = (0..400).map(|i| format!("  N{i} --> N{}", i + 1)).collect();
    let src = format!("graph TD\n{}", chain.join("\n"));
    let job = Job {
        key: Key {
            src: hash(&src),
            colours: 0,
        },
        src: Src::Diagram(src),
        colours: String::new(),
    };
    assert!(render(job).drawn.is_err());
}

#[test]
fn a_size_that_arrives_lays_the_preview_out_again() {
    let mut d = Diagrams {
        cell: Some((10, 20)),
        ..Default::default()
    };
    assert_eq!(d.fit("graph TD\n  A-->B", 100), None);
    let jobs = d.jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(d.fit("graph TD\n  A-->B", 100), None);
    assert!(d.jobs().is_empty());
    let before = d.laid;
    let done = render(jobs.into_iter().next().unwrap());
    d.done(done);
    assert_eq!(d.laid, before + 1);
    assert!(d.fit("graph TD\n  A-->B", 100).is_some());
}

#[test]
fn off_draws_nothing_and_asks_for_nothing() {
    let mut d = Diagrams::default();
    assert_eq!(d.fit("graph TD\n  A-->B", 100), None);
    assert!(d.pic("graph TD\n  A-->B").is_none());
    assert!(d.jobs().is_empty());
}

fn file(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("merl-files-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("pic.png");
    std::fs::write(&path, tag).unwrap();
    path
}

fn decoded(d: &mut Diagrams, path: &std::path::Path, frames: &[u32]) {
    d.done(Done {
        key: file_key(path).unwrap(),
        drawn: Ok(Drawn {
            frames: frames.iter().map(|&ms| (vec![1], ms)).collect(),
            natural: (20, 10),
            h: 10,
        }),
    });
}

#[test]
fn a_gif_shows_the_frame_its_clock_has_reached() {
    let mut d = on();
    let path = file("frames");
    decoded(&mut d, &path, &[100, 100]);
    let first = d.file_pic(&path).unwrap().id();
    let key = file_key(&path).unwrap();
    d.started
        .insert(key, Instant::now() - Duration::from_millis(150));
    assert_eq!(d.file_pic(&path).unwrap().id(), first + 1);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_file_picture_outlives_a_theme_change() {
    let mut d = on();
    let path = file("theme");
    decoded(&mut d, &path, &[0]);
    let id = d.file_pic(&path).unwrap().id();
    let light = crate::theme::load("github-light").unwrap();
    d.theme(&light, light.line_hl_dim);
    assert_eq!(d.file_pic(&path).unwrap().id(), id);
    assert!(d.jobs().is_empty());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn file_pictures_off_the_screen_go_past_the_last_four() {
    let mut d = on();
    let paths: Vec<_> = (0..6).map(|i| file(&format!("lru{i}"))).collect();
    for p in &paths {
        decoded(&mut d, p, &[0]);
    }
    let first = d.file_pic(&paths[0]).unwrap().id();
    for p in &paths[1..] {
        d.file_pic(p).unwrap();
    }
    d.want.push(Place {
        id: first,
        placement: 1,
        x: 0,
        y: 0,
        cols: 1,
        rows: 1,
        crop_y: 0,
        crop_h: 10,
    });
    flushed(&mut d);
    d.begin();
    for p in &paths[1..] {
        d.file_pic(p).unwrap();
    }
    d.begin();
    assert!(
        d.file_pic(&paths[1]).is_some(),
        "five on the last screen stay"
    );
    assert!(d.file_pic(&paths[0]).is_none(), "the sixth, off it, goes");
    assert!(flushed(&mut d).contains(&format!("a=d,d=I,i={first},")));
    assert_eq!(
        d.jobs().len(),
        1,
        "and is asked for again when it comes back"
    );
    for p in &paths {
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }
}
