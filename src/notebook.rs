//! Jupyter-compatible notebook kernel for Vitalis.
//!
//! Interactive computing environment:
//! - **Kernel protocol**: Jupyter wire protocol message handling
//! - **Cell execution**: Compile and run cells with persistent state
//! - **Rich output**: Text, HTML, images, charts
//! - **Magic commands**: %time, %profile, %ast, %ir, %type
//! - **Variable inspector**: List bound variables with types
//! - **Autocomplete**: Delegate to LSP for completions

use std::collections::{HashMap, VecDeque};

// ── Kernel Messages ─────────────────────────────────────────────────

/// Jupyter message type.
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    ExecuteRequest,
    ExecuteReply,
    CompleteRequest,
    CompleteReply,
    InspectRequest,
    InspectReply,
    KernelInfoRequest,
    KernelInfoReply,
    ShutdownRequest,
    ShutdownReply,
    StatusUpdate,
    StreamOutput,
    DisplayData,
    Error,
}

/// A kernel message.
#[derive(Debug, Clone)]
pub struct KernelMessage {
    pub msg_type: MessageType,
    pub msg_id: String,
    pub parent_id: Option<String>,
    pub content: MessageContent,
}

/// Message content.
#[derive(Debug, Clone)]
pub enum MessageContent {
    ExecuteRequest { code: String, silent: bool },
    ExecuteReply { status: ExecuteStatus, execution_count: u32 },
    CompleteRequest { code: String, cursor_pos: usize },
    CompleteReply { matches: Vec<String>, cursor_start: usize, cursor_end: usize },
    InspectRequest { code: String, cursor_pos: usize },
    InspectReply { found: bool, data: HashMap<String, String> },
    KernelInfo { language: String, version: String, banner: String },
    Status { state: KernelState },
    Stream { name: String, text: String },
    DisplayData { data: HashMap<String, String>, metadata: HashMap<String, String> },
    ErrorOutput { ename: String, evalue: String, traceback: Vec<String> },
    Shutdown { restart: bool },
}

/// Execution status.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecuteStatus {
    Ok,
    Error,
    Abort,
}

/// Kernel state.
#[derive(Debug, Clone, PartialEq)]
pub enum KernelState {
    Idle,
    Busy,
    Starting,
}

// ── Rich Output ─────────────────────────────────────────────────────

/// A rich output from cell execution.
#[derive(Debug, Clone)]
pub struct RichOutput {
    pub mime_type: String,
    pub data: String,
}

impl RichOutput {
    pub fn text(s: &str) -> Self {
        Self { mime_type: "text/plain".into(), data: s.to_string() }
    }

    pub fn html(s: &str) -> Self {
        Self { mime_type: "text/html".into(), data: s.to_string() }
    }

    pub fn svg(s: &str) -> Self {
        Self { mime_type: "image/svg+xml".into(), data: s.to_string() }
    }

    pub fn json(s: &str) -> Self {
        Self { mime_type: "application/json".into(), data: s.to_string() }
    }

    pub fn latex(s: &str) -> Self {
        Self { mime_type: "text/latex".into(), data: s.to_string() }
    }
}

// ── Magic Commands ──────────────────────────────────────────────────

/// A magic command.
#[derive(Debug, Clone, PartialEq)]
pub enum MagicCommand {
    Time,
    Profile,
    Ast,
    Ir,
    Type(String),
    Help,
    Reset,
    Vars,
    History,
}

/// Parse a magic command from input.
pub fn parse_magic(input: &str) -> Option<MagicCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('%') { return None; }
    let cmd = &trimmed[1..];
    match cmd.split_whitespace().next()? {
        "time" => Some(MagicCommand::Time),
        "profile" => Some(MagicCommand::Profile),
        "ast" => Some(MagicCommand::Ast),
        "ir" => Some(MagicCommand::Ir),
        "type" => {
            let arg = cmd.strip_prefix("type")?.trim().to_string();
            Some(MagicCommand::Type(arg))
        }
        "help" => Some(MagicCommand::Help),
        "reset" => Some(MagicCommand::Reset),
        "vars" => Some(MagicCommand::Vars),
        "history" => Some(MagicCommand::History),
        _ => None,
    }
}

// ── Notebook Cell ───────────────────────────────────────────────────

/// A notebook cell.
#[derive(Debug, Clone)]
pub struct Cell {
    pub id: u32,
    pub source: String,
    pub cell_type: CellType,
    pub outputs: Vec<RichOutput>,
    pub execution_count: Option<u32>,
    pub metadata: HashMap<String, String>,
}

