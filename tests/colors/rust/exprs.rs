use regex::Regex;

macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut items = vec![1, 2, 3];
    let names: Vec<&str> = ["a", "b"].iter().copied().collect::<Vec<_>>();
    let (c, b, raw) = ('\n', b'x', r#"raw "quoted" str"#);
    let n = 0xFF_u8 as u32 + 1_000i64 as u32 + 2.5e3f64 as u32;
    let re = Regex::new(r"^\d{3}-[a-z]+$")?;

    'outer: for (i, x) in items.iter_mut().enumerate() {
        while let Some(y) = names.get(i) {
            if *x > 2 && !y.is_empty() || i == 0 {
                break 'outer;
            }
            continue;
        }
        *x += square!(2);
    }

    let kind = match n {
        0 => "zero",
        1..=9 | 11 => "small",
        k if k % 2 == 0 => "even",
        _ => "odd",
    };

    let add = |a: i32, b: i32| -> i32 { a + b };
    let moved = move || items.len();
    if let Some(ref first) = names.first() {
        println!("{} {kind} {:?} {}", first, c, add(1, 2));
    }
    assert_eq!(moved(), 3, "len was {}", 3);
    assert!(re.is_match("123-abc"));
    let s = String::from("text").to_uppercase();
    let owned = Box::new(&s[..]);
    let t = std::mem::size_of::<u64>() as usize;
    unsafe {
        let p = &raw const t;
    }
    loop {
        if t > 0 {
            break;
        }
    }
    let r: Result<u8, _> = "7".parse();
    let v = r.map_err(|e| e.to_string())?;
    if v == 7 && b != 0 && raw.len() > 0 && owned.is_empty() == false {
        panic!("never {}", true);
    }
    Ok(())
}
