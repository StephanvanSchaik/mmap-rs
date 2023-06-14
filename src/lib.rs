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

    #[test]
    fn split_off() {
        use crate::MmapOptions;

        let mut mapping = MmapOptions::new(2 * MmapOptions::page_size())
            .unwrap()
            .map_mut()
            .unwrap();

        assert!(mapping.split_off(1).is_err());

        mapping[0] = 0x1;
        mapping[MmapOptions::page_size()] = 0x2;

        let rest = mapping.split_off(MmapOptions::page_size()).unwrap();

        assert_eq!(mapping[0], 0x1);
        assert_eq!(rest[0], 0x2);
        assert_eq!(mapping.len(), MmapOptions::page_size());
        assert_eq!(rest.len(), MmapOptions::page_size());
        assert!(mapping.as_ptr() < rest.as_ptr());
    }

    #[test]
    fn split_to() {
        use crate::MmapOptions;

        let mut mapping = MmapOptions::new(2 * MmapOptions::page_size())
            .unwrap()
            .map_mut()
            .unwrap();

        assert!(mapping.split_to(1).is_err());

        mapping[0] = 0x1;
        mapping[MmapOptions::page_size()] = 0x2;

        let rest = mapping.split_to(MmapOptions::page_size()).unwrap();

        assert_eq!(mapping[0], 0x2);
        assert_eq!(rest[0], 0x1);
        assert_eq!(mapping.len(), MmapOptions::page_size());
        assert_eq!(rest.len(), MmapOptions::page_size());
        assert!(mapping.as_ptr() > rest.as_ptr());
    }
}
