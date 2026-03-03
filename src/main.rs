//! Vitalis Compiler CLI — `vtc`
//!
//! Usage:
//!   vtc run <file.sl>          — Compile and JIT-execute
//!   vtc check <file.sl>        — Parse and type-check only
//!   vtc dump-ast <file.sl>     — Dump parsed AST
//!   vtc dump-ir <file.sl>      — Dump lowered IR
//!   vtc lex <file.sl>          — Dump lexer tokens

use vitalis::lexer;
use vitalis::parser;
use vitalis::types;
use vitalis::ir;
use vitalis::codegen;

use clap::{Parser, Subcommand};
use miette::{miette, Result};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "vtc",
    version = "0.1.0",
    about = "Vitalis Compiler — a language built for self-evolving AI",
    long_about = "Vitalis is a systems language purpose-built for \
                  autonomous code evolution, structured memory, and capability-based safety."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile and execute a .sl file via JIT
    Run {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Parse and type-check a .sl file without executing
    Check {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Dump the parsed AST as debug output
    DumpAst {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Dump the lowered IR
    DumpIr {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Dump lexer tokens
    Lex {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Run an inline expression
    Eval {
        /// Vitalis expression (wrapped in fn main)
        #[arg(short, long)]
        expr: String,
    },
    /// Start the interactive REPL
    Repl,
    /// Build a standalone native executable (AOT compilation)
    Build {
        /// Path to the .sl source file
        file: PathBuf,
        /// Output file path
        #[arg(short, long, default_value = "a.out")]
        output: PathBuf,
        /// Target triple (e.g., x86_64-linux-gnu, aarch64-linux-gnu, riscv64-linux-gnu)
        #[arg(short, long)]
        target: Option<String>,
    },
    /// List available cross-compilation targets
    Targets,
    /// Run the compiler bootstrap pipeline
    Bootstrap,
}

fn read_source(path: &PathBuf) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| miette!("cannot read '{}': {}", path.display(), e))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { file } => {
            let source = read_source(&file)?;
            match codegen::compile_and_run(&source) {
                Ok(result) => {
                    println!("=> {}", result);
                    Ok(())
                }
                Err(e) => Err(miette!("{}", e)),
            }
        }

        Command::Check { file } => {
            let source = read_source(&file)?;

            let (program, parse_errors) = parser::parse(&source);
            if !parse_errors.is_empty() {
                for e in &parse_errors {
                    eprintln!("  error: {}", e);
                }
                return Err(miette!("{} parse error(s)", parse_errors.len()));
            }

            let type_errors = types::TypeChecker::new().check(&program);
            if !type_errors.is_empty() {
                for e in &type_errors {
                    eprintln!("  warning: {}", e);
                }
                eprintln!("{} type warning(s)", type_errors.len());
            }

            println!("✓ {} — {} items, no errors", file.display(), program.items.len());
            Ok(())
        }

        Command::DumpAst { file } => {
            let source = read_source(&file)?;
            let (program, errors) = parser::parse(&source);
            if !errors.is_empty() {
                for e in &errors {
                    eprintln!("  error: {}", e);
                }
            }
            println!("{:#?}", program);
            Ok(())
        }

        Command::DumpIr { file } => {
            let source = read_source(&file)?;
            let (program, errors) = parser::parse(&source);
            if !errors.is_empty() {
                for e in &errors {
                    eprintln!("  error: {}", e);
                }
                return Err(miette!("cannot generate IR with parse errors"));
            }
            let ir_module = ir::IrBuilder::new().build(&program);
            for func in &ir_module.functions {
                print!("{}", func);
            }
            Ok(())
        }

        Command::Lex { file } => {
            let source = read_source(&file)?;
            let (tokens, errors) = lexer::lex(&source);
            for tok in &tokens {
                println!("{:>4}..{:<4}  {:?}", tok.span.start, tok.span.end, tok.token);
            }
            if !errors.is_empty() {
                eprintln!("\n{} lex error(s):", errors.len());
                for e in &errors {
                    eprintln!("  {}", e);
                }
            }
            Ok(())
        }

        Command::Eval { expr } => {
            let source = format!("fn main() -> i64 {{ {} }}", expr);
            match codegen::compile_and_run(&source) {
                Ok(result) => {
                    println!("{}", result);
                    Ok(())
                }
                Err(e) => Err(miette!("{}", e)),
            }
        }

        Command::Repl => {
            vitalis::repl::run_interactive();
            Ok(())
        }

        Command::Build { file, output, target } => {
            let source = read_source(&file)?;

            let target_triple = if let Some(t) = target {
                vitalis::aot::TargetTriple::parse(&t)
                    .ok_or_else(|| miette!("unknown target: '{}'. Use `vtc targets` to list available targets.", t))?
            } else {
                vitalis::aot::TargetTriple::host()
            };

            let config = vitalis::aot::AotConfig {
                target: target_triple,
                output,
                verbose: true,
                ..Default::default()
            };

            let mut compiler = vitalis::aot::AotCompiler::new(config);
            match compiler.compile_source(&source) {
                Ok(result) => {
                    println!("{}", result);
                    Ok(())
                }
                Err(e) => Err(miette!("{}", e)),
            }
        }

        Command::Targets => {
            let cc = vitalis::cross_compile::CrossCompiler::new();
            println!("{}", cc);
            for target_name in cc.available_targets() {
                if let Some(info) = cc.target_info(target_name) {
                    println!("\n{}", info);
                }
            }
            Ok(())
        }

        Command::Bootstrap => {
            let config = vitalis::bootstrap::BootstrapConfig {
                verbose: true,
                ..Default::default()
            };
            let mut pipeline = vitalis::bootstrap::BootstrapPipeline::new(config);
            let report = pipeline.run_full_bootstrap();
            println!("{}", report);
            if report.is_success() {
                Ok(())
            } else {
                Err(miette!("bootstrap failed"))
            }
        }
    }
}
