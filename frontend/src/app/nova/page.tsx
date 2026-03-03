"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import Link from "next/link";

/* ═══════════════════════════════════════════════════════════
   ALIEN COLOR PALETTE (matching techstack)
   ═══════════════════════════════════════════════════════════ */
const A = {
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
   NOVA SOURCE INVENTORY
   ═══════════════════════════════════════════════════════════ */
const MODULES = [
  { name: "app.rs", loc: 1081, dir: "studio", purpose: "Nova Studio GUI — real-time training dashboard (egui/glow)" },
  { name: "ops.rs", loc: 856, dir: "tensor", purpose: "100+ tensor operations — matmul, softmax, RoPE, layer_norm, GELU" },
  { name: "main.rs", loc: 681, dir: "src", purpose: "CLI entry — train, generate, bench, data commands" },
  { name: "backward.rs", loc: 546, dir: "training", purpose: "Full backward pass — cross-entropy grad, attention grad, FFN grad" },
  { name: "synthetic.rs", loc: 436, dir: "data", purpose: "Synthetic data generation — math, code, reasoning tasks" },
  { name: "mod.rs", loc: 430, dir: "tensor", purpose: "Tensor core — shape broadcasting, slicing, CUDA dispatch" },
  { name: "state.rs", loc: 330, dir: "studio", purpose: "GUI state management — training metrics, GPU telemetry" },
  { name: "transformer.rs", loc: 324, dir: "nn", purpose: "Transformer block — multi-head attention + FFN + RMSNorm" },
  { name: "kernels.rs", loc: 322, dir: "gpu", purpose: "Custom CUDA kernels — fused ops, memory-efficient attention" },
  { name: "domains.rs", loc: 316, dir: "data", purpose: "Curriculum domains — Shakespeare, code, math, science" },
  { name: "config.rs", loc: 314, dir: "model", purpose: "Model config — d_model, n_layers, n_heads, rope_base, vocab_size" },
  { name: "cuda.rs", loc: 310, dir: "gpu", purpose: "CUDA device management — alloc, copy, synchronize, cuBLAS" },
  { name: "autonomous.rs", loc: 301, dir: "evolution", purpose: "Autonomous evolution — architecture mutation, fitness selection" },
  { name: "attention.rs", loc: 297, dir: "nn", purpose: "Grouped-query attention — KV cache, RoPE positional encoding" },
  { name: "web_fetcher.rs", loc: 277, dir: "training", purpose: "Auto-fetches training data from web (Shakespeare, wiki)" },
  { name: "fitness.rs", loc: 268, dir: "evolution", purpose: "Fitness evaluation — loss, speed, memory efficiency scoring" },
  { name: "bpe.rs", loc: 263, dir: "tokenizer", purpose: "Byte-pair encoding — train vocab, encode, decode, merge rules" },
  { name: "training.rs", loc: 260, dir: "panels", purpose: "Training panel — live loss chart, learning rate, step counter" },
  { name: "mutator.rs", loc: 254, dir: "evolution", purpose: "Architecture mutator — add/remove layers, heads, dimensions" },
  { name: "optimizer.rs", loc: 218, dir: "training", purpose: "AdamW optimizer — weight decay, gradient clipping, EMA" },
  { name: "autograd.rs", loc: 199, dir: "tensor", purpose: "Automatic differentiation — computation graph, backward pass" },
  { name: "trainer.rs", loc: 208, dir: "training", purpose: "Training loop — forward, backward, optimizer step, checkpoints" },
  { name: "norm.rs", loc: 192, dir: "nn", purpose: "RMSNorm — pre-norm residual connections, eps stability" },
  { name: "generate.rs", loc: 189, dir: "model", purpose: "Text generation — temperature, top-k, top-p sampling" },
  { name: "shape.rs", loc: 164, dir: "tensor", purpose: "Shape algebra — broadcast rules, reshape, transpose, permute" },
  { name: "scheduler.rs", loc: 87, dir: "training", purpose: "Cosine annealing LR scheduler with linear warmup" },
];

const ARCH_LAYERS = [
  { name: "Tokenizer", desc: "BPE · 8K vocab", color: A.green },
  { name: "Embedding", desc: "Token + RoPE pos", color: A.cyan },
  { name: "Transformer ×4", desc: "GQA + SwiGLU FFN", color: A.magenta },
  { name: "RMSNorm", desc: "Pre-norm residual", color: A.violet },
  { name: "LM Head", desc: "Tied weights", color: A.orange },
  { name: "Softmax", desc: "Next token probs", color: A.gold },
];

const TRAINING_STATS = {
  totalSteps: 50000,
  currentStep: 157,
  bestLoss: 7.2347,
  tokPerSec: 295,
  gpu: "NVIDIA RTX 5060",
  vram: "2,248 MB",
  batchSize: 4,
  gradAccum: 4,
  seqLen: 512,
  lr: "5e-4",
  vocabSize: 8000,
  dModel: 128,
  nLayers: 4,
  nHeads: 4,
  params: "~5M",
};

const TECH_DEPS = [
  { name: "cudarc", version: "0.19", purpose: "Safe CUDA bindings — nvrtc, cuBLAS, driver API" },
  { name: "rayon", version: "1.10", purpose: "CPU parallelism — data loading, tokenization" },
  { name: "memmap2", version: "0.9", purpose: "Memory-mapped files — 0-copy dataset access" },
  { name: "eframe/egui", version: "0.31", purpose: "GPU-accelerated native GUI — training dashboard" },
  { name: "serde + toml", version: "1 / 0.8", purpose: "Config serialization — model hyperparameters" },
  { name: "indicatif", version: "0.17", purpose: "Training progress bars — ETA, throughput" },
  { name: "half", version: "2", purpose: "FP16 half-precision — reduced VRAM" },
  { name: "clap", version: "4", purpose: "CLI — train, generate, bench subcommands" },
  { name: "ureq", version: "2", purpose: "HTTP — auto-fetch Shakespeare/training data" },
  { name: "chrono", version: "0.4", purpose: "Timestamps — checkpoint naming, logs" },
];

/* ═══════════════════════════════════════════════════════════
   SHARED UI COMPONENTS
   ═══════════════════════════════════════════════════════════ */
function ParticleField() {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    let raf: number;
    const COLORS = ["rgba(255,106,0,","rgba(255,0,229,","rgba(0,240,255,","rgba(176,38,255,"];
    const ps: { x:number;y:number;r:number;dx:number;dy:number;c:string;a:number;p:number }[] = [];
    function resize(){ c!.width=window.innerWidth; c!.height=window.innerHeight; }
    resize(); window.addEventListener("resize",resize);
    for(let i=0;i<60;i++) ps.push({
      x:Math.random()*(c.width||1920), y:Math.random()*(c.height||1080),
      r:Math.random()*2+0.3, dx:(Math.random()-0.5)*0.3, dy:(Math.random()-0.5)*0.3,
      c:COLORS[Math.floor(Math.random()*COLORS.length)], a:Math.random()*0.4+0.1, p:Math.random()*Math.PI*2,
    });
    function draw(){
      ctx!.clearRect(0,0,c!.width,c!.height);
      const t=performance.now()*0.001;
      for(const p of ps){
        p.x+=p.dx; p.y+=p.dy;
        if(p.x<-10)p.x=c!.width+10; if(p.x>c!.width+10)p.x=-10;
        if(p.y<-10)p.y=c!.height+10; if(p.y>c!.height+10)p.y=-10;
        const a=p.a*(0.5+0.5*Math.sin(t*0.03*60+p.p));
        ctx!.beginPath(); ctx!.arc(p.x,p.y,p.r,0,Math.PI*2);
        ctx!.fillStyle=p.c+a.toFixed(3)+")"; ctx!.fill();
        ctx!.beginPath(); ctx!.arc(p.x,p.y,p.r*3,0,Math.PI*2);
        ctx!.fillStyle=p.c+(a*0.1).toFixed(3)+")"; ctx!.fill();
      }
      raf=requestAnimationFrame(draw);
    }
    draw();
    return()=>{cancelAnimationFrame(raf);window.removeEventListener("resize",resize)};
  }, []);
  return <canvas ref={ref} style={{position:"fixed",inset:0,zIndex:0,pointerEvents:"none"}} />;
}

