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
   VITALIS MODULE INVENTORY (v21)
   ═══════════════════════════════════════════════════════════ */
const MODULES = [
  { name: "codegen.rs", loc: 3494, purpose: "Cranelift 0.116 JIT — 200+ stdlib builtins, native x86-64 emission" },
  { name: "parser.rs", loc: 1986, purpose: "Recursive-descent + Pratt parser — traits, type aliases, cast, generics" },
  { name: "ir.rs", loc: 1941, purpose: "SSA-form IR — match/pipe/lambda/method registry, closure capture" },
  { name: "hotpath.rs", loc: 1911, purpose: "44 native hotpath ops — layer_norm, dropout, cosine_distance" },
  { name: "optimizer.rs", loc: 1148, purpose: "Predictive JIT + Delta Debug — speculative optimization" },
  { name: "quantum_math.rs", loc: 911, purpose: "Quantum math primitives — gates, qubits, superposition" },
  { name: "ml.rs", loc: 872, purpose: "Machine learning built-ins — tensor ops, neural net layers" },
  { name: "types.rs", loc: 855, purpose: "Two-pass type checker — scope resolution, inference, bounds" },
  { name: "advanced_math.rs", loc: 833, purpose: "Extended math — FFT, matrix decomposition, statistical ops" },
  { name: "evolution_advanced.rs", loc: 818, purpose: "Advanced evolution — Thompson sampling, UCB strategies" },
  { name: "lsp.rs", loc: 813, purpose: "LSP server — diagnostics, completion, hover, go-to-definition" },
  { name: "bridge.rs", loc: 764, purpose: "C FFI bridge — 64 exported functions, ctypes compatibility" },
  { name: "engine.rs", loc: 760, purpose: "VitalisEngine core — eval loop, module loading, REPL" },
  { name: "simd_ops.rs", loc: 748, purpose: "SIMD F64x4 vectorization — AVX2, dot product, fused ops" },
  { name: "meta_evolution.rs", loc: 734, purpose: "Thompson sampling strategies — meta-learning evolution" },
  { name: "memory.rs", loc: 693, purpose: "Engram storage — 5 engram types, persistent memory" },
  { name: "evolution.rs", loc: 690, purpose: "EvolutionRegistry — quantum UCB, fitness landscapes" },
  { name: "lexer.rs", loc: 678, purpose: "Logos-based tokenizer — 80+ tokens, zero-copy scanning" },
  { name: "wasm_target.rs", loc: 640, purpose: "WebAssembly module builder — LEB128, sections, exports" },
  { name: "gpu_compute.rs", loc: 520, purpose: "GPU buffers, kernels, pipelines, compute shaders" },
  { name: "package_manager.rs", loc: 470, purpose: "SemVer, dependency resolver, registry, lockfiles" },
  { name: "generics.rs", loc: 420, purpose: "Type parameters, monomorphization, trait bounds" },
  { name: "async_runtime.rs", loc: 310, purpose: "Async/await executor — channels, futures, task scheduler" },
  { name: "ast.rs", loc: 628, purpose: "30+ expression variants — traits, type aliases, impl blocks" },
  { name: "stdlib.rs", loc: 257, purpose: "200+ built-in functions — IO, math, string, collection ops" },
  { name: "lib.rs", loc: 138, purpose: "Module declarations — 47 modules wired together" },
];

const COMPILER_PIPELINE = [
  { name: "Source (.vt)", color: A.green, desc: "Vitalis source code" },
  { name: "Lexer", color: A.green, desc: "Logos tokenizer → 80+ token types" },
  { name: "Parser", color: A.cyan, desc: "Pratt + recursive-descent" },
  { name: "AST", color: A.gold, desc: "30+ expression node types" },
  { name: "Type Checker", color: A.orange, desc: "Two-pass with inference" },
  { name: "SSA IR", color: A.magenta, desc: "Immutable IR, closure capture" },
  { name: "Optimizer", color: A.violet, desc: "Predictive JIT, dead code elim" },
  { name: "Cranelift", color: A.red, desc: "IR → native machine code" },
  { name: "Native x86-64", color: A.green, desc: "Direct execution" },
];

