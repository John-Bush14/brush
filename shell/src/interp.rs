use anyhow::Result;

use log::error;
use parser::ast::{self, CommandPrefixOrSuffixItem};

use crate::context::ExecutionContext;

pub struct ExecutionResult {
    pub exit_code: i32,
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

pub trait Execute {
    fn execute(&self, _context: &mut ExecutionContext) -> Result<ExecutionResult>;
}

impl Execute for ast::Program {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        let mut result = ExecutionResult { exit_code: 0 };

        for command in &self.complete_commands {
            result = command.execute(context)?;
        }

        Ok(result)
    }
}

impl Execute for ast::CompleteCommand {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        let mut result = ExecutionResult { exit_code: 0 };

        for (ao_list, sep) in self {
            let run_async = matches!(sep, ast::SeparatorOperator::Async);

            if run_async {
                todo!("asynchronous execution")
            }

            result = ao_list.first.execute(context)?;

            for next_ao in &ao_list.additional {
                let (is_and, pipeline) = match next_ao {
                    ast::AndOr::And(p) => (true, p),
                    ast::AndOr::Or(p) => (false, p),
                };

                if is_and {
                    if !result.is_success() {
                        break;
                    }
                } else {
                    if result.is_success() {
                        break;
                    }
                }

                result = pipeline.execute(context)?;
            }
        }

        Ok(result)
    }
}

impl Execute for ast::Pipeline {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        //
        // TODO: handle bang
        // TODO: implement logic deciding when to abort
        // TODO: confirm whether exit code comes from first or last in pipeline
        //

        let mut result = ExecutionResult { exit_code: 0 };

        for command in self.seq.iter() {
            result = command.execute(context)?;
        }

        Ok(result)
    }
}

impl Execute for ast::Command {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        match self {
            ast::Command::Simple(simple) => simple.execute(context),
            ast::Command::Compound(compound, _redirects) => {
                //
                // TODO: handle redirects
                //

                compound.execute(context)
            }
            ast::Command::Function(func) => func.execute(context),
        }
    }
}

enum WhileOrUtil {
    While,
    Util,
}

impl Execute for ast::CompoundCommand {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        match self {
            ast::CompoundCommand::BraceGroup(g) => g.execute(context),
            ast::CompoundCommand::Subshell(_) => todo!("subshell command"),
            ast::CompoundCommand::ForClause(f) => f.execute(context),
            ast::CompoundCommand::CaseClause(c) => c.execute(context),
            ast::CompoundCommand::IfClause(i) => i.execute(context),
            ast::CompoundCommand::WhileClause(w) => (WhileOrUtil::While, w).execute(context),
            ast::CompoundCommand::UntilClause(u) => (WhileOrUtil::Util, u).execute(context),
        }
    }
}

impl Execute for ast::ForClauseCommand {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        let mut result = ExecutionResult { exit_code: 0 };

        if let Some(vs) = &self.values {
            for value in vs {
                // Update the variable.
                context
                    .parameters
                    .insert(self.variable_name.clone(), value.clone());

                result = self.body.execute(context)?;
            }
        }

        Ok(result)
    }
}

impl Execute for ast::CaseClauseCommand {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        todo!("execute case clause command")
    }
}

impl Execute for ast::IfClauseCommand {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        //
        // The if command shall execute a compound-list and use its exit status to
        // determine whether to execute another compound-list.
        //
        // The if compound-list shall be executed; if its exit status is zero, the
        // then compound-list shall be executed and the command shall complete.
        // Otherwise, each elif compound-list shall be executed, in turn, and if
        // its exit status is zero, the then compound-list shall be executed and the
        // command shall complete. Otherwise, the else compound-list shall be executed.
        //
        // The exit status of the if command shall be the exit status of the then or
        // else compound-list that was executed, or zero, if none was executed.
        //

        let condition = self.condition.execute(context)?;

        if condition.is_success() {
            return self.then.execute(context);
        }