function Reveal({children,className="",delay=0}:{children:React.ReactNode;className?:string;delay?:number}){
  const ref=useRef<HTMLDivElement>(null);
  const[v,setV]=useState(false);
  useEffect(()=>{
    const el=ref.current; if(!el)return;
    const obs=new IntersectionObserver(([e])=>{if(e.isIntersecting){setV(true);obs.disconnect();}},{threshold:0.08,rootMargin:"0px 0px -60px 0px"});
    obs.observe(el);
    // Fallback: ensure content visible after 1.2s even if observer fails
    const timer=setTimeout(()=>setV(true),1200);
    return()=>{obs.disconnect();clearTimeout(timer);};
  },[]);
  return(
    <div ref={ref} style={{opacity:v?1:0,transform:v?"translateY(0)":"translateY(24px)",transition:`opacity 0.7s ease ${delay}s, transform 0.7s ease ${delay}s`}} className={className}>
      {children}
    </div>
  );
}

function Card({children,border,className=""}:{children:React.ReactNode;border?:string;className?:string}){
  const ref=useRef<HTMLDivElement>(null);
  const onMove=useCallback((e:React.MouseEvent<HTMLDivElement>)=>{
    const el=ref.current; if(!el)return;
    const r=el.getBoundingClientRect();
    const rx=(e.clientY-r.top-r.height/2)/r.height*-4;
    const ry=(e.clientX-r.left-r.width/2)/r.width*4;
    el.style.transform=`perspective(800px) rotateX(${rx}deg) rotateY(${ry}deg) translateY(-2px) scale(1.008)`;
  },[]);
  const onLeave=useCallback(()=>{
    const el=ref.current; if(el) el.style.transform="perspective(800px) rotateX(0) rotateY(0) translateY(0) scale(1)";
  },[]);
  return(
    <div ref={ref} onMouseMove={onMove} onMouseLeave={onLeave} className={className}
      style={{background:"rgba(17,17,39,0.65)",border:`1px solid ${border||"rgba(0,240,255,0.12)"}`,borderRadius:16,padding:"1.5rem",backdropFilter:"blur(12px)",transition:"transform 0.25s ease, box-shadow 0.25s ease",willChange:"transform"}}>
      {children}
    </div>
  );
}

