"use client";

import { useState, useRef, useCallback, useEffect } from "react";

/* ═══════════════════════════════════════════════════════════
   CONSULTING LANDING PAGE — v2 PREMIUM
   Futuristic 3D/shader-inspired UI with glassmorphism,
   animated particles, holographic borders, perspective grid.
   ═══════════════════════════════════════════════════════════ */

const AL = {
  cyan: "#00f0ff",
  green: "#39ff14",
  magenta: "#ff00e5",
  gold: "#ffd700",
  orange: "#ff6a00",
  violet: "#b026ff",
  red: "#ff003c",
  dim: "#5a6a8a",
  bg: "#060a14",
  card: "rgba(8, 12, 28, 0.72)",
};

/* ═══════════════════ DATA ═══════════════════ */
const SERVICES = [
  {
    icon: "\u{1F512}", title: "AI Security Audit", price: "From \u00A3350/session",
    description: "Comprehensive review of your AI deployment \u2014 prompt injection vectors, data exfiltration risks, model poisoning surface, and DLP policy gaps. Includes a written report with prioritised remediation steps.",
    deliverables: ["Threat model of your AI/LLM stack", "Prompt injection & jailbreak testing", "Data flow analysis (PII, IP, regulated data)", "Microsoft Purview / DLP policy recommendations", "Prioritised remediation roadmap"],
    tags: ["Purview", "DLP", "Prompt Safety", "OWASP LLM Top 10"], color: AL.red, cta: "Book Security Audit",
  },
  {
    icon: "\u{1F3D7}\uFE0F", title: "AI Architecture Advisory", price: "\u00A3400/hour",
    description: "60-minute deep-dive into your AI architecture \u2014 RAG pipelines, vector stores, multi-agent orchestration, inference backends, and scaling strategy. Walk away with a concrete action plan.",
    deliverables: ["Architecture review & gap analysis", "RAG pipeline optimisation", "Vector store selection (ChromaDB, FAISS, Pinecone)", "Multi-agent / swarm design patterns", "Scaling & cost optimisation strategy"],
    tags: ["RAG", "LLM Ops", "Multi-Agent", "Vector DB"], color: AL.cyan, cta: "Book Architecture Call",
  },
  {
    icon: "\u26A1", title: "Custom AI Toolchain Build", price: "Project-based",
    description: "Need a custom compiler, DSL, or high-performance native library? I build production Rust toolchains with Python FFI \u2014 the same architecture powering Vitalis (35,632 LOC, 748 tests, 7.5x faster than Python).",
    deliverables: ["Custom Rust library or DSL", "Python FFI integration (ctypes/cffi)", "Benchmark suite & performance validation", "CI/CD pipeline & deployment", "Documentation & handoff"],
    tags: ["Rust", "FFI", "Cranelift", "SIMD", "Custom DSL"], color: AL.orange, cta: "Discuss Your Project",
  },
  {
    icon: "\u{1F6E1}\uFE0F", title: "Microsoft Purview Implementation", price: "From \u00A32,500/engagement",
    description: "End-to-end Microsoft Purview deployment for AI governance \u2014 sensitivity labels, DLP policies, insider risk, communication compliance, and eDiscovery configured for your AI workflows.",
    deliverables: ["11-policy Purview baseline deployment", "Sensitivity label taxonomy design", "AI-specific DLP rules (ChatGPT, Copilot, custom LLMs)", "Insider risk policy configuration", "Staff training & documentation"],
    tags: ["Microsoft 365", "Purview", "Compliance", "Governance"], color: AL.violet, cta: "Start Purview Project",
  },
];

const BLUEPRINTS = [
  {
    icon: "\u{1F4D8}", title: "The AI Security Playbook", price: "\u00A397",
    description: "40-page technical guide covering every AI security vector \u2014 with exact Purview policies, DLP rules, prompt safety patterns, and audit checklists. Copy-paste implementation.",
    features: ["11 ready-to-deploy Purview policies", "Prompt injection defence patterns", "Data classification framework for AI", "LLM threat model template", "Audit checklist (50+ items)"],
    buyLink: "#", color: AL.cyan,
  },
  {
    icon: "\u{1F4D9}", title: "The AI Architecture Blueprint", price: "\u00A3149",
    description: "Complete architecture reference for building production AI systems \u2014 RAG pipelines, multi-agent orchestration, evolution engines, vector store selection, and deployment patterns.",
    features: ["Reference architecture diagrams", "RAG pipeline implementation guide", "Multi-agent consensus patterns", "Self-evolution engine design", "Docker + CI/CD templates"],
    buyLink: "#", color: AL.green,
  },
];

const CREDENTIALS = [
  { label: "Lines of Production Rust", value: "35,632", numericValue: 35632 },
  { label: "FFI Exports Shipped", value: "405", numericValue: 405 },
  { label: "Algorithm Libraries", value: "14", numericValue: 14 },
  { label: "Benchmark: Avg Speedup", value: "7.5\u00D7", numericValue: 7.5, suffix: "\u00D7" },
  { label: "Open Source Tests Passing", value: "748", numericValue: 748 },
  { label: "Python APIs Delivered", value: "318", numericValue: 318 },
];

const WHY_ME = [
  { icon: "\u{1F527}", title: "I Build, Not Just Advise", body: "I wrote a 35,632-line Rust compiler and a from-scratch LLM training engine with CUDA, deployed a self-evolving AI system, and open-sourced everything. When I advise on architecture, I\u2019ve done it myself \u2014 at production scale." },
  { icon: "\u{1F4CA}", title: "Benchmarked, Not Theorised", body: "Every claim has data behind it. 74 benchmarks, 7.5x average speedup, 748 passing tests. I bring the same rigour to your project." },
  { icon: "\u{1F512}", title: "Security-First Mindset", body: "I\u2019ve deployed Microsoft Purview DLP policies, built capability-based sandboxing into my AI runtime, and implemented guardrail layers that intercept every model output before it reaches users. Prompt injection testing, data exfiltration prevention, sensitivity labelling \u2014 I engineer security at every layer, not just tick compliance boxes." },
  { icon: "\u26A1", title: "Enterprise & Startup Fluent", body: "From Microsoft Purview in regulated enterprise environments to scrappy Rust compilers \u2014 I bridge the gap between corporate governance and cutting-edge engineering." },
];

/* ═══════════════════ VISUAL COMPONENTS ═══════════════════ */

/** Animated counter that counts up when visible */
function AnimatedCounter({ value, suffix }: { value: number; suffix?: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  const [display, setDisplay] = useState("0");
  const started = useRef(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(([e]) => {
      if (e.isIntersecting && !started.current) {
        started.current = true;
        const dur = 1600;
        const start = performance.now();
        const tick = (now: number) => {
          const t = Math.min((now - start) / dur, 1);
          const ease = 1 - Math.pow(1 - t, 3);
          const v = ease * value;
          setDisplay(value >= 100 ? Math.round(v).toLocaleString() : v.toFixed(1));
          if (t < 1) requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      }
    }, { threshold: 0.3 });
    obs.observe(el);
    return () => obs.disconnect();
  }, [value]);

  return <span ref={ref}>{display}{suffix || ""}</span>;
}

/** Reveal-on-scroll wrapper */
function Reveal({ children, delay = 0 }: { children: React.ReactNode; delay?: number }) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(([e]) => {
      if (e.isIntersecting) { setVisible(true); obs.disconnect(); }
    }, { threshold: 0.15 });
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  return (
    <div
      ref={ref}
      className={`reveal-wrap ${visible ? "revealed" : ""}`}
      style={{ transitionDelay: `${delay}ms` }}
    >
      {children}
    </div>
  );
}

