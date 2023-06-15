pub mod action;
pub mod actions;
pub mod args;
pub mod blackboard;
pub mod forester;
pub mod rnode;
pub mod rtree;

use crate::tree::TreeError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub enum TickResult {
    Success,
    Failure,
    Running,
}
#[derive(Debug)]
pub enum RuntimeErrorCause {
    UnImplementedAction(String),
    BlackBoardError(String),
    IOError(String),
    Unexpected(String),
}

impl RuntimeErrorCause {
    fn io(v: String) -> RuntimeErrorCause {
        RuntimeErrorCause::IOError(v)
    }
    fn un(v: String) -> RuntimeErrorCause {
        RuntimeErrorCause::Unexpected(v)
    }
}