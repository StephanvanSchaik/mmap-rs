use mmap_rs::{Error, MemoryMaps, ProtectionFlags};

fn main() -> Result<(), Error> {
    let maps = MemoryMaps::open(None)?;

    for area in maps {
        let area = area?;

        println!("{:x}-{:x} {}{}{}{}{}",
            area.range.start,
            area.range.end,
            if area.protection.contains(ProtectionFlags::READ) {
                "r"
            } else {
                "-"
            },
            if area.protection.contains(ProtectionFlags::WRITE) {
                "w"
            } else {
                "-"
            },
            if area.protection.contains(ProtectionFlags::EXECUTE) {
                "x"
            } else {
                "-"
            },
            if area.protection.contains(ProtectionFlags::COPY_ON_WRITE) {
                "s"
            } else {
                "p"
            },
            area.path.map(|(path, offset)| format!(" {:x} {}", offset, path.display())).unwrap_or(String::new()),
        );
    }

    Ok(())
}