/** Premium glassmorphism card with holographic animated border */
function GlassCard({ children, borderColor, className }: { children: React.ReactNode; borderColor?: string; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const onMove = useCallback((e: React.MouseEvent) => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const x = (e.clientX - r.left) / r.width;
    const y = (e.clientY - r.top) / r.height;
    const rx = -((y - 0.5) * 8);
    const ry = (x - 0.5) * 8;
    el.style.transform = `perspective(900px) rotateX(${rx}deg) rotateY(${ry}deg) translateY(-6px) scale(1.02)`;
    el.style.setProperty("--mx", `${x * 100}%`);
    el.style.setProperty("--my", `${y * 100}%`);
  }, []);
  const onLeave = useCallback(() => {
    const el = ref.current;
    if (el) el.style.transform = "perspective(900px) rotateX(0) rotateY(0) translateY(0) scale(1)";
  }, []);

  return (
    <div
      ref={ref}
      className={`glass-card ${className || ""}`}
      onMouseMove={onMove}
      onMouseLeave={onLeave}
      style={{ "--card-accent": borderColor || AL.cyan } as React.CSSProperties}
    >
      <div className="glass-card-inner">
        {children}
      </div>
    </div>
  );
}

function Tag({ children, color }: { children: React.ReactNode; color?: string }) {
  return <span className="holo-tag" style={{ "--tag-color": color || AL.cyan } as React.CSSProperties}>{children}</span>;
}

/** SVG Infinity Mark - compact premium logo for consulting hero */
function InfinityMark() {
  return (
    <div className="consult-logo">
      <svg viewBox="0 0 200 100" xmlns="http://www.w3.org/2000/svg" className="consult-infinity-svg">
        <defs>
          <linearGradient id="cInfGrad" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor={AL.cyan} />
            <stop offset="50%" stopColor={AL.green} />
            <stop offset="100%" stopColor={AL.cyan} />
          </linearGradient>
          <linearGradient id="cInfGrad2" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor={AL.magenta} stopOpacity="0.6" />
            <stop offset="50%" stopColor={AL.cyan} stopOpacity="0.4" />
            <stop offset="100%" stopColor={AL.green} stopOpacity="0.6" />
          </linearGradient>
          <filter id="cInfGlow" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur in="SourceGraphic" stdDeviation="3" result="b1" />
            <feGaussianBlur in="SourceGraphic" stdDeviation="8" result="b2" />
            <feGaussianBlur in="SourceGraphic" stdDeviation="14" result="b3" />
            <feMerge><feMergeNode in="b3" /><feMergeNode in="b2" /><feMergeNode in="b1" /><feMergeNode in="SourceGraphic" /></feMerge>
          </filter>
          <filter id="cInfHaze" x="-60%" y="-60%" width="220%" height="220%">
            <feGaussianBlur in="SourceGraphic" stdDeviation="10" />
          </filter>
        </defs>
        {/* Ambient haze */}
        <path d="M100 50 C100 26,60 14,42 30 C22 48,22 58,42 72 C60 84,100 74,100 50 C100 26,140 14,158 30 C178 48,178 58,158 72 C140 84,100 74,100 50 Z"
          fill="none" stroke="url(#cInfGrad)" strokeWidth="12" strokeLinecap="round"
          filter="url(#cInfHaze)" opacity="0.4" className="cinf-haze" />
        {/* Outer glow */}
        <path d="M100 50 C100 26,60 14,42 30 C22 48,22 58,42 72 C60 84,100 74,100 50 C100 26,140 14,158 30 C178 48,178 58,158 72 C140 84,100 74,100 50 Z"
          fill="none" stroke="url(#cInfGrad)" strokeWidth="6" strokeLinecap="round"
          filter="url(#cInfGlow)" className="cinf-glow" />
        {/* Core line */}
        <path d="M100 50 C100 26,60 14,42 30 C22 48,22 58,42 72 C60 84,100 74,100 50 C100 26,140 14,158 30 C178 48,178 58,158 72 C140 84,100 74,100 50 Z"
          fill="none" stroke="url(#cInfGrad)" strokeWidth="2.5" strokeLinecap="round" className="cinf-core" />
        {/* Hot white edge */}
        <path d="M100 50 C100 26,60 14,42 30 C22 48,22 58,42 72 C60 84,100 74,100 50 C100 26,140 14,158 30 C178 48,178 58,158 72 C140 84,100 74,100 50 Z"
          fill="none" stroke="rgba(255,255,255,0.3)" strokeWidth="1" strokeLinecap="round" />
        {/* Orbiting particle */}
        <circle r="2" fill="white" opacity="0.9">
          <animateMotion dur="4s" repeatCount="indefinite"
            path="M100 50 C100 26,60 14,42 30 C22 48,22 58,42 72 C60 84,100 74,100 50 C100 26,140 14,158 30 C178 48,178 58,158 72 C140 84,100 74,100 50 Z" />
        </circle>
        <circle r="5" fill={AL.cyan} opacity="0.2">
          <animateMotion dur="4s" repeatCount="indefinite"
            path="M100 50 C100 26,60 14,42 30 C22 48,22 58,42 72 C60 84,100 74,100 50 C100 26,140 14,158 30 C178 48,178 58,158 72 C140 84,100 74,100 50 Z" />
        </circle>
        {/* Second particle, offset */}
        <circle r="1.5" fill={AL.green} opacity="0.7">
          <animateMotion dur="4s" repeatCount="indefinite" begin="-2s"
            path="M100 50 C100 26,60 14,42 30 C22 48,22 58,42 72 C60 84,100 74,100 50 C100 26,140 14,158 30 C178 48,178 58,158 72 C140 84,100 74,100 50 Z" />
        </circle>
      </svg>
    </div>
  );
}

/* ═══════════════════ CONTACT FORM ═══════════════════ */
function ContactForm() {
  const [form, setForm] = useState({ name: "", email: "", company: "", message: "", service: "AI Security Audit" });
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const subject = encodeURIComponent(`[Consulting] ${form.service} \u2014 ${form.company || form.name}`);
    const body = encodeURIComponent(
      `Name: ${form.name}\nEmail: ${form.email}\nCompany: ${form.company}\nService: ${form.service}\n\n${form.message}`
    );
    window.open(`mailto:b.chmiel20@gmail.com?subject=${subject}&body=${body}`, "_blank");
    setSubmitted(true);
  };

  if (submitted) {
    return (
      <div className="form-success">
        <div className="success-check">{"\u2713"}</div>
        <h3>Request Sent</h3>
        <p>I&apos;ll respond within 24 hours. Check your email for confirmation.</p>
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} className="consult-form">
      <div className="form-row">
        <input className="neon-input" placeholder="Your Name *" required value={form.name}
          onChange={(e) => setForm({ ...form, name: e.target.value })} />
        <input className="neon-input" type="email" placeholder="Email *" required value={form.email}
          onChange={(e) => setForm({ ...form, email: e.target.value })} />
      </div>
      <div className="form-row">
        <input className="neon-input" placeholder="Company" value={form.company}
          onChange={(e) => setForm({ ...form, company: e.target.value })} />
        <select className="neon-input neon-select" value={form.service}
          onChange={(e) => setForm({ ...form, service: e.target.value })}>
          {SERVICES.map((s) => <option key={s.title} value={s.title}>{s.title}</option>)}
        </select>
      </div>
      <textarea className="neon-input neon-textarea" placeholder="Tell me about your project or challenge..."
        value={form.message} onChange={(e) => setForm({ ...form, message: e.target.value })} />
      <button type="submit" className="neon-submit">
        <span>Send Enquiry</span>
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
          <path d="M5 12h14M13 6l6 6-6 6" />
        </svg>
      </button>
    </form>
  );
}

