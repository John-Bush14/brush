use anyhow::Result;
use clap::Parser;

use crate::builtin::{BuiltinCommand, BuiltinExitCode};

#[derive(Parser, Debug)]
pub(crate) struct UnsetCommand {
    #[clap(flatten)]
    name_interpretation: UnsetNameInterpretation,

    names: Vec<String>,
}

#[derive(Parser, Debug)]
#[clap(group = clap::ArgGroup::new("name-interpretation").multiple(false).required(false))]
pub(crate) struct UnsetNameInterpretation {
    #[arg(
        short = 'f',
        group = "name-interpretation",
        help = "treat each name as a shell function"
    )]
    names_are_shell_functions: bool,

    #[arg(
        short = 'v',
        group = "name-interpretation",
        help = "treat each name as a shell variable"
    )]
    names_are_shell_variables: bool,

    #[arg(
        short = 'n',
        group = "name-interpretation",
        help = "treat each name as a name reference"
    )]
    names_are_name_references: bool,
}

impl UnsetNameInterpretation {
    pub fn unspecified(&self) -> bool {
        !self.names_are_shell_functions
            && !self.names_are_shell_variables
            && !self.names_are_name_references
    }
}

impl BuiltinCommand for UnsetCommand {
    fn execute(
        &self,
        context: &mut crate::builtin::BuiltinExecutionContext,
    ) -> Result<crate::builtin::BuiltinExitCode> {
        //
        // TODO: implement nameref
        //
        if self.name_interpretation.names_are_name_references {
            todo!("unset: name references are not yet implemented")
        }

        let unspecified = self.name_interpretation.unspecified();

        let mut errors = false;

        for name in &self.names {
            if unspecified || self.name_interpretation.names_are_shell_variables {
                if let Some(variable) = context.shell.parameters.get(name) {
                    if variable.readonly {
                        log::error!("unset: {}: cannot unset: readonly variable", name);
                        errors = true;
                    }
                }

                if context.shell.parameters.remove(name).is_some() {
                    continue;
                }
            }

            // TODO: Check if functions can be readonly.
            if unspecified || self.name_interpretation.names_are_shell_functions {
                if context.shell.funcs.remove(name).is_some() {
                    continue;
                }
            }
        }

        Ok(if errors {
            BuiltinExitCode::Custom(1)
        } else {
            BuiltinExitCode::Success
        })
    }
}