function SectionHeading({icon,title,badge,color=A.cyan}:{icon:string;title:string;badge?:string;color?:string}){
  return(
    <div style={{display:"flex",alignItems:"center",gap:12,marginBottom:24}}>
      <span style={{fontSize:28,filter:`drop-shadow(0 0 8px ${color})`}}>{icon}</span>
      <h2 style={{fontSize:22,fontWeight:800,letterSpacing:"0.02em",background:`linear-gradient(135deg,${color},${A.magenta})`,WebkitBackgroundClip:"text",WebkitTextFillColor:"transparent",margin:0}}>{title}</h2>
      {badge&&<span style={{fontSize:10,fontWeight:700,letterSpacing:"0.08em",textTransform:"uppercase",padding:"3px 10px",borderRadius:20,border:`1px solid ${color}40`,color,background:`${color}0d`}}>{badge}</span>}
    </div>
  );
}

function StatBox({label,value,color=A.cyan,sub}:{label:string;value:string;color?:string;sub?:string}){
  return(
    <div style={{textAlign:"center",padding:"1rem"}}>
      <div style={{fontSize:28,fontWeight:900,fontFamily:"monospace",color,textShadow:`0 0 20px ${color}40`}}>{value}</div>
      <div style={{fontSize:11,color:A.dim,marginTop:4,letterSpacing:"0.06em",textTransform:"uppercase"}}>{label}</div>
      {sub&&<div style={{fontSize:10,color:"#4e4e6e",marginTop:2}}>{sub}</div>}
    </div>
  );
}

