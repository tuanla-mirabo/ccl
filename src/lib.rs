//! ccl is a library with fast and concurrect data structures for rust
//! at the moment ccl requires the use of a nightly toolchain

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod dhashmap;
#[cfg(feature = "std")]
pub mod hltimedcache;
#[cfg(feature = "std")]
pub mod timedcache;

mod parking_lot {
    #[cfg(not(feature = "std"))]
    pub use spin::*;

    #[cfg(feature = "std")]
    pub use ::parking_lot::*;
}

mod std {
    #[cfg(not(feature = "std"))]
    pub use ::core::*;
    #[cfg(not(feature = "std"))]
    pub use ::alloc::*;
    #[cfg(not(feature = "std"))]
    pub mod sync {
        pub use ::alloc::sync::*;
    }

    #[cfg(feature = "std")]
    pub use ::std::*;
}
