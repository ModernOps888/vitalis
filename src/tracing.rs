//! Structured distributed tracing for Vitalis.
//!
//! Span-based instrumentation and distributed trace propagation:
//! - **Span trees**: Hierarchical trace spans with key-value fields
//! - **Trace context**: W3C TraceContext propagation headers
//! - **Flame graph export**: Convert span trees to flame graph format
//! - **Log correlation**: Link structured logs to trace spans
//! - **OpenTelemetry format**: OTLP-compatible trace export

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ── Core Types ──────────────────────────────────────────────────────

/// A unique trace identifier (128-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(pub u64, pub u64);

/// A unique span identifier (64-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(pub u64);

/// Span status.
#[derive(Debug, Clone, PartialEq)]
pub enum SpanStatus {
    Ok,
    Error(String),
    Unset,
}

/// Span kind (client, server, internal, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum SpanKind {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

/// Severity level for log events.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// An attribute value.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    StringArray(Vec<String>),
    IntArray(Vec<i64>),
}

// ── Span ────────────────────────────────────────────────────────────

/// A trace span representing a unit of work.
#[derive(Debug, Clone)]
pub struct Span {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_id: Option<SpanId>,
    pub name: String,
    pub kind: SpanKind,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub status: SpanStatus,
    pub attributes: HashMap<String, AttrValue>,
    pub events: Vec<SpanEvent>,
    pub links: Vec<SpanLink>,
}

/// An event within a span (log line).
#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: u64,
    pub severity: Severity,
    pub attributes: HashMap<String, AttrValue>,
}

/// A link to another span (causal relationship).
#[derive(Debug, Clone)]
pub struct SpanLink {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub attributes: HashMap<String, AttrValue>,
}

impl Span {
    pub fn new(trace_id: TraceId, span_id: SpanId, name: &str, kind: SpanKind) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
        Self {
            trace_id,
            span_id,
            parent_id: None,
            name: name.to_string(),
            kind,
            start_time: now,
            end_time: None,
            status: SpanStatus::Unset,
            attributes: HashMap::new(),
            events: Vec::new(),
            links: Vec::new(),
        }
    }

    pub fn with_parent(mut self, parent: SpanId) -> Self {
        self.parent_id = Some(parent);
        self
    }

    pub fn set_attribute(&mut self, key: &str, value: AttrValue) {
        self.attributes.insert(key.to_string(), value);
    }

    pub fn add_event(&mut self, name: &str, severity: Severity) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
        self.events.push(SpanEvent {
            name: name.to_string(),
            timestamp: now,
            severity,
            attributes: HashMap::new(),
        });
    }

    pub fn end(&mut self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
        self.end_time = Some(now);
    }

    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| Duration::from_nanos(end - self.start_time))
    }

    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

// ── Trace Context (W3C) ─────────────────────────────────────────────

/// W3C TraceContext propagation header.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceContext {
    pub version: u8,
    pub trace_id: TraceId,
    pub parent_id: SpanId,
    pub trace_flags: u8,
}

impl TraceContext {
    pub fn new(trace_id: TraceId, parent_id: SpanId) -> Self {
        Self {
            version: 0,
            trace_id,
            parent_id,
            trace_flags: 0x01, // sampled
        }
    }

    /// Serialize to W3C traceparent header format.
    pub fn to_header(&self) -> String {
        format!(
            "{:02x}-{:016x}{:016x}-{:016x}-{:02x}",
            self.version, self.trace_id.0, self.trace_id.1, self.parent_id.0, self.trace_flags
        )
    }

    /// Parse from W3C traceparent header.
    pub fn from_header(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 { return None; }
        let version = u8::from_str_radix(parts[0], 16).ok()?;
        let trace_hi = u64::from_str_radix(&parts[1][..16], 16).ok()?;
        let trace_lo = u64::from_str_radix(&parts[1][16..], 16).ok()?;
        let parent = u64::from_str_radix(parts[2], 16).ok()?;
        let flags = u8::from_str_radix(parts[3], 16).ok()?;
        Some(Self {
            version,
            trace_id: TraceId(trace_hi, trace_lo),
            parent_id: SpanId(parent),
            trace_flags: flags,
        })
    }

    pub fn is_sampled(&self) -> bool {
        self.trace_flags & 0x01 != 0
    }
}

// ── Tracer ──────────────────────────────────────────────────────────

/// A tracer that collects spans.
pub struct Tracer {
    pub service_name: String,
    spans: Vec<Span>,
    next_span_id: u64,
    next_trace_id: u64,
}

