// Every construct `f` folds in Rust, and the cases that once broke it.
use std::collections::HashMap;  // f: 2-3
use std::fmt;  // f: 2-3
// a comment ends a run of uses
use std::{  // f: 5-8
    io,  // f: 5-8
    path::Path,  // f: 5-8
};  // f: 5-8

/* a block comment /* nested */ still open {
} */

#[derive(  // f: 13-16
    Debug,  // f: 13-16
    Clone,  // f: 13-16
)]  // f: 13-16
pub(crate) struct Point<'a> {  // f: 17-20
    name: &'a str,  // f: 17-20
    x: i32,  // f: 17-20
}  // f: 17-20

pub enum Shape {  // f: 22-27
    Circle(f64),  // f: 22-27
    Square {  // f: 22-27
        side: f64,  // f: 22-27
    },  // f: 22-27
}  // f: 22-27

pub trait Area {  // f: 29-31
    fn area(&self) -> f64;  // f: 29-31
}  // f: 29-31

impl<'a> fmt::Display for Point<'a>  // f: 33-44
where  // f: 33-44
    'a: 'static,  // f: 33-44
{  // f: 33-44
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {  // f: 37-43
        write!(  // f: 38-42
            f,  // f: 37-43
            "({}, {})",  // f: 37-43
            self.x, self.name,  // f: 37-43
        )  // f: 37-43
    }  // f: 37-43
}  // f: 33-44

const LIMITS: [u32; 3] = [  // f: 46-49
    1, 2,  // f: 46-49
    3,  // f: 46-49
];  // f: 46-49

type Table =  // f: 51-52
    HashMap<String, Vec<u32>>;  // f: 51-52

macro_rules! square {  // f: 54-58
    ($x:expr) => {  // f: 54-58
        $x * $x  // f: 54-58
    };  // f: 54-58
}  // f: 54-58

extern "C" {  // f: 60-62
    fn abs(x: i32) -> i32;  // f: 60-62
}  // f: 60-62

pub fn classify(points: &[Point<'_>], raw: &str) -> Result<usize, String> {  // f: 64-114
    let quote = '"';  // f: 64-114
    let brace = '{';  // f: 64-114
    let text = r#"a raw string { with "quotes"  // f: 67-72
    } over lines"#;  // f: 64-114
    let total = points  // f: 69-70
        .iter()  // f: 64-114
        .filter(|p| p.x > 0)  // f: 64-114
        .count();  // f: 64-114
    let origin = Point {  // f: 73-76
        name: "origin",  // f: 64-114
        x: 0,  // f: 64-114
    };  // f: 64-114
    let Some(first) = points.first() else {  // f: 77-91
        return Err(format!(  // f: 78-80
            "no points in {raw}"  // f: 64-114
        ));  // f: 64-114
    };  // f: 64-114
    let sizes = if first.x > 0 {  // f: 82-83
        vec![1, 2]  // f: 64-114
    } else {  // f: 84-86
        vec![3]  // f: 64-114
    }  // f: 64-114
    .into_iter()  // f: 64-114
    .map(|n| -> u32 {  // f: 88-90
        n * 2  // f: 64-114
    })  // f: 64-114
    .collect::<Vec<_>>();  // f: 64-114
    'outer: for p in points {  // f: 92-106
        match p.x {  // f: 93-99
            0 => continue,  // f: 64-114
            n if n > 10 => {  // f: 95-97
                break 'outer;  // f: 64-114
            }  // f: 64-114
            _ => {}  // f: 64-114
        }  // f: 64-114
        while total > 0 {  // f: 100-105
            #[cfg(debug_assertions)]  // f: 102-104
            {  // f: 102-104
                println!("{}", square!(n));  // f: 64-114
            }  // f: 64-114
        }  // f: 64-114
    }  // f: 64-114
    unsafe {  // f: 107-109
        abs(-1);  // f: 64-114
    }  // f: 64-114
    loop {  // f: 110-112
        break;  // f: 64-114
    }  // f: 64-114
    Ok(sizes.len() + text.len() + quote.len_utf8() + brace.len_utf8() + origin.x as usize)  // f: 64-114
}  // f: 64-114

#[cfg(test)]
mod tests {  // f: 117-127
    use super::*;  // f: 117-127

    #[test]  // f: 117-127
    fn classifies() {  // f: 121-126
        assert_eq!(  // f: 122-125
            classify(&[], "x"),  // f: 121-126
            Err("no points in x".into()),  // f: 121-126
        );  // f: 121-126
    }  // f: 121-126
}  // f: 117-127
