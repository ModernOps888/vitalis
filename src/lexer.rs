//! Vitalis Lexer — Token definitions using logos for zero-copy lexing.
//!
//! Designed for error-recovery: unknown tokens produce `Token::Error`
//! instead of crashing, which is critical because LLM-generated source
//! may contain imperfect syntax.

use logos::Logos;
use std::fmt;

/// Every token the Vitalis lexer can produce.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r]+")]  // skip whitespace (not newlines — they're significant for some rules)
pub enum Token {
    // ── Literals ──────────────────────────────────────────────────────
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLiteral(i64),

    #[regex(r"-?[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    FloatLiteral(f64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    StringLiteral(String),

    #[token("true")]
    True,

    #[token("false")]
    False,

    // ── Keywords ──────────────────────────────────────────────────────
    #[token("module")]
    Module,

    #[token("fn")]
    Fn,

    #[token("let")]
    Let,

    #[token("mut")]
    Mut,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("match")]
    Match,

    #[token("for")]
    For,

    #[token("in")]
    In,

    #[token("while")]
    While,

    #[token("loop")]
    Loop,

    #[token("break")]
    Break,

    #[token("continue")]
    Continue,

    #[token("return")]
    Return,

    #[token("struct")]
    Struct,

    #[token("enum")]
    Enum,

    #[token("impl")]
    Impl,

    #[token("trait")]
    Trait,

    #[token("type")]
    TypeKw,

    #[token("import")]
    Import,

    #[token("extern")]
    Extern,

    #[token("pub")]
    Pub,

    #[token("self")]
    SelfKw,

    #[token("as")]
    As,

    // ── Error handling keywords ───────────────────────────────────────
    #[token("try")]
    Try,

    #[token("catch")]
    Catch,

    #[token("throw")]
    Throw,

    // ── Evolution keywords ────────────────────────────────────────────
    #[token("evolve")]
    Evolve,

    #[token("mutation")]
    Mutation,

    #[token("pipeline")]
    Pipeline,

    #[token("stage")]
    Stage,

    #[token("sandbox")]
    Sandbox,

    #[token("rollback")]
    Rollback,

    #[token("version")]
    Version,

    #[token("fitness")]
    Fitness,

    // ── Memory keywords ───────────────────────────────────────────────
    #[token("memory")]
    Memory,

    #[token("schema")]
    Schema,

    #[token("index")]
    Index,

    #[token("decay")]
    Decay,

    #[token("consolidate")]
    Consolidate,

    #[token("recall")]
    Recall,

    #[token("store")]
    Store,

    // ── Concurrency keywords ──────────────────────────────────────────
    #[token("parallel")]
    Parallel,

    #[token("race")]
    Race,

    #[token("async")]
    Async,

    #[token("await")]
    Await,

    #[token("spawn")]
    Spawn,

    // ── Safety keywords ───────────────────────────────────────────────
    #[token("trust_tier")]
    TrustTier,

    #[token("capability")]
    Capability,

    #[token("immutable")]
    Immutable,

    #[token("evolvable")]
    Evolvable,

    // ── Introspection keywords ────────────────────────────────────────
    #[token("reflect")]
    Reflect,

    #[token("awareness")]
    Awareness,

    #[token("introspect")]
    Introspect,

    #[token("qualia")]
    Qualia,

    #[token("signal")]
    Signal,

    #[token("dream")]
    Dream,

    // ── Meta-evolution keywords ───────────────────────────────────────
    #[token("strategy")]
    Strategy,

    #[token("breed")]
    Breed,

    #[token("extinct")]
    Extinct,

    #[token("adapt")]
    Adapt,

    #[token("explore")]
    Explore,

    #[token("exploit")]
    Exploit,

    // ── Swarm & distributed AI keywords ──────────────────────────────
    #[token("swarm")]
    Swarm,

    #[token("hive")]
    Hive,

    #[token("distribute")]
    Distribute,

    #[token("consensus")]
    Consensus,

    #[token("broadcast")]
    Broadcast,

    #[token("aggregate")]
    Aggregate,

    // ── Quantum-inspired keywords ─────────────────────────────────────
    #[token("quantum")]
    Quantum,

    #[token("superpose")]
    Superpose,

    #[token("entangle")]
    Entangle,

    #[token("decohere")]
    Decohere,

    // ── Inference & Bayesian keywords ─────────────────────────────────
    #[token("infer")]
    Infer,

    #[token("propagate")]
    Propagate,

    // ── Built-in type keywords ────────────────────────────────────────
    #[token("i32")]
    I32,

    #[token("i64")]
    I64,

    #[token("f32")]
    F32,

    #[token("f64")]
    F64,

    #[token("bool")]
    Bool,

    #[token("str")]
    Str,

    #[token("void")]
    Void,

    // ── Operators ─────────────────────────────────────────────────────
    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[token("==")]
    EqEq,

    #[token("!=")]
    NotEq,

    #[token("<=")]
    LtEq,

    #[token(">=")]
    GtEq,

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("&&")]
    AndAnd,

    #[token("||")]
    OrOr,

    #[token("!")]
    Bang,

    #[token("=")]
    Eq,

    #[token("+=")]
    PlusEq,

    #[token("-=")]
    MinusEq,

    #[token("*=")]
    StarEq,

    #[token("/=")]
    SlashEq,

    #[token("?")]
    Question,

    #[token("|>")]
    PipeArrow,

    #[token("|")]
    Pipe,

    #[token("=>")]
    FatArrow,

    #[token("->")]
    Arrow,

    // ── Delimiters ────────────────────────────────────────────────────
    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    // ── Punctuation ───────────────────────────────────────────────────
    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

    #[token("::")]
    ColonColon,

    #[token(";")]
    Semicolon,

    #[token(".")]
    Dot,

    #[token("..")]
    DotDot,

    #[token("@")]
    At,

    #[token("#")]
    Hash,

    #[token("\n")]
    Newline,

    // ── Identifiers ───────────────────────────────────────────────────
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // ── Comments (skipped, but captured for doc-comments later) ──────
    #[regex(r"//[^\n]*", logos::skip)]
    LineComment,

    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip)]
    BlockComment,

    // ── Annotation ────────────────────────────────────────────────────
    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice()[1..].to_string())]
    Annotation(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::IntLiteral(n) => write!(f, "{}", n),
            Token::FloatLiteral(n) => write!(f, "{}", n),
            Token::StringLiteral(s) => write!(f, "\"{}\"", s),
            Token::Ident(s) => write!(f, "{}", s),
            Token::Annotation(s) => write!(f, "@{}", s),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::Eq => write!(f, "="),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            _ => write!(f, "{:?}", self),
        }
    }
}