impl Tracer {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            spans: Vec::new(),
            next_span_id: 1,
            next_trace_id: 1,
        }
    }

    /// Start a new root span.
    pub fn start_span(&mut self, name: &str, kind: SpanKind) -> SpanId {
        let trace_id = TraceId(0, self.next_trace_id);
        self.next_trace_id += 1;
        let span_id = SpanId(self.next_span_id);
        self.next_span_id += 1;
        let span = Span::new(trace_id, span_id, name, kind);
        self.spans.push(span);
        span_id
    }

    /// Start a child span.
    pub fn start_child_span(&mut self, parent: SpanId, name: &str, kind: SpanKind) -> SpanId {
        let parent_trace = self.spans.iter()
            .find(|s| s.span_id == parent)
            .map(|s| s.trace_id)
            .unwrap_or(TraceId(0, 0));
        let span_id = SpanId(self.next_span_id);
        self.next_span_id += 1;
        let span = Span::new(parent_trace, span_id, name, kind).with_parent(parent);
        self.spans.push(span);
        span_id
    }

    /// End a span.
    pub fn end_span(&mut self, id: SpanId) {
        if let Some(span) = self.spans.iter_mut().find(|s| s.span_id == id) {
            span.end();
        }
    }

    /// Get a span by ID.
    pub fn get_span(&self, id: SpanId) -> Option<&Span> {
        self.spans.iter().find(|s| s.span_id == id)
    }

    /// Get mutable span.
    pub fn get_span_mut(&mut self, id: SpanId) -> Option<&mut Span> {
        self.spans.iter_mut().find(|s| s.span_id == id)
    }

    /// All completed spans.
    pub fn completed_spans(&self) -> Vec<&Span> {
        self.spans.iter().filter(|s| s.end_time.is_some()).collect()
    }

    /// Total span count.
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Get children of a span.
    pub fn children(&self, parent: SpanId) -> Vec<&Span> {
        self.spans.iter().filter(|s| s.parent_id == Some(parent)).collect()
    }

    /// Export to flame graph folded stacks format.
    pub fn to_flame_graph(&self) -> Vec<String> {
        let mut stacks = Vec::new();
        for span in &self.spans {
            if span.is_root() && span.end_time.is_some() {
                let mut stack_parts = vec![span.name.clone()];
                self.collect_flame_stacks(span.span_id, &mut stack_parts, &mut stacks);
            }
        }
        stacks
    }

    fn collect_flame_stacks(&self, parent: SpanId, path: &mut Vec<String>, result: &mut Vec<String>) {
        let children = self.children(parent);
        if children.is_empty() {
            let parent_span = self.get_span(parent).unwrap();
            let duration_us = parent_span.duration().map(|d| d.as_micros()).unwrap_or(0);
            result.push(format!("{} {}", path.join(";"), duration_us));
        } else {
            for child in children {
                path.push(child.name.clone());
                self.collect_flame_stacks(child.span_id, path, result);
                path.pop();
            }
        }
    }
}

// ── Log Record ──────────────────────────────────────────────────────

/// A structured log record correlated with a trace.
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub timestamp: u64,
    pub severity: Severity,
    pub body: String,
    pub trace_id: Option<TraceId>,
    pub span_id: Option<SpanId>,
    pub attributes: HashMap<String, AttrValue>,
}

/// Structured logger with trace correlation.
pub struct TracingLogger {
    records: Vec<LogRecord>,
    min_severity: Severity,
}

impl TracingLogger {
    pub fn new(min_severity: Severity) -> Self {
        Self {
            records: Vec::new(),
            min_severity,
        }
    }

    pub fn log(&mut self, severity: Severity, body: &str, trace_id: Option<TraceId>, span_id: Option<SpanId>) {
        if severity < self.min_severity { return; }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
        self.records.push(LogRecord {
            timestamp: now,
            severity,
            body: body.to_string(),
            trace_id,
            span_id,
            attributes: HashMap::new(),
        });
    }

    pub fn records(&self) -> &[LogRecord] {
        &self.records
    }

    pub fn records_for_trace(&self, trace_id: TraceId) -> Vec<&LogRecord> {
        self.records.iter().filter(|r| r.trace_id == Some(trace_id)).collect()
    }

    pub fn records_for_span(&self, span_id: SpanId) -> Vec<&LogRecord> {
        self.records.iter().filter(|r| r.span_id == Some(span_id)).collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new(TraceId(0, 1), SpanId(1), "test", SpanKind::Internal);
        assert_eq!(span.name, "test");
        assert!(span.is_root());
        assert!(span.end_time.is_none());
    }

    #[test]
    fn test_span_with_parent() {
        let span = Span::new(TraceId(0, 1), SpanId(2), "child", SpanKind::Client)
            .with_parent(SpanId(1));
        assert!(!span.is_root());
        assert_eq!(span.parent_id, Some(SpanId(1)));
    }

    #[test]
    fn test_span_attributes() {
        let mut span = Span::new(TraceId(0, 1), SpanId(1), "op", SpanKind::Server);
        span.set_attribute("http.method", AttrValue::String("GET".into()));
        span.set_attribute("http.status", AttrValue::Int(200));
        assert_eq!(span.attributes.len(), 2);
    }