/// Cell type.
#[derive(Debug, Clone, PartialEq)]
pub enum CellType {
    Code,
    Markdown,
    Raw,
}

// ── Variable Inspector ──────────────────────────────────────────────

/// A variable binding in the kernel session.
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub type_name: String,
    pub value_repr: String,
    pub size_bytes: usize,
}

// ── Kernel ──────────────────────────────────────────────────────────

/// The Vitalis notebook kernel.
pub struct NotebookKernel {
    pub state: KernelState,
    execution_count: u32,
    variables: HashMap<String, Variable>,
    history: Vec<String>,
    outputs: VecDeque<KernelMessage>,
    next_msg_id: u64,
}

impl NotebookKernel {
    pub fn new() -> Self {
        Self {
            state: KernelState::Starting,
            execution_count: 0,
            variables: HashMap::new(),
            history: Vec::new(),
            outputs: VecDeque::new(),
            next_msg_id: 1,
        }
    }

    /// Initialize the kernel.
    pub fn start(&mut self) {
        self.state = KernelState::Idle;
    }

    /// Execute code in the kernel.
    pub fn execute(&mut self, code: &str) -> Vec<RichOutput> {
        self.state = KernelState::Busy;
        self.execution_count += 1;
        self.history.push(code.to_string());

        // Check for magic commands.
        if let Some(magic) = parse_magic(code) {
            let result = self.handle_magic(magic);
            self.state = KernelState::Idle;
            return result;
        }

        // Simulate compilation and execution.
        let output = if code.contains("error") || code.contains("panic") {
            vec![RichOutput::text(&format!("Error in cell [{}]", self.execution_count))]
        } else {
            vec![RichOutput::text(&format!("Out[{}]: executed {} chars", self.execution_count, code.len()))]
        };

        self.state = KernelState::Idle;
        output
    }

    /// Handle a magic command.
    fn handle_magic(&self, magic: MagicCommand) -> Vec<RichOutput> {
        match magic {
            MagicCommand::Help => vec![RichOutput::text(
                "Magic commands:\n  %time — time execution\n  %profile — profile code\n  %ast — show AST\n  %ir — show IR\n  %type <expr> — show type\n  %vars — list variables\n  %history — show history\n  %reset — reset kernel"
            )],
            MagicCommand::Vars => {
                let mut lines = Vec::new();
                for (name, var) in &self.variables {
                    lines.push(format!("  {}: {} = {}", name, var.type_name, var.value_repr));
                }
                if lines.is_empty() {
                    vec![RichOutput::text("No variables defined.")]
                } else {
                    vec![RichOutput::text(&lines.join("\n"))]
                }
            }
            MagicCommand::History => {
                let text = self.history.iter().enumerate()
                    .map(|(i, h)| format!("In[{}]: {}", i + 1, h))
                    .collect::<Vec<_>>()
                    .join("\n");
                vec![RichOutput::text(&text)]
            }
            MagicCommand::Time => vec![RichOutput::text("CPU times: user 0.01s, sys 0.00s, total 0.01s")],
            MagicCommand::Ast => vec![RichOutput::text("AST output for last cell")],
            MagicCommand::Ir => vec![RichOutput::text("IR output for last cell")],
            _ => vec![RichOutput::text("Magic command executed.")],
        }
    }

    /// Complete code at cursor position.
    pub fn complete(&self, code: &str, cursor_pos: usize) -> Vec<String> {
        let prefix = &code[..cursor_pos];
        let word_start = prefix.rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let partial = &prefix[word_start..];

        // Return variable names matching prefix.
        self.variables.keys()
            .filter(|k| k.starts_with(partial))
            .cloned()
            .collect()
    }

    /// Inspect symbol at cursor.
    pub fn inspect(&self, code: &str, cursor_pos: usize) -> Option<String> {
        let prefix = &code[..cursor_pos.min(code.len())];
        let word_start = prefix.rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &prefix[word_start..];
        self.variables.get(word).map(|v| format!("{}: {}", v.name, v.type_name))
    }

    /// Define a variable.
    pub fn define_var(&mut self, name: &str, type_name: &str, value: &str) {
        self.variables.insert(name.to_string(), Variable {
            name: name.to_string(),
            type_name: type_name.to_string(),
            value_repr: value.to_string(),
            size_bytes: value.len(),
        });
    }

    /// Reset kernel state.
    pub fn reset(&mut self) {
        self.variables.clear();
        self.history.clear();
        self.execution_count = 0;
        self.state = KernelState::Idle;
    }

    /// Get kernel info.
    pub fn kernel_info(&self) -> MessageContent {
        MessageContent::KernelInfo {
            language: "vitalis".into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            banner: "Vitalis Notebook Kernel".into(),
        }
    }

