use process::{Command, ExitStatus};
use std::{io, path::Path, process, time::Duration};
use thiserror::Error;
use wait_timeout::ChildExt;

pub fn binary_kind(binary_path: &Path) -> BinaryKind {
    let exe_parent = binary_path.parent();
    let parent_dir_name = exe_parent
        .and_then(|p| p.file_name())
        .and_then(|name| name.to_str());
    match parent_dir_name {
        Some("deps") => BinaryKind::Test,
        Some(name) if name.starts_with("rustdoctest") => BinaryKind::DocTest,
        _other => BinaryKind::Other,
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum BinaryKind {
    Test,
    DocTest,
    Other,
}

impl BinaryKind {
    pub fn is_test(&self) -> bool {
        match self {
            BinaryKind::Test | BinaryKind::DocTest => true,
            BinaryKind::Other => false,
        }
    }
}

pub fn run_with_timeout(command: &mut Command, timeout: Duration) -> Result<ExitStatus, RunError> {
    let mut child = command.spawn().map_err(|error| RunError::Io {
        context: IoErrorContext::Command {
            command: format!("{:?}", command),
        },
        error,
    })?;
    match child
        .wait_timeout(timeout)
        .map_err(context(IoErrorContext::WaitWithTimeout))?
    {
        None => {
            child.kill().map_err(context(IoErrorContext::KillProcess))?;
            child
                .wait()
                .map_err(context(IoErrorContext::WaitForProcess))?;
            Err(RunError::TimedOut)
        }
        Some(exit_status) => Ok(exit_status),
    }
}

/// Running the disk image failed.
#[derive(Debug, Error)]
pub enum RunError {
    /// Command timed out
    #[error("Command timed out")]
    TimedOut,

    /// An I/O error occured
    #[error("I/O error: {context}")]
    Io {
        /// The operation that caused the I/O error.
        context: IoErrorContext,
        /// The I/O error that occured.
        #[source]
        error: io::Error,
    },
}

/// An I/O error occured while trying to run the disk image.
#[derive(Debug, Error)]
pub enum IoErrorContext {
    /// Failed to execute command
    #[error("Failed to execute command `{command}`")]
    Command {
        /// The Command that was executed
        command: String,
    },

    /// Waiting with timeout failed
    #[error("Failed to wait with timeout")]
    WaitWithTimeout,

    /// Failed to kill process after timeout
    #[error("Failed to kill process after timeout")]
    KillProcess,

    /// Failed to wait for process after killing it after timeout
    #[error("Failed to wait for process after killing it after timeout")]
    WaitForProcess,
}

/// Helper function for IO error construction
fn context(context: IoErrorContext) -> impl FnOnce(io::Error) -> RunError {
    |error| RunError::Io { context, error }
}
