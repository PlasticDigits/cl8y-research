//! CL8Y ecosystem research worker.
//!
//! Dedicated Rust + Postgres repo so [`CL8Y-web`](https://gitlab.com/PlasticDigits/CL8Y-web)
//! stays a static Vite SPA. Implements [`cl8y-research#1`](https://gitlab.com/PlasticDigits/cl8y-research/-/issues/1)
//! (moved from CL8Y-web#4) plus vector search for agents.
//!
//! Canonical voice remains `CL8Y-web/blog_gen/SKILL.md`.

pub mod allowlist;
pub mod collect;
pub mod config;
pub mod embed;
pub mod error;
pub mod invariants;
pub mod lint;
pub mod mdx;
pub mod numeric;
pub mod pipeline;
pub mod publish;
pub mod replicate;
pub mod search;
pub mod secrets;
pub mod store;
pub mod telegram;

pub use config::Config;
pub use error::{Error, Result};
pub use pipeline::{run_week, WeekOpts, WeekRun};
