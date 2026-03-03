"use client";

import { useState, useEffect, useRef, useMemo, useCallback } from "react";

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
  { name: "codegen.rs", loc: 3494, purpose: "Cranelift 0.116 JIT + 200+ stdlib builtins", pct: 100 },
  { name: "parser.rs", loc: 1986, purpose: "Recursive-descent + Pratt parser (traits, type aliases, cast)", pct: 57 },
  { name: "ir.rs", loc: 1941, purpose: "SSA-form IR + match/pipe/lambda/method registry", pct: 56 },
  { name: "hotpath.rs", loc: 1911, purpose: "44 native hotpath ops + layer_norm/dropout/cosine_distance", pct: 55 },
  { name: "optimizer.rs", loc: 1148, purpose: "Predictive JIT + Delta Debug", pct: 33 },
  { name: "quantum_math.rs", loc: 911, purpose: "Quantum math primitives", pct: 26 },
  { name: "ml.rs", loc: 872, purpose: "Machine learning built-ins", pct: 25 },
  { name: "types.rs", loc: 855, purpose: "Two-pass type checker + scopes", pct: 24 },
  { name: "advanced_math.rs", loc: 833, purpose: "Extended math operations", pct: 24 },
  { name: "evolution_advanced.rs", loc: 818, purpose: "Advanced evolution strategies", pct: 23 },
  { name: "bridge.rs", loc: 764, purpose: "C FFI bridge (64 exports)", pct: 22 },
  { name: "engine.rs", loc: 760, purpose: "VitalisEngine core", pct: 22 },
  { name: "simd_ops.rs", loc: 748, purpose: "SIMD F64x4 vectorization (AVX2)", pct: 21 },
  { name: "meta_evolution.rs", loc: 734, purpose: "Thompson sampling strategies", pct: 21 },
  { name: "memory.rs", loc: 693, purpose: "Engram storage (5 engram types)", pct: 20 },
  { name: "evolution.rs", loc: 690, purpose: "EvolutionRegistry + quantum UCB", pct: 20 },
  { name: "ast.rs", loc: 628, purpose: "30+ expression variants + traits + type aliases", pct: 18 },
  { name: "lexer.rs", loc: 678, purpose: "Logos-based tokenizer (80+ tokens)", pct: 19 },
  { name: "stdlib.rs", loc: 257, purpose: "200+ built-in functions", pct: 7 },
  { name: "lib.rs", loc: 138, purpose: "Module declarations (47 modules)", pct: 4 },
  { name: "async_runtime.rs", loc: 310, purpose: "Async/await executor, channels, futures", pct: 9 },
  { name: "generics.rs", loc: 420, purpose: "Type params, monomorphization, bounds", pct: 12 },
  { name: "package_manager.rs", loc: 470, purpose: "SemVer, registry, dependency resolver", pct: 13 },
  { name: "lsp.rs", loc: 813, purpose: "LSP server — diagnostics, completion, hover", pct: 23 },
  { name: "wasm_target.rs", loc: 640, purpose: "WebAssembly module builder + LEB128", pct: 18 },
  { name: "gpu_compute.rs", loc: 520, purpose: "GPU buffers, kernels, pipelines, shaders", pct: 15 },
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
   COMPONENT: Enhanced Particle Field (Mouse-Reactive)
   ═══════════════════════════════════════════════════════════ */
