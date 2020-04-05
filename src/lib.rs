//! # RS-BASIC
//!
//! The BASIC programming language as it was in the 8-bit era.
//! ```text
//! RS-BASIC
//! READY.
//! █
//! ```
//!
//! Binaries for Windows and MacOS are available
//! [on GitHub.](https://github.com/rumbledethumps/rsbasic/releases)
//!
//! Linux requires [Rust](https://www.rust-lang.org/tools/install) then
//! the command `cargo install rsbasic`.
//!
//! [The wiki](https://github.com/rumbledethumps/rsbasic/wiki) contains links and
//! information about programs (mostly games) that you can run on RS-BASIC.
//!

#[path = "doc/introduction.rs"]
#[allow(non_snake_case)]
pub mod _Introduction;

#[path = "doc/chapter_1.rs"]
#[allow(non_snake_case)]
pub mod __Chapter_1;

#[path = "doc/chapter_2.rs"]
#[allow(non_snake_case)]
pub mod __Chapter_2;

#[path = "doc/chapter_3.rs"]
#[allow(non_snake_case)]
pub mod __Chapter_3;

#[path = "doc/appendix_a.rs"]
#[allow(non_snake_case)]
pub mod ___Appendix_A;

pub mod lang;
pub mod mach;
