//! Jira REST adapter. Everything in this module deals with the wire:
//! authentication, HTTP, raw DTOs, pagination and custom-field discovery.
//! Nothing outside `jira::` may depend on Jira's JSON shapes — the
//! `domain::mapper` converts DTOs into domain types at the boundary.

pub mod auth;
pub mod client;
pub mod dto;
pub mod fields;
pub mod search;
