//! REPL — Interactive Read-Eval-Print Loop for Vitalis
//!
//! Supports:
//! - Multi-line input with { } matching
//! - History navigation
//! - `:help`, `:ast`, `:ir`, `:type`, `:clear`, `:exit` commands
//! - Expression evaluation wrapping in fn main()
//! - Type/parse error reporting inline

use crate::codegen;
use crate::ir;
use crate::parser;
use crate::types;

/// REPL session state
pub struct ReplSession {
    pub history: Vec<String>,
    pub line_number: usize,
    pub last_result: Option<i64>,
    pub verbose: bool,
}

impl ReplSession {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            line_number: 0,
            last_result: None,
            verbose: false,
        }
    }

    /// Evaluate a single REPL input line or block.
    /// Returns Ok(Some(value)) for expressions, Ok(None) for commands, Err for errors.
    pub fn eval(&mut self, input: &str) -> Result<Option<i64>, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        // Handle REPL commands
        if trimmed.starts_with(':') {
            return self.handle_command(trimmed);
        }

        self.history.push(input.to_string());
        self.line_number += 1;

        // Try as expression first (wrap in fn main() -> i64 { ... })
        let source = if trimmed.starts_with("fn ") || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ") || trimmed.starts_with("impl ")
            || trimmed.starts_with("trait ")
        {
            // Top-level definition — append a main that returns 0
            format!("{}\nfn main() -> i64 {{ 0 }}", trimmed)
        } else if trimmed.contains("let ") || trimmed.ends_with(';') {
            // Statement — wrap in main
            format!("fn main() -> i64 {{ {}; 0 }}", trimmed.trim_end_matches(';'))
        } else {
            // Expression — wrap as return value
            format!("fn main() -> i64 {{ {} }}", trimmed)
        };

        match codegen::compile_and_run(&source) {
            Ok(val) => {
                self.last_result = Some(val);
                Ok(Some(val))
            }
            Err(e) => Err(e),
        }
    }

    /// Handle REPL meta-commands
    fn handle_command(&mut self, cmd: &str) -> Result<Option<i64>, String> {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        match parts[0] {
            ":help" | ":h" => {
                Ok(None) // Help text is printed by the caller
            }
            ":ast" => {
                let code = parts.get(1).unwrap_or(&"fn main() -> i64 { 0 }");
                let (program, errors) = parser::parse(code);
                if !errors.is_empty() {
                    return Err(errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"));
                }
                // Return ast debug string via Err channel (hack — caller formats)
                Err(format!("{:#?}", program))
            }
            ":ir" => {
                let code = parts.get(1).unwrap_or(&"fn main() -> i64 { 0 }");
                let (program, errors) = parser::parse(code);
                if !errors.is_empty() {
                    return Err(errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"));
                }
                let ir_module = ir::IrBuilder::new().build(&program);
                let mut out = String::new();
                for func in &ir_module.functions {
                    out.push_str(&format!("{}", func));
                }
                Err(out)
            }
            ":type" | ":t" => {
                let code = parts.get(1).unwrap_or(&"fn main() -> i64 { 0 }");
                let source = format!("fn main() -> i64 {{ {} }}", code);
                let (program, errors) = parser::parse(&source);
                if !errors.is_empty() {
                    return Err(errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"));
                }
                let type_errors = types::TypeChecker::new().check(&program);
                if type_errors.is_empty() {
                    Err("Type check passed — no errors".to_string())
                } else {
                    Err(type_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"))
                }
            }
            ":clear" => {
                self.history.clear();
                self.line_number = 0;
                self.last_result = None;
                Ok(None)
            }
            ":history" => {
                let hist = self.history.iter().enumerate()
                    .map(|(i, h)| format!("[{}] {}", i + 1, h))
                    .collect::<Vec<_>>()
                    .join("\n");
                Err(hist)
            }
            ":verbose" => {
                self.verbose = !self.verbose;
                Err(format!("Verbose mode: {}", if self.verbose { "on" } else { "off" }))
            }
            ":exit" | ":quit" | ":q" => {
                Err("__EXIT__".to_string())
            }
            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }

    /// Check if input is a complete block (balanced braces)
    pub fn is_complete(input: &str) -> bool {
        let mut depth: i32 = 0;
        for ch in input.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
        depth <= 0
    }

    /// Get help text
    pub fn help_text() -> &'static str {
        "\
Vitalis REPL Commands:
  :help, :h       Show this help message
  :ast <code>     Dump AST for code
  :ir <code>      Dump IR for code
  :type <expr>    Type-check an expression
  :clear          Clear history and state
  :history        Show input history
  :verbose        Toggle verbose output
  :exit, :q       Exit the REPL

Enter any expression to evaluate (automatically wrapped in fn main).
Definitions (fn, struct, enum) are compiled as top-level items."
    }
}

/// Run the interactive REPL loop (reads from stdin)
pub fn run_interactive() {
    use std::io::{self, Write, BufRead};

    println!("Vitalis REPL v22.0.0 — type :help for commands, :exit to quit");
    println!();

    let mut session = ReplSession::new();
    let stdin = io::stdin();
    let mut buffer = String::new();

    loop {
        // Prompt
        if buffer.is_empty() {
            print!("vtc> ");
        } else {
            print!("...> ");
        }
        io::stdout().flush().unwrap_or(());

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Err(_) => break,
            Ok(_) => {}
        }

        buffer.push_str(&line);

        // Check if input is complete
        if !ReplSession::is_complete(&buffer) {
            continue;
        }

        let input = buffer.trim().to_string();
        buffer.clear();

        if input.is_empty() {
            continue;
        }

        match session.eval(&input) {
            Ok(Some(val)) => println!("=> {}", val),
            Ok(None) => {
                if input == ":help" || input == ":h" {
                    println!("{}", ReplSession::help_text());
                }
            }
            Err(msg) => {
                if msg == "__EXIT__" {
                    println!("Goodbye.");
                    break;
                }
                // Could be output (AST/IR) or an actual error
                println!("{}", msg);
            }
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_eval_expression() {
        let mut session = ReplSession::new();
        let result = session.eval("42");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_eval_arithmetic() {
        let mut session = ReplSession::new();
        let result = session.eval("10 + 32");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_eval_complex() {
        let mut session = ReplSession::new();
        let result = session.eval("if true { 99 } else { 0 }");
        assert_eq!(result.unwrap(), Some(99));
    }

    #[test]
    fn test_repl_help_command() {
        let mut session = ReplSession::new();
        let result = session.eval(":help");
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_repl_clear_command() {
        let mut session = ReplSession::new();
        session.eval("42").unwrap();
        assert_eq!(session.history.len(), 1);
        session.eval(":clear").unwrap();
        assert_eq!(session.history.len(), 0);
    }

    #[test]
    fn test_repl_exit_command() {
        let mut session = ReplSession::new();
        let result = session.eval(":exit");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "__EXIT__");
    }

    #[test]
    fn test_repl_is_complete() {
        assert!(ReplSession::is_complete("42"));
        assert!(ReplSession::is_complete("fn foo() { 42 }"));
        assert!(!ReplSession::is_complete("fn foo() {"));
        assert!(ReplSession::is_complete("fn foo() { { } }"));
    }

    #[test]
    fn test_repl_history() {
        let mut session = ReplSession::new();
        session.eval("1").unwrap();
        session.eval("2").unwrap();
        session.eval("3").unwrap();
        assert_eq!(session.history.len(), 3);
    }

    #[test]
    fn test_repl_verbose_toggle() {
        let mut session = ReplSession::new();
        assert!(!session.verbose);
        let _ = session.eval(":verbose");
        assert!(session.verbose);
        let _ = session.eval(":verbose");
        assert!(!session.verbose);
    }

    #[test]
    fn test_repl_empty_input() {
        let mut session = ReplSession::new();
        let result = session.eval("");
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_repl_unknown_command() {
        let mut session = ReplSession::new();
        let result = session.eval(":nonsense");
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_multiplication() {
        let mut session = ReplSession::new();
        let result = session.eval("6 * 7");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_nested_expression() {
        let mut session = ReplSession::new();
        let result = session.eval("(10 + 5) * 2 + 12");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_let_binding() {
        let mut session = ReplSession::new();
        // Let binding returns 0 since it's a statement
        let result = session.eval("let x = 10;");
        assert_eq!(result.unwrap(), Some(0));
    }

    #[test]
    fn test_repl_negative() {
        let mut session = ReplSession::new();
        let result = session.eval("-5 + 47");
        assert_eq!(result.unwrap(), Some(42));
    }

    #[test]
    fn test_repl_last_result() {
        let mut session = ReplSession::new();
        session.eval("42").unwrap();
        assert_eq!(session.last_result, Some(42));
        session.eval("99").unwrap();
        assert_eq!(session.last_result, Some(99));
    }

    #[test]
    fn test_repl_function_def() {
        let mut session = ReplSession::new();
        // Defining a function returns 0 (the appended main)
        let result = session.eval("fn add(a: i64, b: i64) -> i64 { a + b }");
        assert_eq!(result.unwrap(), Some(0));
    }

    #[test]
    fn test_repl_while_statement() {
        let mut session = ReplSession::new();
        // Test simple loop-count via the codegen path
        let source = "fn main() -> i64 { let mut i = 0; while i < 10 { i = i + 1 } i }";
        let result = crate::codegen::compile_and_run(source);
        assert_eq!(result.unwrap(), 10);
        // Also verify REPL session tracks history
        session.eval("5 + 5").unwrap();
        assert_eq!(session.history.len(), 1);
    }
}
