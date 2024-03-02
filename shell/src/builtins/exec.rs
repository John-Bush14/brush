use anyhow::{anyhow, Result};
use clap::Parser;

use crate::builtin::{BuiltinCommand, BuiltinExitCode};

#[derive(Parser, Debug)]
pub(crate) struct ExecCommand {
    /// Pass given name as zeroth argument to command.
    #[arg(short = 'a')]
    name_for_argv0: Option<String>,

    /// Exec command with an empty environment.
    #[arg(short = 'c')]
    empty_environment: bool,

    /// Exec command as a login shell.
    #[arg(short = 'l')]
    exec_as_login: bool,

    /// Command to exec.
    command: Option<String>,

    /// Arguments to pass to command.
    #[clap(allow_hyphen_values = true)]
    args: Vec<String>,
    // TODO: redirection?
}

#[async_trait::async_trait]
impl BuiltinCommand for ExecCommand {
    async fn execute(
        &self,
        _context: &mut crate::builtin::BuiltinExecutionContext<'_>,
    ) -> Result<crate::builtin::BuiltinExitCode, crate::error::Error> {
        if self.name_for_argv0.is_some() {
            log::error!("UNIMPLEMENTED: exec -a: name as argv[0]");
            return Ok(BuiltinExitCode::Unimplemented);
        }

        if self.empty_environment {
            log::error!("UNIMPLEMENTED: exec -c: empty environment");
            return Ok(BuiltinExitCode::Unimplemented);
        }

        if self.exec_as_login {
            log::error!("UNIMPLEMENTED: exec -l: exec as login");
            return Ok(BuiltinExitCode::Unimplemented);
        }

        if let Some(command) = &self.command {
            let err = exec::Command::new(command).args(&self.args).exec();
            match err {
                exec::Error::BadArgument(_) => {
                    Err(crate::error::Error::Unknown(anyhow!("invalid arguments")))
                }
                exec::Error::Errno(errno) => Err(crate::error::Error::Unknown(errno.into())),
            }
        } else {
            return Ok(BuiltinExitCode::Success);
        }
    }
}