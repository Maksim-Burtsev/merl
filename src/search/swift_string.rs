enum Frame {
    Hole(usize),
    Text { hashes: usize, multi: bool },
}

pub fn swift_in_string(text: &str, at: usize) -> bool {
    let b = text.as_bytes();
    let mut stack: Vec<Frame> = Vec::new();
    let mut i = 0;
    while i < at.min(b.len()) {
        let rest = &b[i..];
        match stack.last_mut() {
            Some(&mut Frame::Text { hashes, multi }) => {
                let hashed = |r: &[u8]| {
                    r.get(..hashes)
                        .is_some_and(|h| h.iter().all(|&c| c == b'#'))
                };
                let quotes = if multi { 3 } else { 1 };
                if rest[0] == b'\\' && hashed(&rest[1..]) {
                    i += 1 + hashes;
                    if b.get(i) == Some(&b'(') {
                        stack.push(Frame::Hole(0));
                    }
                    i += 1;
                    continue;
                }
                if rest.len() >= quotes
                    && rest[..quotes].iter().all(|&c| c == b'"')
                    && hashed(&rest[quotes..])
                {
                    stack.pop();
                    i += quotes + hashes;
                    continue;
                }
                if rest[0] == b'\n' && !multi {
                    stack.pop();
                }
                i += 1;
            }
            frame => {
                let hashes = rest.iter().take_while(|&&c| c == b'#').count();
                if rest.get(hashes) == Some(&b'"') {
                    let multi = rest[hashes..].starts_with(b"\"\"\"");
                    stack.push(Frame::Text { hashes, multi });
                    i += hashes + if multi { 3 } else { 1 };
                    continue;
                }
                if rest.starts_with(b"//") {
                    i += rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len());
                    continue;
                }
                if rest.starts_with(b"/*") {
                    i += block_comment(rest);
                    continue;
                }
                match (frame, rest[0]) {
                    (Some(Frame::Hole(depth)), b'(') => *depth += 1,
                    (Some(Frame::Hole(0)), b')') => {
                        stack.pop();
                    }
                    (Some(Frame::Hole(depth)), b')') => *depth -= 1,
                    _ => {}
                }
                i += hashes.max(1);
            }
        }
    }
    matches!(stack.last(), Some(Frame::Text { .. }))
}

fn block_comment(b: &[u8]) -> usize {
    let mut depth = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"/*") {
            depth += 1;
            i += 2;
        } else if b[i..].starts_with(b"*/") {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    i
}