/** VIKI-2400 Sphere — Atomic particle sphere with orbital rings, energy arcs & pole jets */
function VikiSphere() {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const ctx = c.getContext("2d", { alpha: true });
    if (!ctx) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    let w = 0, h = 0;
    const resize = () => { w = c.offsetWidth; h = c.offsetHeight; c.width = w * dpr; c.height = h * dpr; ctx.setTransform(dpr, 0, 0, dpr, 0, 0); };
    resize();
    /* Fibonacci sphere — 90 evenly distributed vertices */
    const N = 90, golden = (1 + Math.sqrt(5)) / 2;
    const verts: [number, number, number][] = [];
    for (let i = 0; i < N; i++) { const th = Math.acos(1 - 2 * (i + 0.5) / N); const ph = 2 * Math.PI * i / golden; verts.push([Math.sin(th) * Math.cos(ph), Math.sin(th) * Math.sin(ph), Math.cos(th)]); }
    const orbits = [
      { tx: 0.3, tz: 0.15, spd: 0.45, n: 8, rm: 1.4, col: "0,240,255" },
      { tx: -0.6, tz: 0.35, spd: -0.35, n: 6, rm: 1.6, col: "255,0,229" },
      { tx: 0.15, tz: -0.7, spd: 0.28, n: 5, rm: 1.8, col: "57,255,20" },
    ];
    let t = 0, last = 0, anim = 0;
    const PI2 = Math.PI * 2, { cos, sin, min } = Math;
    const proj = (x: number, y: number, z: number, cx: number, cy: number, R: number, ay: number, ax: number): [number, number, number] => {
      const ca = cos(ay), sa = sin(ay), cb = cos(ax), sb = sin(ax);
      const rx = x * ca + z * sa, ry0 = y, rz0 = -x * sa + z * ca;
      const ry = ry0 * cb - rz0 * sb, rz = ry0 * sb + rz0 * cb;
      const s = 500 / (500 + rz * R); return [rx * R * s + cx, ry * R * s + cy, s];
    };
    const draw = (now: number) => {
      if (now - last < 33) { anim = requestAnimationFrame(draw); return; }
      last = now; t += 0.012;
      const cx = w / 2, cy = h / 2, R = min(w, h) * 0.28;
      const ay = t * 0.35, ax = 0.18 + sin(t * 0.15) * 0.06;
      ctx.clearRect(0, 0, w, h);
      const pts = verts.map(([vx, vy, vz]) => proj(vx, vy, vz, cx, cy, R, ay, ax));
      /* wireframe */
      ctx.lineWidth = 0.4;
      for (let i = 0; i < pts.length; i += 2) for (let j = i + 2; j < pts.length; j += 2) {
        const dx = pts[i][0] - pts[j][0], dy = pts[i][1] - pts[j][1], d2 = dx * dx + dy * dy;
        if (d2 < 2800) { ctx.beginPath(); ctx.strokeStyle = `rgba(0,240,255,${(1 - d2 / 2800) * 0.1 * min(pts[i][2], pts[j][2])})`; ctx.moveTo(pts[i][0], pts[i][1]); ctx.lineTo(pts[j][0], pts[j][1]); ctx.stroke(); }
      }
      /* equatorial ring */
      ctx.beginPath(); ctx.strokeStyle = `rgba(0,240,255,${0.06 + sin(t) * 0.02})`; ctx.lineWidth = 0.6;
      for (let a = 0; a <= PI2; a += 0.04) { const [sx, sy] = proj(1.05 * cos(a), 0, 1.05 * sin(a), cx, cy, R, ay, ax); a === 0 ? ctx.moveTo(sx, sy) : ctx.lineTo(sx, sy); }
      ctx.closePath(); ctx.stroke();
      /* core */
      const cg = ctx.createRadialGradient(cx, cy, 0, cx, cy, R * 0.6);
      cg.addColorStop(0, `rgba(0,240,255,${0.05 + sin(t * 1.8) * 0.02})`); cg.addColorStop(0.35, "rgba(176,38,255,0.015)"); cg.addColorStop(1, "rgba(0,0,0,0)");
      ctx.beginPath(); ctx.arc(cx, cy, R * 0.6, 0, PI2); ctx.fillStyle = cg; ctx.fill();
      /* inner ripple */
      const rp = (t * 0.7) % 1;
      ctx.beginPath(); ctx.arc(cx, cy, R * (0.1 + rp * 0.8), 0, PI2);
      ctx.strokeStyle = `rgba(0,240,255,${(1 - rp) * 0.04})`; ctx.lineWidth = 0.6; ctx.stroke();
      /* particles */
      for (const [sx, sy, sc] of pts) {
        const r = 1 + sc * 0.6; ctx.beginPath(); ctx.arc(sx, sy, r, 0, PI2);
        ctx.fillStyle = `rgba(0,240,255,${0.2 + 0.6 * sc})`; ctx.fill();
        if (sc > 0.92) { ctx.beginPath(); ctx.arc(sx, sy, r * 2.5, 0, PI2); const pg = ctx.createRadialGradient(sx, sy, 0, sx, sy, r * 2.5); pg.addColorStop(0, `rgba(0,240,255,${(0.2 + 0.6 * sc) * 0.12})`); pg.addColorStop(1, "rgba(0,240,255,0)"); ctx.fillStyle = pg; ctx.fill(); }
      }
      /* energy arcs */
      const ab = Math.floor(t * 1.5), af = t * 1.5 - ab;
      if (af < 0.6) { ctx.lineWidth = 0.8; for (let a = 0; a < 3; a++) {
        const i1 = (ab * 13 + a * 29) % pts.length, i2 = (ab * 7 + a * 17 + 3) % pts.length;
        const adx = pts[i1][0] - pts[i2][0], ady = pts[i1][1] - pts[i2][1], ad2 = adx * adx + ady * ady;
        if (ad2 < 8000 && ad2 > 400) { ctx.beginPath(); ctx.strokeStyle = `rgba(0,240,255,${(0.6 - af) * 0.5})`;
          ctx.moveTo(pts[i1][0], pts[i1][1]); ctx.quadraticCurveTo((pts[i1][0] + pts[i2][0]) / 2 + sin(t * 50 + a * 100) * 8, (pts[i1][1] + pts[i2][1]) / 2 + cos(t * 47 + a * 80) * 6, pts[i2][0], pts[i2][1]); ctx.stroke(); }
      }}
      /* pole jets */
      const [pt0, pt1] = [proj(0, -1.15, 0, cx, cy, R, ay, ax), proj(0, 1.15, 0, cx, cy, R, ay, ax)];
      const jh = R * 0.35 * (0.7 + sin(t * 3) * 0.3);
      const jg1 = ctx.createLinearGradient(pt0[0], pt0[1] - jh, pt0[0], pt0[1]);
      jg1.addColorStop(0, "rgba(0,240,255,0)"); jg1.addColorStop(1, "rgba(0,240,255,0.06)");
      ctx.beginPath(); ctx.moveTo(pt0[0] - 2, pt0[1]); ctx.lineTo(pt0[0], pt0[1] - jh); ctx.lineTo(pt0[0] + 2, pt0[1]); ctx.fillStyle = jg1; ctx.fill();
      const jg2 = ctx.createLinearGradient(pt1[0], pt1[1], pt1[0], pt1[1] + jh);
      jg2.addColorStop(0, "rgba(255,0,229,0.04)"); jg2.addColorStop(1, "rgba(255,0,229,0)");
      ctx.beginPath(); ctx.moveTo(pt1[0] - 2, pt1[1]); ctx.lineTo(pt1[0], pt1[1] + jh); ctx.lineTo(pt1[0] + 2, pt1[1]); ctx.fillStyle = jg2; ctx.fill();
      /* orbital rings + particles */
      for (const orb of orbits) {
        const ot = t * orb.spd;
        ctx.beginPath(); ctx.strokeStyle = `rgba(${orb.col},0.03)`; ctx.lineWidth = 0.4;
        for (let a = 0; a <= PI2; a += 0.06) { const [sx, sy] = proj(orb.rm * cos(a), orb.rm * sin(a) * cos(orb.tx), orb.rm * sin(a) * sin(orb.tx) + orb.rm * cos(a) * sin(orb.tz) * 0.3, cx, cy, R, ay * 0.5, ax * 0.5); a === 0 ? ctx.moveTo(sx, sy) : ctx.lineTo(sx, sy); }
        ctx.closePath(); ctx.stroke();
        for (let i = 0; i < orb.n; i++) { const a = (i / orb.n) * PI2 + ot;
          const [sx, sy, sc] = proj(orb.rm * cos(a), orb.rm * sin(a) * cos(orb.tx), orb.rm * sin(a) * sin(orb.tx) + orb.rm * cos(a) * sin(orb.tz) * 0.3, cx, cy, R, ay * 0.5, ax * 0.5);
          const r = 1.2 + sc * 0.8; ctx.beginPath(); ctx.arc(sx, sy, r, 0, PI2); ctx.fillStyle = `rgba(${orb.col},${0.5 * sc})`; ctx.fill();
          ctx.beginPath(); ctx.arc(sx, sy, r * 3, 0, PI2); const og = ctx.createRadialGradient(sx, sy, 0, sx, sy, r * 3); og.addColorStop(0, `rgba(${orb.col},${0.12 * sc})`); og.addColorStop(1, `rgba(${orb.col},0)`); ctx.fillStyle = og; ctx.fill();
        }
      }
      /* pulse waves */
      const pw1 = (t * 0.3) % 1, pw2 = ((t * 0.3) + 0.5) % 1;
      ctx.beginPath(); ctx.arc(cx, cy, R * (1 + pw1 * 1.5), 0, PI2); ctx.strokeStyle = `rgba(0,240,255,${(1 - pw1) * 0.035})`; ctx.lineWidth = 0.8; ctx.stroke();
      ctx.beginPath(); ctx.arc(cx, cy, R * (1 + pw2 * 1.5), 0, PI2); ctx.strokeStyle = `rgba(255,0,229,${(1 - pw2) * 0.02})`; ctx.lineWidth = 0.5; ctx.stroke();
      anim = requestAnimationFrame(draw);
    };
    anim = requestAnimationFrame(draw);
    window.addEventListener("resize", resize);
    return () => { cancelAnimationFrame(anim); window.removeEventListener("resize", resize); };
  }, []);
  return <canvas ref={ref} className="viki-sphere-canvas" />;
}