function ParticleField() {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let raf: number;
    let mouseX = -1000, mouseY = -1000;
    const COLORS = [
      "rgba(0,240,255,",
      "rgba(255,0,229,",
      "rgba(57,255,20,",
      "rgba(176,38,255,",
      "rgba(255,215,0,",
      "rgba(255,106,0,",
    ];

    const particles: {
      x: number; y: number; r: number;
      dx: number; dy: number;
      color: string; alpha: number;
      pulseSpeed: number; phase: number;
    }[] = [];

    function resize() {
      canvas!.width = window.innerWidth;
      canvas!.height = window.innerHeight;
    }
    resize();
    window.addEventListener("resize", resize);

    const onMouse = (e: MouseEvent) => { mouseX = e.clientX; mouseY = e.clientY; };
    document.addEventListener("mousemove", onMouse);

    for (let i = 0; i < 90; i++) {
      particles.push({
        x: Math.random() * (canvas.width || 1920),
        y: Math.random() * (canvas.height || 1080),
        r: Math.random() * 2.5 + 0.3,
        dx: (Math.random() - 0.5) * 0.4,
        dy: (Math.random() - 0.5) * 0.4,
        color: COLORS[Math.floor(Math.random() * COLORS.length)],
        alpha: Math.random() * 0.5 + 0.1,
        pulseSpeed: Math.random() * 0.02 + 0.008,
        phase: Math.random() * Math.PI * 2,
      });
    }

    function draw() {
      ctx!.clearRect(0, 0, canvas!.width, canvas!.height);
      const t = performance.now() * 0.001;

      for (const p of particles) {
        const mdx = p.x - mouseX;
        const mdy = p.y - mouseY;
        const mDist = Math.sqrt(mdx * mdx + mdy * mdy);
        if (mDist < 120 && mDist > 0) {
          const force = (120 - mDist) / 120 * 2;
          p.x += (mdx / mDist) * force;
          p.y += (mdy / mDist) * force;
        }

        p.x += p.dx;
        p.y += p.dy;
        if (p.x < -10) p.x = canvas!.width + 10;
        if (p.x > canvas!.width + 10) p.x = -10;
        if (p.y < -10) p.y = canvas!.height + 10;
        if (p.y > canvas!.height + 10) p.y = -10;

        const a = p.alpha * (0.5 + 0.5 * Math.sin(t * p.pulseSpeed * 60 + p.phase));

        ctx!.beginPath();
        ctx!.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx!.fillStyle = p.color + a.toFixed(3) + ")";
        ctx!.fill();

        ctx!.beginPath();
        ctx!.arc(p.x, p.y, p.r * 4, 0, Math.PI * 2);
        ctx!.fillStyle = p.color + (a * 0.12).toFixed(3) + ")";
        ctx!.fill();
      }

      for (let i = 0; i < particles.length; i++) {
        for (let j = i + 1; j < particles.length; j++) {
          const dx = particles[i].x - particles[j].x;
          const dy = particles[i].y - particles[j].y;
          const dist = Math.sqrt(dx * dx + dy * dy);
          if (dist < 180) {
            const alpha = (1 - dist / 180) * 0.1;
            ctx!.beginPath();
            ctx!.moveTo(particles[i].x, particles[i].y);
            ctx!.lineTo(particles[j].x, particles[j].y);
            ctx!.strokeStyle = particles[i].color + alpha.toFixed(3) + ")";
            ctx!.lineWidth = 0.5;
            ctx!.stroke();
          }
        }
      }

      raf = requestAnimationFrame(draw);
    }
    draw();
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
      document.removeEventListener("mousemove", onMouse);
    };
  }, []);

  return <canvas ref={ref} className="particle-canvas" />;
}

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
   COMPONENT: Data Stream Overlay (Falling Characters)
   ═══════════════════════════════════════════════════════════ */
function DataStream() {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = ref.current;
    if (!container) return;
    const chars = "01\u221E\u03BB\u03A3\u0394\u221A\u03C0\u03A9\u2248\u222B{}[]<>:;=+*&|^~";
    const colors = [ALIEN.cyan, ALIEN.magenta, ALIEN.green, ALIEN.violet, ALIEN.gold];
    let active = true;

    function spawn() {
      if (!active || !container) return;
      const el = document.createElement("span");
      el.className = "data-char";
      el.textContent = chars[Math.floor(Math.random() * chars.length)];
      el.style.left = Math.random() * 100 + "%";
      el.style.animationDuration = (Math.random() * 10 + 6) + "s";
      el.style.fontSize = (Math.random() * 8 + 8) + "px";
      el.style.color = colors[Math.floor(Math.random() * colors.length)];
      el.style.opacity = "0";
      container.appendChild(el);
      el.addEventListener("animationend", () => el.remove());
      setTimeout(spawn, Math.random() * 500 + 300);
    }

    const timer = setTimeout(spawn, 2000);
    return () => { active = false; clearTimeout(timer); };
  }, []);

  return <div ref={ref} className="data-stream-overlay" />;
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Glitch Text
   ═══════════════════════════════════════════════════════════ */