    #[test]
    fn test_span_events() {
        let mut span = Span::new(TraceId(0, 1), SpanId(1), "op", SpanKind::Internal);
        span.add_event("cache_hit", Severity::Debug);
        span.add_event("db_query", Severity::Info);
        assert_eq!(span.events.len(), 2);
    }

    #[test]
    fn test_span_end() {
        let mut span = Span::new(TraceId(0, 1), SpanId(1), "op", SpanKind::Internal);
        span.end();
        assert!(span.end_time.is_some());
        assert!(span.duration().is_some());
    }

    #[test]
    fn test_trace_context_header() {
        let ctx = TraceContext::new(TraceId(0x1234, 0x5678), SpanId(0xABCD));
        let header = ctx.to_header();
        assert!(header.starts_with("00-"));
        let parsed = TraceContext::from_header(&header).unwrap();
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert_eq!(parsed.parent_id, ctx.parent_id);
    }

    #[test]
    fn test_trace_context_sampled() {
        let ctx = TraceContext::new(TraceId(0, 1), SpanId(1));
        assert!(ctx.is_sampled());
    }

    #[test]
    fn test_tracer_root_span() {
        let mut tracer = Tracer::new("test-service");
        let id = tracer.start_span("request", SpanKind::Server);
        assert_eq!(tracer.span_count(), 1);
        tracer.end_span(id);
        assert_eq!(tracer.completed_spans().len(), 1);
    }

    #[test]
    fn test_tracer_child_span() {
        let mut tracer = Tracer::new("test-service");
        let parent = tracer.start_span("request", SpanKind::Server);
        let child = tracer.start_child_span(parent, "db_query", SpanKind::Client);
        assert_eq!(tracer.span_count(), 2);
        let child_span = tracer.get_span(child).unwrap();
        assert_eq!(child_span.parent_id, Some(parent));
    }

    #[test]
    fn test_tracer_children() {
        let mut tracer = Tracer::new("svc");
        let root = tracer.start_span("root", SpanKind::Internal);
        tracer.start_child_span(root, "c1", SpanKind::Internal);
        tracer.start_child_span(root, "c2", SpanKind::Internal);
        assert_eq!(tracer.children(root).len(), 2);
    }

    #[test]
    fn test_flame_graph_export() {
        let mut tracer = Tracer::new("svc");
        let root = tracer.start_span("main", SpanKind::Internal);
        let child = tracer.start_child_span(root, "work", SpanKind::Internal);
        tracer.end_span(child);
        tracer.end_span(root);
        let stacks = tracer.to_flame_graph();
        assert!(!stacks.is_empty());
        assert!(stacks[0].contains("main;work"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Trace < Severity::Debug);
        assert!(Severity::Debug < Severity::Info);
        assert!(Severity::Info < Severity::Warn);
        assert!(Severity::Warn < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
    }

    #[test]
    fn test_tracing_logger() {
        let mut logger = TracingLogger::new(Severity::Info);
        logger.log(Severity::Debug, "debug msg", None, None); // filtered
        logger.log(Severity::Info, "info msg", None, None);
        logger.log(Severity::Error, "error msg", None, None);
        assert_eq!(logger.records().len(), 2);
    }

    #[test]
    fn test_logger_trace_correlation() {
        let mut logger = TracingLogger::new(Severity::Trace);
        let tid = TraceId(0, 42);
        let sid = SpanId(1);
        logger.log(Severity::Info, "in span", Some(tid), Some(sid));
        logger.log(Severity::Info, "other", None, None);
        assert_eq!(logger.records_for_trace(tid).len(), 1);
        assert_eq!(logger.records_for_span(sid).len(), 1);
    }

    #[test]
    fn test_span_status() {
        let mut span = Span::new(TraceId(0, 1), SpanId(1), "op", SpanKind::Internal);
        span.set_status(SpanStatus::Error("timeout".into()));
        assert_eq!(span.status, SpanStatus::Error("timeout".into()));
    }

    #[test]
    fn test_attr_value_variants() {
        let v1 = AttrValue::String("hello".into());
        let v2 = AttrValue::Int(42);
        let v3 = AttrValue::Float(3.14);
        let v4 = AttrValue::Bool(true);
        let v5 = AttrValue::StringArray(vec!["a".into()]);
        let v6 = AttrValue::IntArray(vec![1, 2, 3]);
        assert_ne!(v1, v2);
        assert_ne!(v3, v4);
        assert_ne!(v5, v6);
    }

    #[test]
    fn test_span_kind_variants() {
        let kinds = vec![
            SpanKind::Internal, SpanKind::Server, SpanKind::Client,
            SpanKind::Producer, SpanKind::Consumer,
        ];
        assert_eq!(kinds.len(), 5);
    }
}