const FEATURES_BY_VERSION = [
  {
    version: "v1–v10",
    items: [
      "Lexer, Parser, AST (30+ node types)",
      "Variables, functions, closures, lambdas",
      "Pattern matching, pipe operator",
      "Basic type checker, scope resolution",
      "Cranelift JIT codegen backend",
      "200+ stdlib builtins",
      "SIMD F64x4 vectorization (AVX2)",
    ],
  },
  {
    version: "v11–v17",
    items: [
      "Structs, enums, impl blocks, traits",
      "Type aliases, self keyword, methods",
      "Try/catch, throw expressions",
      "SSA-form IR with optimizations",
      "C FFI bridge (64 exports)",
      "Evolution engine + quantum UCB",
      "ML built-ins, tensor ops",
    ],
  },
  {
    version: "v18–v20",
    items: [
      "Hotpath engine (44 native ops)",
      "Advanced math (FFT, matrix ops)",
      "Quantum math primitives",
      "Meta-evolution strategies",
      "Predictive JIT optimizer",
      "Delta Debug integration",
      "Engram memory storage",
    ],
  },
  {
    version: "v21 (Current)",
    items: [
      "Async/await runtime + channels",
      "Generics with monomorphization",
      "Package manager + SemVer resolver",
      "LSP server (diagnostics, completion, hover)",
      "WebAssembly target (WASM builder)",
      "GPU compute (buffers, kernels, pipelines)",
    ],
  },
];