function GlitchText({ text, className = "" }: { text: string; className?: string }) {
  return (
    <span className={`glitch-text ${className}`} data-text={text}>
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

  const onMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const rotateX = (y - rect.height / 2) / rect.height * -6;
    const rotateY = (x - rect.width / 2) / rect.width * 6;
    const glowX = (x / rect.width) * 100;
    const glowY = (y / rect.height) * 100;
    el.style.transform = `perspective(800px) rotateX(${rotateX}deg) rotateY(${rotateY}deg) translateY(-4px) scale(1.015)`;
    el.style.boxShadow = `0 0 40px rgba(0,240,255,0.08), inset 0 0 60px rgba(0,240,255,0.02), ${glowX < 50 ? '-' : ''}${Math.abs(glowX - 50) * 0.3}px ${glowY < 50 ? '-' : ''}${Math.abs(glowY - 50) * 0.3}px 60px rgba(0,240,255,0.05)`;
  }, []);

  const onMouseLeave = useCallback(() => {
    const el = ref.current;
    if (el) {
      el.style.transform = "perspective(800px) rotateX(0) rotateY(0) translateY(0) scale(1)";
      el.style.boxShadow = "";
    }
  }, []);

  return (
    <div
      ref={ref}
      className={`alien-card ${featured ? "card-featured" : ""} ${className}`}
      style={borderColor ? { borderColor } : undefined}
      onMouseMove={onMouseMove}
      onMouseLeave={onMouseLeave}
    >
      {children}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Progress Bar
   ═══════════════════════════════════════════════════════════ */
function ProgressBar({ label, value, max, color }: {
  label: string; value: string; max: number; color: string;
}) {
  return (
    <div className="progress-container">
      <div className="progress-label">
        <span>{label}</span>
        <span>{value}</span>
      </div>
      <div className="progress-track">
        <div className="progress-fill" style={{ width: `${max}%`, background: color }} />
      </div>
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
   COMPONENT: Circuit Traces (SVG overlay)
   ═══════════════════════════════════════════════════════════ */
function CircuitTraces() {
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    function draw() {
      const svg = svgRef.current;
      if (!svg) return;
      const w = window.innerWidth;
      const h = document.documentElement.scrollHeight;
      svg.setAttribute("viewBox", `0 0 ${w} ${h}`);
      svg.style.height = `${h}px`;
      svg.innerHTML = "";

      const ns = "http://www.w3.org/2000/svg";
      const colors = ["", "magenta", "green", "", "magenta", "", "green", ""];

      for (let t = 0; t < 8; t++) {
        const path = document.createElementNS(ns, "path");
        const startX = Math.random() * w * 0.3 + (t % 2 === 0 ? 20 : w * 0.7);
        let d = `M ${startX} 0`;
        let y = 0;
        let x = startX;

        while (y < h) {
          const segLen = Math.random() * 200 + 100;
          const dir = Math.random() > 0.5 ? 1 : -1;
          if (Math.random() > 0.4) {
            y += segLen;
            d += ` L ${x} ${Math.min(y, h)}`;
          } else {
            const jog = (Math.random() * 80 + 20) * dir;
            x = Math.max(20, Math.min(w - 20, x + jog));
            d += ` L ${x} ${y}`;
            const node = document.createElementNS(ns, "circle");
            node.setAttribute("cx", String(x));
            node.setAttribute("cy", String(y));
            node.setAttribute("r", "3");
            node.classList.add("circuit-node");
            node.style.animationDelay = `${Math.random() * 3}s`;
            svg.appendChild(node);
            y += segLen * 0.5;
            d += ` L ${x} ${Math.min(y, h)}`;
          }
        }

        path.setAttribute("d", d);
        path.classList.add("circuit-line");
        if (colors[t]) path.classList.add(colors[t]);
        path.style.animationDelay = `${t * 0.5}s`;
        svg.appendChild(path);
      }
    }

    draw();
    const onResize = () => setTimeout(draw, 300);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return (
    <svg
      ref={svgRef}
      className="circuit-svg"
      style={{ position: "absolute", top: 0, left: 0, width: "100%", pointerEvents: "none", zIndex: 0, overflow: "visible" }}
    />
  );
}

/* ═══════════════════════════════════════════════════════════
   COMPONENT: Parallax Background Hook
   ═══════════════════════════════════════════════════════════ */
function useParallax() {
  useEffect(() => {
    function update() {
      const scrollY = window.pageYOffset;
      const starField = document.querySelector(".star-field") as HTMLElement | null;
      const nebula = document.querySelector(".nebula-overlay") as HTMLElement | null;
      if (starField) starField.style.transform = `translateY(${scrollY * 0.15}px)`;
      if (nebula) nebula.style.transform = `translateY(${scrollY * 0.08}px) scale(1.1)`;
    }
    window.addEventListener("scroll", update, { passive: true });
    return () => window.removeEventListener("scroll", update);
  }, []);
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

/* ═══════ CUSTOM ANIMATED CURSOR ═══════ */
function CyberCursor() {
  const dotRef = useRef<HTMLDivElement>(null);
  const ringRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (typeof window === "undefined") return;
    if (window.matchMedia("(pointer: coarse)").matches) return;

    const dot = dotRef.current;
    const ring = ringRef.current;
    if (!dot || !ring) return;

    let mx = -100, my = -100, rx = -100, ry = -100, hovering = false, alive = true;

    const onMove = (e: MouseEvent) => { mx = e.clientX; my = e.clientY; };
    const onEnter = () => { hovering = true; };
    const onLeave = () => { hovering = false; };

    const bind = () => {
      document.querySelectorAll("a,button,.alien-card,.snav-link,.nav-dot,.arch-box,.tech-tag").forEach(el => {
        el.addEventListener("mouseenter", onEnter);
        el.addEventListener("mouseleave", onLeave);
      });
    };

    const loop = () => {
      if (!alive) return;
      dot.style.transform = `translate(${mx - 4}px,${my - 4}px)`;
      rx += (mx - rx) * 0.12;
      ry += (my - ry) * 0.12;
      const s = hovering ? 2 : 1;
      ring.style.transform = `translate(${rx - 20}px,${ry - 20}px) scale(${s})`;
      ring.style.opacity = hovering ? "0.7" : "0.4";
      requestAnimationFrame(loop);
    };

    document.addEventListener("mousemove", onMove);
    bind();
    const iv = setInterval(bind, 4000);
    requestAnimationFrame(loop);
    return () => { alive = false; document.removeEventListener("mousemove", onMove); clearInterval(iv); };
  }, []);

  return (
    <>
      <div ref={dotRef} className="cyber-cursor-dot" />
      <div ref={ringRef} className="cyber-cursor-ring" />
    </>
  );
}

/* ═══════ FILM GRAIN OVERLAY ═══════ */
function FilmGrain() {
  return <div className="film-grain" />;
}

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
              name: "Ben Chmiel",
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

      {/* Cinematic preloader + custom cursor */}
      <CinematicLoader />
      <CyberCursor />

      {/* Background layers */}
      <div className="star-field" />
      <div className="nebula-overlay" />
      <ParticleField />
      <DataStream />
      <ScrollProgress />
      <CircuitTraces />
      <FilmGrain />
      <FloatingNav />

      {/* ═══════════ HERO ═══════════ */}
      <header className="hero">
        <div className="hero-logo" aria-label="Infinity — Autonomous AI System">
          <svg className="hero-infinity-svg" viewBox="0 0 400 220" xmlns="http://www.w3.org/2000/svg">
            <defs>
              {/* Primary emerald gradient */}
              <linearGradient id="igPrimary" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#059669" />
                <stop offset="25%" stopColor="#10b981" />
                <stop offset="50%" stopColor="#34d399" />
                <stop offset="75%" stopColor="#00f0ff" />
                <stop offset="100%" stopColor="#059669" />
              </linearGradient>
              {/* Secondary darker accent */}
              <linearGradient id="igSecondary" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#047857" />
                <stop offset="50%" stopColor="#0d9488" />
                <stop offset="100%" stopColor="#047857" />
              </linearGradient>
              {/* Metallic top-light */}
              <linearGradient id="igSheen" x1="0%" y1="0%" x2="0%" y2="100%">
                <stop offset="0%" stopColor="rgba(255,255,255,0.45)" />
                <stop offset="40%" stopColor="rgba(255,255,255,0.05)" />
                <stop offset="60%" stopColor="rgba(255,255,255,0)" />
                <stop offset="100%" stopColor="rgba(255,255,255,0.1)" />
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
              {/* Reflection fade */}
              <linearGradient id="igReflFade" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="white" stopOpacity="0.2" />
                <stop offset="100%" stopColor="white" stopOpacity="0" />
              </linearGradient>
              <mask id="igReflMask">
                <rect x="0" y="155" width="400" height="65" fill="url(#igReflFade)" />
              </mask>
            </defs>

            {/* Ambient nebula behind the symbol */}
            <ellipse cx="200" cy="100" rx="140" ry="60" fill="rgba(16,185,129,0.06)" className="hero-ambient" />

            {/* Layer 1: Wide soft haze */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="url(#igPrimary)" strokeWidth="18"
              strokeLinecap="round" strokeLinejoin="round"
              filter="url(#igHaze)" opacity="0.35" className="hero-inf-haze"
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
              fill="none" stroke="rgba(255,255,255,0.35)" strokeWidth="1.5"
              strokeLinecap="round" strokeLinejoin="round"
            />

            {/* Metallic sheen overlay */}
            <path
              d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              fill="none" stroke="url(#igSheen)" strokeWidth="6"
              strokeLinecap="round" strokeLinejoin="round"
              opacity="0.5"
            />

            {/* Energy particle 1 — bright white */}
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

            {/* Energy particle 2 — cyan accent, offset */}
            <circle r="2" fill="#00f0ff" className="hero-particle">
              <animate attributeName="opacity" values="0.7;0.2;0.7" dur="5s" repeatCount="indefinite" />
              <animateMotion
                dur="5s" repeatCount="indefinite" begin="-2.5s"
                path="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
              />
            </circle>

            {/* Reflection */}
            <g mask="url(#igReflMask)" transform="translate(0, 130) scale(1, -0.25)">
              <path
                d="M200 100 C200 58, 118 32, 85 60 C48 92, 48 118, 85 145 C118 168, 200 142, 200 100 C200 58, 282 32, 315 60 C352 92, 352 118, 315 145 C282 168, 200 142, 200 100 Z"
                fill="none" stroke="url(#igPrimary)" strokeWidth="5"
                strokeLinecap="round" strokeLinejoin="round" opacity="0.25"
                filter="url(#igGlow)"
              />
            </g>

            {/* Ground glow line */}
            <line x1="60" y1="185" x2="340" y2="185" stroke="url(#igPrimary)" strokeWidth="0.5" opacity="0.15" />
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
                          ? `${formatNum(stats.tech_stack.rust.loc)} LOC \u00B7 ${stats.tech_stack.rust.files} Files \u00B7 ${stats.tech_stack.rust.tests ?? 870} Tests`
                          : "35,856 LOC \u00B7 47 Modules \u00B7 870 Tests"}
                      </div>
                    </div>
                  </div>
                  <div className="card-body">
                    Custom compiled language with <strong>Cranelift JIT</strong>, SIMD vectorization,
                    predictive optimizer, quantum-inspired evolution (annealing, UCB, Pareto, CMA-ES),
                    async/await runtime, generics with monomorphization, WebAssembly target,
                    LSP server for IDE support, GPU compute backend, package manager,
                    consciousness substrate, kernel sentinel, 98 stdlib builtins, and 44 native hotpath ops
                    including softmax, cross-entropy, batch sigmoid/ReLU, cosine similarity, entropy, and EMA.
                    Compiles <code>.sl</code> &rarr; native x86-64 or WASM via SSA IR.
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
                          ? `${formatNum(stats.tech_stack.python.loc)} LOC \u00B7 ${stats.tech_stack.python.files} Files \u00B7 ${stats.tech_stack.python.modules ?? 72} Modules`
                          : "95,337 LOC \u00B7 344 Files \u00B7 72 Modules"}
                      </div>
                    </div>
                  </div>
                  <div className="card-body">
                    <strong>FastAPI</strong> server on port 8002 with {stats?.modules_loaded ?? 72} cortex modules covering
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
                          ? `${formatNum(stats.tech_stack.typescript.loc)} LOC \u00B7 ${stats.tech_stack.typescript.files} Files \u00B7 ${stats.tech_stack.typescript.routes ?? 25} Routes`
                          : "14,308 LOC \u00B7 51 Files \u00B7 25 Routes"}
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

            {/* LOC Distribution */}
            <div style={{ marginTop: "2rem" }}>
              <ProgressBar
                label="Python" value={`${formatNum(stats?.tech_stack?.python?.loc ?? 95337)} LOC (67.1%)`}
                max={67.1}
                color={`linear-gradient(90deg, ${ALIEN.green}, rgba(57,255,20,0.5))`}
              />
              <ProgressBar
                label="Rust" value={`${formatNum(stats?.tech_stack?.rust?.loc ?? 32349)} LOC (22.8%)`}
                max={22.8}
                color={`linear-gradient(90deg, ${ALIEN.orange}, rgba(255,106,0,0.5))`}
              />
              <ProgressBar
                label="TypeScript / TSX" value={`${formatNum(stats?.tech_stack?.typescript?.loc ?? 14308)} LOC (10.1%)`}
                max={10.1}
                color={`linear-gradient(90deg, ${ALIEN.cyan}, rgba(0,240,255,0.5))`}
              />
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
                      <span className="arch-chip arch-chip--dim">25 Routes</span>
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
                      <span className="arch-chip arch-chip--dim">34 hotpath</span>
                    </div>
                    <div className="arch-box-stat">
                      <span className="arch-stat-dot arch-stat-dot--orange"></span>
                      285 Tests
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
                        <div className="card-subtitle">bridge.rs (983 LOC) &middot; 64 FFI exports &middot; vitalis.py</div>
                      </div>
                    </div>
                    <div className="card-body">
                      <strong>64 extern &quot;C&quot;</strong> FFI exports with <code>#[unsafe(no_mangle)]</code>.
                      Strings via <code>CString::into_raw()</code>, freed via <code>slang_free_string()</code>.
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
            <SectionHeader icon={"\uD83E\uDDEC"} title="Self-Evolution Engine" badge={"\u25CF EVOLVING"} badgeType="active" />
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
                  <StatRow label="Vitalis Functions Tracked" value="8" />
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
                </div>
                <div className="tags-row">
                  <Tag variant="ai">{stats?.swarm?.active_agents ?? 4}&times; Agents</Tag>
                  <Tag variant="ai">Consensus</Tag>
                  <Tag variant="rust">Native Tally</Tag>
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
            <SectionHeader icon={"\uD83D\uDDA5\uFE0F"} title="Cyberpunk Frontend \u2014 24 Routes" badge="PORT 3002" badgeType="native" />
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
                    <div className="card-title">Dashboard Pages ({stats?.tech_stack?.typescript?.routes ?? 25})</div>
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
                  <StatRow label="Rust Tests" value="870 \u2713" color={ALIEN.green} />
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
            <SectionHeader icon={"\uD83D\uDCCB"} title="Full Source Inventory" badge="456 FILES" badgeType="native" />

            <Card>
              <div className="card-header-row">
                <span className="card-icon">{"\uD83E\uDD80"}</span>
                <div>
                  <div className="card-title">Rust &mdash; Vitalis Compiler Modules</div>
                  <div className="card-subtitle">47 files &middot; 35,856 LOC &middot; vitalis/src/</div>
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

      {/* ═══════════ PROJECT DEEP DIVES ═══════════ */}
      <div className="container">
        <Reveal>
          <section style={{ marginTop: 48 }}>
            <SectionHeader icon="🔬" title="Deep Dives" badge="Projects" badgeType="native" />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(340px, 1fr))", gap: 20 }}>
              <a href="/nova" style={{ textDecoration: "none", color: "inherit" }}>
                <Card borderColor={ALIEN.orange}>
                  <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
                    <span style={{ fontSize: 28 }}>⚡</span>
                    <div>
                      <div style={{ fontSize: 18, fontWeight: 800, color: ALIEN.orange }}>NOVA</div>
                      <div style={{ fontSize: 11, color: ALIEN.dim }}>Self-Evolving Native LLM Engine</div>
                    </div>
                  </div>
                  <p style={{ fontSize: 12, color: "#b0b0cc", lineHeight: 1.6, margin: "0 0 12px" }}>
                    From-scratch LLM training in pure Rust + CUDA. Custom tensor library, autograd, BPE tokenizer,
                    transformer architecture — 12,119 LOC, 57 source files, RTX 5060 GPU.
                  </p>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    {["Rust", "CUDA", "~5M params", "Custom Tensors"].map(t => (
                      <span key={t} style={{ fontSize: 10, padding: "2px 8px", borderRadius: 12, border: `1px solid ${ALIEN.orange}30`, color: ALIEN.orange, background: `${ALIEN.orange}08` }}>{t}</span>
                    ))}
                  </div>
                </Card>
              </a>
              <a href="/vitalis" style={{ textDecoration: "none", color: "inherit" }}>
                <Card borderColor={ALIEN.green}>
                  <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
                    <span style={{ fontSize: 28 }}>🧪</span>
                    <div>
                      <div style={{ fontSize: 18, fontWeight: 800, color: ALIEN.green }}>VITALIS</div>
                      <div style={{ fontSize: 11, color: ALIEN.dim }}>AI-Native Programming Language v21</div>
                    </div>
                  </div>
                  <p style={{ fontSize: 12, color: "#b0b0cc", lineHeight: 1.6, margin: "0 0 12px" }}>
                    Custom language with Cranelift JIT, SIMD, generics, async/await, WASM target, LSP server,
                    GPU compute — 47 modules, 870 tests, 35,856 LOC. 88× faster than Python.
                  </p>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    {["Cranelift JIT", "870 Tests", "SIMD/AVX2", "47 Modules"].map(t => (
                      <span key={t} style={{ fontSize: 10, padding: "2px 8px", borderRadius: 12, border: `1px solid ${ALIEN.green}30`, color: ALIEN.green, background: `${ALIEN.green}08` }}>{t}</span>
                    ))}
                  </div>
                </Card>
              </a>
            </div>
          </section>
        </Reveal>
      </div>

      {/* ═══════════ FOOTER ═══════════ */}
      <div className="container">
        <footer className="alien-footer">
          <span className="footer-symbol">{"\u221E"}</span>
          <p>INFINITY &mdash; Autonomous Self-Evolving AI System</p>
          <p className="footer-stats">
            {formatNum(stats?.total_loc ?? 126000)} LOC &middot; 456 Files &middot; 3 Languages &middot; {stats?.modules_loaded ?? 72} Modules &middot; 285 Tests &middot; {formatNum(stats?.memory_count ?? 0)} Memories
          </p>
          <p className="footer-tech">
            Rust 2024 &middot; Python 3.12 &middot; Next.js 15 &middot; Cranelift 0.116 &middot; ChromaDB &middot; Claude Sonnet 4.6
          </p>
          <p style={{ marginTop: '0.6rem' }}>
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
          </p>
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
          background:
            radial-gradient(1.5px 1.5px at 10% 20%, rgba(0,240,255,0.5) 50%, transparent 100%),
            radial-gradient(1px 1px at 20% 50%, rgba(255,0,229,0.4) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 30% 80%, rgba(57,255,20,0.3) 50%, transparent 100%),
            radial-gradient(1px 1px at 40% 10%, rgba(176,38,255,0.4) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 50% 60%, rgba(0,240,255,0.3) 50%, transparent 100%),
            radial-gradient(1px 1px at 60% 30%, rgba(255,215,0,0.3) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 70% 70%, rgba(255,0,229,0.3) 50%, transparent 100%),
            radial-gradient(1px 1px at 80% 40%, rgba(57,255,20,0.4) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 90% 90%, rgba(0,240,255,0.4) 50%, transparent 100%),
            radial-gradient(1px 1px at 15% 65%, rgba(176,38,255,0.3) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 25% 35%, rgba(255,215,0,0.3) 50%, transparent 100%),
            radial-gradient(1px 1px at 45% 85%, rgba(255,106,0,0.3) 50%, transparent 100%),
            radial-gradient(1.2px 1.2px at 55% 15%, rgba(0,240,255,0.5) 50%, transparent 100%),
            radial-gradient(1px 1px at 75% 55%, rgba(255,0,229,0.3) 50%, transparent 100%),
            radial-gradient(1.5px 1.5px at 95% 25%, rgba(57,255,20,0.4) 50%, transparent 100%);
          animation: starDrift 120s linear infinite;
        }
        @keyframes starDrift { 0%{transform:translateY(0)} 100%{transform:translateY(-200px)} }

        /* ═══════ NEBULA OVERLAY ═══════ */
        .nebula-overlay {
          position: fixed; inset: -10%; z-index: 0; pointer-events: none;
          background:
            radial-gradient(ellipse 600px 400px at 20% 50%, rgba(0,240,255,0.04) 0%, transparent 70%),
            radial-gradient(ellipse 500px 500px at 80% 30%, rgba(255,0,229,0.03) 0%, transparent 70%),
            radial-gradient(ellipse 400px 600px at 50% 80%, rgba(176,38,255,0.03) 0%, transparent 70%);
          animation: nebulaPulse 12s ease-in-out infinite alternate;
        }
        @keyframes nebulaPulse { 0%{opacity:0.6;transform:scale(1)} 100%{opacity:1;transform:scale(1.05)} }

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

        .particle-canvas { position: fixed; inset: 0; z-index: 0; pointer-events: none; }

        /* ═══════ DATA STREAM ═══════ */
        .data-stream-overlay { position: fixed; inset: 0; pointer-events: none; z-index: 1; overflow: hidden; }
        .data-char {
          position: absolute; font-family: var(--font-mono); font-size: 10px;
          opacity: 0; animation: dataFall linear infinite; pointer-events: none;
        }
        @keyframes dataFall {
          0%{opacity:0;transform:translateY(-20px)} 10%{opacity:0.25} 90%{opacity:0.12} 100%{opacity:0;transform:translateY(100vh)}
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

        /* ═══════ GLITCH TEXT ═══════ */
        .glitch-text { position: relative; display: inline-block; }
        .glitch-text::before, .glitch-text::after {
          content: attr(data-text); position: absolute; top: 0; left: 0;
          width: 100%; height: 100%; pointer-events: none;
        }
        .glitch-text::before {
          color: var(--alien-cyan); animation: glitchShift 3s infinite;
          clip-path: polygon(0 0, 100% 0, 100% 35%, 0 35%);
        }
        .glitch-text::after {
          color: var(--alien-magenta); animation: glitchShift2 2.5s infinite;
          clip-path: polygon(0 65%, 100% 65%, 100% 100%, 0 100%);
        }
        @keyframes glitchShift {
          0%,90%,100%{transform:translate(0)} 92%{transform:translate(-3px,1px)}
          94%{transform:translate(2px,-1px)} 96%{transform:translate(-1px,2px)} 98%{transform:translate(3px,0)}
        }
        @keyframes glitchShift2 {
          0%,88%,100%{transform:translate(0)} 90%{transform:translate(3px,-2px)}
          93%{transform:translate(-2px,1px)} 95%{transform:translate(1px,-1px)} 97%{transform:translate(-3px,2px)}
        }

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
        .energy-connector { position: relative; height: 60px; display: flex; justify-content: center; overflow: hidden; z-index: 2; }
        .energy-connector::before {
          content: ''; position: absolute; width: 2px; height: 100%;
          background: linear-gradient(to bottom,transparent,var(--alien-cyan),transparent);
          animation: energyPulse 2s ease-in-out infinite;
        }
        .energy-connector::after {
          content: ''; position: absolute; width: 8px; height: 8px; border-radius: 50%;
          background: var(--alien-cyan);
          box-shadow: 0 0 15px var(--alien-cyan), 0 0 30px rgba(0,240,255,0.3);
          animation: energyBall 2s ease-in-out infinite; top: -4px;
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
          transition: all 0.3s cubic-bezier(0.16,1,0.3,1); transform-style: preserve-3d; perspective: 1000px;
        }
        .alien-card::after {
          content: ''; position: absolute; bottom: 0; left: 0; right: 0; height: 40%;
          background: linear-gradient(to top,rgba(0,240,255,0.02),transparent);
          pointer-events: none; border-radius: 0 0 12px 12px;
        }
        .alien-card:hover { border-color: var(--alien-cyan); box-shadow: 0 0 30px rgba(0,240,255,0.1), inset 0 0 30px rgba(0,240,255,0.03); }
        @supports (backdrop-filter: blur(1px)) {
          .alien-card { backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px); }
          .metrics-bar { backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px); }
        }
        .card-featured { animation: borderPulse 4s ease-in-out infinite; }
        @keyframes borderPulse { 0%,100%{border-color:rgba(0,240,255,0.15)} 50%{border-color:rgba(0,240,255,0.35)} }
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

        /* ═══════ PROGRESS BAR ═══════ */
        .progress-container { margin-bottom: 0.6rem; }
        .progress-label { display: flex; justify-content: space-between; font-size: 0.78rem; font-family: var(--font-mono); margin-bottom: 0.3rem; color: #8a9ab5; }
        .progress-track { height: 6px; background: rgba(0,240,255,0.06); border-radius: 3px; overflow: hidden; }
        .progress-fill { height: 100%; border-radius: 3px; transition: width 1.5s cubic-bezier(0.16,1,0.3,1); box-shadow: 0 0 8px rgba(0,240,255,0.2); }

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

        /* ═══════ SCROLL REVEAL (enhanced with blur) ═══════ */
        .scroll-reveal {
          opacity: 0; transform: translateY(40px) scale(0.97);
          filter: blur(8px);
          transition: opacity 1s cubic-bezier(0.16,1,0.3,1),
                      transform 1s cubic-bezier(0.16,1,0.3,1),
                      filter 1.2s cubic-bezier(0.16,1,0.3,1);
          will-change: opacity, transform, filter;
        }
        .scroll-reveal.revealed { opacity: 1; transform: translateY(0) scale(1); filter: blur(0px); }

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
          stroke: var(--alien-cyan); stroke-width: 1; fill: none;
          opacity: 0.15; stroke-dasharray: 8 4;
          animation: circuitFlow 4s linear infinite;
        }
        .circuit-line.magenta { stroke: var(--alien-magenta); }
        .circuit-line.green { stroke: var(--alien-green); }
        @keyframes circuitFlow { 0%{stroke-dashoffset:0} 100%{stroke-dashoffset:-24} }
        .circuit-node {
          fill: var(--alien-cyan); opacity: 0;
          animation: circuitNodePulse 3s ease-in-out infinite;
        }
        @keyframes circuitNodePulse { 0%,100%{opacity:0;r:2} 50%{opacity:0.6;r:4} }

        /* ═══════ HOLO SHIMMER ═══════ */
        .alien-card::before {
          content: ''; position: absolute; inset: 0;
          background: linear-gradient(105deg,transparent 20%,rgba(0,240,255,0.03) 30%,rgba(255,0,229,0.06) 40%,rgba(176,38,255,0.06) 50%,rgba(0,240,255,0.03) 60%,transparent 80%);
          animation: holoSweep 3s ease-in-out infinite;
          z-index: 2; border-radius: 12px; pointer-events: none;
          opacity: 0; transition: opacity 0.3s;
        }
        .alien-card:hover::before { opacity: 1; }
        @keyframes holoSweep { 0%{transform:translateX(-100%)} 50%{transform:translateX(100%)} 100%{transform:translateX(-100%)} }

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

        /* ═══════ CUSTOM ANIMATED CURSOR ═══════ */
        @media (pointer: fine) {
          .alien-page { cursor: none; }
          .alien-page a, .alien-page button, .alien-page .snav-link,
          .alien-page .nav-dot, .alien-page .alien-card { cursor: none; }
        }
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

        /* ═══════ FILM GRAIN OVERLAY ═══════ */
        .film-grain {
          position: fixed; inset: -50%; z-index: 3; pointer-events: none;
          opacity: 0.03;
          background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
          background-repeat: repeat;
          background-size: 256px 256px;
          animation: grainShift 0.4s steps(5) infinite;
        }
        @keyframes grainShift {
          0%   { transform: translate(0,0); }
          20%  { transform: translate(-3%,-3%); }
          40%  { transform: translate(3%,1%); }
          60%  { transform: translate(-1%,3%); }
          80%  { transform: translate(2%,-2%); }
          100% { transform: translate(0,0); }
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
        }

        /* ═══════ ACCESSIBILITY — REDUCED MOTION ═══════ */
        @media (prefers-reduced-motion: reduce) {
          *, *::before, *::after {
            animation-duration: 0.01ms !important;
            animation-iteration-count: 1 !important;
            transition-duration: 0.01ms !important;
          }
          .star-field, .nebula-overlay, .data-stream-overlay, .film-grain { display: none; }
          .cine-loader { display: none; }
          .cyber-cursor-dot, .cyber-cursor-ring { display: none; }
          .scroll-progress { animation: none; background-size: 100% 100%; }
          .scroll-reveal { opacity: 1; transform: none; filter: none; }
          .hero-logo { animation: none; }
          .hero-infinity-svg { animation: none; }
          .hero-inf-path, .hero-inf-core, .hero-inf-haze, .hero-ambient, .hero-particle { animation: none; }
          .alien-card { transition: none; }
          .alien-card:hover { transform: none; }
          .sticky-nav { transition: none; }
        }
      `}</style>
    </div>
  );
}
