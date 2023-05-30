use mmap_rs::{Error, MmapFlags, MmapOptions, UnsafeMmapFlags};

fn main() -> Result<(), Error> {
    // Allocate a single page of anonymous memory that is private and mutable.
    let mut mapping = unsafe {
        MmapOptions::new(MmapOptions::page_size())?
            .with_flags(MmapFlags::COPY_ON_WRITE)
            .with_unsafe_flags(UnsafeMmapFlags::DONT_COMMIT)
            .map_mut()?
    };

    mapping.commit(0..4).unwrap();

    mapping[0..4].copy_from_slice(b"test");

    let mapping = match mapping.make_read_only() {
        Ok(mapping) => mapping,
        Err((_, e)) => {
            println!("error: {}", e);
            return Err(e);
        }
    };

    println!("mapping: {:x?}", &mapping[0..4]);

    Ok(())
}
