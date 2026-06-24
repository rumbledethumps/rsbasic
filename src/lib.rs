//! # RS-BASIC
//!
//! Rumbledethumps' Stack BASIC. RS-BASIC is compatible with programs from the
//! beginning of personal computing. It is designed to capture and preserve the
//! best parts of the BASIC experience. Getting a programming manual with your new
//! computer hardware is best — and this is yours.
//!
//! The source code lives on [GitHub](https://github.com/rumbledethumps/rsbasic).
//!
//! ## Install
//!
//! RS-BASIC is written in [Rust](https://www.rust-lang.org/). With a Rust
//! toolchain installed, build and install the latest release with Cargo:
//!
//! ```text
//! cargo install rsbasic
//! ```
//!
//! Then launch it from a terminal — type `CTRL-D` to exit:
//!
//! ```text
//! $ rsbasic
//! RS-BASIC
//! READY.
//! █
//! ```
//!
//! ## Ready to play
//!
//! A collection of classic programs is ready to play. Pass a program name
//! preceded by two slashes and RS-BASIC will fetch it for you:
//!
//! ```text
//! $ rsbasic //superstartrek
//! ```
//!
//! Browse the full list in the
//! [patch folder](https://github.com/rumbledethumps/rsbasic/tree/main/patch).
//!
//! ## Manual
//!
//! New here? Start with the [Introductory Tutorial](crate::_Introduction). The rest
//! of this manual is reference material covering everything RS-BASIC can do:
//!
//! - [Introductory Tutorial](crate::_Introduction)
//! - [Chapter 1 — Expressions and Types](crate::__Chapter_1)
//! - [Chapter 2 — Statements](crate::__Chapter_2)
//! - [Chapter 3 — Functions](crate::__Chapter_3)
//! - [Appendix A — Conversions and Compatibility](crate::___Appendix_A)
//! - [Appendix B — Error Codes and Messages](crate::___Appendix_B)
//! - [Appendix C — Limits and Internals](crate::___Appendix_C)
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

#[path = "doc/appendix_b.rs"]
#[allow(non_snake_case)]
pub mod ___Appendix_B;

#[path = "doc/appendix_c.rs"]
#[allow(non_snake_case)]
pub mod ___Appendix_C;

pub mod lang;
pub mod mach;
