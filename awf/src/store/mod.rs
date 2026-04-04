//! Filesystem store layer for `awf`.
//!
//! This module handles locating and accessing the `.awf/` directory that acts
//! as the root of an `awf` workspace. It provides:
//!
//! - **Resolution**: Walking up the directory tree to find the nearest `.awf/`
//!   root ([`resolve::find_awf_root`]).

pub mod resolve;
