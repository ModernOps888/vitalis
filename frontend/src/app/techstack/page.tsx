"use client";

import { useState, useEffect, useRef, useMemo } from "react";

/* ═══════════════════════════════════════════════════════════
   TYPES
   ═══════════════════════════════════════════════════════════ */
interface TechComponent {
  component: string;
  loc: number;
  files: number;
  tests?: number;
  modules?: number;
  routes?: number;
  features: string[];
}

interface PipelineStage {
  name: string;
  status: string;
}

interface Stats {
  timestamp: number;
  status: string;
  uptime_seconds: number;
  modules_loaded: number;
  module_names: string[];
  memory_count: number;
  active_goals: number;
  total_goals: number;
  evolution: {
    total_attempts: number;
    successes: number;
    failures: number;
    rollbacks: number;
  };
  vitalis: Record<string, unknown> | null;
  swarm: { active_agents: number; total_tasks: number; max_concurrent: number } | null;
  infrastructure: {
    cpu_percent: number;
    ram_total_gb: number;
    ram_used_gb: number;
    ram_percent: number;
    platform: string;
    python_version: string;
    cpu_cores: number;
  };
  guardian: { checks_passed: number; checks_blocked: number } | null;
  tech_stack: Record<string, TechComponent>;
  compiler_pipeline: PipelineStage[];
  total_loc: number;
  version: string;
  error?: string;
}

/* ═══════════════════════════════════════════════════════════
   ALIEN COLOR PALETTE
   ═══════════════════════════════════════════════════════════ */
const ALIEN = {
  cyan: "#00f0ff",
  magenta: "#ff00e5",
  green: "#39ff14",
  violet: "#b026ff",
  orange: "#ff6a00",
  gold: "#ffd700",
  red: "#ff003c",
  dim: "#5a6a8a",
};

/* ═══════════════════════════════════════════════════════════
   API
   ═══════════════════════════════════════════════════════════ */
const API = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8002";

async function fetchStats(): Promise<Stats | null> {
  try {
    const r = await fetch(`${API}/public/stats`, { cache: "no-store" });
    if (!r.ok) return null;
    return await r.json();
  } catch {
    return null;
  }
}

/* ═══════════════════════════════════════════════════════════
   HELPERS
   ═══════════════════════════════════════════════════════════ */
