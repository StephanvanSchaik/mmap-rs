#![doc = include_str!("../README.md")]
#![deny(missing_docs, rustdoc::broken_intra_doc_links, missing_debug_implementations )]

mod areas;
pub mod error;
mod mmap;
mod os_impl;

pub use areas::*;
pub use error::Error;
pub use mmap::*;

#[cfg(test)]
mod tests {
    #[test]
    fn map_none() {
        use crate::MmapOptions;

        let mapping = MmapOptions::new(MmapOptions::page_size())
            .unwrap()
            .map_none()
            .unwrap();

        assert!(mapping.as_ptr() != std::ptr::null());
    }

    #[test]
    fn map() {
        use crate::MmapOptions;

        let mapping = MmapOptions::new(MmapOptions::page_size())
            .unwrap()
            .map()
            .unwrap();

        assert!(mapping.as_ptr() != std::ptr::null());
    }

    #[test]
    fn map_mut() {
        use crate::MmapOptions;

        let mut mapping = MmapOptions::new(MmapOptions::page_size())
            .unwrap()
            .map_mut()
            .unwrap();

        mapping[0] = 0x42;

        assert!(mapping.as_ptr() != std::ptr::null());
        assert_eq!(mapping[0], 0x42);
    }
}
