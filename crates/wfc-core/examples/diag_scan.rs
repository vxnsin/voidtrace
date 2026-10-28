//! Why does the scanner miss the inventory? For every `"MiscItems":[` anchor:
//! walk back to the opening brace like the scanner does and report where and
//! why it stops; then report how far forward brace matching gets.
//! cargo run --release -p wfc-core --example diag_scan

fn back(buf: &[u8], pos: usize) -> (Result<usize, String>, usize) {
    let (mut depth, mut in_str, mut i) = (0i32, false, pos);
    while i > 0 {
        i -= 1;
        let c = buf[i];
        if in_str {
            if c == b'"' && (i == 0 || buf[i - 1] != b'\\') {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'}' | b']' => depth += 1,
            b'{' | b'[' => {
                if depth == 0 {
                    return (if c == b'{' { Ok(i) } else { Err(format!("hit '[' at depth 0, {} back", pos - i)) }, pos - i);
                }
                depth -= 1;
            }
            0 => return (Err(format!("NUL at {} back (depth {depth})", pos - i)), pos - i),
            _ => {}
        }
    }
    (Err("region start".into()), pos)
}

fn fwd(s: &[u8]) -> Result<usize, String> {
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    for (i, &c) in s.iter().enumerate() {
        if in_str {
            if esc { esc = false } else if c == b'\\' { esc = true } else if c == b'"' { in_str = false }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => { depth -= 1; if depth == 0 { return Ok(i + 1); } }
            0 => return Err(format!("NUL after {i} bytes (depth {depth})")),
            _ => {}
        }
    }
    Err(format!("buffer end after {} bytes", s.len()))
}

fn show(b: &[u8]) -> String {
    b.iter().map(|&c| if (0x20..0x7f).contains(&c) { (c as char).to_string() } else { format!("\\x{c:02x}") }).collect()
}

fn main() -> anyhow::Result<()> {
    // Concatenate contiguous regions exactly like the scanner's windows do.
    let mut runs: Vec<(usize, Vec<u8>)> = Vec::new();
    wfc_core::memory::walk_memory(|base, buf| {
        match runs.last_mut() {
            Some((b, v)) if *b + v.len() == base && v.len() < (256 << 20) => v.extend_from_slice(buf),
            _ => runs.push((base, buf.to_vec())),
        }
    })?;
    for (base, buf) in &runs {
        for pos in memchr::memmem::find_iter(buf, b"\"MiscItems\":[") {
            let (start, dist) = back(buf, pos);
            println!("anchor {:#x}", base + pos);
            match start {
                Ok(s) => {
                    println!("  start {:#x} ({dist} back): {}", base + s, show(&buf[s..(s + 60).min(buf.len())]));
                    println!("  before start: {}", show(&buf[s.saturating_sub(40)..s]));
                    match fwd(&buf[s..]) {
                        Ok(n) => {
                            let doc = &buf[s..s + n];
                            let has = |k: &[u8]| memchr::memmem::find(doc, k).is_some();
                            println!("  closed after {n} bytes; Recipes={} XPInfo={} parse={:?}", has(b"\"Recipes\""), has(b"\"XPInfo\""),
                                serde_json::from_slice::<serde_json::Value>(doc).map(|_| "ok").map_err(|e| e.to_string()));
                        }
                        Err(e) => {
                            println!("  forward: {e}");
                            if let Some(z) = buf[s..].iter().position(|&c| c == 0) {
                                let at = s + z;
                                println!("  around NUL {:#x}: {}", base + at, show(&buf[at.saturating_sub(60)..(at + 200).min(buf.len())]));
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("  back: {e}");
                    let at = pos - dist;
                    println!("  around stop {:#x}: {}", base + at, show(&buf[at.saturating_sub(80)..(at + 120).min(buf.len())]));
                }
            }
        }
    }
    Ok(())
}
