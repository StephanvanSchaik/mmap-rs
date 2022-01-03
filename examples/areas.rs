use mmap_rs::{Error, MemoryMaps, Protection, ShareMode};

fn main() -> Result<(), Error> {
    let maps = MemoryMaps::open(None)?;

    for area in maps {
        let area = area?;

        println!("{:x}-{:x} {}{}{}{}{}",
            area.range.start,
            area.range.end,
            if area.protection.contains(Protection::READ) {
                "r"
            } else {
                "-"
            },
            if area.protection.contains(Protection::WRITE) {
                "w"
            } else {
                "-"
            },
            if area.protection.contains(Protection::EXECUTE) {
                "x"
            } else {
                "-"
            },
            if area.share_mode == ShareMode::Shared {
                "s"
            } else {
                "p"
            },
            area.path.map(|(path, offset)| format!(" {:x} {}", offset, path.display())).unwrap_or(String::new()),
        );
    }

    Ok(())
}
