//! Automation action runtime: dispatch, execution, and action transports.
//!
//! The engine stays a set of `AppCore` impl blocks grouped by transport so
//! each file owns one responsibility: [`dispatch`](self) holds the test
//! harnesses and live event fan-out, [`execution`](self) the timeout wrapper
//! plus run recording and core action-type dispatch, [`media_actions`](self)
//! local audio playback, and [`http_actions`](self) HTTP actions with the
//! hardened sender shared by declarative plugin fetches.

mod dispatch;
mod execution;
mod http_actions;
mod media_actions;
#[cfg(test)]
mod tests;

pub(crate) use http_actions::HardenedHttpRequest;
