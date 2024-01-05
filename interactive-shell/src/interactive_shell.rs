use anyhow::Result;

type Editor = rustyline::Editor<(), rustyline::history::MemHistory>;

pub struct InteractiveShell {
    pub shell: shell::Shell,

    editor: Editor,
}

enum InteractiveExecutionResult {
    Executed(shell::ExecutionResult),
    Eof,
}

impl InteractiveShell {
    pub async fn new(options: &shell::CreateOptions) -> Result<InteractiveShell> {
        Ok(InteractiveShell {
            editor: Self::new_editor()?,
            shell: shell::Shell::new(options).await?,
        })
    }

    fn new_editor() -> Result<Editor> {
        let config = rustyline::config::Builder::new()
            .max_history_size(1000)?
            .history_ignore_dups(true)?
            .auto_add_history(true)
            .build();

        // TODO: Create an editor with a helper object so we can do completion.
        let editor = rustyline::Editor::<(), _>::with_history(
            config,
            rustyline::history::MemHistory::with_config(config),
        )?;

        Ok(editor)
    }

    pub async fn run_interactively(&mut self) -> Result<()> {
        loop {
            let result = self.run_interactively_once().await?;
            match result {
                InteractiveExecutionResult::Executed(shell::ExecutionResult {
                    exit_shell,
                    return_from_function_or_script,
                    ..
                }) => {
                    if exit_shell {
                        break;
                    }

                    if return_from_function_or_script {
                        log::error!("return from non-function/script");
                    }
                }
                InteractiveExecutionResult::Eof => {
                    break;
                }
            }
        }

        if self.shell.options.interactive {
            eprintln!("exit");
        }

        Ok(())
    }

    async fn run_interactively_once(&mut self) -> Result<InteractiveExecutionResult> {
        // If there's a variable called PROMPT_COMMAND, then run it first.
        if let Some(prompt_cmd) = self.shell.env.get("PROMPT_COMMAND") {
            let prompt_cmd: String = (&prompt_cmd.value).into();
            self.shell.run_string(prompt_cmd.as_str(), false).await?;
        }

        // Now that we've done that, compose the prompt.
        let prompt = self.shell.compose_prompt().await?;

        match self.editor.readline(&prompt) {
            Ok(read_result) => {
                let result = self.shell.run_string(&read_result, false).await?;
                Ok(InteractiveExecutionResult::Executed(result))
            }
            Err(rustyline::error::ReadlineError::Eof) => Ok(InteractiveExecutionResult::Eof),
            Err(rustyline::error::ReadlineError::Interrupted) => {
                self.shell.last_exit_status = 130;
                Ok(InteractiveExecutionResult::Executed(
                    shell::ExecutionResult::new(130),
                ))
            }
            Err(e) => Err(e.into()),
        }
    }
}