function formatUptime(s: number): string {
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatNum(n: number): string {
  return n.toLocaleString();
}

/* ═══════════════════════════════════════════════════════════
   STATIC DATA — Rust Module Inventory
   ═══════════════════════════════════════════════════════════ */
const RUST_MODULES = [
  { name: "codegen.rs", loc: 3494, purpose: "Cranelift 0.116 JIT + Phase 25 stdlib (98 builtins)", pct: 100 },
  { name: "hotpath.rs", loc: 1911, purpose: "44 native hotpath ops + layer_norm/dropout/cosine_distance/huber/mse", pct: 55 },
  { name: "ir.rs", loc: 1853, purpose: "SSA-form IR + match/pipe codegen", pct: 53 },
  { name: "parser.rs", loc: 1729, purpose: "Recursive-descent + Pratt parser", pct: 49 },
  { name: "optimizer.rs", loc: 1148, purpose: "Predictive JIT + Delta Debug", pct: 33 },
  { name: "tensor_engine.rs", loc: 983, purpose: "Custom tensor algebra engine", pct: 28 },
  { name: "quantum_math.rs", loc: 911, purpose: "Quantum-gate math primitives", pct: 26 },
  { name: "ml.rs", loc: 872, purpose: "Machine learning algorithms", pct: 25 },
  { name: "quantum_algorithms.rs", loc: 861, purpose: "Quantum algorithm implementations", pct: 25 },
  { name: "types.rs", loc: 855, purpose: "Two-pass type checker + scopes", pct: 24 },
  { name: "advanced_math.rs", loc: 833, purpose: "Advanced mathematical operations", pct: 24 },
  { name: "evolution_advanced.rs", loc: 818, purpose: "Advanced evolution strategies", pct: 23 },
  { name: "deep_learning.rs", loc: 789, purpose: "Neural network layers & training", pct: 23 },
  { name: "neuromorphic.rs", loc: 781, purpose: "Neuromorphic computing engine", pct: 22 },
  { name: "bridge.rs", loc: 764, purpose: "C FFI bridge (43 exports)", pct: 22 },
  { name: "engine.rs", loc: 760, purpose: "VitalisEngine core", pct: 22 },
  { name: "simd_ops.rs", loc: 748, purpose: "SIMD F64x4 vectorization (AVX2)", pct: 21 },
  { name: "meta_evolution.rs", loc: 734, purpose: "Thompson sampling strategies", pct: 21 },
  { name: "quantum.rs", loc: 721, purpose: "Quantum-inspired optimization", pct: 21 },
  { name: "graph.rs", loc: 716, purpose: "Graph algorithms & traversal", pct: 20 },
  { name: "bioinformatics.rs", loc: 702, purpose: "Bioinformatics sequence analysis", pct: 20 },
  { name: "memory.rs", loc: 693, purpose: "Engram storage (5 engram types)", pct: 20 },
  { name: "evolution.rs", loc: 690, purpose: "EvolutionRegistry + quantum UCB", pct: 20 },
  { name: "combinatorial.rs", loc: 660, purpose: "Combinatorial optimization (TSP, knapsack)", pct: 19 },
  { name: "numerical.rs", loc: 632, purpose: "Numerical methods & integration", pct: 18 },
  { name: "geometry.rs", loc: 627, purpose: "Computational geometry primitives", pct: 18 },
  { name: "ml_training.rs", loc: 625, purpose: "ML training pipeline & optimizers", pct: 18 },
  { name: "analytics.rs", loc: 609, purpose: "Statistical analytics engine", pct: 17 },
  { name: "automata.rs", loc: 591, purpose: "Finite & pushdown automata", pct: 17 },
  { name: "probability.rs", loc: 575, purpose: "Probabilistic distributions & sampling", pct: 16 },
  { name: "ast.rs", loc: 559, purpose: "27 expression variants + @annotation", pct: 16 },
  { name: "chemistry_advanced.rs", loc: 542, purpose: "Advanced chemistry simulations", pct: 16 },
  { name: "gpu_compute.rs", loc: 517, purpose: "GPU compute shader dispatch", pct: 15 },
  { name: "string_algorithms.rs", loc: 513, purpose: "Levenshtein · Jaro-Winkler · Hamming", pct: 15 },
  { name: "signal_processing.rs", loc: 503, purpose: "FFT, convolution & DSP", pct: 14 },
  { name: "lexer.rs", loc: 487, purpose: "Logos-based tokenizer (127 tokens)", pct: 14 },
  { name: "compression.rs", loc: 479, purpose: "LZ77, Huffman, RLE compression", pct: 14 },
  { name: "sorting.rs", loc: 465, purpose: "Hybrid sorting algorithms", pct: 13 },
  { name: "model_inference.rs", loc: 453, purpose: "Model inference & quantization", pct: 13 },
  { name: "science.rs", loc: 424, purpose: "Scientific computing & physics", pct: 12 },
  { name: "scoring.rs", loc: 413, purpose: "Scoring & ranking algorithms", pct: 12 },
  { name: "crypto.rs", loc: 392, purpose: "Cryptographic primitives (SHA-256, AES)", pct: 11 },
  { name: "security.rs", loc: 367, purpose: "Security hardening & tamper detection", pct: 11 },
  { name: "bpe_tokenizer.rs", loc: 315, purpose: "BPE tokenization for LLMs", pct: 9 },
  { name: "stdlib.rs", loc: 257, purpose: "98 built-in functions (Phase 25)", pct: 7 },
  { name: "main.rs", loc: 149, purpose: "CLI binary (vtc) + clap", pct: 4 },
  { name: "lib.rs", loc: 112, purpose: "Module declarations (47 modules)", pct: 3 },
];

const LOC_BAR_COLORS = [
  `linear-gradient(90deg, ${ALIEN.cyan}, ${ALIEN.magenta})`,
  `linear-gradient(90deg, ${ALIEN.green}, ${ALIEN.cyan})`,
  `linear-gradient(90deg, ${ALIEN.magenta}, ${ALIEN.violet})`,
  `linear-gradient(90deg, ${ALIEN.violet}, ${ALIEN.magenta})`,
  `linear-gradient(90deg, ${ALIEN.gold}, ${ALIEN.orange})`,
  `linear-gradient(90deg, ${ALIEN.orange}, ${ALIEN.red})`,
  `linear-gradient(90deg, ${ALIEN.red}, ${ALIEN.magenta})`,
  `linear-gradient(90deg, ${ALIEN.cyan}, ${ALIEN.green})`,
  `linear-gradient(90deg, ${ALIEN.green}, ${ALIEN.gold})`,
  `linear-gradient(90deg, ${ALIEN.gold}, ${ALIEN.cyan})`,
];

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Enhanced Particle Field — STRIPPED for performance
   ═══════════════════════════════════════════════════════════ */
function ParticleField() { return null; }
    const TARGET_FPS = 30;
    const FRAME_TIME = 1000 / TARGET_FPS;
/* ═══════════════════════════════════════════════════════════
   COMPONENT: Scroll Progress Bar
   ═══════════════════════════════════════════════════════════ */
function ScrollProgress() {
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    const onScroll = () => {
      const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
      const scrollHeight = document.documentElement.scrollHeight - document.documentElement.clientHeight;
      setProgress(scrollHeight > 0 ? (scrollTop / scrollHeight) * 100 : 0);
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return <div className="scroll-progress" style={{ width: `${progress}%` }} />;
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Data Stream Overlay — STRIPPED for performance
   ═══════════════════════════════════════════════════════════ */
function DataStream() { return null; }

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Clean Text (no glitch)
   ═══════════════════════════════════════════════════════════ */
function GlitchText({ text, className = "" }: { text: string; className?: string }) {
  return (
    <span className={className} style={{ display: "inline-block" }}>
      {text}
    </span>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Energy Connector
   ═══════════════════════════════════════════════════════════ */
function EnergyConnector() {
  return <div className="energy-connector" />;
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Section Header (with kinetic text reveal)
   ═══════════════════════════════════════════════════════════ */
function SectionHeader({ icon, title, badge, badgeType = "active" }: {
  icon: string; title: string; badge: string; badgeType?: "active" | "native" | "jit";
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(
      ([e]) => { if (e.isIntersecting) { setVisible(true); obs.disconnect(); } },
      { threshold: 0.3 }
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  return (
    <div ref={ref} className="section-header">
      <span className="section-icon">{icon}</span>
      <h2 className="section-title-text kinetic-title">
        {title.split("").map((ch, i) => (
          <span
            key={i}
            className={`kinetic-char ${visible ? "kinetic-visible" : ""}`}
            style={{ transitionDelay: `${i * 0.03}s` }}
          >
            {ch === " " ? "\u00A0" : ch}
          </span>
        ))}
      </h2>
      <span className={`section-badge badge-${badgeType}`}>{badge}</span>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Stat Row
   ═══════════════════════════════════════════════════════════ */
function StatRow({ label, value, color = ALIEN.cyan }: { label: string; value: string; color?: string }) {
  return (
    <div className="stat-row">
      <span className="stat-key">{label}</span>
      <span className="stat-val" style={{ color }}>{value}</span>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Tech Tag
   ═══════════════════════════════════════════════════════════ */
function Tag({ children, variant = "cyan" }: { children: React.ReactNode; variant?: string }) {
  return <span className={`tech-tag tag-${variant}`}>{children}</span>;
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Card (with 3D tilt on hover)
   ═══════════════════════════════════════════════════════════ */
function Card({ children, featured = false, className = "", borderColor }: {
  children: React.ReactNode; featured?: boolean; className?: string; borderColor?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);

  return (
    <div
      ref={ref}
      className={`alien-card ${featured ? "card-featured" : ""} ${className}`}
      style={borderColor ? { borderColor } : undefined}
    >
      {children}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Progress Bar
   ═══════════════════════════════════════════════════════════ */
function ProgressBar({ label, value, max, color, icon, accentColor }: {
  label: string; value: string; max: number; color: string; icon?: string; accentColor?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(([e]) => {
      if (e.isIntersecting) { setVisible(true); obs.disconnect(); }
    }, { threshold: 0.3 });
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  return (
    <div ref={ref} className="progress-container-v2">
      <div className="progress-header-v2">
        <div className="progress-label-left">
          {icon && <span className="progress-icon">{icon}</span>}
          <span className="progress-lang">{label}</span>
        </div>
        <div className="progress-label-right">
          <span className="progress-value-text">{value}</span>
        </div>
      </div>
      <div className="progress-track-v2">
        <div
          className="progress-fill-v2"
          style={{
            width: visible ? `${max}%` : "0%",
            background: color,
            boxShadow: `0 0 12px ${accentColor ?? "rgba(0,240,255,0.3)"}, 0 0 24px ${accentColor ?? "rgba(0,240,255,0.15)"}`,
          }}
        >
          <div className="progress-shine" />
        </div>
        <div className="progress-glow-dot" style={{
          left: visible ? `${max}%` : "0%",
          background: accentColor ?? ALIEN.cyan,
          boxShadow: `0 0 8px ${accentColor ?? ALIEN.cyan}, 0 0 16px ${accentColor ?? ALIEN.cyan}60`,
          opacity: visible ? 1 : 0,
        }} />
      </div>
      <div className="progress-pct-v2" style={{ color: accentColor ?? ALIEN.cyan }}>
        {max.toFixed(1)}%
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: LOC Donut Chart (SVG)
   ═══════════════════════════════════════════════════════════ */
function LOCDonut({ segments }: { segments: { label: string; pct: number; color: string; loc: number }[] }) {
  const r = 62;
  const circumference = 2 * Math.PI * r;
  let offset = 0;

  return (
    <div className="loc-donut-wrap">
      <svg width={180} height={180} viewBox="0 0 180 180">
        <defs>
          <filter id="donutGlow">
            <feGaussianBlur stdDeviation="4" result="blur"/>
            <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
          </filter>
        </defs>
        <circle cx={90} cy={90} r={r} fill="none" stroke="rgba(0,240,255,0.06)" strokeWidth={14} />
        {segments.map((seg, i) => {
          const dashLen = (seg.pct / 100) * circumference;
          const dashGap = circumference - dashLen;
          const currentOffset = offset;
          offset += dashLen;
          return (
            <circle
              key={seg.label}
              cx={90} cy={90} r={r} fill="none"
              stroke={seg.color}
              strokeWidth={14}
              strokeDasharray={`${dashLen} ${dashGap}`}
              strokeDashoffset={-currentOffset}
              strokeLinecap="round"
              transform="rotate(-90 90 90)"
              filter="url(#donutGlow)"
              className="donut-segment"
              style={{ animationDelay: `${i * 0.3}s` }}
            />
          );
        })}
        <text x={90} y={78} textAnchor="middle" fill="#e0e8f8" fontSize="22" fontWeight="900" fontFamily="var(--font-mono)">
          {`${Math.round(segments.reduce((s, seg) => s + seg.loc, 0) / 1000)}K`}
        </text>
        <text x={90} y={98} textAnchor="middle" fill="rgba(138,154,181,0.7)" fontSize="9" letterSpacing="0.15em">
          TOTAL LOC
        </text>
        <text x={90} y={115} textAnchor="middle" fill="rgba(0,240,255,0.5)" fontSize="8" letterSpacing="0.1em">
          3 LANGUAGES
        </text>
      </svg>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Gauge Ring (SVG)
   ═══════════════════════════════════════════════════════════ */
function GaugeRing({ value, max, label, color }: { value: number; max: number; label: string; color: string }) {
  const pct = Math.min(100, (value / max) * 100);
  const circumference = 2 * Math.PI * 42;
  const offset = circumference - (pct / 100) * circumference;

  return (
    <div style={{ textAlign: "center" }}>
      <svg width={100} height={100} viewBox="0 0 100 100">
        <circle cx={50} cy={50} r={42} fill="none" stroke="rgba(0,240,255,0.08)" strokeWidth={4} />
        <circle
          cx={50} cy={50} r={42} fill="none" stroke={color} strokeWidth={4}
          strokeDasharray={circumference} strokeDashoffset={offset}
          strokeLinecap="round" transform="rotate(-90 50 50)"
          style={{ transition: "stroke-dashoffset 1s ease", filter: `drop-shadow(0 0 8px ${color})` }}
        />
        <text x={50} y={46} textAnchor="middle" fill={color} fontSize={18} fontWeight={800}
          fontFamily="monospace">{Math.round(pct)}%</text>
        <text x={50} y={62} textAnchor="middle" fill={ALIEN.dim} fontSize={8}
          letterSpacing="0.1em">{label}</text>
      </svg>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Pipeline Visualizer (color-coded stages)
   ═══════════════════════════════════════════════════════════ */
const PIPE_COLORS: Record<string, { color: string; border: string }> = {
  "Source (.sl)": { color: ALIEN.green, border: "rgba(57,255,20,0.3)" },
  "Lexer": { color: ALIEN.green, border: "rgba(57,255,20,0.3)" },
  "Parser": { color: ALIEN.cyan, border: "rgba(0,240,255,0.3)" },
  "AST": { color: ALIEN.gold, border: "rgba(255,215,0,0.3)" },
  "TypeChecker": { color: ALIEN.orange, border: "rgba(255,106,0,0.3)" },
  "SSA IR": { color: ALIEN.magenta, border: "rgba(255,0,229,0.3)" },
  "Optimizer": { color: ALIEN.violet, border: "rgba(176,38,255,0.3)" },
  "Cranelift": { color: ALIEN.red, border: "rgba(255,0,60,0.3)" },
  "Native x86-64": { color: ALIEN.green, border: "rgba(57,255,20,0.4)" },
};

function PipelineViz({ stages }: { stages: PipelineStage[] }) {
  const [activeCount, setActiveCount] = useState(0);
  const sectionRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = sectionRef.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          stages.forEach((_, i) => {
            setTimeout(() => setActiveCount(c => Math.max(c, i + 1)), i * 200);
          });
        }
      },
      { threshold: 0.2 }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [stages]);

  return (
    <div ref={sectionRef} className="pipeline-wrap">
      <div className="pipeline-row">
        {stages.map((s, i) => {
          const cfg = PIPE_COLORS[s.name] || { color: ALIEN.cyan, border: "rgba(0,240,255,0.12)" };
          const isActive = i < activeCount;
          return (
            <div key={s.name} className="pipe-item">
              <div
                className={`pipe-stage ${isActive ? "pipe-active" : ""}`}
                style={{
                  color: cfg.color,
                  borderColor: isActive ? cfg.border : "rgba(0,240,255,0.08)",
                  boxShadow: isActive ? `0 0 20px ${cfg.color}40` : "none",
                }}
              >
                {s.name}
              </div>
              {i < stages.length - 1 && (
                <span className={`pipe-arrow ${isActive ? "pipe-active" : ""}`}>{"\u2192"}</span>
              )}
            </div>
          );
        })}
      </div>
      <div className="pipeline-ffi-note">
        C FFI Bridge (extern &quot;C&quot;) &harr; Python (ctypes.c_void_p) &harr; vitalis.py wrapper
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Scroll Reveal Wrapper
   ═══════════════════════════════════════════════════════════ */
function Reveal({ children, className = "", delay = 0 }: {
  children: React.ReactNode; className?: string; delay?: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) setVisible(true);
      },
      { threshold: 0.08, rootMargin: "0px 0px -60px 0px" }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      ref={ref}
      className={`scroll-reveal ${visible ? "revealed" : ""} ${className}`}
      style={{ transitionDelay: `${delay}s` }}
    >
      {children}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Floating Navigation Dots
   ═══════════════════════════════════════════════════════════ */
const NAV_SECTIONS = [
  { id: "overview", label: "Overview" },
  { id: "vitalis-oss", label: "\uD83E\uDDEC Vitalis OSS" },
  { id: "architecture", label: "Architecture" },
  { id: "compiler", label: "Compiler" },
  { id: "simd", label: "SIMD" },
  { id: "optimizer", label: "Optimizer" },
  { id: "evolution", label: "Evolution" },
  { id: "ai-swarm", label: "AI & Swarm" },
  { id: "backend", label: "Backend" },
  { id: "frontend-sec", label: "Frontend" },
  { id: "safety", label: "Safety" },
  { id: "consciousness", label: "Consciousness" },
  { id: "algorithms", label: "Algorithms" },
  { id: "infra", label: "Infrastructure" },
  { id: "inventory", label: "Inventory" },
];

function FloatingNav() {
  const [active, setActive] = useState(0);

  useEffect(() => {
    function onScroll() {
      const scrollY = window.pageYOffset + window.innerHeight / 3;
      let idx = 0;
      NAV_SECTIONS.forEach((s, i) => {
        const el = document.getElementById(s.id);
        if (el && el.offsetTop <= scrollY) idx = i;
      });
      setActive(idx);
    }
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <nav className="nav-float">
      {NAV_SECTIONS.map((s, i) => (
        <a
          key={s.id}
          className={`nav-dot ${i === active ? "nav-active" : ""}`}
          href={`#${s.id}`}
          title={s.label}
          onClick={(e) => {
            e.preventDefault();
            document.getElementById(s.id)?.scrollIntoView({ behavior: "smooth" });
          }}
        />
      ))}
    </nav>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Sticky Section Nav Bar
   ═══════════════════════════════════════════════════════════ */
function StickyNav() {
  const [active, setActive] = useState(0);
  const [visible, setVisible] = useState(false);
  const navRef = useRef<HTMLElement>(null);

  useEffect(() => {
    function onScroll() {
      const scrollY = window.pageYOffset;
      // Show after scrolling past hero (~320px)
      setVisible(scrollY > 320);

      const scrollPos = scrollY + 140;
      let idx = 0;
      NAV_SECTIONS.forEach((s, i) => {
        const el = document.getElementById(s.id);
        if (el && el.offsetTop <= scrollPos) idx = i;
      });
      setActive(idx);
    }
    window.addEventListener("scroll", onScroll, { passive: true });
    onScroll();
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  // Auto-scroll the active tab into view within the nav bar
  useEffect(() => {
    if (!navRef.current) return;
    const activeEl = navRef.current.querySelector(".snav-link.snav-active") as HTMLElement | null;
    if (activeEl) {
      activeEl.scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" });
    }
  }, [active]);

  return (
    <nav ref={navRef} className={`sticky-nav ${visible ? "sticky-nav-visible" : ""}`} aria-label="Section navigation">
      <div className="snav-inner">
        {NAV_SECTIONS.map((s, i) => (
          <a
            key={s.id}
            className={`snav-link ${i === active ? "snav-active" : ""}`}
            href={`#${s.id}`}
            onClick={(e) => {
              e.preventDefault();
              document.getElementById(s.id)?.scrollIntoView({ behavior: "smooth" });
            }}
          >
            {s.label}
          </a>
        ))}
      </div>
    </nav>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Circuit Traces — STRIPPED for performance
   ═══════════════════════════════════════════════════════════ */
function CircuitTraces() { return null; }

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Parallax Background Hook — STRIPPED (was causing twitching)
   ═══════════════════════════════════════════════════════════ */
function useParallax() {
  // Intentionally empty — removed to prevent competing transforms on star-field
}

/* ═══════ CINEMATIC LOADING SCREEN ═══════ */
function CinematicLoader() {
  const [progress, setProgress] = useState(0);
  const [done, setDone] = useState(false);
  const [hidden, setHidden] = useState(false);

  useEffect(() => {
    const start = Date.now();
    let alive = true;
    const tick = () => {
      if (!alive) return;
      const elapsed = Date.now() - start;
      const p = Math.min(100, (elapsed / 2000) * 100);
      setProgress(p);
      if (p >= 100) {
        setDone(true);
        setTimeout(() => setHidden(true), 900);
      } else {
        requestAnimationFrame(tick);
      }
    };
    requestAnimationFrame(tick);
    return () => { alive = false; };
  }, []);

  if (hidden) return null;
  return (
    <div className={`cine-loader ${done ? "cine-done" : ""}`}>
      <div className="cine-inner">
        <div className="cine-symbol">{"\u221E"}</div>
        <div className="cine-title">INFINITY</div>
        <div className="cine-bar-track"><div className="cine-bar-fill" style={{ width: `${progress}%` }} /></div>
        <div className="cine-pct">{Math.floor(progress)}%</div>
        <div className="cine-label">INITIALIZING SYSTEMS</div>
      </div>
      <div className="cine-scanlines" />
      <div className="cine-grid" />
    </div>
  );
}

/* ═══════ CUSTOM ANIMATED CURSOR — STRIPPED for performance ═══════ */
function CyberCursor() { return null; }

/* ═══════ FILM GRAIN OVERLAY — STRIPPED for performance ═══════ */
function FilmGrain() { return null; }

/* ═══════ AURORA FIELD (CSS-only atmospheric ribbons) — KEPT ═══════ */
function AuroraField() {
  return <div className="aurora-field" />;
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Quantum Neural Field — STRIPPED for performance
   (Was the heaviest component: 30 nodes, vortex, plasma tendrils, rAF canvas)
   ═══════════════════════════════════════════════════════════ */
function HolographicWaves() { return null; }

/* ═══════════════════════════════════════════════════════════
   MAIN PAGE
   ═══════════════════════════════════════════════════════════ */
export default function TechStackPage() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  useParallax();

  useEffect(() => {
    let mounted = true;
    async function load() {
      const data = await fetchStats();
      if (mounted && data) {
        setStats(data);
        setLastUpdate(new Date());
        setLoading(false);
      } else if (mounted) {
        setLoading(false);
      }
    }
    load();
    const interval = setInterval(load, 8000);
    return () => { mounted = false; clearInterval(interval); };
  }, []);

  const evoRate = useMemo(() => {
    if (!stats || !stats.uptime_seconds || stats.uptime_seconds < 60) return "0";
    return (stats.evolution.successes / (stats.uptime_seconds / 3600)).toFixed(1);
  }, [stats]);

  return (
    <div className="alien-page">
      {/* JSON-LD Structured Data for Google Rich Results */}
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify({
            "@context": "https://schema.org",
            "@type": "WebApplication",
            name: "Infinity — Self-Evolving AI System",
            url: "https://infinitytechstack.uk",
            description:
              "Live interactive tech stack map for an autonomous self-evolving AI system. Built with Rust (Vitalis compiler), Python (FastAPI), and TypeScript (Next.js).",
            applicationCategory: "DeveloperApplication",
            operatingSystem: "Web",
            offers: { "@type": "Offer", price: "0", priceCurrency: "GBP" },
            author: {
              "@type": "Person",
              name: "Bart Chmiel",
              url: "https://www.linkedin.com/in/modern-workplace-tech365/",
            },
            sameAs: [
              "https://www.linkedin.com/in/modern-workplace-tech365/",
            ],
            softwareVersion: stats?.version ?? "1.0.0",
            featureList: [
              "Live system monitoring",
              "Custom Rust compiler (Vitalis) with Cranelift JIT",
              "Self-evolving code engine",
              "Multi-agent swarm intelligence",
              "142 REST API endpoints",
              "72 AI modules",
              "Interactive architecture visualization",
            ],
          }),
        }}
      />

      {/* Cinematic preloader */}
      <CinematicLoader />

      {/* Background layers (stripped heavy canvas/rAF components for smooth scrolling) */}
      <div className="star-field" />
      <div className="nebula-overlay" />
      <AuroraField />
      <div className="cyber-grid"><div className="cyber-grid-inner" /></div>
      <div className="holo-scan" />
      <ScrollProgress />
      <div className="holo-shapes">
        <div className="holo-shape" />
        <div className="holo-shape" />
        <div className="holo-shape" />
        <div className="holo-shape" />
        <div className="holo-shape" />
      </div>
      <FloatingNav />

      {/* ═══════════ HERO ═══════════ */}
      <header className="hero">
        <div className="hero-logo" aria-label="Infinity — Autonomous AI System">
          <svg className="hero-infinity-svg" viewBox="0 0 400 220" xmlns="http://www.w3.org/2000/svg">
            <defs>
              {/* Holographic animated gradient */}
              <linearGradient id="igPrimary" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#059669">
                  <animate attributeName="stop-color" values="#059669;#00f0ff;#ff00e5;#059669" dur="8s" repeatCount="indefinite" />
                </stop>
                <stop offset="25%" stopColor="#10b981">
                  <animate attributeName="stop-color" values="#10b981;#34d399;#b026ff;#10b981" dur="8s" repeatCount="indefinite" />
                </stop>
                <stop offset="50%" stopColor="#34d399">
                  <animate attributeName="stop-color" values="#34d399;#ff00e5;#00f0ff;#34d399" dur="8s" repeatCount="indefinite" />
                </stop>
                <stop offset="75%" stopColor="#00f0ff">
                  <animate attributeName="stop-color" values="#00f0ff;#39ff14;#ffd700;#00f0ff" dur="8s" repeatCount="indefinite" />
                </stop>
                <stop offset="100%" stopColor="#059669">
                  <animate attributeName="stop-color" values="#059669;#00f0ff;#ff00e5;#059669" dur="8s" repeatCount="indefinite" />
                </stop>
              </linearGradient>
              {/* Chromatic aberration red channel */}
              <linearGradient id="igChromaR" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#ff003c" stopOpacity="0.15" />
                <stop offset="100%" stopColor="#ff003c" stopOpacity="0.05" />
              </linearGradient>
              {/* Chromatic aberration blue channel */}
              <linearGradient id="igChromaB" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#0066ff" stopOpacity="0.05" />
                <stop offset="100%" stopColor="#0066ff" stopOpacity="0.15" />
              </linearGradient>
              {/* Metallic top-light with sweep */}
              <linearGradient id="igSheen" x1="0%" y1="0%" x2="0%" y2="100%">
                <stop offset="0%" stopColor="rgba(255,255,255,0.55)" />
                <stop offset="30%" stopColor="rgba(255,255,255,0.08)" />
                <stop offset="60%" stopColor="rgba(255,255,255,0)" />
                <stop offset="100%" stopColor="rgba(255,255,255,0.12)" />
              </linearGradient>
              {/* Multi-layer glow */}
              <filter id="igGlow" x="-40%" y="-40%" width="180%" height="180%">
                <feGaussianBlur in="SourceGraphic" stdDeviation="3" result="b1" />
                <feGaussianBlur in="SourceGraphic" stdDeviation="8" result="b2" />
                <feGaussianBlur in="SourceGraphic" stdDeviation="16" result="b3" />
                <feMerge>
                  <feMergeNode in="b3" />
                  <feMergeNode in="b2" />
                  <feMergeNode in="b1" />
                  <feMergeNode in="SourceGraphic" />
                </feMerge>
              </filter>
              {/* Soft outer haze */}
              <filter id="igHaze" x="-60%" y="-60%" width="220%" height="220%">
                <feGaussianBlur in="SourceGraphic" stdDeviation="22" />
              </filter>
              {/* Center energy glow */}
              <radialGradient id="igCenterPulse" cx="50%" cy="50%" r="50%">
                <stop offset="0%" stopColor="#00f0ff" stopOpacity="0.4">
                  <animate attributeName="stop-color" values="#00f0ff;#ff00e5;#39ff14;#00f0ff" dur="6s" repeatCount="indefinite" />
                </stop>
                <stop offset="100%" stopColor="#00f0ff" stopOpacity="0" />
              </radialGradient>
              {/* Reflection fade */}
              <linearGradient id="igReflFade" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="white" stopOpacity="0.2" />
                <stop offset="100%" stopColor="white" stopOpacity="0" />
              </linearGradient>
              <mask id="igReflMask">
                <rect x="0" y="155" width="400" height="65" fill="url(#igReflFade)" />
              </mask>
            </defs>

            {/* Ambient nebula */}
            <ellipse cx="200" cy="100" rx="140" ry="60" fill="rgba(16,185,129,0.06)" className="hero-ambient" />

            {/* Chromatic aberration - red offset (left) */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="url(#igChromaR)" strokeWidth="8"
              strokeLinecap="round" strokeLinejoin="round"
              transform="translate(-2.5, 0)" filter="url(#igGlow)" className="hero-chroma-r"
            />
            {/* Chromatic aberration - blue offset (right) */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="url(#igChromaB)" strokeWidth="8"
              strokeLinecap="round" strokeLinejoin="round"
              transform="translate(2.5, 0)" filter="url(#igGlow)" className="hero-chroma-b"
            />

            {/* Layer 1: Wide soft haze */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="url(#igPrimary)" strokeWidth="18"
              strokeLinecap="round" strokeLinejoin="round"
              filter="url(#igHaze)" opacity="0.4" className="hero-inf-haze"
            />

            {/* Layer 2: Primary glow stroke */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="url(#igPrimary)" strokeWidth="10"
              strokeLinecap="round" strokeLinejoin="round"
              filter="url(#igGlow)" className="hero-inf-path"
            />

            {/* Layer 3: Bright core */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="url(#igPrimary)" strokeWidth="4.5"
              strokeLinecap="round" strokeLinejoin="round"
              className="hero-inf-core"
            />

            {/* Layer 4: White-hot inner edge */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="rgba(255,255,255,0.4)" strokeWidth="1.5"
              strokeLinecap="round" strokeLinejoin="round"
            />

            {/* Metallic sheen overlay */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="url(#igSheen)" strokeWidth="6"
              strokeLinecap="round" strokeLinejoin="round"
              opacity="0.5"
            />

            {/* Rotating orbital ring */}
            <ellipse cx="200" cy="100" rx="120" ry="25" fill="none"
              stroke="url(#igPrimary)" strokeWidth="0.6" opacity="0.2"
              strokeDasharray="6 8" className="hero-orbital">
              <animateTransform attributeName="transform" type="rotate"
                values="0 200 100;360 200 100" dur="12s" repeatCount="indefinite" />
            </ellipse>
            {/* Second orbital ring - counter-rotate */}
            <ellipse cx="200" cy="100" rx="135" ry="18" fill="none"
              stroke="url(#igPrimary)" strokeWidth="0.4" opacity="0.12"
              strokeDasharray="4 10" className="hero-orbital-2">
              <animateTransform attributeName="transform" type="rotate"
                values="360 200 100;0 200 100" dur="18s" repeatCount="indefinite" />
            </ellipse>

            {/* Center crossover energy pulse */}
            <circle cx="200" cy="100" r="12" fill="url(#igCenterPulse)" className="hero-center-pulse">
              <animate attributeName="r" values="8;15;8" dur="3s" repeatCount="indefinite" />
              <animate attributeName="opacity" values="0.6;0.3;0.6" dur="3s" repeatCount="indefinite" />
            </circle>

            {/* Energy particle 1 - bright white */}
            <circle r="3.5" fill="white" className="hero-particle">
              <animate attributeName="opacity" values="0.9;0.4;0.9" dur="4s" repeatCount="indefinite" />
              <animateMotion
                dur="5s" repeatCount="indefinite"
                path="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              />
            </circle>
            {/* Particle glow trail */}
            <circle r="8" fill="#10b981" opacity="0.3">
              <animateMotion
                dur="5s" repeatCount="indefinite"
                path="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              />
            </circle>

            {/* Energy particle 2 - cyan accent, offset */}
            <circle r="2" fill="#00f0ff" className="hero-particle">
              <animate attributeName="opacity" values="0.7;0.2;0.7" dur="5s" repeatCount="indefinite" />
              <animateMotion
                dur="5s" repeatCount="indefinite" begin="-2.5s"
                path="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              />
            </circle>

            {/* Energy particle 3 - magenta, slower */}
            <circle r="2.5" fill="#ff00e5" className="hero-particle">
              <animate attributeName="opacity" values="0.6;0.15;0.6" dur="6s" repeatCount="indefinite" />
              <animateMotion
                dur="7s" repeatCount="indefinite" begin="-1s"
                path="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              />
            </circle>
            {/* Magenta particle trail */}
            <circle r="6" fill="#ff00e5" opacity="0.15">
              <animateMotion
                dur="7s" repeatCount="indefinite" begin="-1s"
                path="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              />
            </circle>

            {/* Reflection */}
            <g mask="url(#igReflMask)" transform="translate(0, 130) scale(1, -0.25)">
              <path
                d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
                fill="none" stroke="url(#igPrimary)" strokeWidth="5"
                strokeLinecap="round" strokeLinejoin="round" opacity="0.3"
                filter="url(#igGlow)"
              />
            </g>

            {/* Ground glow line - enhanced */}
            <line x1="50" y1="185" x2="350" y2="185" stroke="url(#igPrimary)" strokeWidth="0.8" opacity="0.2">
              <animate attributeName="opacity" values="0.15;0.25;0.15" dur="4s" repeatCount="indefinite" />
            </line>
          </svg>
        </div>
        <h1 className="hero-h1">
          <GlitchText text="I N F I N I T Y" />
        </h1>
        <p className="hero-subtitle">Autonomous &middot; Self-Evolving &middot; AI System</p>
        <p className="hero-tagline">
          {"// compiled with Vitalis \u2022 enforced by Asimov\u2019s Laws \u2022 powered by swarm intelligence"}
        </p>
        <div className="hero-status">
          <span className={`status-dot ${stats?.status === "running" ? "alive" : "dead"}`} />
          <span className="status-label">
            {loading ? "CONNECTING\u2026" : stats?.status === "running" ? "SYSTEM ONLINE" : stats?.error ? "OFFLINE" : "STANDBY"}
          </span>
          {lastUpdate && (
            <span className="last-update">Updated {lastUpdate.toLocaleTimeString()}</span>
          )}
        </div>
      </header>

      {/* ═══════════ STICKY SECTION NAV ═══════════ */}
      <StickyNav />

      {/* ═══════════ LIVE METRICS BAR ═══════════ */}
      {stats && (
        <div className="container">
          <Reveal>
            <div className="metrics-bar">
              {[
                { value: formatNum(stats.total_loc), label: "Lines of Code", color: "" },
                { value: formatNum(stats.modules_loaded), label: "AI Modules", color: "green" },
                { value: String(stats.evolution.total_attempts), label: "Evolution Attempts", color: "gold" },
                { value: String(stats.evolution.successes), label: "Evolutions", color: "green" },
                { value: formatNum(stats.memory_count), label: "Memories", color: "magenta" },
                { value: String(stats.swarm?.active_agents ?? 4), label: "Swarm Agents", color: "" },
                { value: String(stats.active_goals), label: "Active Goals", color: "gold" },
                { value: formatUptime(stats.uptime_seconds), label: "Uptime", color: "green" },
              ].map((m, i) => (
                <div key={i} className="metric">
                  <div className={`metric-value ${m.color}`}>{m.value}</div>
                  <div className="metric-label">{m.label}</div>
                </div>
              ))}
            </div>
          </Reveal>
        </div>
      )}

      {/* ═══════════ SYSTEM OVERVIEW ═══════════ */}
      <EnergyConnector />
      <div className="container" id="overview">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83C\uDF0C"} title="System Overview" badge={"\u25CF ONLINE"} badgeType="active" />

            <div className="grid grid-3">
              <Reveal delay={0}>
                <Card>
                  <div className="card-header-row">
                    <span className="card-icon">{"\uD83E\uDD80"}</span>
                    <div>
                      <div className="card-title">Rust &mdash; Vitalis Compiler</div>
                      <div className="card-subtitle">
                        {stats?.tech_stack?.rust
                          ? `${formatNum(stats.tech_stack.rust.loc)} LOC \u00B7 ${stats.tech_stack.rust.files} Files \u00B7 ${stats.tech_stack.rust.tests ?? 748} Tests`
                          : "35,632 LOC \u00B7 47 Modules \u00B7 748 Tests"}
                      </div>
                    </div>
                  </div>
                  <div className="card-body">
                    Custom compiled language with <strong>Cranelift JIT</strong>, SIMD vectorization,
                    predictive optimizer, quantum-inspired evolution (annealing, UCB, Pareto, CMA-ES),
                    consciousness substrate, kernel sentinel, 98 stdlib builtins, and 44 native hotpath ops
                    including softmax, cross-entropy, batch sigmoid/ReLU, cosine similarity, entropy, and EMA.
                    Compiles <code>.sl</code> &rarr; native x86-64 via SSA IR.
                  </div>
                  <div className="tags-row">
                    <Tag variant="rust">Rust 2024</Tag>
                    <Tag variant="rust">Cranelift 0.116</Tag>
                    <Tag variant="rust">Quantum Evolution</Tag>
                    <Tag variant="rust">cdylib + rlib</Tag>
                    <Tag variant="hw">AVX2 SIMD</Tag>
                  </div>
                </Card>
              </Reveal>

              <Reveal delay={0.1}>
                <Card>
                  <div className="card-header-row">
                    <span className="card-icon">{"\uD83D\uDC0D"}</span>
                    <div>
                      <div className="card-title">Python &mdash; AI Backend</div>
                      <div className="card-subtitle">
                        {stats?.tech_stack?.python
                          ? `${formatNum(stats.tech_stack.python.loc)} LOC \u00B7 ${stats.tech_stack.python.files} Files \u00B7 ${stats.tech_stack.python.modules ?? 94} Modules`
                          : "99,349 LOC \u00B7 352 Files \u00B7 94 Modules"}
                      </div>
                    </div>
                  </div>
                  <div className="card-body">
                    <strong>FastAPI</strong> server on port 8002 with {stats?.modules_loaded ?? 94} cortex modules covering
                    inference, memory, reasoning, code analysis, swarm consensus, evolution
                    orchestration, voice processing, and <strong>ChromaDB</strong> vector store.
                  </div>
                  <div className="tags-row">
                    <Tag variant="python">Python 3.12</Tag>
                    <Tag variant="python">FastAPI</Tag>
                    <Tag variant="python">ChromaDB</Tag>
                    <Tag variant="python">Pydantic</Tag>
                  </div>
                </Card>
              </Reveal>

              <Reveal delay={0.2}>
                <Card>
                  <div className="card-header-row">
                    <span className="card-icon">{"\u269B\uFE0F"}</span>
                    <div>
                      <div className="card-title">TypeScript &mdash; Cyberpunk Frontend</div>
                      <div className="card-subtitle">
                        {stats?.tech_stack?.typescript
                          ? `${formatNum(stats.tech_stack.typescript.loc)} LOC \u00B7 ${stats.tech_stack.typescript.files} Files \u00B7 ${stats.tech_stack.typescript.routes ?? 26} Routes`
                          : "18,894 LOC \u00B7 54 Files \u00B7 26 Routes"}
                      </div>
                    </div>
                  </div>
                  <div className="card-body">
                    <strong>Next.js</strong> on port 3002 with GPU-accelerated <strong>WebGL shaders</strong>,
                    holographic UI primitives, cyberpunk HUD overlays, neural particle backgrounds,
                    and real-time event streaming via SSE.
                  </div>
                  <div className="tags-row">
                    <Tag variant="ts">Next.js 15</Tag>
                    <Tag variant="ts">TypeScript</Tag>
                    <Tag variant="ts">Tailwind CSS</Tag>
                    <Tag variant="ts">WebGL</Tag>
                  </div>
                </Card>
              </Reveal>
            </div>

            {/* LOC Distribution — Premium Breakdown */}
            <div className="loc-distribution-v2">
              <LOCDonut segments={[
                { label: "Python", pct: 64.6, color: ALIEN.green, loc: stats?.tech_stack?.python?.loc ?? 99349 },
                { label: "Rust", pct: 23.2, color: ALIEN.orange, loc: stats?.tech_stack?.rust?.loc ?? 35632 },
                { label: "TypeScript", pct: 12.3, color: ALIEN.cyan, loc: stats?.tech_stack?.typescript?.loc ?? 18894 },
              ]} />
              <div className="loc-bars-v2">
                <ProgressBar
                  label="Python" icon="🐍"
                  value={`${formatNum(stats?.tech_stack?.python?.loc ?? 99349)} LOC`}
                  max={64.6}
                  color={`linear-gradient(90deg, ${ALIEN.green}, rgba(57,255,20,0.4))`}
                  accentColor={ALIEN.green}
                />
                <ProgressBar
                  label="Rust" icon="🦀"
                  value={`${formatNum(stats?.tech_stack?.rust?.loc ?? 35632)} LOC`}
                  max={23.2}
                  color={`linear-gradient(90deg, ${ALIEN.orange}, rgba(255,106,0,0.4))`}
                  accentColor={ALIEN.orange}
                />
                <ProgressBar
                  label="TypeScript" icon="⚛️"
                  value={`${formatNum(stats?.tech_stack?.typescript?.loc ?? 18894)} LOC`}
                  max={12.3}
                  color={`linear-gradient(90deg, ${ALIEN.cyan}, rgba(0,240,255,0.4))`}
                  accentColor={ALIEN.cyan}
                />
              </div>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ VITALIS FLAGSHIP PANEL ═══════════ */}
      <EnergyConnector />
      <div className="container" id="vitalis-oss">
        <Reveal>
          <section className="vitalis-flagship">
            <div className="vitalis-flagship-glow" />
            <div className="vitalis-flagship-grid">
              {/* LEFT — Hero Info */}
              <div className="vitalis-flagship-hero">
                <div className="vitalis-flagship-badge">🧬 OPEN SOURCE FLAGSHIP</div>
                <h2 className="vitalis-flagship-title">
                  Vitalis<span className="vitalis-flagship-version">v20</span>
                </h2>
                <p className="vitalis-flagship-tagline">
                  A self-evolving, JIT-compiled programming language built from scratch in Rust.
                  Powering Infinity&rsquo;s autonomous AI with native-speed hotpath operations,
                  quantum-inspired evolution strategies, and real-time code mutation.
                </p>
                <div className="vitalis-flagship-stats-row">
                  <div className="vitalis-flagship-stat">
                    <span className="vitalis-flagship-stat-value">35,632</span>
                    <span className="vitalis-flagship-stat-label">Lines of Rust</span>
                  </div>
                  <div className="vitalis-flagship-stat">
                    <span className="vitalis-flagship-stat-value">47</span>
                    <span className="vitalis-flagship-stat-label">Modules</span>
                  </div>
                  <div className="vitalis-flagship-stat">
                    <span className="vitalis-flagship-stat-value">652</span>
                    <span className="vitalis-flagship-stat-label">FFI Exports</span>
                  </div>
                  <div className="vitalis-flagship-stat">
                    <span className="vitalis-flagship-stat-value">748</span>
                    <span className="vitalis-flagship-stat-label">Tests</span>
                  </div>
                </div>
                <div className="vitalis-flagship-buttons">
                  <a
                    href="https://github.com/ModernOps888/vitalis"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="vitalis-flagship-btn-primary"
                  >
                    <svg width="20" height="20" viewBox="0 0 16 16" fill="currentColor" style={{marginRight:'0.45rem'}}>
                      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
                    </svg>
                    View on GitHub
                  </a>
                  <a href="#compiler" className="vitalis-flagship-btn-secondary">
                    Compiler Deep Dive &darr;
                  </a>
                </div>
              </div>

              {/* RIGHT — Feature Cards */}
              <div className="vitalis-flagship-features">
                <div className="vitalis-feature-card">
                  <div className="vitalis-feature-icon">⚡</div>
                  <div className="vitalis-feature-title">Cranelift JIT</div>
                  <div className="vitalis-feature-desc">Compiles .sl → native x86-64 via SSA IR in milliseconds</div>
                </div>
                <div className="vitalis-feature-card">
                  <div className="vitalis-feature-icon">🧬</div>
                  <div className="vitalis-feature-title">Self-Evolution</div>
                  <div className="vitalis-feature-desc">Quantum UCB, CMA-ES, Pareto, Thompson sampling strategies</div>
                </div>
                <div className="vitalis-feature-card">
                  <div className="vitalis-feature-icon">🔐</div>
                  <div className="vitalis-feature-title">Kernel Sentinel</div>
                  <div className="vitalis-feature-desc">Tamper-proof integrity protection with SHA-256 hashing</div>
                </div>
                <div className="vitalis-feature-card">
                  <div className="vitalis-feature-icon">🚀</div>
                  <div className="vitalis-feature-title">44 Hotpath Ops</div>
                  <div className="vitalis-feature-desc">Native Rust: softmax, cross-entropy, cosine similarity, EMA &amp; more</div>
                </div>
                <div className="vitalis-feature-card">
                  <div className="vitalis-feature-icon">🧠</div>
                  <div className="vitalis-feature-title">Consciousness</div>
                  <div className="vitalis-feature-desc">Self-awareness substrate with reflection &amp; introspection</div>
                </div>
                <div className="vitalis-feature-card">
                  <div className="vitalis-feature-icon">📡</div>
                  <div className="vitalis-feature-title">652 FFI Exports</div>
                  <div className="vitalis-feature-desc">C ABI bridge → Python ctypes → seamless Infinity integration</div>
                </div>
              </div>
            </div>

            {/* Bottom bar — compiler pipeline mini-viz */}
            <div className="vitalis-flagship-pipeline">
              <span className="pipeline-step">.sl Source</span>
              <span className="pipeline-arrow">→</span>
              <span className="pipeline-step">Lexer</span>
              <span className="pipeline-arrow">→</span>
              <span className="pipeline-step">Parser</span>
              <span className="pipeline-arrow">→</span>
              <span className="pipeline-step">AST</span>
              <span className="pipeline-arrow">→</span>
              <span className="pipeline-step">Type Check</span>
              <span className="pipeline-arrow">→</span>
              <span className="pipeline-step">IR (SSA)</span>
              <span className="pipeline-arrow">→</span>
              <span className="pipeline-step vitalis-pipeline-jit">Cranelift JIT</span>
              <span className="pipeline-arrow">→</span>
              <span className="pipeline-step vitalis-pipeline-native">Native x86-64</span>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ ARCHITECTURE DIAGRAM ═══════════ */}
      <EnergyConnector />
      <div className="container" id="architecture">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83C\uDFD7\uFE0F"} title="System Architecture" badge="MULTI-TIER" badgeType="native" />
            {/* ── Interactive Architecture Diagram ── */}
            <div className="arch-visual">
              {/* Outer frame */}
              <div className="arch-frame">
                <div className="arch-frame-title">
                  <span className="arch-glyph">{"\u2588\u2593\u2592\u2591"}</span>
                  <span>I N F I N I T Y &nbsp;&nbsp; S T A C K</span>
                  <span className="arch-glyph">{"\u2591\u2592\u2593\u2588"}</span>
                </div>

                {/* TIER 1: Frontend */}
                <div className="arch-tier">
                  <div className="arch-box arch-box--cyan">
                    <div className="arch-box-header">
                      <span className="arch-box-icon">{"\u2B22"}</span>
                      <span className="arch-box-label">FRONTEND</span>
                      <span className="arch-box-port">:3002</span>
                    </div>
                    <div className="arch-box-chips">
                      <span className="arch-chip arch-chip--cyan">WebGL Shaders</span>
                      <span className="arch-chip arch-chip--cyan">CyberpunkHUD</span>
                      <span className="arch-chip arch-chip--cyan">HoloUI</span>
                      <span className="arch-chip arch-chip--cyan">Neural FX</span>
                      <span className="arch-chip arch-chip--dim">26 Routes</span>
                      <span className="arch-chip arch-chip--dim">Tailwind CSS</span>
                      <span className="arch-chip arch-chip--dim">TypeScript Strict</span>
                    </div>
                    <div className="arch-box-tech">Next.js 15 &middot; React 19</div>
                  </div>
                </div>

                {/* Connector */}
                <div className="arch-connector">
                  <div className="arch-line arch-line--cyan"></div>
                  <div className="arch-connector-label arch-connector-label--cyan">REST + SSE</div>
                  <div className="arch-line arch-line--cyan"></div>
                </div>

                {/* TIER 2: API */}
                <div className="arch-tier">
                  <div className="arch-box arch-box--green">
                    <div className="arch-box-header">
                      <span className="arch-box-icon">{"\u2B22"}</span>
                      <span className="arch-box-label">API GATEWAY</span>
                      <span className="arch-box-port">:8002</span>
                    </div>
                    <div className="arch-box-chips">
                      <span className="arch-chip arch-chip--green">/health</span>
                      <span className="arch-chip arch-chip--green">/status</span>
                      <span className="arch-chip arch-chip--green">/api/chat</span>
                      <span className="arch-chip arch-chip--green">/evolution</span>
                      <span className="arch-chip arch-chip--green">/api/roadmap</span>
                    </div>
                    <div className="arch-box-tech">FastAPI &middot; Uvicorn &middot; Python 3.12</div>
                  </div>
                </div>

                {/* Connector */}
                <div className="arch-connector">
                  <div className="arch-line arch-line--green"></div>
                  <div className="arch-connector-label arch-connector-label--green">Module Loader</div>
                  <div className="arch-line arch-line--green"></div>
                </div>

                {/* TIER 3: Kernel */}
                <div className="arch-tier">
                  <div className="arch-box arch-box--magenta">
                    <div className="arch-box-header">
                      <span className="arch-box-icon">{"\u2B22"}</span>
                      <span className="arch-box-label">KERNEL</span>
                      <span className="arch-box-port">core</span>
                    </div>
                    <div className="arch-box-chips">
                      <span className="arch-chip arch-chip--magenta">Guardian</span>
                      <span className="arch-chip arch-chip--magenta">Sandbox</span>
                      <span className="arch-chip arch-chip--magenta">Evolution</span>
                      <span className="arch-chip arch-chip--magenta">Watchdog</span>
                      <span className="arch-chip arch-chip--dim">Algorithm Forge</span>
                      <span className="arch-chip arch-chip--dim">CoEvolution</span>
                      <span className="arch-chip arch-chip--dim">Config</span>
                    </div>
                    <div className="arch-box-tech">Microkernel &middot; Hot-Swap &middot; Asimov Enforced</div>
                  </div>
                </div>

                {/* Triple connector to subsystems */}
                <div className="arch-connector arch-connector--triple">
                  <div className="arch-line arch-line--gold"></div>
                  <div className="arch-line arch-line--violet"></div>
                  <div className="arch-line arch-line--orange"></div>
                </div>

                {/* TIER 4: Subsystems — 3-column grid */}
                <div className="arch-tier arch-tier--triple">
                  {/* Cortex AI */}
                  <div className="arch-box arch-box--gold">
                    <div className="arch-box-header">
                      <span className="arch-box-icon">{"\uD83E\uDDE0"}</span>
                      <span className="arch-box-label">CORTEX AI</span>
                    </div>
                    <div className="arch-box-chips">
                      <span className="arch-chip arch-chip--gold">72 Modules</span>
                      <span className="arch-chip arch-chip--gold">Swarm (4x)</span>
                      <span className="arch-chip arch-chip--gold">Reasoning</span>
                      <span className="arch-chip arch-chip--gold">Inference</span>
                    </div>
                    <div className="arch-box-stat">
                      <span className="arch-stat-dot arch-stat-dot--gold"></span>
                      Active
                    </div>
                  </div>

                  {/* Memory */}
                  <div className="arch-box arch-box--violet">
                    <div className="arch-box-header">
                      <span className="arch-box-icon">{"\uD83D\uDCBE"}</span>
                      <span className="arch-box-label">MEMORY</span>
                    </div>
                    <div className="arch-box-chips">
                      <span className="arch-chip arch-chip--violet">ChromaDB</span>
                      <span className="arch-chip arch-chip--violet">FAISS</span>
                      <span className="arch-chip arch-chip--violet">Episodic</span>
                      <span className="arch-chip arch-chip--violet">Engrams</span>
                    </div>
                    <div className="arch-box-stat">
                      <span className="arch-stat-dot arch-stat-dot--violet"></span>
                      Vector Store
                    </div>
                  </div>

                  {/* Vitalis */}
                  <div className="arch-box arch-box--orange">
                    <div className="arch-box-header">
                      <span className="arch-box-icon">{"\uD83E\uDD80"}</span>
                      <span className="arch-box-label">VITALIS</span>
                      <span className="arch-box-port">Rust</span>
                    </div>
                    <div className="arch-box-chips">
                      <span className="arch-chip arch-chip--orange">Cranelift JIT</span>
                      <span className="arch-chip arch-chip--orange">SIMD F64{"\u00D7"}4</span>
                      <span className="arch-chip arch-chip--orange">Optimizer</span>
                      <span className="arch-chip arch-chip--orange">Evolution</span>
                      <span className="arch-chip arch-chip--dim">98 stdlib</span>
                      <span className="arch-chip arch-chip--dim">44 hotpath</span>
                      <span className="arch-chip arch-chip--dim">14 algo libs</span>
                    </div>
                    <div className="arch-box-stat">
                      <span className="arch-stat-dot arch-stat-dot--orange"></span>
                      748 Tests
                    </div>
                  </div>
                </div>

                {/* Connector: C FFI */}
                <div className="arch-connector">
                  <div className="arch-line arch-line--orange"></div>
                  <div className="arch-connector-label arch-connector-label--orange">C FFI (ctypes) &middot; vitalis.py</div>
                  <div className="arch-line arch-line--orange"></div>
                </div>

                {/* TIER 5: Safety Layer */}
                <div className="arch-tier">
                  <div className="arch-box arch-box--red">
                    <div className="arch-box-header">
                      <span className="arch-box-icon">{"\uD83D\uDEE1\uFE0F"}</span>
                      <span className="arch-box-label">SAFETY LAYER</span>
                      <span className="arch-box-port">armed</span>
                    </div>
                    <div className="arch-box-laws">
                      <div className="arch-law"><span className="arch-law-num">I</span>No harm to humans</div>
                      <div className="arch-law"><span className="arch-law-num">II</span>Obey human orders</div>
                      <div className="arch-law"><span className="arch-law-num">III</span>Self-preservation</div>
                    </div>
                    <div className="arch-box-chips">
                      <span className="arch-chip arch-chip--red">Guardian</span>
                      <span className="arch-chip arch-chip--red">Sandbox</span>
                      <span className="arch-chip arch-chip--red">SHA-256 Sentinel</span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Scan line overlay */}
              <div className="arch-scanline"></div>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ COMPILER PIPELINE ═══════════ */}
      <EnergyConnector />
      <div className="container" id="compiler">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\u26A1"} title="Vitalis Compiler \u2014 Custom Language" badge="CRANELIFT JIT" badgeType="jit" />
            <p className="section-desc">
              Purpose-built compiled language for autonomous AI code evolution.
              Source <code>.sl</code> programs are lexed, parsed, type-checked, lowered to SSA IR,
              optimized, and JIT-compiled to native x86-64 via <strong>Cranelift 0.116</strong>.
            </p>
            <h3 className="subsection-title">Compilation Pipeline</h3>
            {stats?.compiler_pipeline && stats.compiler_pipeline.length > 0 ? (
              <PipelineViz stages={stats.compiler_pipeline} />
            ) : (
              <div className="pipeline-wrap">
                <div className="pipeline-row">
                  {["SOURCE .sl", "LEXER", "PARSER", "TYPE CHECK", "SSA IR", "OPTIMIZE", "CRANELIFT JIT", "NATIVE x86-64"].map((stage, i, arr) => (
                    <div key={stage} className="pipe-item">
                      <div className="pipe-stage pipe-active" style={{ borderColor: `rgba(0,240,255,${0.15 + i * 0.05})`, color: i === arr.length - 1 ? ALIEN.green : ALIEN.cyan }}>
                        {stage}
                      </div>
                      {i < arr.length - 1 && <span className="pipe-arrow pipe-active">{"\u2192"}</span>}
                    </div>
                  ))}
                </div>
                <div className="pipeline-ffi-note">{"\u2193"} C FFI (ctypes) {"\u2193"} Python interop via vitalis.py</div>
              </div>
            )}

                <div className="grid grid-2" style={{ marginTop: "1.5rem" }}>
                  <Card>
                    <div className="card-header-row">
                      <span className="card-icon">{"\uD83D\uDD24"}</span>
                      <div>
                        <div className="card-title">Lexer + Parser</div>
                        <div className="card-subtitle">lexer.rs (480 LOC) &middot; parser.rs (1,572 LOC)</div>
                      </div>
                    </div>
                    <div className="card-body">
                      <strong>Logos</strong>-based zero-copy tokenizer with 127 token variants including
                      evolution keywords. Recursive-descent + <strong>Pratt parser</strong> producing a typed AST.
                    </div>
                    <div className="tags-row">
                      <Tag variant="rust">Logos Lexer</Tag>
                      <Tag variant="rust">Pratt Parsing</Tag>
                      <Tag variant="rust">Zero-Copy</Tag>
                    </div>
                  </Card>

                  <Card>
                    <div className="card-header-row">
                      <span className="card-icon">{"\uD83C\uDF32"}</span>
                      <div>
                        <div className="card-title">AST + Type Checker</div>
                        <div className="card-subtitle">ast.rs (535 LOC) &middot; types.rs (834 LOC)</div>
                      </div>
                    </div>
                    <div className="card-body">
                      <strong>27 expression variants</strong> with <code>@annotation</code> support and Origin tracking.
                      Two-pass type checker with scope chains using <code>std::mem::replace</code> pattern.
                    </div>
                    <div className="tags-row">
                      <Tag variant="rust">27 AST Nodes</Tag>
                      <Tag variant="rust">Two-Pass</Tag>
                      <Tag variant="rust">Scope Chains</Tag>
                    </div>
                  </Card>

                  <Card>
                    <div className="card-header-row">
                      <span className="card-icon">{"\uD83D\uDCD0"}</span>
                      <div>
                        <div className="card-title">IR + Code Generator</div>
                        <div className="card-subtitle">ir.rs (1,311 LOC) &middot; codegen.rs (1,460 LOC)</div>
                      </div>
                    </div>
                    <div className="card-body">
                      <strong>SSA-form IR</strong> with ~30 instruction variants. Cranelift 0.116 JIT backend
                      with 98 stdlib builtins including AI activations (sigmoid thru logit), loss functions (huber, mse), hyperbolic (sinh, cosh), and extended math.
                    </div>
                    <div className="tags-row">
                      <Tag variant="rust">SSA IR</Tag>
                      <Tag variant="rust">Cranelift 0.116</Tag>
                      <Tag variant="rust">JIT x86-64</Tag>
                    </div>
                  </Card>

                  <Card>
                    <div className="card-header-row">
                      <span className="card-icon">{"\uD83D\uDD17"}</span>
                      <div>
                        <div className="card-title">FFI Bridge + Python</div>
                        <div className="card-subtitle">bridge.rs (764 LOC) &middot; 652 FFI exports &middot; vitalis.py (3,036 LOC)</div>
                      </div>
                    </div>
                    <div className="card-body">
                      <strong>652 extern &quot;C&quot;</strong> FFI exports across 47 source files with <code>#[unsafe(no_mangle)]</code>.
                      318 Python APIs via <code>vitalis.py</code> (3,036 LOC). Strings via <code>CString::into_raw()</code>, freed via <code>slang_free_string()</code>.
                    </div>
                    <div className="tags-row">
                      <Tag variant="rust">extern &quot;C&quot;</Tag>
                      <Tag variant="python">ctypes</Tag>
                      <Tag variant="rust">CString FFI</Tag>
                    </div>
                  </Card>
                </div>
              </section>
            </Reveal>
          </div>

      {/* ═══════════ SIMD ENGINE ═══════════ */}
      <EnergyConnector />
      <div className="container" id="simd">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83D\uDE80"} title="SIMD Vectorization Engine" badge="NATIVE F64\u00D74" badgeType="native" />
            <div className="grid grid-2">
              <Card featured>
                <div className="card-header-row">
                  <span className="card-icon">{"\u26A1"}</span>
                  <div>
                    <div className="card-title">simd_ops.rs</div>
                    <div className="card-subtitle">748 LOC &middot; 15 FFI Exports &middot; Hardware-Accelerated</div>
                  </div>
                </div>
                <div className="card-body">
                  Native Rust SIMD engine processing <strong>4&times; f64 lanes</strong> in parallel.
                  Auto-detects CPU capabilities at runtime.
                </div>
                <h4 className="subsection-label">OPERATIONS (15)</h4>
                <div className="tags-row">
                  <Tag variant="hw">simd_sum</Tag>
                  <Tag variant="hw">simd_mean</Tag>
                  <Tag variant="hw">simd_dot_product</Tag>
                  <Tag variant="hw">simd_min / max</Tag>
                  <Tag variant="hw">simd_variance</Tag>
                  <Tag variant="hw">simd_std_dev</Tag>
                  <Tag variant="hw">simd_norm_l2</Tag>
                  <Tag variant="hw">simd_cosine_similarity</Tag>
                  <Tag variant="hw">simd_softmax</Tag>
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDDA5\uFE0F"}</span>
                  <div>
                    <div className="card-title">CPU Capabilities</div>
                    <div className="card-subtitle">Runtime Detection &middot; Auto-Dispatch</div>
                  </div>
                </div>
                <div className="cap-grid">
                  {[
                    { name: "SSE 4.2", on: true },
                    { name: "AVX2", on: true },
                    { name: "FMA", on: true },
                    { name: "AVX-512", on: false },
                  ].map(c => (
                    <div key={c.name} className={`cap-item ${c.on ? "supported" : "unsupported"}`}>
                      <span className={`cap-dot ${c.on ? "on" : "off"}`} />
                      <span>{c.name}</span>
                    </div>
                  ))}
                </div>
                <div style={{ marginTop: "1.2rem" }}>
                  <StatRow label="Vector Width" value="4\u00D7 f64 (256-bit)" />
                  <StatRow label="Throughput Gain" value="~4\u00D7 vs scalar" color={ALIEN.green} />
                  <StatRow label="Tail Handling" value="Scalar fallback" />
                  <StatRow label="Python API" value="vitalis.hotpath_simd_*" color={ALIEN.gold} />
                </div>
              </Card>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ PREDICTIVE OPTIMIZER ═══════════ */}
      <EnergyConnector />
      <div className="container" id="optimizer">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83E\uDDE0"} title="Predictive Optimizer Suite" badge="ADAPTIVE JIT" badgeType="jit" />
            <div className="grid grid-2">
              <Reveal delay={0}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCBE"}</span>
                  <div>
                    <div className="card-title">Compilation Cache</div>
                    <div className="card-subtitle">Hash-Indexed &middot; LRU Eviction</div>
                  </div>
                </div>
                <div className="card-body">
                  Caches compiled JIT outputs indexed by <strong>SHA-256 hash</strong> of source code.
                  Avoids redundant compilation of unchanged functions. Tracks hit/miss ratios
                  for cache effectiveness monitoring.
                </div>
                <div className="tags-row">
                  <Tag variant="rust">SHA-256</Tag>
                  <Tag variant="rust">LRU Cache</Tag>
                </div>
              </Card></Reveal>

              <Reveal delay={0.1}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCC8"}</span>
                  <div>
                    <div className="card-title">Trajectory Predictor</div>
                    <div className="card-subtitle">Exponential Smoothing &middot; Trend Detection</div>
                  </div>
                </div>
                <div className="card-body">
                  <strong>Exponential smoothing</strong> forecaster that predicts which functions will be called next
                  based on historical execution patterns. Pre-compiles predicted hot paths before they&apos;re needed.
                </div>
                <div className="tags-row">
                  <Tag variant="rust">Exponential Smoothing</Tag>
                  <Tag variant="rust">Prediction</Tag>
                </div>
              </Card></Reveal>

              <Reveal delay={0.2}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD2C"}</span>
                  <div>
                    <div className="card-title">Delta Debugger</div>
                    <div className="card-subtitle">Binary Search &middot; Minimal Failing Input</div>
                  </div>
                </div>
                <div className="card-body">
                  Implements <strong>delta debugging</strong> algorithm to find the minimal set of code changes
                  that cause a regression. Binary-search reduction of diff hunks. Essential for
                  evolution rollback decisions.
                </div>
                <div className="tags-row">
                  <Tag variant="rust">Delta Debug</Tag>
                  <Tag variant="rust">Bisection</Tag>
                </div>
              </Card></Reveal>

              <Reveal delay={0.3}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83C\uDFAF"}</span>
                  <div>
                    <div className="card-title">Inlining Oracle</div>
                    <div className="card-subtitle">Cost Model &middot; Depth-Bounded</div>
                  </div>
                </div>
                <div className="card-body">
                  Decides whether to <strong>inline</strong> function calls based on call-site cost model:
                  body size, call frequency, recursion depth, and estimated register pressure.
                </div>
                <div className="tags-row">
                  <Tag variant="rust">Cost Model</Tag>
                  <Tag variant="rust">Inlining</Tag>
                </div>
              </Card></Reveal>

              <Reveal delay={0.4}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83C\uDF00"}</span>
                  <div>
                    <div className="card-title">Quantum Landscape</div>
                    <div className="card-subtitle">Fitness Topology &middot; Gradient Estimation</div>
                  </div>
                </div>
                <div className="card-body">
                  Models the <strong>fitness landscape</strong> as a multi-dimensional surface.
                  Estimates gradients to guide evolution towards optimal configurations.
                  Tracks local minima/maxima and saddle points for escape strategies.
                </div>
                <div className="tags-row">
                  <Tag variant="rust">Fitness Landscape</Tag>
                  <Tag variant="rust">Gradient</Tag>
                </div>
              </Card></Reveal>

              <Reveal delay={0.5}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD27"}</span>
                  <div>
                    <div className="card-title">IR Optimization Passes</div>
                    <div className="card-subtitle">Constant Folding &middot; Dead Code Elimination</div>
                  </div>
                </div>
                <div className="card-body">
                  Multi-pass IR optimizer: <strong>constant folding</strong> (evaluates compile-time expressions),
                  <strong>dead code elimination</strong>, and <strong>strength reduction</strong>.
                  Runs between type-check and codegen.
                </div>
                <div className="tags-row">
                  <Tag variant="rust">Const Fold</Tag>
                  <Tag variant="rust">DCE</Tag>
                  <Tag variant="rust">Strength Reduce</Tag>
                </div>
              </Card></Reveal>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ EVOLUTION ENGINE ═══════════ */}
      <EnergyConnector />
      <div className="container" id="evolution">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83E\uDDEC"} title="Self-Evolution Engine" badge={"\u25CF 3-MIN CYCLES"} badgeType="active" />
            <div className="grid grid-2">
              <Card featured>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD04"}</span>
                  <div>
                    <div className="card-title">Evolution Pipeline</div>
                    <div className="card-subtitle">Autonomous Code Improvement</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Vitalis Functions Tracked" value="27+" />
                  <StatRow label="Evolution Attempts" value={String(stats?.evolution.total_attempts ?? 8)} color={ALIEN.gold} />
                  <StatRow label="Successful Evolutions" value={String(stats?.evolution.successes ?? 3)} color={ALIEN.green} />
                  <StatRow label="Guardian Blocks" value={String(stats?.guardian?.checks_blocked ?? 4)} color={ALIEN.red} />
                  <StatRow label="Rollbacks" value={String(stats?.evolution.rollbacks ?? 0)} />
                  <StatRow label="Evolution Rate" value={`${evoRate}/hr`} color={ALIEN.magenta} />
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\u2699\uFE0F"}</span>
                  <div>
                    <div className="card-title">Evolution Flow</div>
                    <div className="card-subtitle">LLM &rarr; Guardian &rarr; Type Check &rarr; Vitalis &rarr; Git</div>
                  </div>
                </div>
                <div className="card-body flow-steps">
                  <p>{"\u2460"} <strong>LLM proposes</strong> code improvements (Claude Sonnet 4.6)</p>
                  <p>{"\u2461"} <strong>Guardian reviews</strong> for Asimov&apos;s Laws compliance</p>
                  <p>{"\u2462"} <strong>Type checker</strong> validates via Vitalis compiler</p>
                  <p>{"\u2463"} <strong>Sandbox</strong> executes in capability-restricted jail</p>
                  <p>{"\u2464"} <strong>Fitness scoring</strong> via code quality metrics</p>
                  <p>{"\u2465"} <strong>Git commit</strong> with generation tracking</p>
                  <p>{"\u2466"} <strong>Rollback</strong> if fitness degrades</p>
                </div>
                <div className="tags-row">
                  <Tag variant="ai">Claude Sonnet 4.6</Tag>
                  <Tag variant="safety">Asimov Enforced</Tag>
                  <Tag variant="rust">Vitalis Validated</Tag>
                </div>
              </Card>
            </div>

            <div className="grid grid-3" style={{ marginTop: "1.2rem" }}>
              <Reveal delay={0}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83C\uDFED"}</span>
                  <div><div className="card-title">Algorithm Forge</div></div>
                </div>
                <div className="card-body">
                  Library of algorithms across multiple domains.
                  LLM generates candidates, Forge evaluates fitness.
                </div>
              </Card></Reveal>
              <Reveal delay={0.1}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83E\uDDEC"}</span>
                  <div><div className="card-title">Meta-Evolution</div></div>
                </div>
                <div className="card-body">
                  <strong>Thompson sampling</strong> over evolution strategies.
                  Learns which mutation operators produce best fitness gains.
                </div>
              </Card></Reveal>
              <Reveal delay={0.2}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD00"}</span>
                  <div><div className="card-title">CoEvolution Bridge</div></div>
                </div>
                <div className="card-body">
                  Bridges Python evolution engine with <strong>Rust Vitalis</strong>.
                  Registers <code>@evolvable</code> functions across runtimes.
                </div>
              </Card></Reveal>
            </div>

            <div className="grid grid-2" style={{ marginTop: "1.2rem" }}>
              <Reveal delay={0}><Card featured borderColor="rgba(176,38,255,0.3)">
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83E\uDDE0"}</span>
                  <div>
                    <div className="card-title">Self-Evolution Orchestrator</div>
                    <div className="card-subtitle">kernel/self_evolution_orchestrator.py &middot; 27+ Vitalis Hotpath Ops</div>
                  </div>
                </div>
                <div className="card-body">
                  Autonomous optimization engine using <strong>27+ Vitalis native functions</strong>:
                  quantum annealing, Bayesian UCB, CMA-ES, L&eacute;vy flights,
                  Pareto-optimal selection, Shannon diversity, spectral analysis,
                  and 12-domain algorithm coverage. Drives recursive self-improvement
                  at native Rust speed.
                </div>
                <div className="tags-row">
                  <Tag variant="rust">Quantum Annealing</Tag>
                  <Tag variant="rust">CMA-ES</Tag>
                  <Tag variant="rust">Pareto Front</Tag>
                  <Tag variant="ai">Bayesian UCB</Tag>
                </div>
              </Card></Reveal>
              <Reveal delay={0.1}><Card borderColor="rgba(57,255,20,0.2)">
                <div className="card-header-row">
                  <span className="card-icon">{"\u26A1"}</span>
                  <div>
                    <div className="card-title">Vitalis-Powered Cortex</div>
                    <div className="card-subtitle">Native Ops Across All Modules</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="swarm.py" value="5 Vitalis ops (tally, diversity, Pareto, Boltzmann)" color={ALIEN.green} />
                  <StatRow label="faiss_index.py" value="4 Vitalis ops (cosine, L2, argmax, softmax)" color={ALIEN.cyan} />
                  <StatRow label="inference core" value="4 Vitalis ops (p95, EMA, weighted, bucket)" color={ALIEN.magenta} />
                  <StatRow label="code_analyzer.py" value="6+ Vitalis scoring functions" color={ALIEN.gold} />
                  <StatRow label="rate_limiter.py" value="Token bucket + sliding window (native)" color={ALIEN.orange} />
                </div>
              </Card></Reveal>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ AI & SWARM ═══════════ */}
      <EnergyConnector />
      <div className="container" id="ai-swarm">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83E\uDD16"} title="AI & Swarm Intelligence" badge={"\u25CF SPRINT MODE"} badgeType="active" />
            <div className="grid grid-3">
              <Card featured>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDC1D"}</span>
                  <div>
                    <div className="card-title">Swarm Consensus</div>
                    <div className="card-subtitle">{stats?.swarm?.active_agents ?? 4} Parallel Agents</div>
                  </div>
                </div>
                <div className="card-body">
                  <strong>{stats?.swarm?.active_agents ?? 4} persistent agent swarms</strong> running in parallel.
                  Votes tallied via <code>hotpath_tally_string_votes()</code> (Rust native).
                  Shannon diversity, Pareto-optimal selection, Boltzmann sampling, weighted consensus &mdash; all via Vitalis native ops.
                </div>
                <div className="tags-row">
                  <Tag variant="ai">{stats?.swarm?.active_agents ?? 4}&times; Agents</Tag>
                  <Tag variant="ai">Pareto Selection</Tag>
                  <Tag variant="rust">Native Tally</Tag>
                  <Tag variant="rust">Shannon Diversity</Tag>
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83E\uDDEA"}</span>
                  <div>
                    <div className="card-title">LLM Infrastructure</div>
                    <div className="card-subtitle">Cloud-First &middot; Local Fallback</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Primary Model" value="Sonnet 4.6 (default)" color={ALIEN.magenta} />
                  <StatRow label="Frontier Model" value="Opus 4.6 (evolution only)" color={ALIEN.violet} />
                  <StatRow label="Provider" value="OpenRouter.ai" />
                  <StatRow label="Daily Budget" value="$5/day cap" color={ALIEN.gold} />
                  <StatRow label="Local Fallback" value="Ollama (optional)" />
                  <StatRow label="Rate Limiting" value="Sliding window + token bucket" />
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCD1"}</span>
                  <div>
                    <div className="card-title">Goals & Roadmap</div>
                    <div className="card-subtitle">Autonomous Planning</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Active Goals" value={String(stats?.active_goals ?? 10)} color={ALIEN.green} />
                  <StatRow label="Total Goals" value={String(stats?.total_goals ?? 335)} />
                  <StatRow label="AI Modules" value={String(stats?.modules_loaded ?? 72)} color={ALIEN.gold} />
                </div>
              </Card>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ NOVA LLM ENGINE ═══════════ */}
      <EnergyConnector />
      <div className="container" id="nova-llm">
        <Reveal>
          <section className="section">
            <SectionHeader icon="\uD83E\uDDE0" title="Nova LLM Engine \u2014 From-Scratch Rust" badge="\u25CF TRAINING" badgeType="active" />
            <p className="section-intro" style={{ color: ALIEN.dim, marginBottom: 24, fontSize: 15, lineHeight: 1.7 }}>
              A full large language model built entirely from scratch in Rust \u2014 no PyTorch, no HuggingFace, no shortcuts.
              Custom tensor library, transformer architecture, BPE tokenizer, training loop, and real-time monitoring studio.
              Currently training on consumer hardware (RTX 5060, Blackwell).
            </p>

            {/* Nova Architecture Overview */}
            <div className="nova-arch-banner">
              <div className="nova-arch-flow">
                <div className="nova-arch-node" style={{ borderColor: ALIEN.cyan }}>
                  <span className="nova-arch-icon">📝</span>
                  <span>Raw Text</span>
                </div>
                <span className="nova-arch-arrow">\u2192</span>
                <div className="nova-arch-node" style={{ borderColor: ALIEN.green }}>
                  <span className="nova-arch-icon">🧩</span>
                  <span>BPE Tokenizer</span>
                </div>
                <span className="nova-arch-arrow">\u2192</span>
                <div className="nova-arch-node" style={{ borderColor: ALIEN.magenta }}>
                  <span className="nova-arch-icon">🧠</span>
                  <span>Transformer</span>
                </div>
                <span className="nova-arch-arrow">\u2192</span>
                <div className="nova-arch-node" style={{ borderColor: ALIEN.violet }}>
                  <span className="nova-arch-icon">📊</span>
                  <span>Nova Studio</span>
                </div>
                <span className="nova-arch-arrow">\u2192</span>
                <div className="nova-arch-node" style={{ borderColor: ALIEN.gold }}>
                  <span className="nova-arch-icon">🧬</span>
                  <span>Self-Evolution</span>
                </div>
              </div>
            </div>

            <div className="grid grid-3" style={{ marginTop: 20 }}>
              {/* Tensor Engine */}
              <Card featured borderColor={ALIEN.cyan}>
                <div className="card-header-row">
                  <span className="card-icon">🧮</span>
                  <div>
                    <div className="card-title">Custom Tensor Engine</div>
                    <div className="card-subtitle">tensor/ \u2022 1,761 LOC</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Storage" value="CPU + CUDA backends" color={ALIEN.cyan} />
                  <StatRow label="Dtypes" value="f32 / f16 / bf16" />
                  <StatRow label="Autograd" value="Computation graph + backward" color={ALIEN.green} />
                  <StatRow label="Matmul" value="cuBLAS SGEMM + Rayon fallback" color={ALIEN.gold} />
                  <StatRow label="Ops" value="33 (matmul, softmax, CE, norm\u2026)" />
                </div>
                <div className="tags-row">
                  <Tag variant="rust">Zero-Copy</Tag>
                  <Tag variant="cyan">Rayon</Tag>
                  <Tag variant="magenta">Autograd</Tag>
                </div>
              </Card>

              {/* Transformer */}
              <Card featured borderColor={ALIEN.magenta}>
                <div className="card-header-row">
                  <span className="card-icon">⚡</span>
                  <div>
                    <div className="card-title">Decoder-Only Transformer</div>
                    <div className="card-subtitle">nn/ \u2022 7 modules \u2022 1,119 LOC</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Attention" value="Multi-Head + GQA" color={ALIEN.magenta} />
                  <StatRow label="Positional" value="RoPE (Rotary Embeddings)" />
                  <StatRow label="Normalization" value="RMSNorm (pre-norm)" />
                  <StatRow label="Activation" value="SwiGLU FFN" color={ALIEN.green} />
                  <StatRow label="Architecture" value="GPT/LLaMA-style causal" />
                </div>
                <div className="tags-row">
                  <Tag variant="magenta">RoPE</Tag>
                  <Tag variant="rust">RMSNorm</Tag>
                  <Tag variant="green">SwiGLU</Tag>
                </div>
              </Card>

              {/* CUDA GPU */}
              <Card featured borderColor={ALIEN.green}>
                <div className="card-header-row">
                  <span className="card-icon">🎮</span>
                  <div>
                    <div className="card-title">CUDA GPU Acceleration</div>
                    <div className="card-subtitle">gpu/ \u2022 cuBLAS SGEMM \u2022 724 LOC</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Target" value="RTX 5060 (Blackwell, CC 12.0)" color={ALIEN.green} />
                  <StatRow label="Dispatch" value="cuBLAS SGEMM for all matmul" />
                  <StatRow label="Precision" value="FP32 (cudarc 0.19.3)" color={ALIEN.gold} />
                  <StatRow label="Library" value="cudarc 0.19 (safe Rust)" />
                  <StatRow label="VRAM" value="8 GB (dynamic allocation)" />
                </div>
                <div className="tags-row">
                  <Tag variant="green">CUDA 13.1</Tag>
                  <Tag variant="rust">cudarc</Tag>
                  <Tag variant="gold">Blackwell</Tag>
                </div>
              </Card>

              {/* Training Pipeline */}
              <Card borderColor={`${ALIEN.orange}40`}>
                <div className="card-header-row">
                  <span className="card-icon">🏋</span>
                  <div>
                    <div className="card-title">Training Pipeline</div>
                    <div className="card-subtitle">training/ \u2022 7 modules</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Optimizer" value="AdamW (\u03B21=0.9, \u03B22=0.95)" color={ALIEN.orange} />
                  <StatRow label="Scheduler" value="Cosine with Linear Warmup" />
                  <StatRow label="Grad Clip" value="Max norm = 1.0" />
                  <StatRow label="Checkpoints" value="Auto-save every 500 steps" color={ALIEN.green} />
                  <StatRow label="Data" value="14.3 MB corpus (Project Gutenberg)" />
                </div>
                <div className="tags-row">
                  <Tag variant="orange">AdamW</Tag>
                  <Tag variant="cyan">Cosine LR</Tag>
                  <Tag variant="magenta">Grad Accum</Tag>
                </div>
              </Card>

              {/* BPE Tokenizer */}
              <Card borderColor={`${ALIEN.violet}40`}>
                <div className="card-header-row">
                  <span className="card-icon">💬</span>
                  <div>
                    <div className="card-title">BPE Tokenizer</div>
                    <div className="card-subtitle">tokenizer/ \u2022 From-scratch</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Type" value="Byte-level BPE" color={ALIEN.violet} />
                  <StatRow label="Vocab" value="8,000 tokens (trainable)" />
                  <StatRow label="Special" value="<pad>, <unk>, <bos>, <eos>" />
                  <StatRow label="Training" value="2,000 merges on 500K chars" />
                  <StatRow label="Speed" value="\u223C2M tokens/sec" color={ALIEN.green} />
                </div>
                <div className="tags-row">
                  <Tag variant="violet">BPE</Tag>
                  <Tag variant="cyan">Unicode</Tag>
                </div>
              </Card>

              {/* Nova Studio */}
              <Card borderColor={`${ALIEN.gold}40`}>
                <div className="card-header-row">
                  <span className="card-icon">💻</span>
                  <div>
                    <div className="card-title">Nova Studio (GUI)</div>
                    <div className="card-subtitle">studio/ \u2022 eframe/egui</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Framework" value="eframe + egui (GPU-rendered)" color={ALIEN.gold} />
                  <StatRow label="Panels" value="Dashboard, GPU, Training, Gen" />
                  <StatRow label="Charts" value="Loss, LR, Throughput, Grad Norm" />
                  <StatRow label="Monitoring" value="nvidia-smi polling (live)" color={ALIEN.green} />
                  <StatRow label="IPC" value="JSON metrics (nova-cli \u2194 studio)" />
                </div>
                <div className="tags-row">
                  <Tag variant="gold">egui</Tag>
                  <Tag variant="green">Real-time</Tag>
                  <Tag variant="magenta">GPU Monitor</Tag>
                </div>
              </Card>
            </div>

            {/* Nova Stats Banner */}
            <div className="nova-stats-bar">
              <div className="nova-stat">
                <span className="nova-stat-value" style={{ color: ALIEN.cyan }}>11,457</span>
                <span className="nova-stat-label">Lines of Rust</span>
              </div>
              <div className="nova-stat">
                <span className="nova-stat-value" style={{ color: ALIEN.magenta }}>1.8M</span>
                <span className="nova-stat-label">Parameters</span>
              </div>
              <div className="nova-stat">
                <span className="nova-stat-value" style={{ color: ALIEN.green }}>cuBLAS</span>
                <span className="nova-stat-label">GPU Matmul</span>
              </div>
              <div className="nova-stat">
                <span className="nova-stat-value" style={{ color: ALIEN.gold }}>33</span>
                <span className="nova-stat-label">Tensor Ops</span>
              </div>
              <div className="nova-stat">
                <span className="nova-stat-value" style={{ color: ALIEN.violet }}>7</span>
                <span className="nova-stat-label">NN Modules</span>
              </div>
              <div className="nova-stat">
                <span className="nova-stat-value" style={{ color: ALIEN.orange }}>0</span>
                <span className="nova-stat-label">Dependencies on PyTorch</span>
              </div>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ SAFETY ═══════════ */}
      <EnergyConnector />
      <div className="container" id="safety">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83D\uDEE1\uFE0F"} title="Safety & Governance" badge={"\u25CF ARMED"} badgeType="active" />
            <div className="grid grid-3">
              <Card borderColor="rgba(255,0,60,0.2)">
                <div className="card-header-row">
                  <span className="card-icon">{"\u2696\uFE0F"}</span>
                  <div>
                    <div className="card-title">Asimov&apos;s Three Laws</div>
                    <div className="card-subtitle">kernel/laws.py &middot; kernel/guardian.py</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Law 1" value="No harm to humans" color={ALIEN.red} />
                  <StatRow label="Law 2" value="Obey human orders" color={ALIEN.orange} />
                  <StatRow label="Law 3" value="Self-preservation" color={ALIEN.gold} />
                  <p className="card-note">
                    Pattern-based code review blocks dangerous operations:
                    subprocess, os.system, network access, file deletion.
                  </p>
                </div>
              </Card>

              <Card borderColor="rgba(255,0,60,0.2)">
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD12"}</span>
                  <div>
                    <div className="card-title">Capability Sandbox</div>
                    <div className="card-subtitle">kernel/sandbox.py</div>
                  </div>
                </div>
                <div className="card-body">
                  All evolved code executes in a <strong>capability-restricted jail</strong>.
                  No file system, no network, no process spawning. Timeout enforcement.
                </div>
                <div className="tags-row">
                  <Tag variant="safety">Jail</Tag>
                  <Tag variant="safety">Whitelist</Tag>
                  <Tag variant="safety">Timeout</Tag>
                </div>
              </Card>

              <Card borderColor="rgba(255,0,60,0.2)">
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83C\uDFF0"}</span>
                  <div>
                    <div className="card-title">Kernel Sentinel</div>
                    <div className="card-subtitle">kernel_sentinel.rs (957 LOC)</div>
                  </div>
                </div>
                <div className="card-body">
                  <strong>SHA-256 integrity</strong> checks on critical kernel files.
                  Tamper detection runs continuously.
                  Native Rust for bypass resistance.
                </div>
                <div className="tags-row">
                  <Tag variant="rust">SHA-256</Tag>
                  <Tag variant="safety">Tamper-Proof</Tag>
                </div>
              </Card>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ BACKEND MODULES BREAKDOWN ═══════════ */}
      <EnergyConnector />
      <div className="container" id="backend">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83D\uDC0D"} title="Python Backend \u2014 72 Modules" badge={"\u25CF PORT 8002"} badgeType="active" />
            <div className="grid grid-2">
              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83E\uDDE0"}</span>
                  <div>
                    <div className="card-title">Kernel Layer</div>
                    <div className="card-subtitle">11 modules &middot; System Core</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="boot.py" value="System orchestrator \u2014 2,200+ LOC" />
                  <StatRow label="guardian.py" value="Asimov\u2019s Laws enforcement" />
                  <StatRow label="laws.py" value="Three Laws definitions" />
                  <StatRow label="sandbox.py" value="Capability-based code jail" />
                  <StatRow label="evolution.py" value="Python evolution engine" />
                  <StatRow label="algorithm_forge.py" value="Algorithm generation & evaluation" />
                  <StatRow label="coevolution.py" value="Python \u2194 Vitalis bridge" />
                  <StatRow label="config.py" value="Pydantic Settings config" />
                  <StatRow label="loader.py" value="Dynamic module loader" />
                  <StatRow label="service_registry.py" value="Service discovery" />
                  <StatRow label="self_evolution_orchestrator.py" value="27+ Vitalis hotpath self-evolution" color={ALIEN.violet} />
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83C\uDF10"}</span>
                  <div>
                    <div className="card-title">Cortex AI Modules</div>
                    <div className="card-subtitle">93 files &middot; Intelligence Layer</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="inference_pkg/" value="LLM inference + monitoring" />
                  <StatRow label="swarm_pkg/" value="Multi-agent consensus (4 agents)" />
                  <StatRow label="episodic/" value="Episodic memory storage" />
                  <StatRow label="faiss_pkg/" value="FAISS vector similarity search" />
                  <StatRow label="consciousness/" value="Self-awareness substrate" />
                  <StatRow label="mempersist/" value="Memory persistence layer" />
                  <StatRow label="comm_pkg/" value="External communication" />
                  <StatRow label="voice/" value="Voice processing" />
                  <StatRow label="taskq/" value="Task queue management" />
                  <StatRow label="modules/" value="Plugin modules" />
                </div>
              </Card>
            </div>

            <div className="grid grid-4" style={{ marginTop: "1.2rem" }}>
              <Reveal delay={0}><Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83D\uDCA1"}</span><div><div className="card-title">Reasoning</div></div></div>
                <div className="card-body">Chain-of-thought, multi-step planning, hypothesis testing</div>
                <div className="tags-row"><Tag variant="ai">reasoning.py</Tag></div>
              </Card></Reveal>
              <Reveal delay={0.05}><Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83D\uDCBB"}</span><div><div className="card-title">Coder</div></div></div>
                <div className="card-body">Code generation, refactoring proposals, PR creation</div>
                <div className="tags-row"><Tag variant="ai">coder.py</Tag></div>
              </Card></Reveal>
              <Reveal delay={0.1}><Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83D\uDD0D"}</span><div><div className="card-title">Researcher</div></div></div>
                <div className="card-body">Web search, knowledge synthesis, citation tracking</div>
                <div className="tags-row"><Tag variant="ai">researcher.py</Tag></div>
              </Card></Reveal>
              <Reveal delay={0.15}><Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83D\uDCCA"}</span><div><div className="card-title">Code Analyzer</div></div></div>
                <div className="card-body">Cyclomatic & cognitive complexity, quality scores</div>
                <div className="tags-row"><Tag variant="ai">code_analyzer.py</Tag></div>
              </Card></Reveal>
              <Reveal delay={0.2}><Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83E\uDDE0"}</span><div><div className="card-title">Memory</div></div></div>
                <div className="card-body">ChromaDB vectors, {formatNum(stats?.memory_count ?? 1478)} stored memories, semantic search</div>
                <div className="tags-row"><Tag variant="ai">memory.py</Tag></div>
              </Card></Reveal>
              <Reveal delay={0.25}><Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83D\uDD0A"}</span><div><div className="card-title">Voice</div></div></div>
                <div className="card-body">Audio processing, TTS, speech-to-text pipeline</div>
                <div className="tags-row"><Tag variant="ai">voice/</Tag></div>
              </Card></Reveal>
              <Reveal delay={0.3}><Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83C\uDF10"}</span><div><div className="card-title">Hybrid LLM</div></div></div>
                <div className="card-body">OpenRouter cloud + local Ollama fallback routing</div>
                <div className="tags-row"><Tag variant="ai">hybrid_llm.py</Tag></div>
              </Card></Reveal>
              <Reveal delay={0.35}><Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83D\uDCE1"}</span><div><div className="card-title">Subagents</div></div></div>
                <div className="card-body">Spawns specialized sub-agents for complex tasks</div>
                <div className="tags-row"><Tag variant="ai">subagents.py</Tag></div>
              </Card></Reveal>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ CYBERPUNK FRONTEND ═══════════ */}
      <EnergyConnector />
      <div className="container" id="frontend-sec">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83D\uDDA5\uFE0F"} title="Cyberpunk Frontend \u2014 26 Routes" badge="PORT 3002" badgeType="native" />
            <div className="grid grid-2">
              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83C\uDFA8"}</span>
                  <div>
                    <div className="card-title">Visual Engine</div>
                    <div className="card-subtitle">GPU-Accelerated &middot; WebGL &middot; Holographic</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="ShaderBackground" value="WebGL fragment shaders" />
                  <StatRow label="CyberpunkHUD" value="Heads-up display overlay" />
                  <StatRow label="HoloElements" value="Holographic UI primitives" />
                  <StatRow label="NeuralBackground" value="Particle neural network" />
                  <StatRow label="InfinityAvatar" value="Animated AI avatar" />
                </div>
                <div className="tags-row">
                  <Tag variant="ts">WebGL 2.0</Tag>
                  <Tag variant="ts">Fragment Shaders</Tag>
                  <Tag variant="ts">CSS Animations</Tag>
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCC4"}</span>
                  <div>
                    <div className="card-title">Dashboard Pages ({stats?.tech_stack?.typescript?.routes ?? 26})</div>
                    <div className="card-subtitle">Full System Control Interface</div>
                  </div>
                </div>
                <div className="card-body" style={{ columnCount: 2, columnGap: "1.5rem", fontSize: "0.8rem" }}>
                  <StatRow label="Dashboard" value="Home" />
                  <StatRow label="Agents" value="Swarm" />
                  <StatRow label="Chat" value="LLM" />
                  <StatRow label="Code Analyzer" value="Quality" />
                  <StatRow label="Consciousness" value="Self" />
                  <StatRow label="Diagnostics" value="Health" />
                  <StatRow label="Evolution" value="Gene" />
                  <StatRow label="Goals" value="Plan" />
                  <StatRow label="Infrastructure" value="Infra" />
                  <StatRow label="Logs" value="Logs" />
                  <StatRow label="Memory" value="Mem" />
                  <StatRow label="Modules" value="Core" />
                  <StatRow label="Plugins" value="Ext" />
                  <StatRow label="Roadmap" value="Road" />
                  <StatRow label="Settings" value="Cfg" />
                  <StatRow label="Sprint" value="Agile" />
                  <StatRow label="Voice" value="Audio" />
                  <StatRow label="What&apos;s New" value="Log" />
                </div>
              </Card>
            </div>

            <div className="grid grid-4" style={{ marginTop: "1.2rem" }}>
              <Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83D\uDCE6"}</span><div><div className="card-title">Lib: API</div></div></div>
                <div className="card-body">Typed API client with 19 functions, SSE event stream</div>
              </Card>
              <Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83C\uDFB5"}</span><div><div className="card-title">Lib: Sounds</div></div></div>
                <div className="card-body">Web Audio API sound effects, shared utility</div>
              </Card>
              <Card>
                <div className="card-header-row"><span className="card-icon">{"\u2728"}</span><div><div className="card-title">Lib: Shaders</div></div></div>
                <div className="card-body">GLSL vertex + fragment shaders for GPU effects</div>
              </Card>
              <Card>
                <div className="card-header-row"><span className="card-icon">{"\uD83D\uDCE1"}</span><div><div className="card-title">Lib: Events</div></div></div>
                <div className="card-body">SSE hook for real-time streaming updates</div>
              </Card>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ CONSCIOUSNESS SUBSTRATE ═══════════ */}
      <EnergyConnector />
      <div className="container" id="consciousness">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83D\uDC41\uFE0F"} title="Consciousness Substrate" badge="SELF-AWARE" badgeType="jit" />
            <div className="grid grid-3">
              <Reveal delay={0}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83E\uDDEC"}</span>
                  <div>
                    <div className="card-title">Native Substrate</div>
                    <div className="card-subtitle">consciousness.rs (1,125 LOC)</div>
                  </div>
                </div>
                <div className="card-body">
                  Rust-native <strong>self-model</strong> with Signal enum for internal state tracking.
                  Maintains awareness of own capabilities, limitations, and current operational state.
                  Feeds into evolution fitness decisions.
                </div>
              </Card></Reveal>

              <Reveal delay={0.1}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCD4"}</span>
                  <div>
                    <div className="card-title">Mind Journal</div>
                    <div className="card-subtitle">mind/ directory</div>
                  </div>
                </div>
                <div className="card-body">
                  Persistent <strong>JSON journal</strong> tracking goals, reflections, roadmap progress,
                  and internal state transitions. {stats?.total_goals ?? 335} goals loaded, {stats?.active_goals ?? 10} active at any time.
                  Survives restarts.
                </div>
              </Card></Reveal>

              <Reveal delay={0.2}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD78\uFE0F"}</span>
                  <div>
                    <div className="card-title">Episodic Memory</div>
                    <div className="card-subtitle">cortex/episodic/</div>
                  </div>
                </div>
                <div className="card-body">
                  Time-stamped <strong>episodic memories</strong> of significant events.
                  Used for experience-based learning and avoiding past mistakes.
                  Integrated with ChromaDB for semantic retrieval.
                </div>
              </Card></Reveal>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ ALGORITHM LIBRARIES ═══════════ */}
      <EnergyConnector />
      <div className="container" id="algorithms">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83E\uDDEE"} title="Vitalis v20.0 — 27 Native Algorithm Libraries" badge="652 FFI EXPORTS" badgeType="jit" />
            <p className="section-desc">
              Compiled Rust algorithm libraries exposed via <strong>652 FFI exports</strong> to Python.
              Each library is benchmarked at <strong>7.5x avg</strong> (29.1x peak) vs pure Python equivalents.
            </p>
            <div className="grid grid-3">
              <Reveal delay={0}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD24"}</span>
                  <div><div className="card-title">String Algorithms</div><div className="card-subtitle">Levenshtein &middot; Jaro-Winkler &middot; Hamming</div></div>
                </div>
                <div className="card-body">Edit distance, fuzzy matching, and phonetic similarity — all native Rust with zero-copy.</div>
                <div className="tags-row"><Tag variant="rust">Levenshtein</Tag><Tag variant="rust">Jaro-Winkler</Tag><Tag variant="rust">Hamming</Tag></div>
              </Card></Reveal>

              <Reveal delay={0.06}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD78\uFE0F"}</span>
                  <div><div className="card-title">Graph Algorithms</div><div className="card-subtitle">PageRank &middot; Toposort &middot; Cycle Detection</div></div>
                </div>
                <div className="card-body">Sparse matrix PageRank, Kahn&apos;s topological sort, and DFS cycle detection for dependency analysis.</div>
                <div className="tags-row"><Tag variant="rust">PageRank</Tag><Tag variant="rust">Toposort</Tag><Tag variant="rust">DFS</Tag></div>
              </Card></Reveal>

              <Reveal delay={0.12}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD10"}</span>
                  <div><div className="card-title">Crypto &amp; Security</div><div className="card-subtitle">SHA-256 &middot; HMAC &middot; SQLi/XSS Detection</div></div>
                </div>
                <div className="card-body">Native hash functions, HMAC signing, SQL injection / XSS pattern detection, and password strength scoring.</div>
                <div className="tags-row"><Tag variant="rust">SHA-256</Tag><Tag variant="rust">HMAC</Tag><Tag variant="rust">SQLi</Tag><Tag variant="rust">XSS</Tag></div>
              </Card></Reveal>

              <Reveal delay={0.18}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCC9"}</span>
                  <div><div className="card-title">Signal Processing</div><div className="card-subtitle">FFT &middot; SMA &middot; EMA</div></div>
                </div>
                <div className="card-body">Fast Fourier Transform, Simple &amp; Exponential Moving Averages for time-series analysis — SIMD-accelerated.</div>
                <div className="tags-row"><Tag variant="rust">FFT</Tag><Tag variant="rust">SMA</Tag><Tag variant="rust">EMA</Tag><Tag variant="hw">SIMD</Tag></div>
              </Card></Reveal>

              <Reveal delay={0.24}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCCA"}</span>
                  <div><div className="card-title">Analytics &amp; Scoring</div><div className="card-subtitle">Elo Rating &middot; Z-Score &middot; Code Quality</div></div>
                </div>
                <div className="card-body">Elo rating system, z-score anomaly detection, code quality scoring, maintainability index, and cognitive complexity.</div>
                <div className="tags-row"><Tag variant="rust">Elo</Tag><Tag variant="rust">Z-Score</Tag><Tag variant="rust">Quality</Tag></div>
              </Card></Reveal>

              <Reveal delay={0.30}><Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\u269B\uFE0F"}</span>
                  <div><div className="card-title">Quantum &amp; Advanced Math</div><div className="card-subtitle">Quantum Optimization &middot; Numerical Methods</div></div>
                </div>
                <div className="card-body">Quantum-inspired annealing, compression algorithms, numerical integration, and advanced mathematical operations.</div>
                <div className="tags-row"><Tag variant="rust">Quantum</Tag><Tag variant="rust">Numerical</Tag><Tag variant="rust">Compression</Tag></div>
              </Card></Reveal>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ INFRASTRUCTURE ═══════════ */}
      <EnergyConnector />
      <div className="container" id="infra">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83C\uDFD7\uFE0F"} title="Infrastructure & DevOps" badge="CONTAINERIZED" badgeType="native" />
            <div className="grid grid-2" style={{ marginBottom: "1.5rem" }}>
              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCCA"}</span>
                  <div><div className="card-title">System Resources</div></div>
                </div>
                <div className="gauge-row">
                  <GaugeRing value={stats?.infrastructure?.cpu_percent ?? 0} max={100} label="CPU" color={ALIEN.cyan} />
                  <GaugeRing value={stats?.infrastructure?.ram_percent ?? 0} max={100} label="RAM" color={ALIEN.magenta} />
                  <GaugeRing
                    value={stats?.evolution?.successes ?? 0}
                    max={Math.max(stats?.evolution?.total_attempts ?? 1, 1)}
                    label="EVO" color={ALIEN.green}
                  />
                  <GaugeRing
                    value={stats?.guardian?.checks_passed ?? 0}
                    max={Math.max((stats?.guardian?.checks_passed ?? 0) + (stats?.guardian?.checks_blocked ?? 0), 1)}
                    label="GUARD" color={ALIEN.gold}
                  />
                </div>
                <div className="infra-pills">
                  <span>{stats?.infrastructure?.platform ?? "Windows-11"}</span>
                  <span>Python {stats?.infrastructure?.python_version ?? "3.12"}</span>
                  <span>{stats?.infrastructure?.cpu_cores ?? 8} Cores</span>
                  <span>{stats?.infrastructure?.ram_total_gb?.toFixed(1) ?? "32.0"} GB RAM</span>
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDC33"}</span>
                  <div>
                    <div className="card-title">Docker & Data</div>
                    <div className="card-subtitle">Multi-Stage Build &middot; Vector Store</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Backend" value="Python 3.12-slim" />
                  <StatRow label="Frontend" value="Node 20-alpine" />
                  <StatRow label="ChromaDB" value={`${formatNum(stats?.memory_count ?? 0)} embeddings`} color={ALIEN.magenta} />
                  <StatRow label="FAISS" value="Vector similarity" />
                  <StatRow label="Rust Tests" value="748 \u2713" color={ALIEN.green} />
                  <StatRow label="Frontend Build" value="25 pages \u2713" color={ALIEN.green} />
                </div>
                <div className="tags-row">
                  <Tag variant="infra">Dockerfile</Tag>
                  <Tag variant="infra">Docker Compose</Tag>
                  <Tag variant="infra">Prometheus</Tag>
                </div>
              </Card>
            </div>

            <div className="grid grid-3">
              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDE80"}</span>
                  <div>
                    <div className="card-title">Deployment</div>
                    <div className="card-subtitle">Vercel + Local Dev</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Frontend Host" value="Vercel Edge" color={ALIEN.cyan} />
                  <StatRow label="Backend Host" value="Local (port 8002)" />
                  <StatRow label="Domain" value="infinitytechstack.uk" color={ALIEN.green} />
                  <StatRow label="SSL" value="Auto (Let's Encrypt)" color={ALIEN.green} />
                </div>
                <div className="tags-row">
                  <Tag variant="infra">Vercel</Tag>
                  <Tag variant="ts">Edge Network</Tag>
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDD12"}</span>
                  <div>
                    <div className="card-title">Monitoring</div>
                    <div className="card-subtitle">Health + Metrics</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="Health Check" value="/health" />
                  <StatRow label="Status API" value="/api/status" />
                  <StatRow label="Refresh Rate" value="8s SSE" color={ALIEN.cyan} />
                  <StatRow label="Logs" value="Structured JSON" />
                </div>
                <div className="tags-row">
                  <Tag variant="infra">Prometheus</Tag>
                  <Tag variant="infra">SSE Stream</Tag>
                </div>
              </Card>

              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCBE"}</span>
                  <div>
                    <div className="card-title">Persistence</div>
                    <div className="card-subtitle">Data Layer</div>
                  </div>
                </div>
                <div className="card-body">
                  <StatRow label="ChromaDB" value="Vector Store" color={ALIEN.magenta} />
                  <StatRow label="FAISS" value="Similarity Index" />
                  <StatRow label="Mind Journal" value="JSON Persistence" />
                  <StatRow label="Git" value="Evolution History" color={ALIEN.green} />
                </div>
                <div className="tags-row">
                  <Tag variant="ai">ChromaDB</Tag>
                  <Tag variant="ai">FAISS</Tag>
                </div>
              </Card>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ SOURCE INVENTORY ═══════════ */}
      <EnergyConnector />
      <div className="container" id="inventory">
        <Reveal>
          <section className="section">
            <SectionHeader icon={"\uD83D\uDCCB"} title="Full Source Inventory" badge="482 FILES" badgeType="native" />

            <Card>
              <div className="card-header-row">
                <span className="card-icon">{"\uD83E\uDD80"}</span>
                <div>
                  <div className="card-title">Rust &mdash; Vitalis Compiler Modules</div>
                  <div className="card-subtitle">47 files &middot; 35,632 LOC &middot; slang/src/</div>
                </div>
              </div>
              <div className="file-table-wrap">
                <table className="file-table">
                  <thead>
                    <tr>
                      <th>Module</th>
                      <th>LOC</th>
                      <th>Purpose</th>
                      <th style={{ width: "25%" }}>Distribution</th>
                    </tr>
                  </thead>
                  <tbody>
                    {RUST_MODULES.map((m, i) => (
                      <tr key={m.name}>
                        <td className="file-name">{m.name}</td>
                        <td>{formatNum(m.loc)}</td>
                        <td>{m.purpose}</td>
                        <td>
                          <span
                            className="loc-bar"
                            style={{
                              width: `${m.pct}%`,
                              background: LOC_BAR_COLORS[i % LOC_BAR_COLORS.length],
                            }}
                          />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>

            <div className="grid grid-2" style={{ marginTop: "1.2rem" }}>
              <Card>
                <div className="card-header-row">
                  <span className="card-icon">{"\uD83D\uDCE6"}</span>
                  <div><div className="card-title">Technology Dependencies</div></div>
                </div>
                <div className="card-body">
                  <h4 className="dep-heading" style={{ color: ALIEN.orange }}>RUST CRATES</h4>
                  <div className="tags-row">
                    <Tag variant="rust">cranelift-codegen</Tag>
                    <Tag variant="rust">cranelift-module</Tag>
                    <Tag variant="rust">cranelift-jit</Tag>
                    <Tag variant="rust">logos</Tag>
                    <Tag variant="rust">clap</Tag>
                    <Tag variant="rust">serde</Tag>
                    <Tag variant="rust">sha2</Tag>
                    <Tag variant="rust">rand</Tag>
                  </div>
                  <h4 className="dep-heading" style={{ color: ALIEN.green }}>PYTHON PACKAGES</h4>
                  <div className="tags-row">
                    <Tag variant="python">FastAPI</Tag>
                    <Tag variant="python">uvicorn</Tag>
                    <Tag variant="python">chromadb</Tag>
                    <Tag variant="python">pydantic</Tag>
                    <Tag variant="python">openai</Tag>
                    <Tag variant="python">httpx</Tag>
                    <Tag variant="python">faiss-cpu</Tag>
                    <Tag variant="python">numpy</Tag>
                  </div>
                  <h4 className="dep-heading" style={{ color: ALIEN.cyan }}>NPM PACKAGES</h4>
                  <div className="tags-row">
                    <Tag variant="ts">next</Tag>
                    <Tag variant="ts">react</Tag>
                    <Tag variant="ts">tailwindcss</Tag>
                    <Tag variant="ts">typescript</Tag>
                    <Tag variant="ts">postcss</Tag>
                  </div>
                </div>
              </Card>

              {stats && stats.module_names.length > 0 && (
                <Card>
                  <div className="card-header-row">
                    <span className="card-icon">{"\uD83C\uDF10"}</span>
                    <div>
                      <div className="card-title">AI Modules ({stats.modules_loaded})</div>
                      <div className="card-subtitle">Loaded Cortex Modules</div>
                    </div>
                  </div>
                  <div className="module-cloud">
                    {stats.module_names.map((m, i) => (
                      <span key={m} className="module-tag" style={{ animationDelay: `${i * 0.03}s` }}>{m}</span>
                    ))}
                  </div>
                </Card>
              )}
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ CONSULTING CTA BANNER ═══════════ */}
      <div className="container">
        <div className="consulting-cta-banner">
          <div className="consulting-cta-glow" />
          <div className="consulting-cta-content">
            <div className="consulting-cta-badge">⚡ AVAILABLE FOR CONSULTING</div>
            <h3 className="consulting-cta-title">
              Need Help Securing or Scaling Your AI?
            </h3>
            <p className="consulting-cta-desc">
              Enterprise AI security audits &middot; Architecture advisory &middot; Custom Rust toolchains &middot; Microsoft Purview implementation
            </p>
            <div className="consulting-cta-buttons">
              <a href="/consulting" className="consulting-cta-btn-primary">
                Book a Consulting Session &rarr;
              </a>
              <a href="/consulting#blueprint" className="consulting-cta-btn-secondary">
                Get the AI Security Playbook &mdash; &pound;97
              </a>
            </div>
            <div className="consulting-cta-stats">
              <span>🔒 AI Security Audits from £350</span>
              <span>🏗️ Architecture Advisory £400/hr</span>
              <span>⚡ Custom Toolchain Builds</span>
            </div>
          </div>
        </div>
      </div>

      {/* ═══════ FLOATING CONSULTING BUTTON ═══════ */}
      <a href="/consulting" className="consulting-float-btn" title="Book Consulting">
        <span className="consulting-float-pulse" />
        <span className="consulting-float-icon">⚡</span>
        <span className="consulting-float-text">Consult</span>
      </a>

      {/* ═══════════ FOOTER ═══════════ */}
      <div className="container">
        <footer className="alien-footer">
          <span className="footer-symbol">{"\u221E"}</span>
          <p>INFINITY &mdash; Autonomous Self-Evolving AI System</p>
          <p className="footer-stats">
            {formatNum(stats?.total_loc ?? 153875)} LOC &middot; 482 Files &middot; 3 Languages &middot; {stats?.modules_loaded ?? 94} Modules &middot; 748 Tests &middot; {formatNum(stats?.memory_count ?? 0)} Memories
          </p>
          <p className="footer-tech">
            Rust 2024 &middot; Python 3.12 &middot; Next.js 15 &middot; Cranelift 0.116 &middot; ChromaDB &middot; Claude Sonnet 4.6
          </p>
          <div style={{ marginTop: '0.6rem', display: 'flex', gap: '1.2rem', justifyContent: 'center', flexWrap: 'wrap' }}>
            <a
              href="https://github.com/ModernOps888/vitalis"
              target="_blank"
              rel="noopener noreferrer"
              style={{ color: '#39ff14', opacity: 0.85, textDecoration: 'none', fontSize: '0.75rem', fontWeight: 700, transition: 'opacity 0.2s', borderBottom: '1px solid rgba(57,255,20,0.4)' }}
              onMouseEnter={e => (e.currentTarget.style.opacity = '1')}
              onMouseLeave={e => (e.currentTarget.style.opacity = '0.85')}
            >
              🧬 Vitalis on GitHub
            </a>
            <a
              href="https://www.linkedin.com/in/modern-workplace-tech365/"
              target="_blank"
              rel="noopener noreferrer"
              style={{ color: 'var(--accent, #0ea5e9)', opacity: 0.6, textDecoration: 'none', fontSize: '0.75rem', transition: 'opacity 0.2s' }}
              onMouseEnter={e => (e.currentTarget.style.opacity = '1')}
              onMouseLeave={e => (e.currentTarget.style.opacity = '0.6')}
            >
              🔗 LinkedIn
            </a>
            <a
              href="/consulting"
              style={{ color: 'var(--accent, #0ea5e9)', opacity: 0.8, textDecoration: 'none', fontSize: '0.75rem', fontWeight: 600, transition: 'opacity 0.2s', borderBottom: '1px solid var(--accent, #0ea5e9)' }}
              onMouseEnter={e => (e.currentTarget.style.opacity = '1')}
              onMouseLeave={e => (e.currentTarget.style.opacity = '0.8')}
            >
              ⚡ Book a Consulting Session
            </a>
          </div>
          <p className="footer-copy">
            INFINITY v{stats?.version ?? "0.2.0"} &middot; Auto-refreshes every 8s
            {lastUpdate && ` \u00B7 Last: ${lastUpdate.toLocaleTimeString()}`}
          </p>
        </footer>
      </div>

      {/* ═══════════════════════════════════════════════════════
          GLOBAL STYLES — ALIEN THEME
          ═══════════════════════════════════════════════════════ */}
      <style jsx global>{`
        :root {
          --alien-cyan: #00f0ff;
          --alien-magenta: #ff00e5;
          --alien-green: #39ff14;
          --alien-violet: #b026ff;
          --alien-orange: #ff6a00;
          --alien-gold: #ffd700;
          --alien-red: #ff003c;
          --bg-void: #030311;
          --bg-panel: rgba(5,10,30,0.85);
          --bg-card: rgba(0,240,255,0.04);
          --border-glow: rgba(0,240,255,0.12);
          --text-dim: #5a6a8a;
          --font-mono: 'SF Mono','Fira Code','JetBrains Mono','Cascadia Code',monospace;
        }
        html { scroll-behavior: smooth; }

        .alien-page {
          min-height: 100vh;
          background: var(--bg-void);
          color: #c8d6e5;
          position: relative;
          z-index: 1;
          overflow-x: hidden;
          font-family: -apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;
        }

        /* ═══════ STAR FIELD (CSS radial-gradients) ═══════ */
        .star-field {
          position: fixed; inset: 0; z-index: 0; pointer-events: none;
          contain: strict;
          background:
            radial-gradient(2px 2px at 10% 20%, rgba(0,240,255,0.85) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 3% 8%, rgba(255,255,255,0.55) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 20% 50%, rgba(255,0,229,0.7) 50%, transparent 100%),
            radial-gradient(1px 1px at 7% 38%, rgba(255,255,255,0.4) 50%, transparent 100%),
            radial-gradient(2px 2px at 30% 80%, rgba(57,255,20,0.6) 50%, transparent 100%),
            radial-gradient(0.8px 0.8px at 12% 72%, rgba(255,255,255,0.35) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 40% 10%, rgba(176,38,255,0.7) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 50% 60%, rgba(0,240,255,0.6) 50%, transparent 100%),
            radial-gradient(1px 1px at 35% 45%, rgba(255,255,255,0.5) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 60% 30%, rgba(255,215,0,0.6) 50%, transparent 100%),
            radial-gradient(0.8px 0.8px at 55% 22%, rgba(255,255,255,0.35) 50%, transparent 100%),
            radial-gradient(2px 2px at 70% 70%, rgba(255,0,229,0.6) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 80% 40%, rgba(57,255,20,0.7) 50%, transparent 100%),
            radial-gradient(1px 1px at 78% 88%, rgba(255,255,255,0.4) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 90% 90%, rgba(0,240,255,0.7) 50%, transparent 100%),
            radial-gradient(0.8px 0.8px at 85% 15%, rgba(255,255,255,0.35) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 15% 65%, rgba(176,38,255,0.6) 50%, transparent 100%),
            radial-gradient(2px 2px at 25% 35%, rgba(255,215,0,0.6) 50%, transparent 100%),
            radial-gradient(1px 1px at 42% 92%, rgba(255,255,255,0.45) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 45% 85%, rgba(255,106,0,0.6) 50%, transparent 100%),
            radial-gradient(0.8px 0.8px at 62% 5%, rgba(255,255,255,0.35) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 55% 15%, rgba(0,240,255,0.8) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 75% 55%, rgba(255,0,229,0.6) 50%, transparent 100%),
            radial-gradient(1px 1px at 92% 58%, rgba(255,255,255,0.4) 50%, transparent 100%),
            radial-gradient(2px 2px at 95% 25%, rgba(57,255,20,0.7) 50%, transparent 100%),
            radial-gradient(0.8px 0.8px at 18% 95%, rgba(255,255,255,0.35) 50%, transparent 100%),
            radial-gradient(1px 1px at 48% 32%, rgba(0,240,255,0.55) 50%, transparent 100%),
            radial-gradient(0.6px 0.6px at 68% 48%, rgba(255,255,255,0.35) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 88% 72%, rgba(176,38,255,0.55) 50%, transparent 100%),
            radial-gradient(1px 1px at 22% 12%, rgba(255,215,0,0.5) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 5% 55%, rgba(0,240,255,0.65) 50%, transparent 100%),
            radial-gradient(1px 1px at 33% 18%, rgba(255,80,180,0.5) 50%, transparent 100%),
            radial-gradient(1.8px 1.8px at 65% 90%, rgba(80,180,255,0.55) 50%, transparent 100%),
            radial-gradient(0.7px 0.7px at 82% 10%, rgba(255,255,255,0.4) 50%, transparent 100%),
            radial-gradient(1.3px 1.3px at 52% 42%, rgba(176,38,255,0.5) 50%, transparent 100%),
            radial-gradient(0.9px 0.9px at 97% 75%, rgba(255,215,0,0.45) 50%, transparent 100%);
          animation: starTwinkle 8s ease-in-out infinite alternate;
        }
        @keyframes starTwinkle { 0%{opacity:0.7} 100%{opacity:0.9} }

        /* ═══════ NEBULA OVERLAY ═══════ */
        .nebula-overlay {
          position: fixed; inset: -10%; z-index: 0; pointer-events: none;
          contain: layout;
          background:
            radial-gradient(ellipse 900px 600px at 15% 40%, rgba(0,240,255,0.15) 0%, transparent 70%),
            radial-gradient(ellipse 800px 700px at 85% 25%, rgba(255,0,229,0.12) 0%, transparent 70%),
            radial-gradient(ellipse 700px 800px at 50% 80%, rgba(176,38,255,0.12) 0%, transparent 70%),
            radial-gradient(ellipse 600px 500px at 70% 60%, rgba(57,255,20,0.08) 0%, transparent 70%),
            radial-gradient(ellipse 500px 600px at 30% 90%, rgba(255,215,0,0.06) 0%, transparent 70%);
          animation: nebulaPulse 12s ease-in-out infinite alternate;
        }
        @keyframes nebulaPulse { 0%{opacity:0.7} 100%{opacity:0.95} }

        /* ═══════ SCROLL PROGRESS BAR ═══════ */
        .scroll-progress {
          position: fixed; top: 0; left: 0; height: 3px; z-index: 9999;
          background: linear-gradient(90deg,var(--alien-cyan),var(--alien-magenta),var(--alien-violet),var(--alien-cyan));
          background-size: 300% 100%;
          animation: progressGradient 3s linear infinite;
          transition: width 0.1s linear;
          box-shadow: 0 0 10px var(--alien-cyan), 0 0 20px rgba(0,240,255,0.3);
        }
        @keyframes progressGradient { 0%{background-position:0% 0%} 100%{background-position:300% 0%} }

        .particle-canvas { position: fixed; inset: 0; z-index: 0; pointer-events: none; will-change: transform; }

        /* ═══════ AMBIENT GRADIENT SWEEPS ═══════ */
        .alien-page::before {
          content: ''; position: fixed; inset: 0; z-index: 0; pointer-events: none;
          background:
            radial-gradient(ellipse 60% 30% at 0% 0%, rgba(0,240,255,0.08) 0%, transparent 100%),
            radial-gradient(ellipse 50% 40% at 100% 100%, rgba(255,0,229,0.07) 0%, transparent 100%),
            radial-gradient(ellipse 40% 50% at 50% 50%, rgba(176,38,255,0.05) 0%, transparent 100%);
          animation: ambientSweep 16s ease-in-out infinite alternate;
        }
        @keyframes ambientSweep {
          0% { opacity: 0.6; }
          50% { opacity: 1; }
          100% { opacity: 0.7; }
        }

        /* film-grain, grainShift — STRIPPED (component returns null) */

        /* ═══════ CYBERPUNK GRID — SIMPLIFIED ═══════ */
        .cyber-grid {
          position: fixed; inset: 0; z-index: 0; pointer-events: none;
        }
        .cyber-grid-inner {
          position: absolute; width: 100%; height: 100%;
          background-image:
            linear-gradient(rgba(0,240,255,0.05) 1px, transparent 1px),
            linear-gradient(90deg, rgba(0,240,255,0.05) 1px, transparent 1px);
          background-size: 80px 80px;
          mask-image: radial-gradient(ellipse 60% 45% at 50% 50%, rgba(0,0,0,0.5) 0%, transparent 100%);
          -webkit-mask-image: radial-gradient(ellipse 60% 45% at 50% 50%, rgba(0,0,0,0.5) 0%, transparent 100%);
          animation: cyberGridScroll 40s linear infinite;
        }
        @keyframes cyberGridScroll { 0%{background-position:0 0} 100%{background-position:80px 80px} }

        /* ═══════ HOLOGRAPHIC SCAN LINE ═══════ */
        .holo-scan {
          position: fixed; left: 0; right: 0; height: 2px; z-index: 1; pointer-events: none;
          background: linear-gradient(90deg,
            transparent 0%, rgba(0,240,255,0.12) 15%, rgba(0,240,255,0.5) 50%, rgba(0,240,255,0.12) 85%, transparent 100%
          );
          box-shadow: 0 0 16px rgba(0,240,255,0.2), 0 0 45px rgba(0,240,255,0.08),
                      0 -25px 60px rgba(0,240,255,0.025), 0 25px 60px rgba(0,240,255,0.025);
          animation: holoScanSweep 7s ease-in-out infinite;
        }
        .holo-scan::after {
          content: ''; position: absolute; left: 0; right: 0; top: -50px; height: 100px;
          background: linear-gradient(180deg, transparent, rgba(0,240,255,0.025), transparent);
          pointer-events: none;
        }
        @keyframes holoScanSweep {
          0%,100% { top: -2px; opacity: 0; }
          5% { opacity: 1; }
          95% { opacity: 0.4; }
          98% { top: 100vh; opacity: 0; }
        }

        /* ═══════ HOLOGRAPHIC WAVE CANVAS ═══════ */
        .holo-wave-canvas { position: fixed; inset: 0; z-index: 0; pointer-events: none; will-change: transform; }

        /* ═══════ AURORA FIELD ═══════ */
        .aurora-field {
          position: fixed; inset: 0; z-index: 0; pointer-events: none; overflow: hidden;
          contain: layout style;
        }
        .aurora-field::before,
        .aurora-field::after {
          content: ''; position: absolute; width: 130%; height: 100%;
          filter: blur(50px);
          will-change: opacity;
        }
        .aurora-field::before {
          top: -30%; left: -30%;
          background: linear-gradient(
            135deg,
            transparent 25%,
            rgba(0,240,255,0.07) 38%,
            rgba(176,38,255,0.05) 48%,
            rgba(255,0,229,0.04) 58%,
            transparent 72%
          );
          animation: auroraA 18s ease-in-out infinite alternate;
        }
        .aurora-field::after {
          top: -20%; right: -30%;
          background: linear-gradient(
            -135deg,
            transparent 30%,
            rgba(255,0,229,0.06) 42%,
            rgba(57,255,20,0.04) 52%,
            rgba(0,240,255,0.05) 62%,
            transparent 75%
          );
          animation: auroraB 22s ease-in-out infinite alternate;
        }
        @keyframes auroraA {
          0%   { opacity: 0.4; }
          50%  { opacity: 0.8; }
          100% { opacity: 0.5; }
        }
        @keyframes auroraB {
          0%   { opacity: 0.35; }
          50%  { opacity: 0.7; }
          100% { opacity: 0.45; }
        }

        /* ═══════ FLOATING HOLOGRAPHIC SHAPES ═══════ */
        .holo-shapes {
          position: fixed; inset: 0; z-index: 0; pointer-events: none; overflow: hidden;
        }
        .holo-shape {
          position: absolute; border: 1px solid;
          animation: holoFloat 20s ease-in-out infinite;
        }
        .holo-shape:nth-child(1) {
          width: 140px; height: 140px; top: 12%; left: 6%; border-radius: 50%;
          border-color: rgba(0,240,255,0.12);
          box-shadow: inset 0 0 40px rgba(0,240,255,0.03), 0 0 30px rgba(0,240,255,0.04);
          animation-duration: 25s;
        }
        .holo-shape:nth-child(2) {
          width: 100px; height: 100px; top: 55%; right: 10%; border-radius: 50%;
          border-color: rgba(255,0,229,0.12);
          box-shadow: inset 0 0 25px rgba(255,0,229,0.03), 0 0 20px rgba(255,0,229,0.04);
          animation-duration: 18s; animation-delay: -5s;
        }
        .holo-shape:nth-child(3) {
          width: 200px; height: 200px; top: 32%; right: 4%;
          border-color: rgba(176,38,255,0.10);
          box-shadow: inset 0 0 50px rgba(176,38,255,0.02), 0 0 35px rgba(176,38,255,0.03);
          animation-duration: 30s; animation-delay: -10s;
          border-radius: 30% 70% 70% 30% / 30% 30% 70% 70%;
        }
        .holo-shape:nth-child(4) {
          width: 80px; height: 80px; bottom: 18%; left: 15%; border-radius: 50%;
          border-color: rgba(57,255,20,0.12);
          box-shadow: inset 0 0 20px rgba(57,255,20,0.03), 0 0 15px rgba(57,255,20,0.04);
          animation-duration: 22s; animation-delay: -8s;
        }
        .holo-shape:nth-child(5) {
          width: 160px; height: 160px; top: 68%; left: 3%;
          border-color: rgba(255,215,0,0.10);
          box-shadow: inset 0 0 40px rgba(255,215,0,0.015), 0 0 25px rgba(255,215,0,0.02);
          animation-duration: 28s; animation-delay: -15s;
          border-radius: 50% 50% 50% 50% / 60% 40% 60% 40%;
        }
        @keyframes holoFloat {
          0%,100% { transform: translateY(0) rotate(0deg) scale(1); opacity: 0.6; }
          25% { transform: translateY(-35px) rotate(90deg) scale(1.08); opacity: 1; }
          50% { transform: translateY(-18px) rotate(180deg) scale(0.95); opacity: 0.75; }
          75% { transform: translateY(-45px) rotate(270deg) scale(1.04); opacity: 0.65; }
        }

        /* ═══════ DATA STREAM ═══════ */
        .data-stream-overlay { position: fixed; inset: 0; pointer-events: none; z-index: 1; overflow: hidden; }
        .data-char {
          position: absolute; font-family: var(--font-mono); font-size: 10px;
          opacity: 0; animation: dataFall linear infinite; pointer-events: none;
        }
        @keyframes dataFall {
          0%{opacity:0;transform:translateY(-20px)} 10%{opacity:0.35} 90%{opacity:0.18} 100%{opacity:0;transform:translateY(100vh)}
        }

        .container { max-width: 1100px; margin: 0 auto; padding: 0 2rem; position: relative; z-index: 2; }
        .container[id] { scroll-margin-top: 56px; }

        /* ═══════ HERO ═══════ */
        .hero { text-align: center; padding: 100px 24px 60px; position: relative; z-index: 2; }

        /* ═══════ INFINITY LOGO (SVG) ═══════ */
        .hero-logo {
          display: flex; flex-direction: column; align-items: center; justify-content: center;
          margin-bottom: 0.25rem; position: relative;
          animation: symbolFloat 6s ease-in-out infinite;
        }
        .hero-infinity-svg {
          width: clamp(240px, 48vw, 440px); height: auto;
          pointer-events: none; user-select: none;
        }
        .hero-inf-haze {
          animation: hazeBreath 5s ease-in-out infinite;
        }
        .hero-inf-path {
          animation: infGlow 4s ease-in-out infinite;
        }
        .hero-inf-core {
          animation: infPulse 3s ease-in-out infinite;
        }
        .hero-ambient {
          animation: ambientPulse 6s ease-in-out infinite;
        }
        @keyframes hazeBreath {
          0%,100% { opacity: 0.3; }
          50% { opacity: 0.5; }
        }
        @keyframes infGlow {
          0%,100% { opacity: 0.7; }
          50% { opacity: 1; }
        }
        @keyframes infPulse {
          0%,100% { stroke-width: 4.5; opacity: 1; }
          50% { stroke-width: 5.5; opacity: 0.9; }
        }
        @keyframes ambientPulse {
          0%,100% { opacity: 0.04; }
          50% { opacity: 0.09; }
        }
        /* Chromatic aberration layers */
        .hero-chroma-r {
          animation: chromaShiftR 4s ease-in-out infinite;
        }
        .hero-chroma-b {
          animation: chromaShiftB 4s ease-in-out infinite;
        }
        @keyframes chromaShiftR {
          0%,100% { transform: translate(-2px, 0); opacity: 0.15; }
          50% { transform: translate(-4px, 1px); opacity: 0.25; }
        }
        @keyframes chromaShiftB {
          0%,100% { transform: translate(2px, 0); opacity: 0.15; }
          50% { transform: translate(4px, -1px); opacity: 0.25; }
        }
        @keyframes symbolFloat {
          0%,100%{transform:translateY(0) rotate(0deg)} 25%{transform:translateY(-10px) rotate(1deg)}
          50%{transform:translateY(-5px) rotate(0deg)} 75%{transform:translateY(-12px) rotate(-1deg)}
        }
        .hero-h1 {
          font-size: clamp(2.5rem,8vw,5rem); font-weight: 900;
          background: linear-gradient(135deg,var(--alien-cyan),#fff,var(--alien-magenta));
          -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;
          margin: 0 0 0.5rem; line-height: 1; letter-spacing: 0.1em;
        }
        .hero-subtitle { font-size: 1rem; color: #c8d6e5; margin: 0 0 0.5rem; letter-spacing: 0.15em; font-weight: 300; }
        .hero-tagline { font-size: 0.75rem; font-family: var(--font-mono); color: var(--text-dim); opacity: 0.6; margin: 0 0 1.5rem; }
        .hero-status { display: flex; align-items: center; justify-content: center; gap: 12px; }
        .status-dot { width: 10px; height: 10px; border-radius: 50%; background: #4b5563; transition: all 0.3s; }
        .status-dot.alive {
          background: var(--alien-green);
          box-shadow: 0 0 12px var(--alien-green), 0 0 24px rgba(57,255,20,0.3);
          animation: pulseGlow 2s infinite;
        }
        .status-dot.dead { background: var(--alien-red); box-shadow: 0 0 8px var(--alien-red); }
        .status-label { font-size: 12px; letter-spacing: 0.2em; font-family: var(--font-mono); color: var(--text-dim); }
        .last-update { font-size: 10px; color: rgba(90,106,138,0.6); }

        /* ═══════ METRICS BAR ═══════ */
        .metrics-bar {
          display: flex; flex-wrap: wrap; justify-content: center; gap: 0;
          background: var(--bg-panel); border: 1px solid var(--border-glow);
          border-radius: 12px; padding: 1rem; position: relative; overflow: hidden;
        }
        .metrics-bar::before {
          content: ''; position: absolute; top: 0; left: -100%;
          width: 200%; height: 100%;
          background: linear-gradient(90deg,transparent 25%,rgba(0,240,255,0.04) 50%,transparent 75%);
          animation: scanLine 4s linear infinite;
        }
        @keyframes scanLine { 0%{transform:translateX(-50%)} 100%{transform:translateX(50%)} }
        .metric { flex: 1; min-width: 100px; text-align: center; padding: 0.8rem 0.5rem; border-right: 1px solid rgba(0,240,255,0.06); }
        .metric:last-child { border-right: none; }
        .metric-value {
          font-size: 1.4rem; font-weight: 800; font-family: var(--font-mono);
          background: linear-gradient(135deg,var(--alien-cyan),var(--alien-green));
          -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;
          font-variant-numeric: tabular-nums;
        }
        .metric-value.magenta { background: linear-gradient(135deg,var(--alien-magenta),var(--alien-violet)); -webkit-background-clip: text; background-clip: text; }
        .metric-value.gold { background: linear-gradient(135deg,var(--alien-gold),var(--alien-orange)); -webkit-background-clip: text; background-clip: text; }
        .metric-value.green { background: linear-gradient(135deg,var(--alien-green),var(--alien-cyan)); -webkit-background-clip: text; background-clip: text; }
        .metric-label { font-size: 0.65rem; letter-spacing: 0.12em; text-transform: uppercase; color: var(--text-dim); margin-top: 0.3rem; }

        /* ═══════ ENERGY CONNECTOR ═══════ */
        .energy-connector { position: relative; height: 80px; display: flex; justify-content: center; overflow: hidden; z-index: 2; }
        .energy-connector::before {
          content: ''; position: absolute; width: 2px; height: 100%;
          background: linear-gradient(to bottom,transparent,var(--alien-cyan),transparent);
          animation: energyPulse 2s ease-in-out infinite;
          box-shadow: 0 0 8px var(--alien-cyan), 0 0 16px rgba(0,240,255,0.2);
        }
        .energy-connector::after {
          content: ''; position: absolute; width: 10px; height: 10px; border-radius: 50%;
          background: var(--alien-cyan);
          box-shadow: 0 0 15px var(--alien-cyan), 0 0 40px rgba(0,240,255,0.4);
          animation: energyBall 2s ease-in-out infinite; top: -5px;
        }
        @keyframes energyPulse { 0%,100%{opacity:0.3} 50%{opacity:1} }
        @keyframes energyBall { 0%{top:-4px;opacity:0} 10%{opacity:1} 90%{opacity:1} 100%{top:calc(100% - 4px);opacity:0} }

        /* ═══════ SECTION ═══════ */
        .section { position: relative; padding: 1rem 0 2rem; }
        .section::before {
          content: ''; position: absolute; top: 0; left: 0; right: 0; height: 1px;
          background: linear-gradient(90deg,transparent,var(--alien-cyan),transparent); opacity: 0.3;
        }
        .section-header { display: flex; align-items: center; gap: 0.8rem; margin-bottom: 1.5rem; }
        .section-icon { font-size: 1.5rem; }
        .section-title-text {
          font-size: 1.2rem; font-weight: 700; flex: 1; margin: 0;
          background: linear-gradient(90deg,var(--alien-cyan),#fff 80%);
          -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;
        }
        .kinetic-title { display: flex; flex-wrap: wrap; }
        .kinetic-char {
          display: inline-block; opacity: 0;
          transform: translateY(15px) rotateX(-60deg);
          filter: blur(4px);
          transition: opacity 0.4s ease, transform 0.5s cubic-bezier(0.16,1,0.3,1), filter 0.5s ease;
        }
        .kinetic-visible { opacity: 1; transform: translateY(0) rotateX(0); filter: blur(0); }
        .section-badge {
          font-size: 0.65rem; font-family: var(--font-mono); font-weight: 600;
          letter-spacing: 0.1em; padding: 0.25rem 0.7rem; border-radius: 20px; white-space: nowrap;
        }
        .badge-active { color: var(--alien-green); border: 1px solid rgba(57,255,20,0.3); background: rgba(57,255,20,0.08); animation: badgePulse 3s ease-in-out infinite; }
        .badge-native { color: var(--alien-cyan); border: 1px solid rgba(0,240,255,0.3); background: rgba(0,240,255,0.08); }
        .badge-jit { color: var(--alien-magenta); border: 1px solid rgba(255,0,229,0.3); background: rgba(255,0,229,0.08); }
        @keyframes badgePulse { 0%,100%{opacity:1} 50%{opacity:0.6} }

        .section-desc { color: var(--text-dim); margin-bottom: 1.5rem; font-size: 0.9rem; line-height: 1.6; }
        .section-desc code { color: var(--alien-cyan); background: rgba(0,240,255,0.06); padding: 0.1rem 0.3rem; border-radius: 3px; font-family: var(--font-mono); font-size: 0.85em; }
        .subsection-title { color: var(--alien-cyan); font-size: 0.85rem; margin-bottom: 0.8rem; letter-spacing: 0.1em; text-transform: uppercase; }
        .subsection-label { color: var(--alien-cyan); font-size: 0.75rem; margin: 1rem 0 0.5rem; letter-spacing: 0.1em; }

        /* ═══════ GRIDS ═══════ */
        .grid { display: grid; gap: 1rem; }
        .grid-2 { grid-template-columns: repeat(2,1fr); }
        .grid-3 { grid-template-columns: repeat(3,1fr); }
        .grid-4 { grid-template-columns: repeat(4,1fr); }

        /* ═══════ CARD ═══════ */
        .alien-card {
          background: var(--bg-panel); border: 1px solid var(--border-glow); border-radius: 12px;
          padding: 1.2rem; position: relative; overflow: hidden;
          transition: border-color 0.3s ease, box-shadow 0.3s ease;
        }
        .alien-card::after {
          content: ''; position: absolute; bottom: 0; left: 0; right: 0; height: 40%;
          background: linear-gradient(to top,rgba(0,240,255,0.02),transparent);
          pointer-events: none; border-radius: 0 0 12px 12px;
        }
        .alien-card:hover { border-color: var(--alien-cyan); box-shadow: 0 0 30px rgba(0,240,255,0.1), inset 0 0 30px rgba(0,240,255,0.03); }
        .card-featured { border-color: rgba(0,240,255,0.25); }
        .card-header-row { display: flex; align-items: flex-start; gap: 0.8rem; margin-bottom: 0.8rem; }
        .card-icon { font-size: 1.5rem; flex-shrink: 0; }
        .card-title { font-size: 1rem; font-weight: 700; color: #e4e4f0; }
        .card-subtitle { font-size: 0.72rem; font-family: var(--font-mono); color: var(--text-dim); margin-top: 0.15rem; }
        .card-body { font-size: 0.82rem; line-height: 1.6; color: #8a9ab5; }
        .card-body strong { color: #c8d6e5; }
        .card-body code { color: var(--alien-cyan); background: rgba(0,240,255,0.06); padding: 0.1rem 0.3rem; border-radius: 3px; font-family: var(--font-mono); font-size: 0.9em; }
        .card-note { margin-top: 0.8rem; font-size: 0.78rem; color: var(--text-dim); }
        .flow-steps p { margin: 0; font-size: 0.8rem; line-height: 2; }

        /* ═══════ TECH TAGS ═══════ */
        .tags-row { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-top: 0.8rem; }
        .tech-tag { font-size: 0.68rem; font-family: var(--font-mono); font-weight: 600; padding: 0.2rem 0.6rem; border-radius: 4px; letter-spacing: 0.03em; }
        .tag-rust { color: var(--alien-orange); background: rgba(255,106,0,0.1); border: 1px solid rgba(255,106,0,0.25); }
        .tag-python { color: var(--alien-green); background: rgba(57,255,20,0.08); border: 1px solid rgba(57,255,20,0.25); }
        .tag-ts { color: var(--alien-cyan); background: rgba(0,240,255,0.08); border: 1px solid rgba(0,240,255,0.25); }
        .tag-ai { color: var(--alien-magenta); background: rgba(255,0,229,0.08); border: 1px solid rgba(255,0,229,0.25); }
        .tag-infra { color: var(--alien-violet); background: rgba(176,38,255,0.08); border: 1px solid rgba(176,38,255,0.25); }
        .tag-hw { color: var(--alien-gold); background: rgba(255,215,0,0.08); border: 1px solid rgba(255,215,0,0.25); }
        .tag-safety { color: var(--alien-red); background: rgba(255,0,60,0.08); border: 1px solid rgba(255,0,60,0.25); }
        .tag-cyan { color: var(--alien-cyan); background: rgba(0,240,255,0.08); border: 1px solid rgba(0,240,255,0.25); }

        /* ═══════ STAT ROW ═══════ */
        .stat-row { display: flex; justify-content: space-between; padding: 0.35rem 0; border-bottom: 1px solid rgba(0,240,255,0.05); font-size: 0.82rem; }
        .stat-row:last-child { border-bottom: none; }
        .stat-key { color: var(--text-dim); }
        .stat-val { font-family: var(--font-mono); font-weight: 600; }

        /* ═══════ CAP GRID ═══════ */
        .cap-grid { display: grid; grid-template-columns: repeat(2,1fr); gap: 0.6rem; margin-top: 0.8rem; }
        .cap-item { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 0.7rem; background: rgba(0,240,255,0.03); border: 1px solid var(--border-glow); border-radius: 8px; font-size: 0.8rem; font-family: var(--font-mono); }
        .cap-item.supported { border-color: rgba(57,255,20,0.3); }
        .cap-item.unsupported { border-color: rgba(255,0,60,0.15); opacity: 0.5; }
        .cap-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
        .cap-dot.on { background: var(--alien-green); box-shadow: 0 0 8px rgba(57,255,20,0.5); }
        .cap-dot.off { background: rgba(255,0,60,0.4); }

        /* ═══════ PROGRESS BAR (LEGACY) ═══════ */
        .progress-container { margin-bottom: 0.6rem; }
        .progress-label { display: flex; justify-content: space-between; font-size: 0.78rem; font-family: var(--font-mono); margin-bottom: 0.3rem; color: #8a9ab5; }
        .progress-track { height: 6px; background: rgba(0,240,255,0.06); border-radius: 3px; overflow: hidden; }
        .progress-fill { height: 100%; border-radius: 3px; transition: width 1.5s cubic-bezier(0.16,1,0.3,1); box-shadow: 0 0 8px rgba(0,240,255,0.2); }

        /* ═══════ LOC DISTRIBUTION V2 ═══════ */
        .loc-distribution-v2 {
          display: flex;
          align-items: center;
          gap: 2.5rem;
          margin-top: 2rem;
          padding: 2rem;
          background: linear-gradient(135deg, rgba(0,240,255,0.03) 0%, rgba(10,15,30,0.8) 50%, rgba(255,0,229,0.02) 100%);
          border: 1px solid rgba(0,240,255,0.1);
          border-radius: 16px;
          position: relative;
          overflow: hidden;
        }
        .loc-distribution-v2::before {
          content: '';
          position: absolute;
          top: 0; left: 0; right: 0;
          height: 1px;
          background: linear-gradient(90deg, transparent, var(--alien-cyan), var(--alien-magenta), transparent);
          opacity: 0.6;
        }
        .loc-distribution-v2::after {
          content: 'CODE DISTRIBUTION';
          position: absolute;
          top: 12px; right: 16px;
          font-size: 0.55rem;
          letter-spacing: 0.2em;
          color: rgba(0,240,255,0.25);
          font-family: var(--font-mono);
        }
        .loc-donut-wrap {
          flex-shrink: 0;
          position: relative;
        }
        .donut-segment {
          transition: stroke-dashoffset 1.5s cubic-bezier(0.16,1,0.3,1);
        }
        .loc-bars-v2 { flex: 1; min-width: 0; }

        /* ═══════ PROGRESS BAR V2 ═══════ */
        .progress-container-v2 {
          display: grid;
          grid-template-columns: 1fr auto;
          gap: 0.4rem 1rem;
          align-items: center;
          margin-bottom: 1.2rem;
          padding: 0.6rem 0;
          border-bottom: 1px solid rgba(0,240,255,0.04);
        }
        .progress-container-v2:last-child { border-bottom: none; }
        .progress-header-v2 {
          display: flex;
          justify-content: space-between;
          align-items: center;
          grid-column: 1 / -1;
        }
        .progress-label-left {
          display: flex;
          align-items: center;
          gap: 0.5rem;
        }
        .progress-icon { font-size: 1.1rem; }
        .progress-lang {
          font-size: 0.9rem;
          font-weight: 700;
          color: #e0e8f8;
          letter-spacing: 0.02em;
        }
        .progress-label-right { text-align: right; }
        .progress-value-text {
          font-size: 0.85rem;
          font-family: var(--font-mono);
          font-weight: 600;
          color: #8a9ab5;
        }
        .progress-track-v2 {
          grid-column: 1;
          height: 10px;
          background: rgba(0,240,255,0.04);
          border-radius: 5px;
          overflow: visible;
          position: relative;
        }
        .progress-fill-v2 {
          height: 100%;
          border-radius: 5px;
          transition: width 2s cubic-bezier(0.16,1,0.3,1);
          position: relative;
          overflow: hidden;
        }
        .progress-shine {
          position: absolute;
          top: 0; left: -100%;
          width: 200%; height: 100%;
          background: linear-gradient(90deg, transparent 25%, rgba(255,255,255,0.15) 50%, transparent 75%);
          animation: barShine 3s ease-in-out infinite;
        }
        @keyframes barShine {
          0% { transform: translateX(-50%); }
          100% { transform: translateX(50%); }
        }
        .progress-glow-dot {
          position: absolute;
          top: 50%;
          transform: translate(-50%, -50%);
          width: 12px;
          height: 12px;
          border-radius: 50%;
          transition: left 2s cubic-bezier(0.16,1,0.3,1), opacity 0.5s ease;
          z-index: 2;
        }
        .progress-pct-v2 {
          grid-column: 2;
          font-size: 0.95rem;
          font-weight: 900;
          font-family: var(--font-mono);
          min-width: 48px;
          text-align: right;
        }

        /* ═══════ CARD ICON GLOW ═══════ */
        .card-icon {
          filter: drop-shadow(0 0 8px rgba(0,240,255,0.4));
          transition: filter 0.3s ease;
        }
        .alien-card:hover .card-icon {
          filter: drop-shadow(0 0 14px rgba(0,240,255,0.7)) drop-shadow(0 0 24px rgba(0,240,255,0.3));
        }

        /* ═══════ METRIC VALUE PULSE ═══════ */
        .metric-value {
          position: relative;
          transition: transform 0.3s ease;
        }
        .metric:hover .metric-value {
          transform: scale(1.08);
        }

        /* ═══════ SECTION HEADER ACCENT LINE ═══════ */
        .section-header {
          position: relative;
        }
        .section-header::after {
          content: '';
          position: absolute;
          bottom: -8px;
          left: 50%;
          transform: translateX(-50%);
          width: 60px;
          height: 2px;
          background: linear-gradient(90deg, transparent, var(--alien-cyan), transparent);
          border-radius: 1px;
        }

        /* ═══════ TAG HOVER ═══════ */
        .tech-tag {
          transition: transform 0.15s ease, box-shadow 0.3s ease;
        }
        .tech-tag:hover {
          transform: translateY(-1px);
          box-shadow: 0 4px 12px rgba(0,240,255,0.15);
        }

        @media (max-width: 768px) {
          .loc-distribution-v2 {
            flex-direction: column;
            gap: 1.5rem;
            padding: 1.5rem 1rem;
          }
          .loc-distribution-v2::after { display: none; }
        }

        /* ═══════ GAUGE ROW ═══════ */
        .gauge-row { display: flex; justify-content: center; gap: 1.5rem; flex-wrap: wrap; margin: 1rem 0; }

        /* ═══════ INFRA PILLS ═══════ */
        .infra-pills { display: flex; justify-content: center; gap: 0.6rem; margin-top: 0.8rem; flex-wrap: wrap; }
        .infra-pills span { font-size: 0.68rem; font-family: var(--font-mono); color: var(--text-dim); padding: 0.25rem 0.6rem; border: 1px solid var(--border-glow); border-radius: 6px; }

        /* ═══════ PIPELINE ═══════ */
        .pipeline-wrap { margin: 1.5rem 0; }
        .pipeline-row { display: flex; align-items: center; justify-content: center; flex-wrap: wrap; gap: 0; }
        .pipe-item { display: flex; align-items: center; }
        .pipe-stage {
          padding: 0.5rem 1rem; background: var(--bg-panel); border: 1px solid rgba(0,240,255,0.08);
          border-radius: 8px; font-size: 0.72rem; font-family: var(--font-mono); font-weight: 600;
          text-transform: uppercase; letter-spacing: 0.06em; white-space: nowrap;
          transition: all 0.5s cubic-bezier(0.16,1,0.3,1); opacity: 0.4; transform: scale(0.95);
        }
        .pipe-stage.pipe-active { opacity: 1; transform: scale(1); }
        .pipe-arrow { color: var(--alien-cyan); font-size: 1rem; padding: 0 0.25rem; opacity: 0; transition: opacity 0.3s; }
        .pipe-arrow.pipe-active { opacity: 0.7; animation: arrowPulse 2s ease-in-out infinite; }
        @keyframes arrowPulse { 0%,100%{opacity:0.5} 50%{opacity:1} }
        .pipeline-ffi-note { text-align: center; margin-top: 0.5rem; color: var(--text-dim); font-size: 0.7rem; font-family: var(--font-mono); }

        /* ═══════ ARCHITECTURE DIAGRAM (Visual) ═══════ */
        .arch-visual { position: relative; overflow: hidden; border-radius: 16px; }
        .arch-scanline {
          position: absolute; inset: 0; pointer-events: none; z-index: 2; border-radius: 16px;
          background: linear-gradient(to bottom, transparent 0%, rgba(0,240,255,0.03) 50%, transparent 100%);
          background-size: 100% 6px;
          animation: archScan 6s linear infinite;
        }
        @keyframes archScan {
          0% { background-position: 0 -100%; }
          100% { background-position: 0 300%; }
        }
        .arch-frame {
          position: relative;
          background: rgba(3,3,17,0.85);
          border: 1px solid rgba(0,240,255,0.15);
          border-radius: 16px;
          padding: 2rem 1.5rem;
          display: flex; flex-direction: column; align-items: center; gap: 0;
          box-shadow: inset 0 0 80px rgba(0,240,255,0.02), 0 0 40px rgba(0,240,255,0.05);
        }
        .arch-frame-title {
          font-family: var(--font-mono); font-size: 0.85rem; letter-spacing: 0.35em;
          color: var(--alien-cyan); text-align: center; margin-bottom: 1.5rem;
          text-shadow: 0 0 20px rgba(0,240,255,0.5);
          display: flex; align-items: center; gap: 1rem;
        }
        .arch-glyph { opacity: 0.4; font-size: 0.7rem; }

        /* Tier row */
        .arch-tier { width: 100%; max-width: 720px; display: flex; justify-content: center; gap: 1rem; }
        .arch-tier--triple { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 0.75rem; max-width: 780px; }

        /* Box */
        .arch-box {
          flex: 1; border-radius: 12px; padding: 1rem 1.2rem;
          background: rgba(8,8,30,0.7); position: relative; overflow: hidden;
          transition: transform 0.3s, box-shadow 0.3s;
        }
        .arch-box:hover { transform: translateY(-2px); }
        .arch-box::before {
          content: ''; position: absolute; inset: 0; border-radius: 12px;
          border: 1px solid transparent; pointer-events: none;
        }
        .arch-box::after {
          content: ''; position: absolute; top: 0; left: 0; right: 0; height: 2px;
          border-radius: 12px 12px 0 0;
        }
        /* Color variants */
        .arch-box--cyan::before { border-color: rgba(0,240,255,0.25); }
        .arch-box--cyan::after { background: linear-gradient(90deg, transparent, rgba(0,240,255,0.6), transparent); }
        .arch-box--cyan:hover { box-shadow: 0 4px 30px rgba(0,240,255,0.15), inset 0 0 30px rgba(0,240,255,0.03); }

        .arch-box--green::before { border-color: rgba(57,255,20,0.25); }
        .arch-box--green::after { background: linear-gradient(90deg, transparent, rgba(57,255,20,0.6), transparent); }
        .arch-box--green:hover { box-shadow: 0 4px 30px rgba(57,255,20,0.15), inset 0 0 30px rgba(57,255,20,0.03); }

        .arch-box--magenta::before { border-color: rgba(255,0,255,0.25); }
        .arch-box--magenta::after { background: linear-gradient(90deg, transparent, rgba(255,0,255,0.6), transparent); }
        .arch-box--magenta:hover { box-shadow: 0 4px 30px rgba(255,0,255,0.15), inset 0 0 30px rgba(255,0,255,0.03); }

        .arch-box--gold::before { border-color: rgba(255,215,0,0.25); }
        .arch-box--gold::after { background: linear-gradient(90deg, transparent, rgba(255,215,0,0.6), transparent); }
        .arch-box--gold:hover { box-shadow: 0 4px 30px rgba(255,215,0,0.15), inset 0 0 30px rgba(255,215,0,0.03); }

        .arch-box--violet::before { border-color: rgba(138,43,226,0.3); }
        .arch-box--violet::after { background: linear-gradient(90deg, transparent, rgba(138,43,226,0.6), transparent); }
        .arch-box--violet:hover { box-shadow: 0 4px 30px rgba(138,43,226,0.15), inset 0 0 30px rgba(138,43,226,0.03); }

        .arch-box--orange::before { border-color: rgba(255,106,0,0.25); }
        .arch-box--orange::after { background: linear-gradient(90deg, transparent, rgba(255,106,0,0.6), transparent); }
        .arch-box--orange:hover { box-shadow: 0 4px 30px rgba(255,106,0,0.15), inset 0 0 30px rgba(255,106,0,0.03); }

        .arch-box--red::before { border-color: rgba(255,50,50,0.25); }
        .arch-box--red::after { background: linear-gradient(90deg, transparent, rgba(255,50,50,0.6), transparent); }
        .arch-box--red:hover { box-shadow: 0 4px 30px rgba(255,50,50,0.15), inset 0 0 30px rgba(255,50,50,0.03); }

        /* Box header */
        .arch-box-header { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.6rem; }
        .arch-box-icon { font-size: 0.85rem; }
        .arch-box-label { font-family: var(--font-mono); font-size: 0.72rem; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
        .arch-box--cyan .arch-box-label { color: var(--alien-cyan); }
        .arch-box--green .arch-box-label { color: var(--alien-green); }
        .arch-box--magenta .arch-box-label { color: var(--alien-magenta); }
        .arch-box--gold .arch-box-label { color: var(--alien-gold); }
        .arch-box--violet .arch-box-label { color: var(--alien-violet); }
        .arch-box--orange .arch-box-label { color: var(--alien-orange); }
        .arch-box--red .arch-box-label { color: var(--alien-red); }
        .arch-box-port {
          font-family: var(--font-mono); font-size: 0.6rem; padding: 0.1rem 0.4rem;
          border-radius: 4px; background: rgba(255,255,255,0.05); color: var(--text-dim);
          margin-left: auto;
        }

        /* Chips */
        .arch-box-chips { display: flex; flex-wrap: wrap; gap: 0.3rem; margin-bottom: 0.5rem; }
        .arch-chip {
          font-family: var(--font-mono); font-size: 0.55rem; padding: 0.15rem 0.45rem;
          border-radius: 4px; letter-spacing: 0.03em;
          background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06);
        }
        .arch-chip--cyan { color: rgba(0,240,255,0.85); border-color: rgba(0,240,255,0.15); background: rgba(0,240,255,0.05); }
        .arch-chip--green { color: rgba(57,255,20,0.85); border-color: rgba(57,255,20,0.15); background: rgba(57,255,20,0.05); }
        .arch-chip--magenta { color: rgba(255,0,255,0.85); border-color: rgba(255,0,255,0.15); background: rgba(255,0,255,0.05); }
        .arch-chip--gold { color: rgba(255,215,0,0.85); border-color: rgba(255,215,0,0.15); background: rgba(255,215,0,0.05); }
        .arch-chip--violet { color: rgba(138,43,226,0.85); border-color: rgba(138,43,226,0.2); background: rgba(138,43,226,0.05); }
        .arch-chip--orange { color: rgba(255,106,0,0.85); border-color: rgba(255,106,0,0.15); background: rgba(255,106,0,0.05); }
        .arch-chip--red { color: rgba(255,50,50,0.85); border-color: rgba(255,50,50,0.15); background: rgba(255,50,50,0.05); }
        .arch-chip--dim { color: var(--text-dim); }

        /* Box tech line */
        .arch-box-tech { font-family: var(--font-mono); font-size: 0.55rem; color: var(--text-dim); opacity: 0.6; }

        /* Box stat (active indicator) */
        .arch-box-stat {
          display: flex; align-items: center; gap: 0.4rem; font-family: var(--font-mono);
          font-size: 0.55rem; color: var(--text-dim); margin-top: 0.3rem;
        }
        .arch-stat-dot {
          width: 6px; height: 6px; border-radius: 50%;
          animation: statPulse 2s ease-in-out infinite;
        }
        .arch-stat-dot--gold { background: var(--alien-gold); box-shadow: 0 0 6px rgba(255,215,0,0.5); }
        .arch-stat-dot--violet { background: var(--alien-violet); box-shadow: 0 0 6px rgba(138,43,226,0.5); }
        .arch-stat-dot--orange { background: var(--alien-orange); box-shadow: 0 0 6px rgba(255,106,0,0.5); }
        @keyframes statPulse { 0%,100%{opacity:0.5;transform:scale(1)} 50%{opacity:1;transform:scale(1.3)} }

        /* Laws */
        .arch-box-laws { display: flex; gap: 0.5rem; margin-bottom: 0.6rem; flex-wrap: wrap; }
        .arch-law {
          display: flex; align-items: center; gap: 0.35rem; font-family: var(--font-mono);
          font-size: 0.55rem; color: rgba(255,50,50,0.7);
          padding: 0.2rem 0.5rem; border-radius: 6px;
          background: rgba(255,50,50,0.04); border: 1px solid rgba(255,50,50,0.1);
        }
        .arch-law-num {
          font-weight: 800; font-size: 0.6rem; color: rgba(255,50,50,0.9);
          min-width: 1.1rem; text-align: center;
        }

        /* Connectors */
        .arch-connector {
          display: flex; flex-direction: column; align-items: center; gap: 0; padding: 0.2rem 0;
        }
        .arch-connector--triple { flex-direction: row; justify-content: center; gap: 4rem; padding: 0.6rem 0; }
        .arch-line {
          width: 2px; height: 18px; border-radius: 1px; position: relative; overflow: hidden;
        }
        .arch-connector--triple .arch-line { width: 2px; height: 22px; }
        .arch-line::after {
          content: ''; position: absolute; top: -100%; left: 0; width: 100%; height: 100%;
          animation: dataFlow 1.5s linear infinite;
        }
        @keyframes dataFlow { 0%{top:-100%} 100%{top:200%} }
        .arch-line--cyan { background: rgba(0,240,255,0.15); }
        .arch-line--cyan::after { background: linear-gradient(to bottom, transparent, rgba(0,240,255,0.8), transparent); }
        .arch-line--green { background: rgba(57,255,20,0.15); }
        .arch-line--green::after { background: linear-gradient(to bottom, transparent, rgba(57,255,20,0.8), transparent); }
        .arch-line--gold { background: rgba(255,215,0,0.15); }
        .arch-line--gold::after { background: linear-gradient(to bottom, transparent, rgba(255,215,0,0.8), transparent); }
        .arch-line--violet { background: rgba(138,43,226,0.15); }
        .arch-line--violet::after { background: linear-gradient(to bottom, transparent, rgba(138,43,226,0.8), transparent); }
        .arch-line--orange { background: rgba(255,106,0,0.15); }
        .arch-line--orange::after { background: linear-gradient(to bottom, transparent, rgba(255,106,0,0.8), transparent); }

        .arch-connector-label {
          font-family: var(--font-mono); font-size: 0.58rem; letter-spacing: 0.06em;
          padding: 0.15rem 0.6rem; border-radius: 4px;
          background: rgba(8,8,30,0.9); border: 1px solid rgba(255,255,255,0.06);
        }
        .arch-connector-label--cyan { color: rgba(0,240,255,0.7); border-color: rgba(0,240,255,0.12); }
        .arch-connector-label--green { color: rgba(57,255,20,0.7); border-color: rgba(57,255,20,0.12); }
        .arch-connector-label--orange { color: rgba(255,106,0,0.7); border-color: rgba(255,106,0,0.12); }

        .hl-cyan { color: var(--alien-cyan); }
        .hl-green { color: var(--alien-green); }
        .hl-magenta { color: var(--alien-magenta); }
        .hl-gold { color: var(--alien-gold); }
        .hl-violet { color: var(--alien-violet); }
        .hl-orange { color: var(--alien-orange); }
        .hl-red { color: var(--alien-red); }

        /* ═══════ FILE TABLE ═══════ */
        .file-table-wrap { overflow-x: auto; margin-top: 0.8rem; }
        .file-table { width: 100%; border-collapse: collapse; font-size: 0.78rem; }
        .file-table th { text-align: left; padding: 0.5rem 0.8rem; font-size: 0.68rem; letter-spacing: 0.08em; text-transform: uppercase; color: var(--text-dim); border-bottom: 1px solid var(--border-glow); font-family: var(--font-mono); }
        .file-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid rgba(0,240,255,0.04); color: #8a9ab5; font-size: 0.75rem; }
        .file-table tr:hover td { background: rgba(0,240,255,0.03); }
        .file-name { font-family: var(--font-mono); font-weight: 600; color: var(--alien-cyan) !important; }
        .loc-bar { display: block; height: 6px; border-radius: 3px; transition: width 1s cubic-bezier(0.16,1,0.3,1); box-shadow: 0 0 4px rgba(0,240,255,0.2); }

        /* ═══════ MODULE CLOUD ═══════ */
        .module-cloud { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-top: 0.8rem; }
        .module-tag {
          padding: 0.2rem 0.6rem; border-radius: 4px; background: rgba(176,38,255,0.06);
          border: 1px solid rgba(176,38,255,0.15); color: var(--alien-violet);
          font-family: var(--font-mono); font-size: 0.68rem;
          animation: fadeSlideIn 0.5s ease forwards; opacity: 0; transition: all 0.2s;
        }
        .module-tag:hover { background: rgba(176,38,255,0.12); border-color: var(--alien-violet); transform: scale(1.05); }

        .dep-heading { font-size: 0.7rem; letter-spacing: 0.1em; margin: 1rem 0 0.4rem; }
        .dep-heading:first-child { margin-top: 0; }

        /* ═══════ FOOTER ═══════ */
        .alien-footer {
          text-align: center; padding: 3rem 0 2rem; color: var(--text-dim);
          font-size: 0.75rem; font-family: var(--font-mono);
          border-top: 1px solid var(--border-glow); position: relative; z-index: 2; margin-top: 2rem;
        }
        .footer-symbol {
          font-size: 2rem; display: block; margin-bottom: 0.5rem;
          background: linear-gradient(135deg,var(--alien-cyan),var(--alien-magenta));
          -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;
        }
        .footer-stats { margin-top: 0.3rem; opacity: 0.5; }
        .footer-tech { margin-top: 0.3rem; opacity: 0.35; }
        .footer-copy { margin-top: 1rem; opacity: 0.25; font-size: 0.65rem; }

        /* ═══════ VITALIS FLAGSHIP PANEL ═══════ */
        .vitalis-flagship {
          position: relative;
          margin: 2rem 0;
          padding: 3rem 2.5rem;
          border-radius: 24px;
          background: linear-gradient(145deg, rgba(57,255,20,0.06) 0%, rgba(0,240,255,0.04) 50%, rgba(176,38,255,0.04) 100%);
          border: 2px solid rgba(57,255,20,0.25);
          overflow: hidden;
        }
        .vitalis-flagship-glow {
          position: absolute;
          top: -60%;
          left: 20%;
          width: 60%;
          height: 200%;
          background: radial-gradient(ellipse, rgba(57,255,20,0.08) 0%, transparent 65%);
          pointer-events: none;
          animation: vitalisGlowPulse 6s ease-in-out infinite;
        }
        @keyframes vitalisGlowPulse {
          0%, 100% { opacity: 0.5; transform: scale(1); }
          50% { opacity: 1; transform: scale(1.15); }
        }
        .vitalis-flagship-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 3rem;
          position: relative;
          z-index: 1;
        }
        @media (max-width: 900px) {
          .vitalis-flagship-grid { grid-template-columns: 1fr; gap: 2rem; }
          .vitalis-flagship { padding: 2rem 1.5rem; }
        }
        .vitalis-flagship-hero {
          display: flex;
          flex-direction: column;
          justify-content: center;
        }
        .vitalis-flagship-badge {
          display: inline-block;
          padding: 0.35rem 1rem;
          font-size: 0.7rem;
          font-weight: 800;
          letter-spacing: 0.15em;
          text-transform: uppercase;
          color: #39ff14;
          background: rgba(57,255,20,0.12);
          border: 1px solid rgba(57,255,20,0.3);
          border-radius: 20px;
          margin-bottom: 1rem;
          width: fit-content;
          animation: badgePulse 3s ease-in-out infinite;
        }
        @keyframes badgePulse {
          0%, 100% { box-shadow: 0 0 8px rgba(57,255,20,0.15); }
          50% { box-shadow: 0 0 20px rgba(57,255,20,0.35); }
        }
        .vitalis-flagship-title {
          font-size: 3.2rem;
          font-weight: 900;
          letter-spacing: -0.02em;
          background: linear-gradient(135deg, #39ff14 0%, #00f0ff 50%, #b026ff 100%);
          -webkit-background-clip: text;
          -webkit-text-fill-color: transparent;
          background-clip: text;
          margin: 0 0 0.4rem;
          line-height: 1.1;
        }
        .vitalis-flagship-version {
          font-size: 1.2rem;
          font-weight: 600;
          vertical-align: super;
          margin-left: 0.3rem;
          opacity: 0.7;
        }
        .vitalis-flagship-tagline {
          color: #8a9ab5;
          font-size: 0.95rem;
          line-height: 1.7;
          margin: 0.8rem 0 1.5rem;
        }
        .vitalis-flagship-stats-row {
          display: flex;
          gap: 1.5rem;
          margin-bottom: 1.8rem;
          flex-wrap: wrap;
        }
        .vitalis-flagship-stat {
          display: flex;
          flex-direction: column;
          text-align: center;
        }
        .vitalis-flagship-stat-value {
          font-size: 1.8rem;
          font-weight: 900;
          color: #39ff14;
          text-shadow: 0 0 15px rgba(57,255,20,0.3);
          line-height: 1.1;
        }
        .vitalis-flagship-stat-label {
          font-size: 0.65rem;
          text-transform: uppercase;
          letter-spacing: 0.1em;
          color: #5a6a8a;
          margin-top: 0.15rem;
        }
        .vitalis-flagship-buttons {
          display: flex;
          gap: 1rem;
          flex-wrap: wrap;
        }
        .vitalis-flagship-btn-primary {
          display: inline-flex;
          align-items: center;
          padding: 0.9rem 2rem;
          background: linear-gradient(135deg, #39ff14, #00f0ff);
          color: #000;
          border-radius: 12px;
          font-weight: 800;
          font-size: 0.9rem;
          text-decoration: none;
          transition: transform 0.15s, box-shadow 0.3s;
          letter-spacing: 0.02em;
        }
        .vitalis-flagship-btn-primary:hover {
          transform: translateY(-3px) scale(1.02);
          box-shadow: 0 8px 40px rgba(57,255,20,0.4), 0 4px 20px rgba(0,240,255,0.2);
        }
        .vitalis-flagship-btn-secondary {
          padding: 0.9rem 2rem;
          border: 1px solid rgba(57,255,20,0.35);
          color: #39ff14;
          border-radius: 12px;
          font-weight: 700;
          font-size: 0.9rem;
          text-decoration: none;
          transition: border-color 0.3s, background 0.3s;
        }
        .vitalis-flagship-btn-secondary:hover {
          border-color: #39ff14;
          background: rgba(57,255,20,0.08);
        }

        /* Feature cards grid */
        .vitalis-flagship-features {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 0.8rem;
        }
        @media (max-width: 600px) {
          .vitalis-flagship-features { grid-template-columns: 1fr; }
        }
        .vitalis-feature-card {
          padding: 1rem 1.2rem;
          border-radius: 12px;
          background: rgba(57,255,20,0.04);
          border: 1px solid rgba(57,255,20,0.12);
          transition: border-color 0.3s, background 0.3s, transform 0.2s;
        }
        .vitalis-feature-card:hover {
          border-color: rgba(57,255,20,0.4);
          background: rgba(57,255,20,0.08);
          transform: translateY(-2px);
        }
        .vitalis-feature-icon {
          font-size: 1.5rem;
          margin-bottom: 0.3rem;
        }
        .vitalis-feature-title {
          font-size: 0.85rem;
          font-weight: 700;
          color: #e0e8f0;
          margin-bottom: 0.2rem;
        }
        .vitalis-feature-desc {
          font-size: 0.72rem;
          color: #5a6a8a;
          line-height: 1.4;
        }

        /* Pipeline mini-viz */
        .vitalis-flagship-pipeline {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 0.4rem;
          margin-top: 2rem;
          padding-top: 1.5rem;
          border-top: 1px solid rgba(57,255,20,0.1);
          flex-wrap: wrap;
          position: relative;
          z-index: 1;
        }
        .pipeline-step {
          padding: 0.35rem 0.7rem;
          font-size: 0.65rem;
          font-weight: 600;
          color: #8a9ab5;
          background: rgba(255,255,255,0.03);
          border: 1px solid rgba(255,255,255,0.08);
          border-radius: 6px;
          white-space: nowrap;
        }
        .pipeline-arrow {
          color: #39ff14;
          font-size: 0.8rem;
          opacity: 0.6;
        }
        .vitalis-pipeline-jit {
          color: #39ff14;
          border-color: rgba(57,255,20,0.3);
          background: rgba(57,255,20,0.08);
          font-weight: 800;
        }
        .vitalis-pipeline-native {
          color: #00f0ff;
          border-color: rgba(0,240,255,0.3);
          background: rgba(0,240,255,0.08);
          font-weight: 800;
        }

        /* ═══════ CONSULTING CTA BANNER ═══════ */
        .consulting-cta-banner {
          position: relative;
          margin: 3rem auto 0;
          padding: 3rem 2rem;
          border-radius: 20px;
          border: 1px solid rgba(0,240,255,0.2);
          background: linear-gradient(135deg, rgba(0,240,255,0.04) 0%, rgba(57,255,20,0.04) 50%, rgba(0,240,255,0.04) 100%);
          overflow: hidden;
          text-align: center;
          max-width: 900px;
        }
        .consulting-cta-glow {
          position: absolute;
          top: -50%;
          left: 50%;
          transform: translateX(-50%);
          width: 500px;
          height: 500px;
          border-radius: 50%;
          background: radial-gradient(circle, rgba(0,240,255,0.12), transparent 70%);
          filter: blur(60px);
          pointer-events: none;
          animation: cta-pulse 4s ease-in-out infinite;
        }
        @keyframes cta-pulse {
          0%, 100% { opacity: 0.5; transform: translateX(-50%) scale(1); }
          50% { opacity: 1; transform: translateX(-50%) scale(1.15); }
        }
        .consulting-cta-content {
          position: relative;
          z-index: 1;
        }
        .consulting-cta-badge {
          display: inline-block;
          padding: 0.35rem 1.2rem;
          font-size: 0.65rem;
          font-weight: 800;
          letter-spacing: 0.2em;
          text-transform: uppercase;
          color: #000;
          background: linear-gradient(135deg, #00f0ff, #39ff14);
          border-radius: 100px;
          margin-bottom: 1.2rem;
          animation: badge-glow 2s ease-in-out infinite;
        }
        @keyframes badge-glow {
          0%, 100% { box-shadow: 0 0 20px rgba(0,240,255,0.3); }
          50% { box-shadow: 0 0 40px rgba(0,240,255,0.6), 0 0 80px rgba(57,255,20,0.2); }
        }
        .consulting-cta-title {
          font-size: clamp(1.4rem, 3vw, 2.2rem);
          font-weight: 900;
          line-height: 1.2;
          margin: 0 0 0.8rem;
          background: linear-gradient(135deg, #00f0ff, #39ff14);
          -webkit-background-clip: text;
          -webkit-text-fill-color: transparent;
          background-clip: text;
        }
        .consulting-cta-desc {
          color: #8a9ab5;
          font-size: 0.9rem;
          line-height: 1.6;
          margin: 0 0 1.8rem;
          max-width: 600px;
          margin-left: auto;
          margin-right: auto;
        }
        .consulting-cta-buttons {
          display: flex;
          gap: 1rem;
          justify-content: center;
          flex-wrap: wrap;
          margin-bottom: 1.5rem;
        }
        .consulting-cta-btn-primary {
          padding: 0.9rem 2.2rem;
          background: linear-gradient(135deg, #00f0ff, #39ff14);
          color: #000;
          border-radius: 10px;
          font-weight: 800;
          font-size: 0.9rem;
          text-decoration: none;
          transition: transform 0.15s, box-shadow 0.3s;
          letter-spacing: 0.02em;
        }
        .consulting-cta-btn-primary:hover {
          transform: translateY(-3px);
          box-shadow: 0 8px 40px rgba(0,240,255,0.4), 0 4px 20px rgba(57,255,20,0.2);
        }
        .consulting-cta-btn-secondary {
          padding: 0.9rem 2.2rem;
          border: 1px solid rgba(0,240,255,0.35);
          color: #00f0ff;
          border-radius: 10px;
          font-weight: 700;
          font-size: 0.9rem;
          text-decoration: none;
          transition: border-color 0.3s, background 0.3s;
        }
        .consulting-cta-btn-secondary:hover {
          border-color: #00f0ff;
          background: rgba(0,240,255,0.08);
        }
        .consulting-cta-stats {
          display: flex;
          gap: 2rem;
          justify-content: center;
          flex-wrap: wrap;
          font-size: 0.75rem;
          color: #5a6a8a;
        }

        /* ═══════ FLOATING CONSULTING BUTTON ═══════ */
        .consulting-float-btn {
          position: fixed;
          bottom: 2rem;
          right: 2rem;
          z-index: 9999;
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.7rem 1.4rem;
          background: linear-gradient(135deg, #00f0ff, #39ff14);
          color: #000;
          border-radius: 100px;
          font-weight: 800;
          font-size: 0.8rem;
          text-decoration: none;
          box-shadow: 0 4px 30px rgba(0,240,255,0.35);
          transition: transform 0.15s, box-shadow 0.3s;
          overflow: hidden;
        }
        .consulting-float-btn:hover {
          transform: translateY(-3px) scale(1.05);
          box-shadow: 0 8px 50px rgba(0,240,255,0.5), 0 4px 20px rgba(57,255,20,0.3);
        }
        .consulting-float-pulse {
          position: absolute;
          inset: -4px;
          border-radius: inherit;
          border: 2px solid rgba(0,240,255,0.6);
          animation: float-ping 2s ease-out infinite;
          pointer-events: none;
        }
        @keyframes float-ping {
          0% { opacity: 1; transform: scale(1); }
          100% { opacity: 0; transform: scale(1.5); }
        }
        .consulting-float-icon { font-size: 1rem; position: relative; z-index: 1; }
        .consulting-float-text { position: relative; z-index: 1; letter-spacing: 0.05em; }

        /* ═══════ SCROLL REVEAL (smooth, no blur — prevents box twitching) ═══════ */
        .scroll-reveal {
          opacity: 0; transform: translateY(24px);
          transition: opacity 0.7s cubic-bezier(0.16,1,0.3,1),
                      transform 0.7s cubic-bezier(0.16,1,0.3,1);
        }
        .scroll-reveal.revealed { opacity: 1; transform: translateY(0); }

        /* ═══════ SCROLLBAR ═══════ */
        ::-webkit-scrollbar { width: 6px; }
        ::-webkit-scrollbar-track { background: var(--bg-void); }
        ::-webkit-scrollbar-thumb { background: rgba(0,240,255,0.2); border-radius: 3px; }
        ::-webkit-scrollbar-thumb:hover { background: rgba(0,240,255,0.4); }

        @keyframes pulseGlow { 0%,100%{opacity:1} 50%{opacity:0.4} }
        @keyframes fadeSlideIn { from{opacity:0;transform:translateY(8px)} to{opacity:0.7;transform:translateY(0)} }

        /* ═══════ FLOATING NAV ═══════ */
        .nav-float {
          position: fixed; right: 1.5rem; top: 50%; transform: translateY(-50%);
          z-index: 100; display: flex; flex-direction: column; gap: 0.5rem;
        }
        .nav-dot {
          width: 10px; height: 10px; border-radius: 50%;
          background: rgba(0,240,255,0.2); border: 1px solid rgba(0,240,255,0.3);
          cursor: pointer; transition: all 0.3s; text-decoration: none; display: block;
          position: relative;
        }
        .nav-dot:hover {
          background: var(--alien-cyan);
          box-shadow: 0 0 12px rgba(0,240,255,0.5);
          transform: scale(1.3);
        }
        .nav-dot.nav-active {
          background: var(--alien-cyan);
          box-shadow: 0 0 12px rgba(0,240,255,0.6);
          transform: scale(1.4);
        }
        .nav-dot::after {
          content: attr(title); position: absolute; right: 20px;
          font-size: 0.6rem; font-family: var(--font-mono); color: var(--alien-cyan);
          white-space: nowrap; opacity: 0; pointer-events: none; transition: opacity 0.2s;
          top: 50%; transform: translateY(-50%);
        }
        .nav-dot:hover::after { opacity: 1; }

        /* ═══════ CIRCUIT TRACES ═══════ */
        .circuit-line {
          stroke: var(--alien-cyan); stroke-width: 1.2; fill: none;
          opacity: 0.25; stroke-dasharray: 12 6;
          animation: circuitFlow 3s linear infinite;
          filter: drop-shadow(0 0 3px currentColor);
        }
        .circuit-line.magenta { stroke: var(--alien-magenta); }
        .circuit-line.green { stroke: var(--alien-green); }
        @keyframes circuitFlow { 0%{stroke-dashoffset:0} 100%{stroke-dashoffset:-36} }
        .circuit-node {
          fill: var(--alien-cyan); opacity: 0;
          animation: circuitNodePulse 3s ease-in-out infinite;
          filter: drop-shadow(0 0 4px currentColor);
        }
        @keyframes circuitNodePulse { 0%,100%{opacity:0;r:2} 50%{opacity:0.7;r:5} }

        /* ═══════ STICKY SECTION NAV BAR ═══════ */
        .sticky-nav {
          position: fixed; top: 0; left: 0; right: 0; z-index: 999;
          transform: translateY(-100%);
          transition: transform 0.35s cubic-bezier(0.16,1,0.3,1);
          background: rgba(6,6,20,0.85);
          border-bottom: 1px solid var(--border-glow);
          backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
        }
        .sticky-nav-visible { transform: translateY(0); }
        .snav-inner {
          display: flex; gap: 0; overflow-x: auto; overflow-y: hidden;
          max-width: 1100px; margin: 0 auto; padding: 0 1rem;
          scrollbar-width: none; -ms-overflow-style: none;
        }
        .snav-inner::-webkit-scrollbar { display: none; }
        .snav-link {
          flex-shrink: 0; padding: 0.65rem 0.9rem; font-size: 0.68rem;
          font-family: var(--font-mono); font-weight: 600; letter-spacing: 0.06em;
          text-transform: uppercase; color: var(--text-dim);
          text-decoration: none; white-space: nowrap; position: relative;
          transition: color 0.25s;
        }
        .snav-link::after {
          content: ''; position: absolute; bottom: 0; left: 0.9rem; right: 0.9rem; height: 2px;
          background: var(--alien-cyan); transform: scaleX(0); transition: transform 0.3s cubic-bezier(0.16,1,0.3,1);
          box-shadow: 0 0 8px var(--alien-cyan);
        }
        .snav-link:hover { color: #c8d6e5; }
        .snav-link.snav-active { color: var(--alien-cyan); }
        .snav-link.snav-active::after { transform: scaleX(1); }

        /* ═══════ CINEMATIC LOADING SCREEN ═══════ */
        .cine-loader {
          position: fixed; inset: 0; z-index: 99999;
          background: #030311;
          display: flex; align-items: center; justify-content: center;
          transition: opacity 0.9s cubic-bezier(0.16,1,0.3,1), visibility 0.9s;
        }
        .cine-done { opacity: 0; visibility: hidden; pointer-events: none; }
        .cine-inner {
          display: flex; flex-direction: column; align-items: center; gap: 1rem;
          position: relative; z-index: 2;
        }
        .cine-symbol {
          font-size: 5rem; font-weight: 900;
          background: linear-gradient(135deg, var(--alien-cyan), var(--alien-magenta));
          -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;
          animation: symbolFloat 3s ease-in-out infinite;
          filter: drop-shadow(0 0 30px rgba(0,240,255,0.4));
        }
        .cine-title {
          font-family: var(--font-mono); font-size: 1.4rem; font-weight: 800;
          letter-spacing: 0.6em; color: rgba(0,240,255,0.7);
          text-shadow: 0 0 30px rgba(0,240,255,0.3);
        }
        .cine-bar-track {
          width: 240px; height: 2px;
          background: rgba(0,240,255,0.08); border-radius: 1px; overflow: hidden;
        }
        .cine-bar-fill {
          height: 100%; border-radius: 1px;
          background: linear-gradient(90deg, var(--alien-cyan), var(--alien-magenta), var(--alien-violet));
          background-size: 300% 100%;
          animation: cineGrad 2s linear infinite;
          box-shadow: 0 0 15px var(--alien-cyan);
          transition: width 0.05s linear;
        }
        @keyframes cineGrad { 0%{background-position:0% 0%} 100%{background-position:300% 0%} }
        .cine-pct {
          font-family: var(--font-mono); font-size: 0.8rem;
          color: var(--alien-cyan); letter-spacing: 0.25em;
          font-variant-numeric: tabular-nums;
          text-shadow: 0 0 10px rgba(0,240,255,0.5);
        }
        .cine-label {
          font-family: var(--font-mono); font-size: 0.55rem;
          color: var(--text-dim); letter-spacing: 0.35em;
          animation: labelFlicker 2s ease-in-out infinite;
        }
        @keyframes labelFlicker { 0%,100%{opacity:0.3} 50%{opacity:0.7} }
        .cine-scanlines {
          position: absolute; inset: 0; pointer-events: none; z-index: 1;
          background: repeating-linear-gradient(
            0deg, transparent, transparent 2px,
            rgba(0,240,255,0.015) 2px, rgba(0,240,255,0.015) 4px
          );
        }
        .cine-grid {
          position: absolute; inset: 0; pointer-events: none; z-index: 0;
          background-image:
            linear-gradient(rgba(0,240,255,0.02) 1px, transparent 1px),
            linear-gradient(90deg, rgba(0,240,255,0.02) 1px, transparent 1px);
          background-size: 80px 80px;
          animation: gridDrift 30s linear infinite;
        }
        @keyframes gridDrift { 0%{background-position:0 0} 100%{background-position:80px 80px} }

        /* ═══════ CUSTOM CURSOR — DISABLED (CyberCursor stripped) ═══════ */
        /* cursor:none removed — no custom cursor component active */
        .cyber-cursor-dot {
          position: fixed; top: 0; left: 0; z-index: 99998;
          width: 8px; height: 8px; border-radius: 50%;
          background: var(--alien-cyan);
          box-shadow: 0 0 12px var(--alien-cyan), 0 0 24px rgba(0,240,255,0.4);
          pointer-events: none; will-change: transform;
          mix-blend-mode: screen;
        }
        .cyber-cursor-ring {
          position: fixed; top: 0; left: 0; z-index: 99997;
          width: 40px; height: 40px; border-radius: 50%;
          border: 1.5px solid rgba(0,240,255,0.3);
          pointer-events: none; will-change: transform;
          transition: opacity 0.3s ease;
          mix-blend-mode: screen;
        }
        @media (pointer: coarse) {
          .cyber-cursor-dot, .cyber-cursor-ring { display: none !important; }
        }

        /* ═══════ FILM GRAIN OVERLAY (moved to inline CSS) ═══════ */

        /* ═══════ NOVA LLM ENGINE SECTION ═══════ */
        .section-intro {
          max-width: 800px;
        }

        .nova-arch-banner {
          background: linear-gradient(135deg, rgba(0,240,255,0.04), rgba(255,0,229,0.04), rgba(57,255,20,0.04));
          border: 1px solid rgba(0,240,255,0.12);
          border-radius: 16px;
          padding: 24px 32px;
          margin: 8px 0;
          overflow-x: auto;
        }

        .nova-arch-flow {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 12px;
          flex-wrap: nowrap;
          min-width: max-content;
        }

        .nova-arch-node {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 6px;
          padding: 14px 20px;
          border: 1.5px solid rgba(0,240,255,0.3);
          border-radius: 12px;
          background: rgba(0,0,0,0.4);
          font-size: 0.8rem;
          color: rgba(255,255,255,0.85);
          letter-spacing: 0.02em;
          transition: all 0.3s ease;
          min-width: 90px;
          text-align: center;
        }
        .nova-arch-node:hover {
          background: rgba(0,240,255,0.08);
          transform: translateY(-2px);
          box-shadow: 0 0 20px rgba(0,240,255,0.15);
        }

        .nova-arch-icon {
          font-size: 1.5rem;
        }

        .nova-arch-arrow {
          font-size: 1.4rem;
          color: rgba(0,240,255,0.5);
          flex-shrink: 0;
        }

        .nova-stats-bar {
          display: flex;
          justify-content: space-around;
          align-items: center;
          gap: 16px;
          margin-top: 28px;
          padding: 20px 24px;
          background: linear-gradient(135deg, rgba(0,240,255,0.03), rgba(255,0,229,0.03));
          border: 1px solid rgba(0,240,255,0.1);
          border-radius: 14px;
          flex-wrap: wrap;
        }

        .nova-stat {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 4px;
        }

        .nova-stat-value {
          font-size: 1.6rem;
          font-weight: 700;
          font-family: 'JetBrains Mono', monospace;
          letter-spacing: -0.02em;
        }

        .nova-stat-label {
          font-size: 0.7rem;
          color: rgba(255,255,255,0.45);
          text-transform: uppercase;
          letter-spacing: 0.08em;
        }

        /* ═══════ RESPONSIVE ═══════ */
        @media (max-width: 900px) {
          .hero-h1 { font-size: 2.2rem; }
          .hero-infinity-svg { width: clamp(180px, 65vw, 340px); }
          .grid-2, .grid-3, .grid-4 { grid-template-columns: 1fr; }
          .pipeline-row { flex-direction: column; gap: 0.4rem; }
          .pipe-arrow { transform: rotate(90deg); }
          .container { padding: 0 1rem; }
          .metrics-bar { flex-direction: column; }
          .metric { border-right: none; border-bottom: 1px solid rgba(0,240,255,0.06); }
          .metric:last-child { border-bottom: none; }
          .arch-tier--triple { grid-template-columns: 1fr; }
          .arch-connector--triple { flex-direction: column; gap: 0.3rem; }
          .arch-frame { padding: 1rem 0.8rem; }
          .arch-frame-title { font-size: 0.65rem; letter-spacing: 0.2em; }
          .arch-box { padding: 0.7rem 0.8rem; }
          .arch-box-label { font-size: 0.62rem; }
          .arch-chip { font-size: 0.48rem; }
          .arch-box-laws { flex-direction: column; gap: 0.3rem; }
          .gauge-row { gap: 0.5rem; }
          .nav-float { display: none; }
          .nova-arch-flow { gap: 6px; }
          .nova-arch-node { padding: 10px 12px; min-width: 70px; font-size: 0.7rem; }
          .nova-arch-icon { font-size: 1.2rem; }
          .nova-arch-arrow { font-size: 1rem; }
          .nova-stats-bar { gap: 12px; padding: 16px; }
          .nova-stat-value { font-size: 1.2rem; }
        }

        /* ═══════ ACCESSIBILITY — REDUCED MOTION ═══════ */
        @media (prefers-reduced-motion: reduce) {
          *, *::before, *::after {
            animation-duration: 0.01ms !important;
            animation-iteration-count: 1 !important;
            transition-duration: 0.01ms !important;
          }
          .star-field, .nebula-overlay, .data-stream-overlay, .film-grain,
          .holo-shapes, .holo-scan, .cyber-grid, .holo-wave-canvas { display: none; }
          .cine-loader { display: none; }
          .cyber-cursor-dot, .cyber-cursor-ring { display: none; }
          .scroll-progress { animation: none; background-size: 100% 100%; }
          .scroll-reveal { opacity: 1; transform: none; }
          .hero-logo { animation: none; }
          .hero-infinity-svg { animation: none; }
          .hero-inf-path, .hero-inf-core, .hero-inf-haze, .hero-ambient, .hero-particle { animation: none; }
          .alien-card { transition: none; }
          .sticky-nav { transition: none; }
        }
      `}</style>
    </div>
  );
}