function GaugeRing({pct,label,color}:{pct:number;label:string;color:string}){
  const c=2*Math.PI*42;
  const off=c-(pct/100)*c;
  return(
    <div style={{textAlign:"center"}}>
      <svg width={100} height={100} viewBox="0 0 100 100">
        <circle cx={50} cy={50} r={42} fill="none" stroke="rgba(0,240,255,0.06)" strokeWidth={4}/>
        <circle cx={50} cy={50} r={42} fill="none" stroke={color} strokeWidth={4}
          strokeDasharray={c} strokeDashoffset={off} strokeLinecap="round" transform="rotate(-90 50 50)"
          style={{transition:"stroke-dashoffset 1s ease",filter:`drop-shadow(0 0 8px ${color})`}}/>
        <text x={50} y={46} textAnchor="middle" fill={color} fontSize={18} fontWeight={800} fontFamily="monospace">{Math.round(pct)}%</text>
        <text x={50} y={62} textAnchor="middle" fill={A.dim} fontSize={8} letterSpacing="0.1em">{label}</text>
      </svg>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   MAIN PAGE
   ═══════════════════════════════════════════════════════════ */
export default function NovaPage() {
  const maxLoc = Math.max(...MODULES.map(m => m.loc));

  return (
    <div style={{ minHeight: "100vh", position: "relative", overflow: "hidden" }}>
      <ParticleField />

      {/* Back nav */}
      <div style={{ position: "fixed", top: 20, left: 20, zIndex: 100 }}>
        <Link href="/techstack" style={{ color: A.cyan, textDecoration: "none", fontSize: 13, fontWeight: 600, letterSpacing: "0.04em", display: "flex", alignItems: "center", gap: 6, opacity: 0.7, transition: "opacity 0.2s" }}
          onMouseEnter={e => (e.currentTarget.style.opacity = "1")}
          onMouseLeave={e => (e.currentTarget.style.opacity = "0.7")}>
          ← Infinity Tech Stack
        </Link>
      </div>

      <main style={{ position: "relative", zIndex: 1, maxWidth: 1100, margin: "0 auto", padding: "80px 24px 100px" }}>

        {/* ═══════ HERO ═══════ */}
        <Reveal>
          <header style={{ textAlign: "center", marginBottom: 80 }}>
            <div style={{ fontSize: 64, marginBottom: 8, filter: `drop-shadow(0 0 30px ${A.orange}60)` }}>⚡</div>
            <h1 style={{ fontSize: 52, fontWeight: 900, letterSpacing: "0.08em", background: `linear-gradient(135deg, ${A.orange}, ${A.magenta}, ${A.cyan})`, WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent", margin: "0 0 12px", lineHeight: 1.1 }}>
              NOVA
            </h1>
            <p style={{ fontSize: 16, color: A.dim, maxWidth: 600, margin: "0 auto 32px", lineHeight: 1.7, letterSpacing: "0.02em" }}>
              A from-scratch LLM training engine built entirely in Rust with CUDA GPU acceleration.
              No PyTorch. No Python. Pure native performance — every tensor op, every gradient, every CUDA kernel hand-written.
            </p>
            <div style={{ display: "flex", justifyContent: "center", gap: 16, flexWrap: "wrap" }}>
              {[
                { l: "Rust", c: A.orange },
                { l: "CUDA", c: A.green },
                { l: "Zero Dependencies*", c: A.cyan },
                { l: "Self-Evolving", c: A.magenta },
              ].map(t => (
                <span key={t.l} style={{ fontSize: 11, fontWeight: 700, letterSpacing: "0.1em", textTransform: "uppercase", padding: "5px 14px", borderRadius: 20, border: `1px solid ${t.c}30`, color: t.c, background: `${t.c}08` }}>
                  {t.l}
                </span>
              ))}
            </div>
            <p style={{ fontSize: 10, color: "#3e3e5e", marginTop: 12 }}>*No ML framework dependency — custom tensor + autograd library</p>
          </header>
        </Reveal>

        {/* ═══════ KEY METRICS ═══════ */}
        <Reveal delay={0.1}>
          <Card border={`${A.orange}30`} className="">
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))", gap: 8, textAlign: "center" }}>
              <StatBox label="Lines of Code" value="12,119" color={A.orange} />
              <StatBox label="Source Files" value="57" color={A.cyan} />
              <StatBox label="Parameters" value="~5M" color={A.magenta} sub="nova-tiny-5m" />
              <StatBox label="Vocab Size" value="8,000" color={A.green} sub="BPE tokens" />
              <StatBox label="GPU" value="RTX 5060" color={A.gold} sub="CUDA 13.1" />
              <StatBox label="Best Loss" value="7.23" color={A.violet} sub="Step 157" />
            </div>
          </Card>
        </Reveal>

        {/* ═══════ ARCHITECTURE ═══════ */}
        <Reveal delay={0.15}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="🧠" title="Transformer Architecture" badge="GPT-Style" color={A.magenta} />
            <Card border={`${A.magenta}20`}>
              <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
                {ARCH_LAYERS.map((l, i) => (
                  <div key={l.name} style={{ display: "flex", alignItems: "center", gap: 16, padding: "14px 16px", borderBottom: i < ARCH_LAYERS.length - 1 ? "1px solid rgba(255,255,255,0.04)" : "none" }}>
                    <div style={{ width: 8, height: 8, borderRadius: "50%", background: l.color, boxShadow: `0 0 10px ${l.color}60`, flexShrink: 0 }} />
                    <div style={{ flex: 1 }}>
                      <span style={{ fontWeight: 700, fontSize: 14, color: l.color }}>{l.name}</span>
                      <span style={{ fontSize: 12, color: A.dim, marginLeft: 12 }}>{l.desc}</span>
                    </div>
                    {i < ARCH_LAYERS.length - 1 && (
                      <span style={{ fontSize: 16, color: "rgba(255,255,255,0.15)" }}>↓</span>
                    )}
                  </div>
                ))}
              </div>

              {/* Model Config */}
              <div style={{ marginTop: 24, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))", gap: 12 }}>
                {[
                  { k: "d_model", v: TRAINING_STATS.dModel },
                  { k: "n_layers", v: TRAINING_STATS.nLayers },
                  { k: "n_heads", v: TRAINING_STATS.nHeads },
                  { k: "d_ff", v: 344 },
                  { k: "max_seq_len", v: TRAINING_STATS.seqLen },
                  { k: "rope_base", v: "10,000" },
                ].map(c => (
                  <div key={c.k} style={{ display: "flex", justifyContent: "space-between", padding: "8px 12px", background: "rgba(0,0,0,0.2)", borderRadius: 8, fontSize: 12 }}>
                    <span style={{ color: A.dim, fontFamily: "monospace" }}>{c.k}</span>
                    <span style={{ color: A.cyan, fontWeight: 700, fontFamily: "monospace" }}>{c.v}</span>
                  </div>
                ))}
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ TRAINING STATUS ═══════ */}
        <Reveal delay={0.15}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="📊" title="Training Status" badge="In Progress" color={A.green} />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 20 }}>
              <Card border={`${A.green}20`}>
                <h3 style={{ fontSize: 14, fontWeight: 700, color: A.green, marginBottom: 16, marginTop: 0 }}>Hyperparameters</h3>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  {[
                    { k: "Learning Rate", v: TRAINING_STATS.lr },
                    { k: "Batch Size", v: `${TRAINING_STATS.batchSize} (× ${TRAINING_STATS.gradAccum} accum)` },
                    { k: "Weight Decay", v: "0.1" },
                    { k: "Grad Clip", v: "1.0" },
                    { k: "β₁ / β₂", v: "0.9 / 0.95" },
                    { k: "Warmup Steps", v: "500" },
                    { k: "Total Steps", v: TRAINING_STATS.totalSteps.toLocaleString() },
                    { k: "Scheduler", v: "Cosine Annealing" },
                  ].map(r => (
                    <div key={r.k} style={{ display: "flex", justifyContent: "space-between", fontSize: 12, padding: "4px 0", borderBottom: "1px solid rgba(255,255,255,0.03)" }}>
                      <span style={{ color: A.dim }}>{r.k}</span>
                      <span style={{ color: "#e4e4f0", fontFamily: "monospace", fontWeight: 600 }}>{r.v}</span>
                    </div>
                  ))}
                </div>
              </Card>

              <Card border={`${A.gold}20`}>
                <h3 style={{ fontSize: 14, fontWeight: 700, color: A.gold, marginBottom: 16, marginTop: 0 }}>GPU Telemetry</h3>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 8, marginBottom: 16 }}>
                  <GaugeRing pct={0.3} label="PROGRESS" color={A.green} />
                  <GaugeRing pct={28} label="VRAM" color={A.gold} />
                  <GaugeRing pct={39} label="TEMP °C" color={A.cyan} />
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  {[
                    { k: "GPU", v: TRAINING_STATS.gpu },
                    { k: "VRAM Used", v: TRAINING_STATS.vram },
                    { k: "Throughput", v: `${TRAINING_STATS.tokPerSec} tok/s` },
                    { k: "Current Step", v: `${TRAINING_STATS.currentStep} / ${TRAINING_STATS.totalSteps.toLocaleString()}` },
                  ].map(r => (
                    <div key={r.k} style={{ display: "flex", justifyContent: "space-between", fontSize: 12, padding: "4px 0", borderBottom: "1px solid rgba(255,255,255,0.03)" }}>
                      <span style={{ color: A.dim }}>{r.k}</span>
                      <span style={{ color: "#e4e4f0", fontFamily: "monospace", fontWeight: 600 }}>{r.v}</span>
                    </div>
                  ))}
                </div>
              </Card>
            </div>
          </section>
        </Reveal>

        {/* ═══════ CUSTOM TENSOR ENGINE ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="🔢" title="Custom Tensor Engine" badge="No PyTorch" color={A.cyan} />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: 20 }}>
              {[
                {
                  title: "Tensor Core", color: A.cyan, icon: "◈",
                  items: ["N-dimensional storage with contiguous/strided layouts", "Shape broadcasting & automatic reshape", "CPU ↔ CUDA device transfer", "In-place and out-of-place operations", "Lazy computation with fused kernels"],
                },
                {
                  title: "Operations (100+)", color: A.magenta, icon: "◎",
                  items: ["MatMul, BatchMatMul, BMM with transpose", "Softmax, LogSoftmax, GELU, SiLU/SwiGLU", "LayerNorm, RMSNorm, Dropout", "RoPE positional encoding", "Cross-entropy loss with label smoothing"],
                },
                {
                  title: "Autograd Engine", color: A.green, icon: "∂",
                  items: ["Reverse-mode automatic differentiation", "Dynamic computation graph", "Gradient accumulation & clipping", "Memory-efficient checkpointing", "Custom backward for attention & FFN"],
                },
                {
                  title: "CUDA Acceleration", color: A.orange, icon: "⚡",
                  items: ["cudarc 0.19 — safe Rust bindings", "cuBLAS SGEMM/DGEMM for matmul", "Custom NVRTC-compiled kernels", "Async memory copies & streams", "Pinned host memory for transfers"],
                },
              ].map(s => (
                <Card key={s.title} border={`${s.color}20`}>
                  <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 14 }}>
                    <span style={{ fontSize: 20, color: s.color }}>{s.icon}</span>
                    <h3 style={{ fontSize: 14, fontWeight: 700, color: s.color, margin: 0 }}>{s.title}</h3>
                  </div>
                  <ul style={{ margin: 0, paddingLeft: 18, listStyleType: "none" }}>
                    {s.items.map((item, i) => (
                      <li key={i} style={{ fontSize: 12, color: "#b0b0cc", lineHeight: 1.8, position: "relative", paddingLeft: 14 }}>
                        <span style={{ position: "absolute", left: 0, color: s.color, fontSize: 8, top: 6 }}>▸</span>
                        {item}
                      </li>
                    ))}
                  </ul>
                </Card>
              ))}
            </div>
          </section>
        </Reveal>

        {/* ═══════ SELF-EVOLUTION ENGINE ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="🧬" title="Self-Evolution Engine" badge="Autonomous" color={A.violet} />
            <Card border={`${A.violet}20`}>
              <p style={{ fontSize: 13, color: "#b0b0cc", lineHeight: 1.7, margin: "0 0 24px" }}>
                Nova includes a built-in autonomous evolution system that can mutate its own architecture — adding/removing
                layers, adjusting attention heads, modifying FFN dimensions — then evaluating fitness and selecting the best
                performing variants. This is the foundation for self-improving AI.
              </p>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: 16 }}>
                {[
                  { module: "autonomous.rs", loc: 301, desc: "Architecture search & mutation scheduling", color: A.violet },
                  { module: "fitness.rs", loc: 268, desc: "Multi-objective fitness: loss, speed, memory", color: A.green },
                  { module: "mutator.rs", loc: 254, desc: "Layer insertion, head pruning, dim scaling", color: A.magenta },
                  { module: "sandbox.rs", loc: 192, desc: "Isolated evaluation with rollback safety", color: A.cyan },
                ].map(m => (
                  <div key={m.module} style={{ padding: "14px 16px", background: "rgba(0,0,0,0.2)", borderRadius: 12, borderLeft: `3px solid ${m.color}` }}>
                    <div style={{ fontFamily: "monospace", fontSize: 13, fontWeight: 700, color: m.color }}>{m.module}</div>
                    <div style={{ fontSize: 11, color: A.dim, marginTop: 4 }}>{m.loc} LOC — {m.desc}</div>
                  </div>
                ))}
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ NOVA STUDIO GUI ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="🖥️" title="Nova Studio" badge="Native GUI" color={A.gold} />
            <Card border={`${A.gold}20`}>
              <p style={{ fontSize: 13, color: "#b0b0cc", lineHeight: 1.7, margin: "0 0 20px" }}>
                Real-time GPU-accelerated training dashboard built with egui/glow. Monitors loss curves,
                learning rate schedules, GPU telemetry, and generation output — all running natively at 60fps
                with zero web overhead.
              </p>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
                {[
                  "Live loss chart",
                  "LR schedule viz",
                  "GPU temp / VRAM",
                  "Generation preview",
                  "Training controls",
                  "Model config editor",
                  "Data pipeline view",
                  "Evolution monitor",
                  "Checkpoint manager",
                ].map(f => (
                  <div key={f} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, color: "#b0b0cc", padding: "8px 12px", background: "rgba(0,0,0,0.15)", borderRadius: 8 }}>
                    <span style={{ color: A.gold, fontSize: 10 }}>◆</span>
                    {f}
                  </div>
                ))}
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ DEPENDENCY STACK ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="📦" title="Dependency Stack" badge={`${TECH_DEPS.length} crates`} color={A.orange} />
            <Card border={`${A.orange}20`}>
              <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
                {TECH_DEPS.map((d, i) => (
                  <div key={d.name} style={{ display: "grid", gridTemplateColumns: "140px 70px 1fr", gap: 12, alignItems: "center", padding: "10px 12px", borderBottom: i < TECH_DEPS.length - 1 ? "1px solid rgba(255,255,255,0.03)" : "none" }}>
                    <span style={{ fontFamily: "monospace", fontWeight: 700, fontSize: 13, color: A.orange }}>{d.name}</span>
                    <span style={{ fontFamily: "monospace", fontSize: 11, color: A.dim }}>{d.version}</span>
                    <span style={{ fontSize: 12, color: "#b0b0cc" }}>{d.purpose}</span>
                  </div>
                ))}
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ SOURCE INVENTORY ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="📁" title="Source Inventory" badge="57 files · 12,119 LOC" color={A.cyan} />
            <Card border={`${A.cyan}15`}>
              <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {MODULES.map((m, i) => {
                  const pct = (m.loc / maxLoc) * 100;
                  const barColor = [
                    `linear-gradient(90deg,${A.orange},${A.magenta})`,
                    `linear-gradient(90deg,${A.cyan},${A.green})`,
                    `linear-gradient(90deg,${A.magenta},${A.violet})`,
                    `linear-gradient(90deg,${A.green},${A.cyan})`,
                    `linear-gradient(90deg,${A.gold},${A.orange})`,
                  ][i % 5];
                  return (
                    <div key={m.name} style={{ display: "grid", gridTemplateColumns: "160px 50px 1fr", gap: 12, alignItems: "center", padding: "6px 8px", borderRadius: 6 }}>
                      <div>
                        <span style={{ fontFamily: "monospace", fontSize: 12, fontWeight: 600, color: "#e4e4f0" }}>{m.name}</span>
                        <span style={{ fontSize: 10, color: A.dim, marginLeft: 6 }}>{m.dir}/</span>
                      </div>
                      <span style={{ fontFamily: "monospace", fontSize: 11, color: A.dim, textAlign: "right" }}>{m.loc}</span>
                      <div style={{ position: "relative", height: 6, background: "rgba(255,255,255,0.04)", borderRadius: 3, overflow: "hidden" }}>
                        <div style={{ position: "absolute", left: 0, top: 0, height: "100%", width: `${pct}%`, background: barColor, borderRadius: 3, transition: "width 1s ease" }} />
                      </div>
                    </div>
                  );
                })}
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ WHY RUST FOR ML ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="🦀" title="Why Rust for Machine Learning?" color={A.orange} />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 20 }}>
              {[
                {
                  title: "Zero-Cost Abstractions",
                  desc: "Rust's type system and ownership model produce code that compiles to the same machine instructions as hand-tuned C — with full memory safety guarantees. No garbage collector pauses during training.",
                  color: A.orange,
                },
                {
                  title: "Fearless Concurrency",
                  desc: "Data races are compile-time errors in Rust. Rayon parallelism for CPU-bound data loading, async CUDA stream management, and lock-free metric reporting — all verified at compile time.",
                  color: A.cyan,
                },
                {
                  title: "Single Binary Deployment",
                  desc: "Nova compiles to a single static binary. No conda environments, no pip dependencies, no virtualenvs, no CUDA version mismatches. Just run the binary.",
                  color: A.green,
                },
                {
                  title: "Native CUDA Integration",
                  desc: "cudarc provides safe Rust bindings to the CUDA driver API — device management, memory allocation, kernel launches, cuBLAS — without unsafe blocks leaking into application code.",
                  color: A.magenta,
                },
              ].map(r => (
                <Card key={r.title} border={`${r.color}20`}>
                  <h3 style={{ fontSize: 15, fontWeight: 700, color: r.color, margin: "0 0 10px" }}>{r.title}</h3>
                  <p style={{ fontSize: 12, color: "#b0b0cc", lineHeight: 1.7, margin: 0 }}>{r.desc}</p>
                </Card>
              ))}
            </div>
          </section>
        </Reveal>

        {/* ═══════ FOOTER ═══════ */}
        <Reveal delay={0.1}>
          <footer style={{ marginTop: 80, textAlign: "center", paddingBottom: 40 }}>
            <div style={{ display: "flex", justifyContent: "center", gap: 24, marginBottom: 16, flexWrap: "wrap" }}>
              <Link href="/techstack" style={{ color: A.cyan, textDecoration: "none", fontSize: 13, fontWeight: 600, letterSpacing: "0.04em" }}>
                ← Tech Stack
              </Link>
              <Link href="/vitalis" style={{ color: A.green, textDecoration: "none", fontSize: 13, fontWeight: 600, letterSpacing: "0.04em" }}>
                Vitalis →
              </Link>
            </div>
            <p style={{ fontSize: 11, color: "#3e3e5e" }}>
              Nova — Part of the Infinity autonomous AI ecosystem
            </p>
          </footer>
        </Reveal>

      </main>
    </div>
  );
}