/* ═══════════════════ MAIN PAGE ═══════════════════ */
export default function ConsultingPage() {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  return (
    <>
      <style>{STYLES}</style>
      <div className={`consult-page ${mounted ? "mounted" : ""}`}>

        {/* Background layers */}
        <div className="bg-grid" aria-hidden="true" />
        <div className="bg-scan-line" aria-hidden="true" />
        <div className="bg-orb bg-orb-1" aria-hidden="true" />
        <div className="bg-orb bg-orb-2" aria-hidden="true" />
        <div className="bg-orb bg-orb-3" aria-hidden="true" />
        <div className="bg-grain" aria-hidden="true" />
        <div className="bg-energy-stream bg-es-1" aria-hidden="true" />
        <div className="bg-energy-stream bg-es-2" aria-hidden="true" />
        <div className="bg-energy-stream bg-es-3" aria-hidden="true" />
        <div className="bg-holo-horizon" aria-hidden="true" />

        {/* JSON-LD */}
        <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify({
          "@context": "https://schema.org", "@type": "ProfessionalService",
          name: "Infinity AI Consulting \u2014 Bart Chmiel",
          url: "https://infinitytechstack.uk/consulting",
          description: "Enterprise AI security audits, architecture advisory, custom Rust toolchain builds, and Microsoft Purview implementation.",
          provider: { "@type": "Person", name: "Bart Chmiel", url: "https://www.linkedin.com/in/modern-workplace-tech365/" },
          areaServed: "Worldwide", priceRange: "\u00A3350 - \u00A35000+",
          hasOfferCatalog: { "@type": "OfferCatalog", name: "Consulting Services",
            itemListElement: SERVICES.map((s, i) => ({ "@type": "Offer",
              itemOffered: { "@type": "Service", name: s.title, description: s.description }, position: i + 1 })) },
        }) }} />

        {/* HERO */}
        <header className="hero-section">
          <VikiSphere />
          <div className="hero-content">
          <Reveal>
            <InfinityMark />
          </Reveal>
          <Reveal delay={150}>
            <div className="hero-label">Enterprise AI Consulting</div>
          </Reveal>
          <Reveal delay={300}>
            <h1 className="hero-title">
              Secure, Scale &amp; Architect<br />Your AI Infrastructure
            </h1>
          </Reveal>
          <Reveal delay={450}>
            <p className="hero-sub">
              From the creator of{" "}
              <a href="https://github.com/ModernOps888/vitalis" className="link-cyan">Vitalis</a>{" "}
              (35,632 LOC Rust compiler, 748 tests, 7.5{"\u00D7"} faster than Python) and{" "}
              <a href="/techstack" className="link-green">Infinity</a>{" "}
              (self-evolving autonomous AI). I help enterprise teams build, secure, and scale AI systems that actually work.
            </p>
          </Reveal>
          <Reveal delay={600}>
            <div className="hero-ctas">
              <a href="#contact" className="cta-primary">
                <span>Book a Call</span>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
              </a>
              <a href="#blueprint" className="cta-outline">Get the Blueprint</a>
            </div>
          </Reveal>
          </div>
        </header>

        {/* CREDIBILITY BAR */}
        <section className="cred-bar">
          <div className="cred-grid">
            {CREDENTIALS.map((c, i) => (
              <Reveal key={c.label} delay={i * 80}>
                <div className="cred-item">
                  <div className="cred-value">
                    <AnimatedCounter value={c.numericValue} suffix={c.suffix} />
                  </div>
                  <div className="cred-label">{c.label}</div>
                </div>
              </Reveal>
            ))}
          </div>
        </section>

        {/* SERVICES */}
        <section className="section">
          <Reveal>
            <h2 className="section-title gradient-cyan-magenta">Consulting Services</h2>
            <p className="section-sub">Enterprise-grade. Technically rigorous. Results-focused.</p>
          </Reveal>
          <div className="service-grid">
            {SERVICES.map((s, i) => (
              <Reveal key={s.title} delay={i * 120}>
                <GlassCard borderColor={s.color}>
                  <div className="card-header">
                    <span className="card-icon">{s.icon}</span>
                    <div>
                      <h3 className="card-title">{s.title}</h3>
                      <span className="card-price" style={{ color: s.color }}>{s.price}</span>
                    </div>
                  </div>
                  <p className="card-desc">{s.description}</p>
                  <div className="card-deliverables">
                    <div className="deliverables-label">Deliverables</div>
                    <ul>{s.deliverables.map((d) => <li key={d}>{d}</li>)}</ul>
                  </div>
                  <div className="card-tags">{s.tags.map((t) => <Tag key={t} color={s.color}>{t}</Tag>)}</div>
                  <a href="#contact" className="card-cta" style={{ color: s.color, borderColor: `${s.color}50`, background: `${s.color}12` }}>
                    {s.cta} <span className="cta-arrow">{"\u2192"}</span>
                  </a>
                </GlassCard>
              </Reveal>
            ))}
          </div>
        </section>

        {/* BLUEPRINTS */}
        <section id="blueprint" className="section">
          <Reveal>
            <h2 className="section-title gradient-green-cyan">Technical Blueprints</h2>
            <p className="section-sub">Implementation guides with exact code, policies, and architecture diagrams.</p>
          </Reveal>
          <div className="blueprint-grid">
            {BLUEPRINTS.map((bp, i) => (
              <Reveal key={bp.title} delay={i * 120}>
                <GlassCard borderColor={bp.color} className="blueprint-card">
                  <div className="card-header">
                    <span className="card-icon-lg">{bp.icon}</span>
                    <div>
                      <h3 className="card-title">{bp.title}</h3>
                      <span className="blueprint-price" style={{ color: bp.color }}>{bp.price}</span>
                    </div>
                  </div>
                  <p className="card-desc">{bp.description}</p>
                  <ul className="blueprint-features">
                    {bp.features.map((f) => <li key={f}><span className="check">{"\u2713"}</span> {f}</li>)}
                  </ul>
                  <a href={bp.buyLink} className="blueprint-buy" style={{ '--bp-color': bp.color } as React.CSSProperties}>
                    Get This Blueprint <span className="cta-arrow">{"\u2192"}</span>
                  </a>
                </GlassCard>
              </Reveal>
            ))}
          </div>
        </section>

        {/* WHY ME */}
        <section className="section section-narrow">
          <Reveal>
            <h2 className="section-title gradient-gold-orange">Why Work With Me</h2>
          </Reveal>
          <div className="why-grid">
            {WHY_ME.map((item, i) => (
              <Reveal key={item.title} delay={i * 100}>
                <div className="why-card">
                  <span className="why-icon">{item.icon}</span>
                  <div>
                    <h4 className="why-title">{item.title}</h4>
                    <p className="why-body">{item.body}</p>
                  </div>
                </div>
              </Reveal>
            ))}
          </div>
        </section>

        {/* CONTACT */}
        <section id="contact" className="section section-narrow">
          <Reveal>
            <h2 className="section-title gradient-cyan-green">Let&apos;s Talk</h2>
            <p className="section-sub">Tell me about your AI challenge. I&apos;ll respond within 24 hours.</p>
          </Reveal>
          <Reveal delay={200}>
            <GlassCard>
              <ContactForm />
            </GlassCard>
          </Reveal>
          <Reveal delay={350}>
            <div className="contact-direct">
              <p className="contact-direct-label">Or reach out directly:</p>
              <div className="contact-buttons">
                <a href="mailto:b.chmiel20@gmail.com" className="contact-btn contact-btn-cyan">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="2" y="4" width="20" height="16" rx="2"/><path d="M22 4L12 13 2 4"/></svg>
                  b.chmiel20@gmail.com
                </a>
                <a href="https://www.linkedin.com/in/modern-workplace-tech365/" target="_blank" rel="noopener noreferrer"
                  className="contact-btn contact-btn-green">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433a2.062 2.062 0 01-2.063-2.065 2.064 2.064 0 112.063 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z"/></svg>
                  Message on LinkedIn
                </a>
              </div>
            </div>
          </Reveal>
        </section>

        {/* FOOTER */}
        <footer className="consult-footer">
          <p>
            <a href="/" className="link-cyan">Infinity</a>
            {" \u00B7 "}
            <a href="/techstack" className="link-cyan">Tech Stack</a>
            {" \u00B7 "}
            <a href="https://github.com/ModernOps888/vitalis" className="link-cyan">Vitalis (Open Source)</a>
          </p>
          <p className="footer-copy">{"\u00A9"} {new Date().getFullYear()} Infinity AI {"\u2014"} Bart Chmiel</p>
        </footer>
      </div>
    </>
  );
}