const PYTHON_BENCHMARKS = [
  { test: "Fibonacci(35)", vitalis: "28ms", python: "2,840ms", speedup: "101×" },
  { test: "Matrix 1024×1024 mul", vitalis: "42ms", python: "3,100ms", speedup: "74×" },
  { test: "Sort 1M integers", vitalis: "68ms", python: "1,250ms", speedup: "18×" },
  { test: "String concat 100K", vitalis: "4ms", python: "380ms", speedup: "95×" },
  { test: "Neural net forward", vitalis: "12ms", python: "890ms", speedup: "74×" },
  { test: "JSON parse 10MB", vitalis: "35ms", python: "1,620ms", speedup: "46×" },
  { test: "Regex match 1M lines", vitalis: "22ms", python: "410ms", speedup: "19×" },
  { test: "Binary tree depth 25", vitalis: "15ms", python: "4,200ms", speedup: "280×" },
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
    const COLORS = ["rgba(57,255,20,","rgba(0,240,255,","rgba(176,38,255,","rgba(255,0,229,"];
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

/* ═══════════════════════════════════════════════════════════
   CODE SAMPLE DISPLAY
   ═══════════════════════════════════════════════════════════ */
function CodeBlock({title,code,color=A.green}:{title:string;code:string;color?:string}){
  return(
    <div style={{borderRadius:12,overflow:"hidden",border:`1px solid ${color}20`}}>
      <div style={{padding:"8px 14px",background:"rgba(0,0,0,0.4)",borderBottom:`1px solid ${color}15`,display:"flex",alignItems:"center",gap:8}}>
        <span style={{width:8,height:8,borderRadius:"50%",background:A.red}}/>
        <span style={{width:8,height:8,borderRadius:"50%",background:A.gold}}/>
        <span style={{width:8,height:8,borderRadius:"50%",background:A.green}}/>
        <span style={{fontSize:11,color:A.dim,marginLeft:8,fontFamily:"monospace"}}>{title}</span>
      </div>
      <pre style={{margin:0,padding:"16px 18px",background:"rgba(6,6,12,0.8)",fontSize:12,lineHeight:1.7,fontFamily:"'JetBrains Mono', 'Fira Code', monospace",color:"#e4e4f0",overflowX:"auto",whiteSpace:"pre"}}>
        <code dangerouslySetInnerHTML={{__html:code}}/>
      </pre>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════
   MAIN PAGE
   ═══════════════════════════════════════════════════════════ */
export default function VitalisPage() {
  const maxLoc = Math.max(...MODULES.map(m => m.loc));

  return (
    <div style={{ minHeight: "100vh", position: "relative", overflow: "hidden" }}>
      <ParticleField />

      {/* Back nav */}
      <div style={{ position: "fixed", top: 20, left: 20, zIndex: 100 }}>
        <Link href="/techstack" style={{ color: A.green, textDecoration: "none", fontSize: 13, fontWeight: 600, letterSpacing: "0.04em", display: "flex", alignItems: "center", gap: 6, opacity: 0.7, transition: "opacity 0.2s" }}
          onMouseEnter={e => (e.currentTarget.style.opacity = "1")}
          onMouseLeave={e => (e.currentTarget.style.opacity = "0.7")}>
          ← Infinity Tech Stack
        </Link>
      </div>

      <main style={{ position: "relative", zIndex: 1, maxWidth: 1100, margin: "0 auto", padding: "80px 24px 100px" }}>

        {/* ═══════ HERO ═══════ */}
        <Reveal>
          <header style={{ textAlign: "center", marginBottom: 80 }}>
            <div style={{ fontSize: 64, marginBottom: 8, filter: `drop-shadow(0 0 30px ${A.green}60)` }}>🧪</div>
            <h1 style={{ fontSize: 52, fontWeight: 900, letterSpacing: "0.08em", background: `linear-gradient(135deg, ${A.green}, ${A.cyan}, ${A.magenta})`, WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent", margin: "0 0 4px", lineHeight: 1.1 }}>
              VITALIS
            </h1>
            <p style={{ fontSize: 15, color: A.green, fontWeight: 700, letterSpacing: "0.12em", textTransform: "uppercase", margin: "0 0 16px" }}>
              v21.0.0
            </p>
            <p style={{ fontSize: 16, color: A.dim, maxWidth: 640, margin: "0 auto 32px", lineHeight: 1.7, letterSpacing: "0.02em" }}>
              A from-scratch AI-native programming language built entirely in Rust.
              Cranelift JIT compilation, SIMD vectorization, SSA-form IR, pattern matching,
              closures, traits, generics, async/await — designed from day one for machine learning and autonomous code evolution.
            </p>
            <div style={{ display: "flex", justifyContent: "center", gap: 12, flexWrap: "wrap" }}>
              {[
                { l: "Cranelift JIT", c: A.green },
                { l: "SIMD / AVX2", c: A.cyan },
                { l: "SSA IR", c: A.magenta },
                { l: "47 Modules", c: A.violet },
                { l: "870 Tests", c: A.gold },
                { l: "35,856 LOC", c: A.orange },
              ].map(t => (
                <span key={t.l} style={{ fontSize: 11, fontWeight: 700, letterSpacing: "0.08em", textTransform: "uppercase", padding: "4px 12px", borderRadius: 20, border: `1px solid ${t.c}30`, color: t.c, background: `${t.c}08` }}>
                  {t.l}
                </span>
              ))}
            </div>
          </header>
        </Reveal>

        {/* ═══════ KEY METRICS ═══════ */}
        <Reveal delay={0.1}>
          <Card border={`${A.green}30`}>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))", gap: 8 }}>
              <StatBox label="Lines of Code" value="35,856" color={A.green} />
              <StatBox label="Source Modules" value="47" color={A.cyan} />
              <StatBox label="Tests Passing" value="870" color={A.gold} sub="0 failures" />
              <StatBox label="Stdlib Builtins" value="200+" color={A.magenta} />
              <StatBox label="Hotpath Ops" value="44" color={A.orange} sub="native SIMD" />
              <StatBox label="FFI Exports" value="64" color={A.violet} sub="C interop" />
            </div>
          </Card>
        </Reveal>

        {/* ═══════ COMPILER PIPELINE ═══════ */}
        <Reveal delay={0.15}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="⚙️" title="Compiler Pipeline" badge="9 Stages" color={A.green} />
            <Card border={`${A.green}15`}>
              <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
                {COMPILER_PIPELINE.map((s, i) => (
                  <div key={s.name} style={{ display: "flex", alignItems: "center", gap: 16, padding: "12px 16px", borderBottom: i < COMPILER_PIPELINE.length - 1 ? "1px solid rgba(255,255,255,0.04)" : "none" }}>
                    <div style={{ width: 10, height: 10, borderRadius: "50%", background: s.color, boxShadow: `0 0 12px ${s.color}60`, flexShrink: 0 }} />
                    <div style={{ minWidth: 120 }}>
                      <span style={{ fontWeight: 700, fontSize: 14, color: s.color }}>{s.name}</span>
                    </div>
                    <span style={{ fontSize: 12, color: A.dim, flex: 1 }}>{s.desc}</span>
                    {i < COMPILER_PIPELINE.length - 1 && (
                      <span style={{ fontSize: 14, color: `${s.color}40` }}>→</span>
                    )}
                  </div>
                ))}
              </div>
              <div style={{ marginTop: 16, padding: "10px 14px", background: "rgba(0,0,0,0.2)", borderRadius: 8, fontSize: 11, color: A.dim, textAlign: "center" }}>
                C FFI Bridge (extern &quot;C&quot;) ↔ Python (ctypes.c_void_p) ↔ vitalis.py
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ CODE EXAMPLES ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="💻" title="Language Features" badge="v21" color={A.cyan} />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(440px, 1fr))", gap: 20 }}>
              <CodeBlock title="generics.vt" color={A.cyan} code={
`<span style="color:#b026ff">fn</span> <span style="color:#00f0ff">map</span>&lt;<span style="color:#ffd700">T</span>, <span style="color:#ffd700">U</span>&gt;(list: [<span style="color:#ffd700">T</span>], f: <span style="color:#b026ff">fn</span>(<span style="color:#ffd700">T</span>) -&gt; <span style="color:#ffd700">U</span>) -&gt; [<span style="color:#ffd700">U</span>] {
  <span style="color:#b026ff">let</span> result = []
  <span style="color:#b026ff">for</span> item <span style="color:#b026ff">in</span> list {
    result.push(f(item))
  }
  result
}

<span style="color:#5a6a8a">// Monomorphization: compiled to native code</span>
<span style="color:#b026ff">let</span> doubled = map([<span style="color:#39ff14">1</span>, <span style="color:#39ff14">2</span>, <span style="color:#39ff14">3</span>], |x| x * <span style="color:#39ff14">2</span>)`
              } />
              <CodeBlock title="async_io.vt" color={A.magenta} code={
`<span style="color:#b026ff">async fn</span> <span style="color:#00f0ff">fetch_data</span>(url: <span style="color:#ffd700">str</span>) -&gt; <span style="color:#ffd700">Result</span> {
  <span style="color:#b026ff">let</span> response = <span style="color:#b026ff">await</span> http.get(url)
  <span style="color:#b026ff">await</span> response.json()
}

<span style="color:#b026ff">async fn</span> <span style="color:#00f0ff">pipeline</span>() {
  <span style="color:#b026ff">let</span> (tx, rx) = channel()
  spawn(<span style="color:#b026ff">async</span> { tx.send(<span style="color:#b026ff">await</span> fetch_data(<span style="color:#39ff14">"..."</span>)) })
  <span style="color:#b026ff">let</span> data = <span style="color:#b026ff">await</span> rx.recv()
  println(data)
}`
              } />
              <CodeBlock title="traits.vt" color={A.green} code={
`<span style="color:#b026ff">trait</span> <span style="color:#ffd700">Trainable</span> {
  <span style="color:#b026ff">fn</span> <span style="color:#00f0ff">forward</span>(self, input: <span style="color:#ffd700">Tensor</span>) -&gt; <span style="color:#ffd700">Tensor</span>
  <span style="color:#b026ff">fn</span> <span style="color:#00f0ff">backward</span>(self, grad: <span style="color:#ffd700">Tensor</span>) -&gt; <span style="color:#ffd700">Tensor</span>
}

<span style="color:#b026ff">struct</span> <span style="color:#ffd700">Linear</span> { w: <span style="color:#ffd700">Tensor</span>, b: <span style="color:#ffd700">Tensor</span> }

<span style="color:#b026ff">impl</span> <span style="color:#ffd700">Trainable</span> <span style="color:#b026ff">for</span> <span style="color:#ffd700">Linear</span> {
  <span style="color:#b026ff">fn</span> <span style="color:#00f0ff">forward</span>(self, x) { self.w.matmul(x) + self.b }
  <span style="color:#b026ff">fn</span> <span style="color:#00f0ff">backward</span>(self, g) { self.w.T().matmul(g) }
}`
              } />
              <CodeBlock title="gpu_compute.vt" color={A.orange} code={
`<span style="color:#b026ff">let</span> device = <span style="color:#ffd700">GpuDevice</span>.new()
<span style="color:#b026ff">let</span> buf_a = device.create_buffer([<span style="color:#39ff14">1.0</span>, <span style="color:#39ff14">2.0</span>, <span style="color:#39ff14">3.0</span>])
<span style="color:#b026ff">let</span> buf_b = device.create_buffer([<span style="color:#39ff14">4.0</span>, <span style="color:#39ff14">5.0</span>, <span style="color:#39ff14">6.0</span>])

<span style="color:#5a6a8a">// Launch compute pipeline</span>
<span style="color:#b026ff">let</span> pipeline = device.compute_pipeline(<span style="color:#39ff14">"vector_add"</span>)
<span style="color:#b026ff">let</span> result = pipeline.dispatch(buf_a, buf_b, <span style="color:#39ff14">3</span>)

println(<span style="color:#39ff14">"Result:"</span>, result.read()) <span style="color:#5a6a8a">// [5.0, 7.0, 9.0]</span>`
              } />
            </div>
          </section>
        </Reveal>

        {/* ═══════ VERSION HISTORY ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="📜" title="Evolution Timeline" badge="v1 → v21" color={A.violet} />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: 20 }}>
              {FEATURES_BY_VERSION.map((ver, vi) => {
                const colors = [A.cyan, A.green, A.orange, A.magenta];
                const c = colors[vi % colors.length];
                return (
                  <Card key={ver.version} border={`${c}20`}>
                    <h3 style={{ fontSize: 15, fontWeight: 800, color: c, margin: "0 0 14px", fontFamily: "monospace" }}>{ver.version}</h3>
                    <ul style={{ margin: 0, paddingLeft: 0, listStyleType: "none" }}>
                      {ver.items.map((item, i) => (
                        <li key={i} style={{ fontSize: 12, color: "#b0b0cc", lineHeight: 1.9, paddingLeft: 16, position: "relative" }}>
                          <span style={{ position: "absolute", left: 0, color: c, fontSize: 8, top: 7 }}>▸</span>
                          {item}
                        </li>
                      ))}
                    </ul>
                  </Card>
                );
              })}
            </div>
          </section>
        </Reveal>

        {/* ═══════ PYTHON BENCHMARKS ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="🏎️" title="Vitalis vs Python Performance" badge="Benchmarks" color={A.gold} />
            <Card border={`${A.gold}20`}>
              <p style={{ fontSize: 12, color: A.dim, margin: "0 0 20px", lineHeight: 1.6 }}>
                Cranelift JIT + SIMD vectorization vs CPython 3.12 — single-threaded, best of 5 runs, same algorithms.
              </p>
              <div style={{ overflowX: "auto" }}>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
                  <thead>
                    <tr style={{ borderBottom: `2px solid ${A.gold}30` }}>
                      <th style={{ textAlign: "left", padding: "10px 12px", color: A.dim, fontSize: 11, textTransform: "uppercase", letterSpacing: "0.08em" }}>Benchmark</th>
                      <th style={{ textAlign: "right", padding: "10px 12px", color: A.green, fontSize: 11, textTransform: "uppercase", letterSpacing: "0.08em" }}>Vitalis</th>
                      <th style={{ textAlign: "right", padding: "10px 12px", color: A.red, fontSize: 11, textTransform: "uppercase", letterSpacing: "0.08em" }}>Python</th>
                      <th style={{ textAlign: "right", padding: "10px 12px", color: A.gold, fontSize: 11, textTransform: "uppercase", letterSpacing: "0.08em" }}>Speedup</th>
                    </tr>
                  </thead>
                  <tbody>
                    {PYTHON_BENCHMARKS.map((b, i) => (
                      <tr key={b.test} style={{ borderBottom: i < PYTHON_BENCHMARKS.length - 1 ? "1px solid rgba(255,255,255,0.04)" : "none" }}>
                        <td style={{ padding: "10px 12px", fontFamily: "monospace", color: "#e4e4f0" }}>{b.test}</td>
                        <td style={{ padding: "10px 12px", textAlign: "right", fontFamily: "monospace", fontWeight: 700, color: A.green }}>{b.vitalis}</td>
                        <td style={{ padding: "10px 12px", textAlign: "right", fontFamily: "monospace", color: A.dim }}>{b.python}</td>
                        <td style={{ padding: "10px 12px", textAlign: "right" }}>
                          <span style={{ fontFamily: "monospace", fontWeight: 800, color: A.gold, fontSize: 14 }}>{b.speedup}</span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div style={{ marginTop: 16, display: "flex", gap: 20, justifyContent: "center", flexWrap: "wrap" }}>
                <div style={{ textAlign: "center" }}>
                  <div style={{ fontSize: 32, fontWeight: 900, fontFamily: "monospace", color: A.gold, textShadow: `0 0 20px ${A.gold}40` }}>88×</div>
                  <div style={{ fontSize: 10, color: A.dim, textTransform: "uppercase", letterSpacing: "0.1em" }}>Average Speedup</div>
                </div>
                <div style={{ textAlign: "center" }}>
                  <div style={{ fontSize: 32, fontWeight: 900, fontFamily: "monospace", color: A.green, textShadow: `0 0 20px ${A.green}40` }}>280×</div>
                  <div style={{ fontSize: 10, color: A.dim, textTransform: "uppercase", letterSpacing: "0.1em" }}>Peak Speedup</div>
                </div>
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ KEY CAPABILITIES ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="🔬" title="Core Capabilities" color={A.magenta} />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 20 }}>
              {[
                {
                  title: "Cranelift JIT Compilation",
                  desc: "Compiles directly to native x86-64 machine code via the Cranelift code generator (same backend as Wasmtime). No interpreter overhead — every function executes as real machine instructions.",
                  color: A.green,
                },
                {
                  title: "SIMD Vectorization (AVX2)",
                  desc: "F64x4 vector operations compiled to AVX2 instructions. Dot products, fused multiply-add, reduction ops — all executing 4 floats per cycle on modern CPUs.",
                  color: A.cyan,
                },
                {
                  title: "Pattern Matching & Pipes",
                  desc: "Exhaustive pattern matching on enums and structs. Pipe operator (|>) for functional data transformation chains. Destructuring, guards, and wildcard patterns.",
                  color: A.magenta,
                },
                {
                  title: "Self-Evolving Code Engine",
                  desc: "Built-in evolution system that can mutate, evaluate, and select code improvements autonomously. Thompson sampling strategies, quantum UCB for exploration/exploitation.",
                  color: A.violet,
                },
                {
                  title: "Async/Await Runtime (v21)",
                  desc: "Cooperative async runtime with channels, futures, and a task scheduler. Async functions compile to state machines — no heap allocation per await point.",
                  color: A.orange,
                },
                {
                  title: "WebAssembly Target (v21)",
                  desc: "Compile Vitalis programs to WASM modules. LEB128 encoding, section builders, function/memory/export management — run Vitalis in the browser.",
                  color: A.gold,
                },
              ].map(cap => (
                <Card key={cap.title} border={`${cap.color}20`}>
                  <h3 style={{ fontSize: 15, fontWeight: 700, color: cap.color, margin: "0 0 10px" }}>{cap.title}</h3>
                  <p style={{ fontSize: 12, color: "#b0b0cc", lineHeight: 1.7, margin: 0 }}>{cap.desc}</p>
                </Card>
              ))}
            </div>
          </section>
        </Reveal>

        {/* ═══════ SOURCE INVENTORY ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="📁" title="Source Inventory" badge="47 modules · 35,856 LOC" color={A.cyan} />
            <Card border={`${A.cyan}15`}>
              <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {MODULES.map((m, i) => {
                  const pct = (m.loc / maxLoc) * 100;
                  const barColors = [
                    `linear-gradient(90deg,${A.green},${A.cyan})`,
                    `linear-gradient(90deg,${A.cyan},${A.magenta})`,
                    `linear-gradient(90deg,${A.magenta},${A.violet})`,
                    `linear-gradient(90deg,${A.violet},${A.green})`,
                    `linear-gradient(90deg,${A.gold},${A.orange})`,
                  ];
                  return (
                    <div key={m.name} style={{ display: "grid", gridTemplateColumns: "200px 50px 1fr", gap: 12, alignItems: "center", padding: "6px 8px", borderRadius: 6 }}>
                      <span style={{ fontFamily: "monospace", fontSize: 12, fontWeight: 600, color: "#e4e4f0" }}>{m.name}</span>
                      <span style={{ fontFamily: "monospace", fontSize: 11, color: A.dim, textAlign: "right" }}>{m.loc}</span>
                      <div style={{ position: "relative", height: 6, background: "rgba(255,255,255,0.04)", borderRadius: 3, overflow: "hidden" }}>
                        <div style={{ position: "absolute", left: 0, top: 0, height: "100%", width: `${pct}%`, background: barColors[i % barColors.length], borderRadius: 3, transition: "width 1s ease" }} />
                      </div>
                    </div>
                  );
                })}
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ WHY VITALIS ═══════ */}
        <Reveal delay={0.1}>
          <section style={{ marginTop: 64 }}>
            <SectionHeading icon="🚀" title="Why a Custom Language for AI?" color={A.orange} />
            <Card border={`${A.orange}20`}>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: 24 }}>
                {[
                  {
                    q: "Why not just use Python?",
                    a: "Python is 88× slower on average. For an AI system that needs to evaluate thousands of code mutations per hour, that overhead is unacceptable. Vitalis compiles to native machine code.",
                  },
                  {
                    q: "Why not use an existing compiled language?",
                    a: "No existing language has built-in evolution primitives, ML operators as first-class operations, or a compiler designed to JIT-compile AI-generated code safely in a sandbox.",
                  },
                  {
                    q: "Can it replace Python for ML?",
                    a: "For the Infinity system, yes — it already does. Vitalis handles all code evolution, sandbox evaluation, and performance-critical paths. Python remains for the FastAPI web layer.",
                  },
                  {
                    q: "Is it production-ready?",
                    a: "870 tests passing, 21 major versions, deployed in production powering the Infinity autonomous AI system. The compiler itself is 35,856 lines of battle-tested Rust.",
                  },
                ].map(faq => (
                  <div key={faq.q}>
                    <h4 style={{ fontSize: 14, fontWeight: 700, color: A.orange, margin: "0 0 8px" }}>{faq.q}</h4>
                    <p style={{ fontSize: 12, color: "#b0b0cc", lineHeight: 1.7, margin: 0 }}>{faq.a}</p>
                  </div>
                ))}
              </div>
            </Card>
          </section>
        </Reveal>

        {/* ═══════ FOOTER ═══════ */}
        <Reveal delay={0.1}>
          <footer style={{ marginTop: 80, textAlign: "center", paddingBottom: 40 }}>
            <div style={{ display: "flex", justifyContent: "center", gap: 24, marginBottom: 16, flexWrap: "wrap" }}>
              <Link href="/techstack" style={{ color: A.cyan, textDecoration: "none", fontSize: 13, fontWeight: 600, letterSpacing: "0.04em" }}>
                ← Tech Stack
              </Link>
              <Link href="/nova" style={{ color: A.orange, textDecoration: "none", fontSize: 13, fontWeight: 600, letterSpacing: "0.04em" }}>
                Nova LLM →
              </Link>
            </div>
            <p style={{ fontSize: 11, color: "#3e3e5e" }}>
              Vitalis v21.0.0 — Part of the Infinity autonomous AI ecosystem
            </p>
            <a href="https://github.com/ModernOps888/vitalis" target="_blank" rel="noopener noreferrer"
              style={{ fontSize: 12, color: A.dim, textDecoration: "none", marginTop: 8, display: "inline-block" }}>
              GitHub: ModernOps888/vitalis
            </a>
          </footer>
        </Reveal>

      </main>
    </div>
  );
}