/// A token with its span (byte offset range in source).
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: std::ops::Range<usize>,
    /// True if one or more newlines appeared before this token in the source.
    pub has_leading_newline: bool,
}

/// Lex source code into a vector of spanned tokens.
/// Never panics — unknown characters produce errors that are filtered out,
/// allowing partial parsing of LLM-generated code.
pub fn lex(source: &str) -> (Vec<SpannedToken>, Vec<LexError>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut lexer = Token::lexer(source);
    let mut had_newline = false;

    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(token) => {
                // Track but skip newlines from the token stream
                if matches!(token, Token::Newline) {
                    had_newline = true;
                    continue;
                }
                tokens.push(SpannedToken { token, span, has_leading_newline: had_newline });
                had_newline = false;
            }
            Err(()) => {
                errors.push(LexError {
                    span: span.clone(),
                    text: source[span.clone()].to_string(),
                });
            }
        }
    }

    (tokens, errors)
}

/// A lexical error — an unrecognized character sequence.
#[derive(Debug, Clone)]
pub struct LexError {
    pub span: std::ops::Range<usize>,
    pub text: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unexpected character(s) '{}' at offset {}..{}",
            self.text, self.span.start, self.span.end
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let (tokens, errors) = lex("fn main() -> i64 { return 42; }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(tokens[0].token, Token::Fn);
        assert!(matches!(tokens[1].token, Token::Ident(ref s) if s == "main"));
        assert_eq!(tokens[2].token, Token::LParen);
        assert_eq!(tokens[3].token, Token::RParen);
        assert_eq!(tokens[4].token, Token::Arrow);
        assert_eq!(tokens[5].token, Token::I64);
        assert_eq!(tokens[6].token, Token::LBrace);
        assert_eq!(tokens[7].token, Token::Return);
        assert_eq!(tokens[8].token, Token::IntLiteral(42));
        assert_eq!(tokens[9].token, Token::Semicolon);
        assert_eq!(tokens[10].token, Token::RBrace);
    }

    #[test]
    fn test_evolution_keywords() {
        let (tokens, _) = lex("evolve mutation pipeline stage sandbox rollback fitness");
        assert_eq!(tokens[0].token, Token::Evolve);
        assert_eq!(tokens[1].token, Token::Mutation);
        assert_eq!(tokens[2].token, Token::Pipeline);
        assert_eq!(tokens[3].token, Token::Stage);
        assert_eq!(tokens[4].token, Token::Sandbox);
        assert_eq!(tokens[5].token, Token::Rollback);
        assert_eq!(tokens[6].token, Token::Fitness);
    }

    #[test]
    fn test_memory_keywords() {
        let (tokens, _) = lex("memory schema index decay consolidate recall store");
        assert_eq!(tokens[0].token, Token::Memory);
        assert_eq!(tokens[1].token, Token::Schema);
        assert_eq!(tokens[2].token, Token::Index);
        assert_eq!(tokens[3].token, Token::Decay);
        assert_eq!(tokens[4].token, Token::Consolidate);
        assert_eq!(tokens[5].token, Token::Recall);
        assert_eq!(tokens[6].token, Token::Store);
    }