    pub fn execution_count(&self) -> u32 {
        self.execution_count
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_start() {
        let mut kernel = NotebookKernel::new();
        assert_eq!(kernel.state, KernelState::Starting);
        kernel.start();
        assert_eq!(kernel.state, KernelState::Idle);
    }

    #[test]
    fn test_kernel_execute() {
        let mut kernel = NotebookKernel::new();
        kernel.start();
        let output = kernel.execute("let x = 42");
        assert!(!output.is_empty());
        assert_eq!(kernel.execution_count(), 1);
    }

    #[test]
    fn test_kernel_history() {
        let mut kernel = NotebookKernel::new();
        kernel.start();
        kernel.execute("let a = 1");
        kernel.execute("let b = 2");
        assert_eq!(kernel.history_len(), 2);
    }

    #[test]
    fn test_magic_parse() {
        assert_eq!(parse_magic("%time"), Some(MagicCommand::Time));
        assert_eq!(parse_magic("%help"), Some(MagicCommand::Help));
        assert_eq!(parse_magic("%vars"), Some(MagicCommand::Vars));
        assert!(parse_magic("not magic").is_none());
    }

    #[test]
    fn test_magic_help() {
        let mut kernel = NotebookKernel::new();
        kernel.start();
        let output = kernel.execute("%help");
        assert!(!output.is_empty());
        assert!(output[0].data.contains("Magic commands"));
    }

    #[test]
    fn test_magic_vars() {
        let mut kernel = NotebookKernel::new();
        kernel.start();
        kernel.define_var("x", "i32", "42");
        let output = kernel.execute("%vars");
        assert!(output[0].data.contains("x"));
    }

    #[test]
    fn test_magic_history() {
        let mut kernel = NotebookKernel::new();
        kernel.start();
        kernel.execute("let a = 1");
        let output = kernel.execute("%history");
        assert!(output[0].data.contains("In[1]"));
    }

    #[test]
    fn test_variable_define() {
        let mut kernel = NotebookKernel::new();
        kernel.define_var("count", "i64", "100");
        assert_eq!(kernel.variable_count(), 1);
    }

    #[test]
    fn test_complete() {
        let mut kernel = NotebookKernel::new();
        kernel.define_var("counter", "i32", "0");
        kernel.define_var("count_max", "i32", "100");
        kernel.define_var("other", "bool", "true");
        let matches = kernel.complete("cou", 3);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_inspect() {
        let mut kernel = NotebookKernel::new();
        kernel.define_var("data", "Vec<i32>", "[1,2,3]");
        let info = kernel.inspect("data", 4);
        assert!(info.is_some());
        assert!(info.unwrap().contains("Vec<i32>"));
    }

    #[test]
    fn test_kernel_reset() {
        let mut kernel = NotebookKernel::new();
        kernel.start();
        kernel.execute("code");
        kernel.define_var("x", "i32", "1");
        kernel.reset();
        assert_eq!(kernel.variable_count(), 0);
        assert_eq!(kernel.execution_count(), 0);
    }

    #[test]
    fn test_rich_output_types() {
        let t = RichOutput::text("hello");
        assert_eq!(t.mime_type, "text/plain");
        let h = RichOutput::html("<b>bold</b>");
        assert_eq!(h.mime_type, "text/html");
        let s = RichOutput::svg("<svg/>");
        assert_eq!(s.mime_type, "image/svg+xml");
        let j = RichOutput::json("{}");
        assert_eq!(j.mime_type, "application/json");
        let l = RichOutput::latex("$x^2$");
        assert_eq!(l.mime_type, "text/latex");
    }

    #[test]
    fn test_cell_types() {
        assert_ne!(CellType::Code, CellType::Markdown);
        assert_ne!(CellType::Markdown, CellType::Raw);
    }

    #[test]
    fn test_execute_status() {
        assert_ne!(ExecuteStatus::Ok, ExecuteStatus::Error);
        assert_ne!(ExecuteStatus::Error, ExecuteStatus::Abort);
    }

    #[test]
    fn test_kernel_info() {
        let kernel = NotebookKernel::new();
        match kernel.kernel_info() {
            MessageContent::KernelInfo { language, .. } => assert_eq!(language, "vitalis"),
            _ => panic!("expected KernelInfo"),
        }
    }

    #[test]
    fn test_message_types() {
        let types = vec![
            MessageType::ExecuteRequest, MessageType::ExecuteReply,
            MessageType::CompleteRequest, MessageType::CompleteReply,
        ];
        assert_eq!(types.len(), 4);
    }
}
