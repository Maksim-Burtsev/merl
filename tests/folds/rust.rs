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

pub fn classify(points: &[Point<'_>], raw: &str) -> Result<usize, String> {  // f: 64-115
    /* /* a nested comment */ closes nothing } */  // f: 64-115
    let quote = '"';  // f: 64-115
    let brace = '{';  // f: 64-115
    let text = r#"a raw string { with "quotes"  // f: 68-73
    } over lines"#;  // f: 64-115
    let total = points  // f: 70-71
        .iter()  // f: 64-115
        .filter(|p| p.x > 0)  // f: 64-115
        .count();  // f: 64-115
    let origin = Point {  // f: 74-77
        name: "origin",  // f: 64-115
        x: 0,  // f: 64-115
    };  // f: 64-115
    let Some(first) = points.first() else {  // f: 78-92
        return Err(format!(  // f: 79-81
            "no points in {raw}"  // f: 64-115
        ));  // f: 64-115
    };  // f: 64-115
    let sizes = if first.x > 0 {  // f: 83-84
        vec![1, 2]  // f: 64-115
    } else {  // f: 85-87
        vec![3]  // f: 64-115
    }  // f: 64-115
    .into_iter()  // f: 64-115
    .map(|n| -> u32 {  // f: 89-91
        n * 2  // f: 64-115
    })  // f: 64-115
    .collect::<Vec<_>>();  // f: 64-115
    'outer: for p in points {  // f: 93-107
        match p.x {  // f: 94-100
            0 => continue,  // f: 64-115
            n if n > 10 => {  // f: 96-98
                break 'outer;  // f: 64-115
            }  // f: 64-115
            _ => {}  // f: 64-115
        }  // f: 64-115
        while total > 0 {  // f: 101-106
            #[cfg(debug_assertions)]  // f: 103-105
            {  // f: 103-105
                println!("{}", square!(n));  // f: 64-115
            }  // f: 64-115
        }  // f: 64-115
    }  // f: 64-115
    unsafe {  // f: 108-110
        abs(-1);  // f: 64-115
    }  // f: 64-115
    loop {  // f: 111-113
        break;  // f: 64-115
    }  // f: 64-115
    Ok(sizes.len() + text.len() + quote.len_utf8() + brace.len_utf8() + origin.x as usize)  // f: 64-115
}  // f: 64-115

#[cfg(test)]
mod tests {  // f: 118-128
    use super::*;  // f: 118-128

    #[test]  // f: 118-128
    fn classifies() {  // f: 122-127
        assert_eq!(  // f: 123-126
            classify(&[], "x"),  // f: 122-127
            Err("no points in x".into()),  // f: 122-127
        );  // f: 122-127
    }  // f: 122-127
}  // f: 118-128