    #[test]
    fn test_ai_native_keywords() {
        let (tokens, errors) = lex("swarm hive distribute consensus broadcast aggregate quantum superpose entangle decohere infer propagate");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(tokens[0].token, Token::Swarm);
        assert_eq!(tokens[1].token, Token::Hive);
        assert_eq!(tokens[2].token, Token::Distribute);
        assert_eq!(tokens[3].token, Token::Consensus);
        assert_eq!(tokens[4].token, Token::Broadcast);
        assert_eq!(tokens[5].token, Token::Aggregate);
        assert_eq!(tokens[6].token, Token::Quantum);
        assert_eq!(tokens[7].token, Token::Superpose);
        assert_eq!(tokens[8].token, Token::Entangle);
        assert_eq!(tokens[9].token, Token::Decohere);
        assert_eq!(tokens[10].token, Token::Infer);
        assert_eq!(tokens[11].token, Token::Propagate);
    }

    #[test]
    fn test_string_literal() {
        let (tokens, errors) = lex(r#""hello world""#);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].token, Token::StringLiteral("hello world".into()));
    }

    #[test]
    fn test_annotations() {
        let (tokens, _) = lex("@evolvable @immutable @trust_tier");
        // Note: @evolvable as annotation overlaps with keyword — logos picks first match
        // Annotations starting with @ are captured by the Annotation regex
        assert!(tokens.len() >= 3);
    }

    #[test]
    fn test_error_recovery() {
        let (tokens, errors) = lex("let x = 42 $ + 3");
        // $ is not a valid token — should produce an error but continue parsing
        assert!(!errors.is_empty());
        // Still got valid tokens around the error
        assert!(tokens.len() >= 4);
    }

    #[test]
    fn test_pipe_arrow() {
        let (tokens, _) = lex("a |> b |> c");
        assert!(matches!(tokens[0].token, Token::Ident(ref s) if s == "a"));
        assert_eq!(tokens[1].token, Token::PipeArrow);
        assert!(matches!(tokens[2].token, Token::Ident(ref s) if s == "b"));
        assert_eq!(tokens[3].token, Token::PipeArrow);
    }

    #[test]
    fn test_float_literal() {
        let (tokens, _) = lex("3.14 2.5e10 1.0");
        assert_eq!(tokens[0].token, Token::FloatLiteral(3.14));
    }

    #[test]
    fn test_introspection_keywords() {
        let (tokens, _) = lex("reflect awareness introspect qualia signal dream");
        assert_eq!(tokens[0].token, Token::Reflect);
        assert_eq!(tokens[1].token, Token::Awareness);
        assert_eq!(tokens[2].token, Token::Introspect);
        assert_eq!(tokens[3].token, Token::Qualia);
        assert_eq!(tokens[4].token, Token::Signal);
        assert_eq!(tokens[5].token, Token::Dream);
    }

    #[test]
    fn test_meta_evolution_keywords() {
        let (tokens, _) = lex("strategy breed extinct adapt explore exploit");
        assert_eq!(tokens[0].token, Token::Strategy);
        assert_eq!(tokens[1].token, Token::Breed);
        assert_eq!(tokens[2].token, Token::Extinct);
        assert_eq!(tokens[3].token, Token::Adapt);
        assert_eq!(tokens[4].token, Token::Explore);
        assert_eq!(tokens[5].token, Token::Exploit);
    }

    #[test]
    fn test_impl_trait_self_tokens() {
        let (tokens, errors) = lex("impl trait self");
        assert!(errors.is_empty());
        assert_eq!(tokens[0].token, Token::Impl);
        assert_eq!(tokens[1].token, Token::Trait);
        assert_eq!(tokens[2].token, Token::SelfKw);
    }

    #[test]
    fn test_type_keyword() {
        let (tokens, _) = lex("type");
        assert_eq!(tokens[0].token, Token::TypeKw);
    }

    #[test]
    fn test_async_await_spawn() {
        let (tokens, _) = lex("async await spawn");
        assert_eq!(tokens[0].token, Token::Async);
        assert_eq!(tokens[1].token, Token::Await);
        assert_eq!(tokens[2].token, Token::Spawn);
    }

    #[test]
    fn test_pub_keyword() {
        let (tokens, _) = lex("pub fn main() {}");
        assert_eq!(tokens[0].token, Token::Pub);
        assert_eq!(tokens[1].token, Token::Fn);
    }

    #[test]
    fn test_safety_keywords() {
        let (tokens, _) = lex("trust_tier capability immutable evolvable");
        assert_eq!(tokens[0].token, Token::TrustTier);
        assert_eq!(tokens[1].token, Token::Capability);
        assert_eq!(tokens[2].token, Token::Immutable);
        assert_eq!(tokens[3].token, Token::Evolvable);
    }

}