/* ═══════════════════════════════════════════════════════════
   CSS
   ═══════════════════════════════════════════════════════════ */
const STYLES = `
/* RESET & BASE */
.consult-page {
  --c-cyan: ${AL.cyan};
  --c-green: ${AL.green};
  --c-magenta: ${AL.magenta};
  --c-gold: ${AL.gold};
  --c-orange: ${AL.orange};
  --c-violet: ${AL.violet};
  --c-dim: ${AL.dim};
  min-height: 100vh;
  background: ${AL.bg};
  color: #e0e8f8;
  font-family: 'Inter', 'SF Pro', system-ui, -apple-system, sans-serif;
  overflow-x: hidden;
  position: relative;
}

/* ═══════ BACKGROUND EFFECTS ═══════ */

/* 3D Perspective Grid Floor */
.bg-grid {
  position: fixed; inset: 0; z-index: 0; pointer-events: none;
  background-image:
    linear-gradient(rgba(0,240,255,0.035) 1px, transparent 1px),
    linear-gradient(90deg, rgba(0,240,255,0.035) 1px, transparent 1px);
  background-size: 60px 60px;
  transform: perspective(500px) rotateX(45deg);
  transform-origin: center 120%;
  mask-image: linear-gradient(to top, rgba(0,0,0,0.3) 0%, transparent 55%);
  -webkit-mask-image: linear-gradient(to top, rgba(0,0,0,0.3) 0%, transparent 55%);
  animation: gridScroll 20s linear infinite;
}
@keyframes gridScroll {
  0% { background-position: 0 0; }
  100% { background-position: 0 60px; }
}

/* Horizontal scanner */
.bg-scan-line {
  position: fixed; left: 0; right: 0; height: 2px; z-index: 1; pointer-events: none;
  background: linear-gradient(90deg, transparent, var(--c-cyan), transparent);
  opacity: 0.1;
  animation: scanDown 8s ease-in-out infinite;
}
@keyframes scanDown {
  0% { top: -2px; }
  100% { top: 100vh; }
}

/* Floating gradient orbs */
.bg-orb {
  position: fixed; border-radius: 50%; pointer-events: none; z-index: 0;
  filter: blur(80px);
  animation: orbFloat 12s ease-in-out infinite;
}
.bg-orb-1 {
  width: 500px; height: 500px; top: -150px; right: -100px;
  background: radial-gradient(circle, rgba(0,240,255,0.07), transparent 70%);
  animation-duration: 14s;
}
.bg-orb-2 {
  width: 400px; height: 400px; bottom: 10%; left: -80px;
  background: radial-gradient(circle, rgba(255,0,229,0.05), transparent 70%);
  animation-duration: 18s; animation-delay: -6s;
}
.bg-orb-3 {
  width: 350px; height: 350px; top: 40%; right: -60px;
  background: radial-gradient(circle, rgba(57,255,20,0.04), transparent 70%);
  animation-duration: 16s; animation-delay: -10s;
}
@keyframes orbFloat {
  0%,100% { transform: translate(0, 0) scale(1); }
  25% { transform: translate(30px, -20px) scale(1.05); }
  50% { transform: translate(-20px, 30px) scale(0.95); }
  75% { transform: translate(15px, 15px) scale(1.02); }
}

/* Film grain noise overlay */
.bg-grain {
  position: fixed; inset: 0; z-index: 1; pointer-events: none;
  opacity: 0.03;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)' opacity='1'/%3E%3C/svg%3E");
  background-repeat: repeat;
  background-size: 256px 256px;
}

/* ═══════ VIKI SPHERE ═══════ */
.viki-sphere-canvas {
  position: absolute;
  top: 50%; left: 50%;
  transform: translate(-50%, -48%);
  width: min(520px, 90vw);
  height: min(520px, 90vw);
  pointer-events: none;
  z-index: 0;
  will-change: contents;
  contain: strict;
}
.hero-content {
  position: relative;
  z-index: 1;
}

/* ═══════ ENERGY STREAMS ═══════ */
.bg-energy-stream {
  position: fixed;
  width: 1px;
  top: 0; bottom: 0;
  z-index: 0;
  pointer-events: none;
  opacity: 0;
  animation: streamPulse 6s ease-in-out infinite;
}
.bg-energy-stream::before {
  content: '';
  position: absolute;
  top: 0; bottom: 0; left: -1px; right: -1px;
  background: linear-gradient(to bottom,
    transparent 0%,
    rgba(0,240,255,0.12) 30%,
    rgba(0,240,255,0.2) 50%,
    rgba(0,240,255,0.12) 70%,
    transparent 100%);
  filter: blur(2px);
}
.bg-es-1 { left: 12%; animation-delay: 0s; }
.bg-es-2 { right: 15%; animation-delay: -2s; }
.bg-es-3 { left: 50%; animation-delay: -4s; }
.bg-es-3::before {
  background: linear-gradient(to bottom,
    transparent 0%,
    rgba(255,0,229,0.06) 40%,
    rgba(255,0,229,0.1) 50%,
    rgba(255,0,229,0.06) 60%,
    transparent 100%);
}
@keyframes streamPulse {
  0%, 100% { opacity: 0; }
  30%, 70% { opacity: 1; }
}

/* ═══════ HOLOGRAPHIC HORIZON ═══════ */
.bg-holo-horizon {
  position: fixed;
  bottom: 0; left: 0; right: 0;
  height: 1px; z-index: 0;
  pointer-events: none;
  background: linear-gradient(90deg,
    transparent 5%, rgba(0,240,255,0.1) 20%,
    rgba(255,0,229,0.08) 50%,
    rgba(57,255,20,0.1) 80%, transparent 95%);
  box-shadow: 0 0 30px rgba(0,240,255,0.04), 0 -1px 60px rgba(0,240,255,0.02);
}

/* ═══════ REVEAL ANIMATION ═══════ */
.reveal-wrap {
  opacity: 0;
  transform: translateY(30px);
  transition: opacity 0.7s cubic-bezier(0.16, 1, 0.3, 1), transform 0.7s cubic-bezier(0.16, 1, 0.3, 1);
}
.consult-page.mounted .reveal-wrap.revealed {
  opacity: 1;
  transform: translateY(0);
}

/* ═══════ GLASS CARD ═══════ */
.glass-card {
  position: relative;
  border-radius: 20px;
  padding: 2px;
  background: linear-gradient(135deg,
    rgba(0,240,255,0.18),
    rgba(255,0,229,0.1),
    rgba(57,255,20,0.12),
    rgba(0,240,255,0.18));
  background-size: 300% 300%;
  animation: holoBorder 6s ease infinite;
  transition: transform 0.2s ease, box-shadow 0.4s ease;
  will-change: transform;
}
.glass-card:hover {
  box-shadow:
    0 8px 40px rgba(0,240,255,0.12),
    0 0 60px rgba(0,240,255,0.05),
    inset 0 0 30px rgba(0,240,255,0.02);
}
@keyframes holoBorder {
  0% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
  100% { background-position: 0% 50%; }
}
.glass-card-inner {
  background: ${AL.card};
  backdrop-filter: blur(20px) saturate(1.4);
  -webkit-backdrop-filter: blur(20px) saturate(1.4);
  border-radius: 18px;
  padding: 2rem;
  position: relative;
  overflow: hidden;
}
/* Mouse-follow light spot */
.glass-card-inner::before {
  content: '';
  position: absolute;
  width: 250px; height: 250px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(0,240,255,0.07), transparent 70%);
  top: var(--my, 50%); left: var(--mx, 50%);
  transform: translate(-50%, -50%);
  pointer-events: none;
  transition: top 0.15s ease, left 0.15s ease;
}

/* ═══════ HOLOGRAPHIC TAGS ═══════ */
.holo-tag {
  display: inline-block;
  padding: 0.25rem 0.7rem;
  font-size: 0.62rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  border-radius: 5px;
  color: var(--tag-color);
  background: color-mix(in srgb, var(--tag-color) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--tag-color) 20%, transparent);
  margin-right: 0.4rem;
  margin-bottom: 0.35rem;
  transition: all 0.25s ease;
}
.holo-tag:hover {
  background: color-mix(in srgb, var(--tag-color) 15%, transparent);
  box-shadow: 0 0 14px color-mix(in srgb, var(--tag-color) 25%, transparent);
  transform: translateY(-1px);
}

/* ═══════ INFINITY LOGO ═══════ */
.consult-logo {
  display: flex; justify-content: center; margin-bottom: 0.5rem;
  animation: logoFloat 5s ease-in-out infinite;
}
.consult-infinity-svg {
  width: clamp(120px, 26vw, 200px); height: auto;
  pointer-events: none; user-select: none;
}
.cinf-haze { animation: cinfPulseHaze 4s ease-in-out infinite; }
.cinf-glow { animation: cinfGlow 3s ease-in-out infinite; }
.cinf-core { animation: cinfPulse 2.5s ease-in-out infinite; }
@keyframes logoFloat {
  0%,100% { transform: translateY(0); }
  50% { transform: translateY(-8px); }
}
@keyframes cinfPulseHaze {
  0%,100% { opacity: 0.3; } 50% { opacity: 0.5; }
}
@keyframes cinfGlow {
  0%,100% { opacity: 0.7; } 50% { opacity: 1; }
}
@keyframes cinfPulse {
  0%,100% { stroke-width: 2.5; } 50% { stroke-width: 3.5; }
}

/* ═══════ HERO ═══════ */
.hero-section {
  position: relative; z-index: 2;
  padding: 5rem 2rem 4rem;
  text-align: center;
}
.hero-label {
  font-size: 0.72rem;
  letter-spacing: 0.35em;
  color: var(--c-cyan);
  text-transform: uppercase;
  margin-bottom: 1.2rem;
  font-weight: 600;
}
.hero-title {
  font-size: clamp(2rem, 5.5vw, 3.8rem);
  font-weight: 900;
  line-height: 1.08;
  margin: 0 0 1.5rem;
  background: linear-gradient(135deg, var(--c-cyan), #fff 40%, var(--c-green));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  filter: drop-shadow(0 0 40px rgba(0,240,255,0.15));
}
.hero-sub {
  font-size: 1.1rem;
  color: #8a9ab5;
  line-height: 1.75;
  max-width: 620px;
  margin: 0 auto 2.5rem;
}
.link-cyan { color: var(--c-cyan); text-decoration: none; transition: color 0.2s; }
.link-cyan:hover { color: #66f7ff; text-decoration: underline; }
.link-green { color: var(--c-green); text-decoration: none; transition: color 0.2s; }
.link-green:hover { color: #70ff50; text-decoration: underline; }

/* ═══════ CTAS ═══════ */
.hero-ctas { display: flex; gap: 1rem; justify-content: center; flex-wrap: wrap; }
.cta-primary {
  display: inline-flex; align-items: center; gap: 0.5rem;
  padding: 0.9rem 2rem;
  background: linear-gradient(135deg, var(--c-cyan), var(--c-green));
  color: #000; border-radius: 10px; font-weight: 800; font-size: 0.9rem;
  text-decoration: none;
  position: relative; overflow: hidden;
  transition: transform 0.2s, box-shadow 0.3s;
}
.cta-primary::after {
  content: '';
  position: absolute; top: 0; left: -100%;
  width: 100%; height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.25), transparent);
  animation: btnShine 3s ease-in-out infinite;
}
@keyframes btnShine {
  0% { left: -100%; }
  50%,100% { left: 100%; }
}
.cta-primary:hover {
  transform: translateY(-3px) scale(1.02);
  box-shadow: 0 10px 40px rgba(0,240,255,0.3), 0 0 20px rgba(57,255,20,0.2);
}
.cta-outline {
  display: inline-flex; align-items: center;
  padding: 0.9rem 2rem;
  border: 1px solid rgba(0,240,255,0.3);
  color: var(--c-cyan); border-radius: 10px;
  font-weight: 700; font-size: 0.9rem;
  text-decoration: none;
  transition: all 0.3s ease;
}
.cta-outline:hover {
  border-color: var(--c-cyan);
  background: rgba(0,240,255,0.06);
  box-shadow: 0 0 20px rgba(0,240,255,0.1);
  transform: translateY(-2px);
}

/* ═══════ CREDIBILITY BAR ═══════ */
.cred-bar {
  position: relative; z-index: 2;
  padding: 3.5rem 2rem;
}
.cred-grid {
  max-width: 960px; margin: 0 auto;
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 0; text-align: center;
}
.cred-item {
  padding: 0.8rem 0.5rem;
  position: relative;
}
.cred-item::after {
  content: '';
  position: absolute; right: 0; top: 20%; height: 60%;
  width: 1px;
  background: linear-gradient(180deg, transparent, rgba(0,240,255,0.1), transparent);
}
.cred-value {
  font-size: 1.5rem; font-weight: 700;
  color: #fff;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  letter-spacing: -0.02em;
}
.cred-label {
  font-size: 0.54rem; color: rgba(138,154,181,0.45);
  letter-spacing: 0.1em; text-transform: uppercase;
  margin-top: 0.3rem; line-height: 1.45;
}

/* ═══════ SECTIONS ═══════ */
.section {
  position: relative; z-index: 2;
  padding: 5rem 2rem;
  max-width: 1100px; margin: 0 auto;
}
.section-narrow { max-width: 800px; }
.section + .section { border-top: none; }
.section-title {
  text-align: center;
  font-size: 0.68rem; font-weight: 600;
  letter-spacing: 0.35em;
  text-transform: uppercase;
  margin: 0 0 0.8rem;
}
.section-title::after { display: none; }
.gradient-cyan-magenta { color: var(--c-cyan); }
.gradient-green-cyan { color: var(--c-green); }
.gradient-gold-orange { color: var(--c-gold); }
.gradient-cyan-green { color: var(--c-cyan); }
.section-sub {
  text-align: center; color: rgba(224,232,248,0.55);
  margin-bottom: 3.5rem; font-size: 1rem;
  font-weight: 300; line-height: 1.7;
  max-width: 520px; margin-left: auto; margin-right: auto;
}

/* ═══════ SERVICE CARDS ═══════ */
.service-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(480px, 1fr));
  gap: 1.5rem;
}
.card-header { display: flex; align-items: flex-start; gap: 1rem; margin-bottom: 1rem; }
.card-icon { font-size: 2.2rem; filter: drop-shadow(0 0 8px rgba(0,240,255,0.3)); }
.card-icon-lg { font-size: 2.8rem; filter: drop-shadow(0 0 10px rgba(0,240,255,0.3)); }
.card-title { font-size: 1.2rem; font-weight: 800; color: #fff; margin: 0; }
.card-price { font-size: 0.85rem; font-weight: 700; }
.card-desc { color: #8a9ab5; font-size: 0.87rem; line-height: 1.75; margin-bottom: 1rem; }
.card-deliverables { margin-bottom: 1rem; }
.deliverables-label {
  font-size: 0.68rem; color: var(--c-dim);
  letter-spacing: 0.12em; text-transform: uppercase; margin-bottom: 0.5rem;
}
.card-deliverables ul {
  margin: 0; padding-left: 1.2rem;
  color: #c0cce0; font-size: 0.82rem; line-height: 2;
}
.card-tags { margin-bottom: 1.2rem; }
.card-cta {
  display: inline-flex; align-items: center; gap: 0.4rem;
  padding: 0.65rem 1.5rem;
  border: 1px solid;
  border-radius: 8px; font-weight: 700; font-size: 0.82rem;
  text-decoration: none;
  transition: all 0.25s ease;
}
.card-cta:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 25px rgba(0,0,0,0.3);
  filter: brightness(1.2);
}
.cta-arrow { transition: transform 0.2s; display: inline-block; }
.card-cta:hover .cta-arrow { transform: translateX(4px); }

/* ═══════ BLUEPRINTS ═══════ */
.blueprint-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(440px, 1fr));
  gap: 1.5rem;
}
.blueprint-price { font-size: 1.1rem; font-weight: 700; }
.blueprint-features {
  margin: 0 0 1.5rem; padding-left: 0;
  list-style: none; color: rgba(192,204,224,0.7); font-size: 0.8rem; line-height: 2.2;
}
.blueprint-features li { display: flex; align-items: center; gap: 0.5rem; }
.check { color: var(--c-green); font-weight: 600; opacity: 0.7; }
.blueprint-buy {
  display: inline-flex; align-items: center; gap: 0.5rem;
  padding: 0.7rem 1.6rem;
  color: var(--bp-color, var(--c-cyan));
  border: 1px solid color-mix(in srgb, var(--bp-color, var(--c-cyan)) 25%, transparent);
  background: color-mix(in srgb, var(--bp-color, var(--c-cyan)) 5%, transparent);
  border-radius: 8px; font-weight: 600; font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-decoration: none;
  transition: all 0.3s ease;
  position: relative; overflow: hidden;
}
.blueprint-buy::after { display: none; }
.blueprint-buy:hover {
  background: color-mix(in srgb, var(--bp-color, var(--c-cyan)) 10%, transparent);
  border-color: var(--bp-color, var(--c-cyan));
  box-shadow: 0 0 25px color-mix(in srgb, var(--bp-color, var(--c-cyan)) 15%, transparent);
  transform: translateY(-2px);
}

/* ═══════ WHY ME ═══════ */
.why-grid { display: grid; gap: 1rem; }
.why-card {
  display: flex; gap: 1.2rem; align-items: flex-start;
  padding: 1.5rem;
  background: rgba(0,240,255,0.02);
  border-radius: 16px;
  border: 1px solid rgba(0,240,255,0.06);
  transition: all 0.3s ease;
}
.why-card:hover {
  background: rgba(0,240,255,0.04);
  border-color: rgba(0,240,255,0.15);
  transform: translateX(6px);
  box-shadow: -4px 0 20px rgba(0,240,255,0.06);
}
.why-icon { font-size: 1.6rem; flex-shrink: 0; filter: drop-shadow(0 0 6px rgba(0,240,255,0.2)); }
.why-title { font-size: 1rem; font-weight: 800; color: #fff; margin: 0 0 0.35rem; }
.why-body { color: #8a9ab5; font-size: 0.85rem; line-height: 1.75; margin: 0; }

/* ═══════ CONTACT FORM ═══════ */
.consult-form { display: flex; flex-direction: column; gap: 1rem; }
.form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.neon-input {
  width: 100%; padding: 0.8rem 1rem;
  background: rgba(0,240,255,0.03);
  border: 1px solid rgba(0,240,255,0.12);
  border-radius: 10px;
  color: #e0e8f8; font-size: 0.9rem; font-family: inherit;
  outline: none;
  transition: border-color 0.3s, box-shadow 0.3s;
}
.neon-input::placeholder { color: rgba(224,232,248,0.3); }
.neon-input:focus {
  border-color: var(--c-cyan);
  box-shadow: 0 0 15px rgba(0,240,255,0.1), inset 0 0 15px rgba(0,240,255,0.03);
}
.neon-select { cursor: pointer; }
.neon-select option { background: #0a0f1e; color: #e0e8f8; }
.neon-textarea { min-height: 120px; resize: vertical; }
.neon-submit {
  display: inline-flex; align-items: center; justify-content: center; gap: 0.6rem;
  padding: 1rem 2rem;
  background: linear-gradient(135deg, var(--c-cyan), var(--c-green));
  color: #000; border: none; border-radius: 10px;
  font-size: 1rem; font-weight: 800; letter-spacing: 0.03em;
  cursor: pointer;
  position: relative; overflow: hidden;
  transition: transform 0.2s, box-shadow 0.3s;
}
.neon-submit::after {
  content: '';
  position: absolute; top: 0; left: -100%;
  width: 100%; height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.25), transparent);
  animation: btnShine 3s ease-in-out infinite;
}
.neon-submit:hover {
  transform: translateY(-2px);
  box-shadow: 0 10px 35px rgba(0,240,255,0.25);
}
.form-success { text-align: center; padding: 3rem 2rem; }
.success-check {
  font-size: 3.5rem; margin-bottom: 1rem;
  color: var(--c-green);
  text-shadow: 0 0 30px rgba(57,255,20,0.4);
  animation: checkPulse 2s ease-in-out infinite;
}
@keyframes checkPulse {
  0%,100% { transform: scale(1); } 50% { transform: scale(1.1); }
}
.form-success h3 { color: var(--c-green); font-size: 1.4rem; margin: 0 0 0.5rem; }
.form-success p { color: #8a9ab5; margin: 0; }

/* ═══════ CONTACT DIRECT ═══════ */
.contact-direct { text-align: center; margin-top: 2rem; }
.contact-direct-label { color: var(--c-dim); font-size: 0.85rem; margin-bottom: 1rem; }
.contact-buttons { display: flex; gap: 1rem; justify-content: center; flex-wrap: wrap; }
.contact-btn {
  display: inline-flex; align-items: center; gap: 0.6rem;
  padding: 0.8rem 1.4rem;
  border-radius: 10px; font-weight: 700; font-size: 0.82rem;
  text-decoration: none;
  transition: all 0.25s ease;
  border: 1px solid;
}
.contact-btn-cyan {
  color: var(--c-cyan);
  background: rgba(0,240,255,0.06);
  border-color: rgba(0,240,255,0.2);
}
.contact-btn-cyan:hover {
  background: rgba(0,240,255,0.12);
  border-color: var(--c-cyan);
  box-shadow: 0 0 20px rgba(0,240,255,0.15);
  transform: translateY(-2px);
}
.contact-btn-green {
  color: var(--c-green);
  background: rgba(57,255,20,0.06);
  border-color: rgba(57,255,20,0.2);
}
.contact-btn-green:hover {
  background: rgba(57,255,20,0.12);
  border-color: var(--c-green);
  box-shadow: 0 0 20px rgba(57,255,20,0.15);
  transform: translateY(-2px);
}

/* ═══════ FOOTER ═══════ */
.consult-footer {
  position: relative; z-index: 2;
  padding: 2rem;
  text-align: center;
  border-top: 1px solid rgba(0,240,255,0.06);
  color: var(--c-dim);
  font-size: 0.75rem;
}
.consult-footer p { margin: 0; }
.footer-copy { margin-top: 0.5rem !important; }

/* ═══════ RESPONSIVE ═══════ */
@media (max-width: 1024px) {
  .service-grid { grid-template-columns: 1fr; }
  .blueprint-grid { grid-template-columns: 1fr; }
}
@media (max-width: 640px) {
  .form-row { grid-template-columns: 1fr; }
  .hero-section { padding: 3.5rem 1.5rem 3rem; }
  .section { padding: 3rem 1.5rem; }
  .cred-grid { grid-template-columns: repeat(3, 1fr); gap: 0; }
  .cred-value { font-size: 1.2rem; }
  .hero-title { font-size: 1.8rem; }
  .contact-buttons { flex-direction: column; align-items: center; }
  .viki-sphere-canvas {
    width: min(340px, 85vw);
    height: min(340px, 85vw);
  }
}

/* ═══════ ACCESSIBILITY ═══════ */
@media (prefers-reduced-motion: reduce) {
  .bg-grid, .bg-scan-line, .bg-orb, .consult-logo,
  .cinf-haze, .cinf-glow, .cinf-core,
  .glass-card, .cta-primary::after, .blueprint-buy::after,
  .neon-submit::after, .success-check, .bg-energy-stream { animation: none !important; }
  .reveal-wrap { opacity: 1; transform: none; transition: none; }
  .glass-card { background-size: 100% 100%; }
  .viki-sphere-canvas { display: none; }
}
`;