        if let Some(elses) = &self.elses {
            for else_clause in elses {
                match &else_clause.condition {
                    Some(else_condition) => {
                        let else_condition_result = else_condition.execute(context)?;
                        if else_condition_result.is_success() {
                            return else_clause.body.execute(context);
                        }
                    }
                    None => {
                        return else_clause.body.execute(context);
                    }
                }
            }
        }

        return Ok(ExecutionResult { exit_code: 0 });
    }
}

impl Execute for (WhileOrUtil, &ast::WhileClauseCommand) {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        todo!("execute while clause command")
    }
}

impl Execute for ast::FunctionDefinition {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        //
        // TODO: confirm whether defining a function resets the last execution.
        //

        context.funcs.insert(self.fname.clone(), self.clone());
        Ok(ExecutionResult { exit_code: 0 })
    }
}

impl Execute for ast::SimpleCommand {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ExecutionResult> {
        //
        // TODO: do something with redirects
        //

        let mut redirects = vec![];
        let mut env_vars = vec![];

        if let Some(prefix_items) = &self.prefix {
            for item in prefix_items {
                match item {
                    CommandPrefixOrSuffixItem::IoRedirect(r) => redirects.push(r),
                    CommandPrefixOrSuffixItem::AssignmentWord(pair) => env_vars.push(pair),
                    CommandPrefixOrSuffixItem::Word(_w) => {
                        // This should not happen.
                    }
                }
            }
        }

        let mut args = vec![];
        if let Some(suffix_items) = &self.suffix {
            for item in suffix_items {
                match item {
                    CommandPrefixOrSuffixItem::IoRedirect(r) => redirects.push(r),
                    CommandPrefixOrSuffixItem::Word(arg) => args.push(arg),
                    CommandPrefixOrSuffixItem::AssignmentWord(_) => {
                        // This should not happen.
                    }
                }
            }
        }

        if redirects.len() > 0 {
            //
            // TODO: handle redirects
            //
            error!("simple command redirects not implemented: {:?}", redirects);
        }

        if let Some(cmd_name) = &self.word_or_name {
            if !cmd_name.contains('/') {
                if let Some(utility) = try_parse_name_as_special_builtin_utility(cmd_name) {
                    execute_special_builtin_utility(utility, &args, env_vars)
                } else if let Some(function_definition) = context.funcs.get(cmd_name) {
                    invoke_shell_function(function_definition, &args, env_vars)
                } else if let Some(utility) = try_parse_name_as_well_known_utility(cmd_name) {
                    execute_well_known_utility(utility, &args, env_vars)
                } else {
                    execute_external_command(cmd_name, &args, env_vars)
                }
            } else {
                execute_external_command(cmd_name, &args, env_vars)
            }
        } else {
            //
            // This must just be an assignment.
            //

            for (name, value) in env_vars {
                // TODO: Handle readonly variables.
                context.parameters.insert(name.clone(), value.clone());
            }

            Ok(ExecutionResult { exit_code: 0 })
        }
    }
}

