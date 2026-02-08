use crate::{modem::ModemImpl, parser::Command, types::ExecutionResult};

/// A trait for services that can execute AT commands.
/// This is used to implement the "Chain of Responsibility" pattern.
pub trait CommandExecutor {
    /// Attempt to execute a command using the full modem context.
    /// If the command is not relevant to this service, return
    /// `ExecutionResult::Unhandled`.
    fn execute(&self, context: &ModemImpl, command: &Command) -> ExecutionResult;
}
