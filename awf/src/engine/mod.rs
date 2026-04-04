//! Engine logic for evaluating workflow flow state.
//!
//! This module contains pure functions that inspect a
//! [`crate::model::workflow::Workflow`] and a [`crate::model::flow::FlowState`]
//! to determine what action to take next, and whether the flow may advance to
//! the next stage.

pub mod actions;
pub mod state;