fn execute_external_command(
    cmd_name: &str,
    args: &Vec<&String>,
    env_vars: Vec<&(String, String)>,
) -> Result<ExecutionResult> {
    let mut cmd = std::process::Command::new(cmd_name);
    for arg in args {
        cmd.arg(arg);
    }

    for (name, value) in env_vars {
        cmd.env(name, value);
    }

    match cmd.spawn() {
        Ok(mut child) => {
            let status = child.wait()?;
            let exit_code = status.code().unwrap();
            Ok(ExecutionResult { exit_code })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::error!("command not found: {}", cmd_name);
            Ok(ExecutionResult { exit_code: 127 })
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug)]
enum SpecialBuiltinUtility {
    Break,
    Colon,
    Continue,
    Dot,
    Eval,
    Exec,
    Exit,
    Export,
    Readonly,
    Return,
    Set,
    Shift,
    Times,
    Trap,
    Unset,
}

fn try_parse_name_as_special_builtin_utility(cmd_name: &str) -> Option<SpecialBuiltinUtility> {
    match cmd_name {
        "break" => Some(SpecialBuiltinUtility::Break),
        ":" => Some(SpecialBuiltinUtility::Colon),
        "continue" => Some(SpecialBuiltinUtility::Continue),
        "." => Some(SpecialBuiltinUtility::Dot),
        "eval" => Some(SpecialBuiltinUtility::Eval),
        "exec" => Some(SpecialBuiltinUtility::Exec),
        "exit" => Some(SpecialBuiltinUtility::Exit),
        "export" => Some(SpecialBuiltinUtility::Export),
        "readonly" => Some(SpecialBuiltinUtility::Readonly),
        "return" => Some(SpecialBuiltinUtility::Return),
        "set" => Some(SpecialBuiltinUtility::Set),
        "shift" => Some(SpecialBuiltinUtility::Shift),
        "times" => Some(SpecialBuiltinUtility::Times),
        "trap" => Some(SpecialBuiltinUtility::Trap),
        "unset" => Some(SpecialBuiltinUtility::Unset),
        _ => None,
    }
}

fn execute_special_builtin_utility(
    utility: SpecialBuiltinUtility,
    _args: &Vec<&String>,
    _env_vars: Vec<&(String, String)>,
) -> Result<ExecutionResult> {
    log::error!("unimplemented: special built-in utility {:?}", utility);
    Ok(ExecutionResult { exit_code: 99 })
}

#[derive(Debug)]
enum WellKnownUtility {
    Alias,
    Bg,
    Cd,
    Command,
    False,
    Fc,
    Fg,
    Getopts,
    Hash,
    Jobs,
    Kill,
    Newgrp,
    Pwd,
    Read,
    True,
    Type,
    Ulimit,
    Umask,
    Unalias,
    Wait,
}

fn try_parse_name_as_well_known_utility(cmd_name: &str) -> Option<WellKnownUtility> {
    match cmd_name {
        "alias" => Some(WellKnownUtility::Alias),
        "bg" => Some(WellKnownUtility::Bg),
        "cd" => Some(WellKnownUtility::Cd),
        "command" => Some(WellKnownUtility::Command),
        "false" => Some(WellKnownUtility::False),
        "fc" => Some(WellKnownUtility::Fc),
        "fg" => Some(WellKnownUtility::Fg),
        "getopts" => Some(WellKnownUtility::Getopts),
        "hash" => Some(WellKnownUtility::Hash),
        "jobs" => Some(WellKnownUtility::Jobs),
        "kill" => Some(WellKnownUtility::Kill),
        "newgrp" => Some(WellKnownUtility::Newgrp),
        "pwd" => Some(WellKnownUtility::Pwd),
        "read" => Some(WellKnownUtility::Read),
        "true" => Some(WellKnownUtility::True),
        "type" => Some(WellKnownUtility::Type),
        "ulimit" => Some(WellKnownUtility::Ulimit),
        "umask" => Some(WellKnownUtility::Umask),
        "unalias" => Some(WellKnownUtility::Unalias),
        "wait" => Some(WellKnownUtility::Wait),
        _ => None,
    }
}

fn execute_well_known_utility(
    utility: WellKnownUtility,
    _args: &Vec<&String>,
    _env_vars: Vec<&(String, String)>,
) -> Result<ExecutionResult> {
    log::error!("unimplemented: well-known utility {:?}", utility);
    Ok(ExecutionResult { exit_code: 99 })
}

fn invoke_shell_function(
    function_definition: &ast::FunctionDefinition,
    _args: &Vec<&String>,
    _env_vars: Vec<&(String, String)>,
) -> Result<ExecutionResult> {
    log::error!("unimplemented: invoke shell function");
    Ok(ExecutionResult { exit_code: 99 })
}