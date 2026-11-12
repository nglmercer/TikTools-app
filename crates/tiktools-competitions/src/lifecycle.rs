//! Durable lifecycle transitions and host-clock deadlines.
use crate::Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Draft,
    Active,
    Paused,
    Finalizing,
    Finished,
    Cancelled,
    Failed,
}

/// Durable lifecycle; absolute deadlines are supplied by the host clock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleState {
    pub lifecycle: Lifecycle,
    pub generation: u64,
    pub deadline_ms: Option<u64>,
    pub remaining_ms: Option<u64>,
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self {
            lifecycle: Lifecycle::Draft,
            generation: 0,
            deadline_ms: None,
            remaining_ms: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    Start,
    Pause,
    Resume,
    Cancel,
    Finish,
    Expire,
}

impl LifecycleState {
    pub fn transition(
        &mut self,
        transition: Transition,
        now: u64,
        duration: Option<u64>,
    ) -> Result<(), Error> {
        match transition {
            Transition::Start if self.lifecycle == Lifecycle::Draft => {
                let deadline = duration
                    .map(|d| now.checked_add(d).ok_or(Error::Overflow))
                    .transpose()?;
                let generation = self.generation.checked_add(1).ok_or(Error::Overflow)?;
                self.deadline_ms = deadline;
                self.generation = generation;
                self.lifecycle = Lifecycle::Active;
            }
            Transition::Pause if self.lifecycle == Lifecycle::Active && !self.is_due(now) => {
                self.remaining_ms = self.deadline_ms.map(|deadline| deadline - now);
                self.deadline_ms = None;
                self.lifecycle = Lifecycle::Paused;
            }
            Transition::Resume if self.lifecycle == Lifecycle::Paused => {
                self.deadline_ms = self
                    .remaining_ms
                    .map(|d| now.checked_add(d).ok_or(Error::Overflow))
                    .transpose()?;
                self.remaining_ms = None;
                self.lifecycle = Lifecycle::Active;
            }
            Transition::Cancel
                if matches!(
                    self.lifecycle,
                    Lifecycle::Draft | Lifecycle::Active | Lifecycle::Paused
                ) =>
            {
                self.lifecycle = Lifecycle::Cancelled;
                self.deadline_ms = None;
                self.remaining_ms = None;
            }
            Transition::Finish
                if matches!(self.lifecycle, Lifecycle::Active | Lifecycle::Paused) =>
            {
                self.lifecycle = Lifecycle::Finalizing;
                self.deadline_ms = None;
                self.remaining_ms = None;
            }
            Transition::Expire if self.is_due(now) => {
                self.lifecycle = Lifecycle::Finalizing;
                self.deadline_ms = None;
            }
            _ => return Err(Error::InvalidTransition),
        }
        Ok(())
    }
    pub fn is_due(&self, now: u64) -> bool {
        self.lifecycle == Lifecycle::Active && self.deadline_ms.is_some_and(|d| now >= d)
    }
}
