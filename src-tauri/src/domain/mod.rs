//! Pure domain core. No I/O, no Jira wire types, no SQL — every module here
//! is unit-testable with plain values. All progress / spillover / insight
//! math lives here and nowhere else; the frontend never re-derives a number.

pub mod insights;
pub mod mapper;
pub mod model;
pub mod progress;
pub mod spillover;
pub mod sprint_naming;
pub mod timeline;
pub mod view;
