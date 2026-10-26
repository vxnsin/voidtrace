//! Why does the scan miss the document? Report where the anchor sits in its
//! region and what the start/end search does from there.
//! cargo run --release -p wfc-core --example mem_probe2

fn main() -> anyhow::Result<()> {
    let anchor = b"\"MiscItems\":[";
    wfc_core::memory::walk_memory(|base, buf| {
        for pos in memchr::memmem::find_iter(buf, anchor) {
            println!("region base={base:#x} size={} anchor@+{pos} ({} bytes to region end)", buf.len(), buf.len() - pos);
            // backwards: distance to the first NUL and to the first '{' at depth 0 (naive)
            let nul_back = buf[..pos].iter().rev().position(|b| *b == 0);
            println!("  nearest NUL before anchor: {:?} bytes back", nul_back);
            let nul_fwd = buf[pos..].iter().position(|b| *b == 0);
            println!("  nearest NUL after anchor: {:?} bytes ahead", nul_fwd);
            let lo = pos.saturating_sub(nul_back.unwrap_or(pos));
            println!("  bytes since NUL: {}", pos - lo);
            println!("  head: {}", String::from_utf8_lossy(&buf[lo..(lo + 160).min(buf.len())]).replace('\n', "\\n"));
            let hi = pos + nul_fwd.unwrap_or(buf.len() - pos);
            println!("  tail: {}", String::from_utf8_lossy(&buf[hi.saturating_sub(160)..hi]).replace('\n', "\\n"));
            // Brace balance of the NUL-delimited chunk.
            let chunk = &buf[lo..hi];
            let (mut d, mut s) = (0i64, false);
            let mut prev = 0u8;
            for &c in chunk {
                if s { if c == b'"' && prev != b'\\' { s = false } }
                else { match c { b'"' => s = true, b'{' | b'[' => d += 1, b'}' | b']' => d -= 1, _ => {} } }
                prev = c;
            }
            println!("  chunk len={} final depth={d} in_string={s}", chunk.len());
        }
    })?;
    Ok(())
}
