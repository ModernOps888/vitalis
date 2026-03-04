# Vitalis Roadmap

This document tracks the development roadmap for the Vitalis programming language.
Completed milestones are marked with ✅, in-progress with 🔄, and planned with 📋.

---

## ✅ Completed

### v1.0 — Foundation
- ✅ Lexer with Logos tokenizer (~70 token variants)
- ✅ Recursive-descent + Pratt parser → AST (30+ expression types)
- ✅ Two-pass type checker with scope chains
- ✅ SSA-form intermediate representation
- ✅ Cranelift 0.116 JIT backend
- ✅ CLI binary (`vtc`) with subcommands
- ✅ 97 stdlib functions

### v5.0 — Type System
- ✅ i64, f64, bool, str type support
- ✅ Heap-allocated arrays
- ✅ SSA IR builder with ~30 instruction variants

### v7.0–v9.0 — Algorithm Libraries
- ✅ Signal processing, cryptography, graph algorithms
- ✅ String algorithms, numerical methods, compression
- ✅ Probability & statistics, quantum simulator
- ✅ Advanced math, science, analytics, security, scoring

### v10.0 — Machine Learning & Optimization
- ✅ ML (k-means, KNN, PCA, DBSCAN)
- ✅ Computational geometry (convex hull, Voronoi)
- ✅ Sorting algorithms, automata & tries
- ✅ Combinatorial optimization (knapsack, TSP, simplex)

### v13.0 — Quantum, Bio & Neuromorphic
- ✅ Quantum algorithms (Grover, Shor, QFT, VQE)
- ✅ Bioinformatics (DNA/RNA, alignment, epidemiology)
- ✅ Neuromorphic computing (LIF, STDP, ESN, NEAT)
- ✅ Advanced chemistry & molecular dynamics
- ✅ Advanced evolutionary computation (DE, PSO, CMA-ES, NSGA-II)

### v15.0 — Language Power
- ✅ Closures & lambda expressions with capture
- ✅ File I/O, maps, JSON support
- ✅ Error handling system
- ✅ Evolution engine with `@evolvable`

### v19.0 — General Purpose
- ✅ Structs + impl blocks + method dispatch
- ✅ Try/catch/throw error handling
- ✅ Sets, tuples, regex
- ✅ Module system with namespaces
- ✅ HTTP networking + async stubs
- ✅ Iterator protocol + comprehensions

### v20.0 — Trait System & Type Power
- ✅ Trait definitions + trait methods
- ✅ Type aliases, cast expressions
- ✅ Enum definitions with variant indexing
- ✅ Method registry for impl dispatch
- ✅ 741 tests passing

### v21.0 — Async, Generics, WASM & GPU
- ✅ Full async/await runtime (executor, channels, futures)
- ✅ Generics + type parameters + monomorphization
- ✅ Package manager + registry + dependency resolver
- ✅ LSP server + IDE support (diagnostics, completion, hover)
- ✅ WebAssembly target (module builder, LEB128, sections)
- ✅ GPU compute backend (buffers, kernels, pipelines, shaders)
- ✅ 870 tests · 47 modules · 35,856 LOC

### v22.0 — Borrow Checker, DAP, REPL & AOT
- ✅ Ownership & borrow checker (move tracking, scope analysis)
- ✅ Incremental compilation (hash caching, dep graph, topo sort)
- ✅ Full trait dispatch with vtables + method resolution
- ✅ Debug Adapter Protocol (breakpoints, stack, variables, stepping)
- ✅ Interactive REPL (eval, commands, history)
- ✅ Lifetime annotations + region-based memory analysis
- ✅ Effect system + capability types + algebraic effects
- ✅ Incremental codegen + hot-reload with file watching
- ✅ Self-hosted compiler bootstrap (Stage 0/1/2 pipeline)
- ✅ Native AOT compilation (standalone executables)
- ✅ Cross-compilation targets (x86-64, AArch64, RISC-V)
- ✅ 1,043 tests · 58 modules · 41,772 LOC

### v23.0 — Non-Lexical Lifetimes
- ✅ NLL borrow analysis with CFG-based liveness
- ✅ Control-flow graph builder from AST
- ✅ Backward dataflow liveness analysis (live_in/live_out)
- ✅ NLL regions as sets of CFG points (not lexical scopes)
- ✅ Borrow conflict detection via overlapping live ranges
- ✅ Modify-while-borrowed checks
- ✅ 1,087 tests · 59 modules · 43,095 LOC

### v24.0 — Effect Handlers & Pattern Exhaustiveness
- ✅ Algebraic effect handler system with `handle { } with { }` blocks
- ✅ First-class continuations (resume/abort) within effect handlers
- ✅ Handler stack with LIFO dispatch and nested handler frames
- ✅ Handler composition — combine/layer multiple handlers
- ✅ Effect dispatcher resolving `perform` through handler chain
- ✅ Handler validation (duplicate effects, unhandled effects, arity checks)
- ✅ Pattern matching exhaustiveness checker (Maranget usefulness algorithm)
- ✅ Or-patterns (`A | B`), guard clauses, nested destructuring
- ✅ Redundant/unreachable arm detection with diagnostics
- ✅ AST extensions: Or/Tuple patterns, Handle expression
- ✅ 1,177 tests · 61 modules · 45,703 LOC

### v25.0 — Code Formatter, Linter & Refinement Types
- ✅ AST-based code formatter with configurable style
- ✅ Static linter with 17 rules and configurable severity
- ✅ Refinement/dependent types with constraint solver and subtype checking
- ✅ 1,284 tests · 64 modules · 47,743 LOC

### v26.0 — Macro System, Compile-Time Eval & Iterators
- ✅ Hygienic macro system with token trees and derive macros
- ✅ Compile-time evaluation (const fns, static assertions, constant folding)
- ✅ Lazy iterator protocol with 13 adapters and generator→state-machine lowering
- ✅ 1,458 tests · 67 modules · 53,359 LOC

### v27.0 — Structured Concurrency, Type Inference & Documentation
- ✅ Structured concurrency (Mutex, RwLock, channels, Select, WaitGroup, atomics)
- ✅ Hindley-Milner Algorithm W type inference with union/intersection types
- ✅ Documentation generation (doc-comment parser, API model, Markdown/HTML output)
- ✅ 1,586 tests · 70 modules · 57,196 LOC

### v28.0 — Graphics Engine, Shaders, GUI & Creative Coding
- ✅ Software rasterizer with 2D/3D primitives and transformation pipeline
- ✅ Shader language compiler (GLSL/HLSL/Metal/WGSL/SPIR-V backends)
- ✅ Retained-mode GUI framework with layout engine and theming
- ✅ Creative coding toolkit (Perlin noise, particle systems, L-systems)
- ✅ Visual node graph editor for data-flow programming
- ✅ Chart rendering (bar, line, pie, scatter, histogram, heatmap)
- ✅ 1,765 tests · 76 modules · 62,700 LOC

### v29.0 — Profiler, Memory Pools, FFI Bindgen, Type Classes, Build System & Benchmarks
- ✅ Execution profiler with call graphs, flame graphs, PGO hints, hot-path detection
- ✅ Advanced memory allocators (arena, pool, slab, buddy) with RC heap and cycle detection
- ✅ Multi-language FFI bindgen — C headers, TypeScript .d.ts, calling conventions, type marshaling
- ✅ Higher-kinded types, type classes, GADTs, type families, type-level naturals, kind checker
- ✅ Build graph DAG with content-addressed cache (SHA-256), work-stealing scheduler, critical path
- ✅ Micro-benchmarking framework with outlier detection, confidence intervals, regression testing
- ✅ 1,931 tests · 82 modules · ~68,200 LOC

### v30.0 — Regex Engine, Serialization, Property Testing, Data Structures, Networking & ECS
- ✅ Thompson NFA + Pike VM regex engine with O(n·m) guaranteed matching (no backtracking)
- ✅ Character classes, quantifiers (greedy/lazy), anchors, alternation, capturing groups
- ✅ JSON parser/stringify with full spec compliance; Base64, Hex, URL encoding, Varint/LEB128, MessagePack
- ✅ JSON path queries for nested data extraction
- ✅ QuickCheck-style property-based testing with automatic shrinking (Xorshift128+ PRNG, binary search shrink)
- ✅ B-Tree, Skip List, Ring Buffer, Union-Find (path compression + union by rank), Interval Tree, LRU Cache
- ✅ URL parser (RFC 3986), HTTP/1.1 request/response builder & parser, HTTP/2 frame codec
- ✅ WebSocket frame codec (RFC 6455), DNS packet builder/parser (RFC 1035), TCP state machine (RFC 793)
- ✅ IP address validation (IPv4/IPv6)
- ✅ Entity-Component-System with generational entity IDs and sparse set storage (O(1) CRUD)
- ✅ Component queries with With/Without filters, system scheduling with dependency ordering
- ✅ 2,108 tests · 88 modules · ~72,000 LOC

---

### The AI Programming Language Arc

> **Vision**: Transform Vitalis from a compiled language *with* AI libraries into a language
> *built for* AI — where tensors are first-class types, every function is differentiable,
> the compiler optimizes itself, and programs can write, test, and improve themselves.
>
> Vitalis already has: self-modifying code (`@evolvable`), Thompson sampling compiler
> oracles, meta-evolution (learning to learn), bio-inspired memory (5 engram types),
> spiking neural networks (STDP, ESN, NEAT), and research-grade evolutionary algorithms
> (CMA-ES, NSGA-II, MAP-Elites, Novelty Search). The roadmap below builds on these
> foundations with a laser focus on **differentiable computing**, **neural architecture**,
> **self-improvement**, and **AI-native language semantics**.

---

### Phase 1: Tensor & Differentiable Computing Foundation

#### v31.0 — Tensor Engine & Accelerated Linear Algebra ✅
> **Goal**: Make tensors a first-class citizen with shape-aware operations and hardware acceleration.
> Everything downstream (autograd, neural nets, transformers) depends on this being fast and correct.

- 📋 **`tensor.rs`** — N-dimensional tensor type with compile-time and runtime shape tracking
  - `Tensor<f32>` / `Tensor<f64>` / `Tensor<bf16>` with contiguous + strided memory layouts
  - Shape inference, broadcasting (NumPy semantics), reshaping, slicing, transposition with zero-copy views
  - Element-wise ops: add, sub, mul, div, pow, exp, log, sqrt, abs, clamp
  - Reduction ops: sum, mean, max, min, argmax, argmin over arbitrary axes
  - Tiled SIMD matrix multiplication (Goto algorithm — 6×16 micro-kernel, L1/L2 cache blocking)
  - Batched matmul for attention heads: `(B, H, S, D) × (B, H, D, S) → (B, H, S, S)`
  - Memory pool integration (reuse `memory_pool.rs` arena allocators for tensor scratch space)
  - In-place mutation API (`.add_()`, `.mul_()`) for memory-efficient training
  - FFI: create, fill, matmul, elementwise, reduce, reshape, slice, broadcast

- 📋 **`autograd.rs`** — Reverse-mode automatic differentiation
  - Wengert list (tape) recording: each operation appends a `TapeEntry { op, inputs, output }`
  - Topological sort backward pass — correct gradient ordering for arbitrary DAGs
  - Gradient accumulation for parameter sharing and multi-use intermediate values
  - Gradient checkpointing (Chen et al. 2016) — trade O(√n) recomputation for O(√n) memory
  - Backward implementations for all tensor ops: matmul, softmax, layer_norm, cross_entropy, etc.
  - `no_grad` context — skip tape recording for inference
  - Gradient clipping (max-norm, value clipping) for training stability
  - Second-order gradients (Hessian-vector products) for meta-learning
  - Integration with `hotpath.rs` existing forward ops (ReLU, GELU, sigmoid, softmax, batch_norm)

#### v32.0 — Neural Network Layers & Training Engine ✅
> **Goal**: Production-grade neural network building blocks. Designed so that layers compose
> cleanly and the training loop handles gradient accumulation, mixed precision, and checkpointing.

- 📋 **`neural_net.rs`** — Layer abstractions (all with forward + backward)
  - **Linear**: `y = xW^T + b` with Kaiming/Xavier initialization
  - **Conv2D**: im2col + GEMM implementation (no FFT — simpler, GEMM-bound anyway)
  - **Embedding**: Lookup table with sparse gradient support
  - **LayerNorm / RMSNorm**: Pre-norm and post-norm variants (RMSNorm for transformers)
  - **Dropout**: Inverted dropout with deterministic mask replay for reproducibility
  - **Residual**: `f(x) + x` with optional projection
  - **Sequential**: Chain layers with automatic shape propagation
  - Weight initialization: Xavier uniform/normal, Kaiming fan-in/fan-out, zero, orthogonal

- 📋 **`training_engine.rs`** — Training loop with optimizer integration
  - **Optimizers**: SGD+momentum, Adam, AdamW (decoupled weight decay), LAMB, Adafactor
    - Extend `ml.rs` existing `adam_step`/`sgd_momentum_step` with parameter groups and state
  - **LR Schedulers**: Cosine annealing with warm restarts, linear warmup, OneCycleLR, polynomial decay
  - Gradient accumulation over N micro-batches (constant memory, effective batch = N × micro)
  - Mixed precision via f32 master weights + bf16 forward/backward (loss scaling for underflow)
  - Checkpointing: save/load model weights + optimizer state + RNG state + step counter
  - EarlyStopping with patience and delta threshold
  - **Loss functions**: Extend `hotpath.rs` with backward variants — cross_entropy_backward, mse_backward, etc.

---

### Phase 2: Transformer Architecture & LLM Stack

#### v33.0 — Transformer & Attention Mechanisms ✅
> **Goal**: Complete transformer implementation — the architecture powering all modern AI.
> Built on v31/v32 tensors and autograd. Designed for both training and efficient inference.

- 📋 **`transformer.rs`** — Transformer building blocks
  - **Scaled Dot-Product Attention**: `softmax(QK^T / √d_k) V` with causal masking
  - **Multi-Head Attention (MHA)**: Parallel attention heads with output projection
  - **Grouped-Query Attention (GQA)**: Key-value sharing across query groups (LLaMA-style)
  - **Positional Encoding**: Sinusoidal, Rotary (RoPE — Su et al. 2021), ALiBi (Press et al. 2022)
  - **Feed-Forward Network**: SwiGLU activation (Shazeer 2020) — `SwiGLU(x) = (xW₁ ⊙ Swish(xV)) W₂`
  - **Pre-Norm Transformer Block**: RMSNorm → Attention → Residual → RMSNorm → FFN → Residual
  - **KV Cache**: Pre-allocated key-value cache for autoregressive generation (O(1) per new token)
  - **Flash Attention approximation**: Tiled softmax with online normalization (Dao et al. 2022 algorithm)
  - Full encoder and decoder stacks with configurable depth, width, heads

- 📋 **`tokenizer.rs`** — Subword tokenization
  - **Byte-Pair Encoding (BPE)**: Sennrich et al. 2016 — merge frequency-based, O(n·V) training
  - **WordPiece**: Schuster & Nakajima 2012 — likelihood-based merging
  - **Unigram**: Kudo 2018 — EM algorithm for subword selection with entropy-based pruning
  - Byte-level fallback for unknown characters (UTF-8 → byte tokens)
  - Special tokens: `<PAD>`, `<BOS>`, `<EOS>`, `<UNK>`, `<MASK>`
  - Vocabulary persistence (save/load), configurable vocab size, merge rules export
  - Pre-tokenization: whitespace splitting, regex-based (GPT-2 pattern), byte-level

#### v34.0 — Inference Engine & Model Adaptation ✅
> **Goal**: Efficient inference for trained models + fine-tuning without full retraining.
> This is where Vitalis becomes practical for deploying and adapting AI models.

- 📋 **`inference.rs`** — High-performance inference runtime
  - **Batched inference**: Dynamic batching with padding and attention masks
  - **KV cache management**: Ring buffer eviction, paged attention (vLLM-inspired)
  - **Speculative decoding**: Draft model generates N tokens, target model verifies in parallel
  - **Sampling strategies**: Temperature, top-k, top-p (nucleus), min-p, repetition penalty, typical sampling
  - **Beam search**: Width-configurable with length normalization and n-gram blocking
  - **Streaming token generation**: Yield tokens as produced, not waiting for full sequence
  - Token-per-second throughput tracking, latency percentiles

- 📋 **`model_adaptation.rs`** — Parameter-efficient fine-tuning
  - **LoRA**: Low-Rank Adaptation (Hu et al. 2021) — `W' = W + BA` where B∈ℝ^(d×r), A∈ℝ^(r×k)
  - **QLoRA**: 4-bit NormalFloat quantized base + fp16 LoRA adapters (Dettmers et al. 2023)
  - **Adapter Layers**: Bottleneck adapters inserted after attention/FFN (Houlsby et al. 2019)
  - **Prefix Tuning**: Learnable prefix tokens prepended to key/value projections (Li & Liang 2021)
  - Adapter merging: Fold trained adapters into base weights for zero-overhead inference
  - Multi-adapter serving: Switch between task-specific adapters at runtime

- 📋 **`quantization.rs`** — Model compression for deployment
  - **INT8 quantization**: Per-tensor and per-channel symmetric/asymmetric (calibrated min-max)
  - **INT4 quantization**: GPTQ (Frantar et al. 2023) — layer-wise Hessian-based optimal quantization
  - **NormalFloat4** (NF4): Information-theoretically optimal 4-bit dtype (QLoRA)
  - **Dynamic quantization**: Quantize weights offline, activations at runtime
  - **Quantized matmul**: INT8×INT8→INT32 accumulate with dequantization
  - **Mixed-precision graph**: Per-layer quantization sensitivity analysis

---

### Phase 3: Self-Improving & Autonomous Intelligence

#### v35.0 — Code Intelligence & Program Synthesis ✅
> **Goal**: The compiler understands code semantically, generates code from specifications,
> and learns from its own history. This connects the evolution system to modern AI techniques.

- 📋 **`code_intelligence.rs`** — AI-powered code understanding
  - **Code embedding**: AST → fixed-dimensional vector (tree-LSTM or GNN on AST structure)
  - **Similarity search**: Cosine similarity over code embeddings for clone detection
  - **Complexity prediction**: ML model predicting execution time from IR features
  - **Bug prediction**: Logistic regression over code metrics (cyclomatic complexity, churn, coupling)
  - **Semantic code search**: Natural language query → ranked code snippet results
  - Integration with `memory.rs` — store code embeddings as semantic engrams for associative recall

- 📋 **`program_synthesis.rs`** — Generate programs from specifications
  - **Type-guided synthesis**: Fill holes in typed programs via constraint satisfaction (SyGuS-style)
  - **Input/Output synthesis**: Generate functions from example input-output pairs (FlashFill algorithm)
  - **Sketch completion**: User provides program skeleton with `??` holes, synthesizer fills them
  - **Counter-example guided refinement** (CEGIS): Synthesize → verify → refine loop
  - **Enumeration with pruning**: Bottom-up search with observational equivalence pruning
  - Integration with `property_testing.rs` for automatic verification of synthesized programs

- 📋 **`self_optimizer.rs`** — ML-driven compiler optimization
  - **RL pass ordering**: Reinforcement learning agent (contextual bandit) selects optimization pass sequence
  - **Cost model**: Neural network predicting execution cycles from IR features
  - **Inlining policy network**: Extend `optimizer.rs` Thompson sampling oracle with learned features
  - **Auto-tuning**: Bayesian optimization (Gaussian Process + Expected Improvement) for compiler flags
  - **Profile-guided optimization**: Use `profiler.rs` data to train cost models
  - **Transfer learning**: Apply optimization knowledge from one program to similar programs

#### v36.0 — Autonomous Evolution & Self-Rewriting ✅
> **Goal**: Vitalis programs can rewrite themselves — not just evolve variants, but
> understand their own structure, propose improvements, and verify safety before applying them.
> This extends the existing `@evolvable` system into a full autonomous improvement loop.

- 📋 **`autonomous_agent.rs`** — Self-improving program agent
  - **Reflection API**: Programs inspect their own AST, types, effects, and performance profile
  - **Mutation operators**: AST-level mutations (swap expressions, change operators, reorder statements)
  - **Crossover**: Homologous crossover between function variants at AST level
  - **Safety verification**: Synthesized code must pass type checker + effect checker + property tests
  - **Improvement budget**: Cap computation spent on self-improvement per cycle (resource bounds)
  - **Improvement journal**: Persistent log of all attempted + accepted mutations with fitness deltas
  - Extend `engine.rs` and `meta_evolution.rs` with AST-aware mutation and crossover

- 📋 **`reward_model.rs`** — Learned fitness functions
  - **Preference learning**: Learn fitness from pairwise comparisons (Bradley-Terry model)
  - **Reward shaping**: Potential-based reward shaping for faster convergence
  - **Multi-objective reward**: Scalarization, Pareto, and hypervolume-based aggregation
  - **Surrogate model**: Gaussian Process regression to predict fitness without full execution
  - **Curiosity-driven exploration**: Intrinsic reward for novel code patterns (prediction error)
  - Integration with `scoring.rs` existing Elo, Pareto, and A/B testing infrastructure

---

### Phase 4: AI-Native Language Semantics

#### v37.0 — Differentiable & Probabilistic Programming ✅
> **Goal**: Make differentiation and probability first-class language concepts.
> Not library calls — actual language semantics where the type system tracks gradients
> and the compiler generates efficient gradient code automatically.

- ✅ **`differentiable.rs`** — Language-level differentiable programming
  - **`@differentiable` annotation**: Mark functions as differentiable, compiler generates backward pass
  - **Dual numbers**: Forward-mode AD via dual number arithmetic (`value + ε·derivative`)
  - **Differentiable control flow**: Differentiate through if/else (straight-through estimator), while loops (scan), recursion (implicit differentiation)
  - **Custom VJP rules**: User-defined vector-Jacobian products for opaque operations
  - **Shape types**: Compile-time tensor shape checking: `Tensor<f32, [B, 768]>` catches shape errors at compile time
  - Integration with `type_inference.rs` — infer gradient types from forward types
  - Integration with `effects.rs` — `Differentiable` as a capability effect

- ✅ **`probabilistic.rs`** — Probabilistic programming primitives
  - **Distribution types**: Normal, Bernoulli, Categorical, Dirichlet, Beta, Poisson as first-class values
  - **`sample` / `observe` / `condition`**: Probabilistic programming operators
  - **Inference engines**: MCMC (Metropolis-Hastings, HMC/NUTS), Variational Inference (ELBO + reparameterization trick)
  - **Bayesian neural networks**: Weight distributions instead of point estimates
  - **Gaussian Process regression**: Kernel functions (RBF, Matérn, periodic), posterior prediction
  - **Probabilistic model checking**: Verify probabilistic safety properties
  - Extend `probability.rs` distributions with sampling, log-probability, and gradient support

#### v38.0 — Reinforcement Learning & Simulation ✅
> **Goal**: Native RL types and simulation framework. Programs define environments,
> agents learn policies, and the `@evolvable` system can use RL for code optimization.

- ✅ **`rl_framework.rs`** — Reinforcement learning primitives
  - **Environment protocol**: `State`, `Action`, `Reward`, `Done` types with `step()` / `reset()` interface
  - **Policy types**: ε-greedy, softmax, Gaussian (continuous), categorical (discrete)
  - **Value functions**: Q-table, linear function approximation, neural value network
  - **Algorithms**: DQN (replay buffer + target network), PPO (clipped surrogate objective), A2C, REINFORCE
  - **Replay buffers**: Uniform, prioritized experience replay (sum-tree), HER (Hindsight Experience Replay)
  - **Multi-agent**: Independent learners, centralized-critic, communication channels
  - Integration with `evolution_advanced.rs` — evolutionary strategies as RL baselines
  - Integration with `autonomous_agent.rs` — RL agent optimizes code via environment interface

- ✅ **`simulation.rs`** — Simulation environments for RL and testing
  - **Grid worlds**: Configurable maze, cliff walking, frozen lake (tabular RL benchmarks)
  - **Continuous control**: CartPole, inverted pendulum, point navigation (function approximation benchmarks)
  - **Code optimization environment**: State=IR, Action=optimization pass, Reward=speedup
  - **Competitive environments**: Two-player adversarial games for coevolutionary training
  - Time-stepped simulation loop with configurable physics and rendering hooks

---

### Phase 5: Production AI Infrastructure

#### v39.0 — Data Pipeline & Experiment Tracking ✅
> **Goal**: Complete ML workflow — from raw data to trained model to deployed inference.
> No dependency on external Python tools for the full AI lifecycle.

- ✅ **`data_pipeline.rs`** — ML data loading and preprocessing
  - **Dataset abstraction**: `Dataset` trait with `len()`, `get(index)`, random access
  - **DataLoader**: Batching, shuffling, prefetching with configurable workers
  - **Transforms**: Normalize, one-hot encode, tokenize, augment (random crop, flip, noise)
  - **Streaming datasets**: Iterator-based for datasets that don't fit in memory
  - **Data formats**: CSV, TSV, JSON Lines, binary tensor format (memory-mapped)
  - **Train/val/test splitting**: Stratified splitting, k-fold cross-validation

- ✅ **`experiment.rs`** — Experiment tracking and reproducibility
  - **Run tracking**: Log hyperparameters, metrics (loss, accuracy, etc.), artifacts per experiment
  - **Metric history**: Time-series of training metrics with visualization data export
  - **Hyperparameter search**: Grid search, random search, Bayesian optimization (GP+EI)
  - **Reproducibility**: Seed management, config snapshots, environment fingerprinting
  - **Model registry**: Version models with metadata, promote candidates to production
  - **Comparison**: Tabular comparison of runs, statistical significance testing via `scoring.rs`

#### v40.0 — Model Serving & AI Observability ✅
> **Goal**: Deploy trained models with monitoring, safety guardrails, and A/B testing.
> The full loop from training to production to monitoring back to retraining.

- ✅ **`model_serving.rs`** — Production inference serving
  - **Model loading**: Weight deserialization, JIT warm-up, memory-mapped weights
  - **Batched request handling**: Dynamic batching with timeout-based flush
  - **Model versioning**: Serve multiple model versions, gradual traffic shifting
  - **ONNX export**: Convert Vitalis models to ONNX for cross-platform deployment
  - **Edge deployment**: Quantized models for resource-constrained environments
  - Integration with `networking.rs` HTTP/2 for gRPC-style model endpoints

- ✅ **`ai_observability.rs`** — AI model monitoring and safety
  - **Drift detection**: Kolmogorov-Smirnov, Population Stability Index (PSI), MMD for feature/prediction drift
  - **Fairness metrics**: Demographic parity, equalized odds, calibration across groups
  - **Explainability**: SHAP values (KernelSHAP), LIME-style local explanations, attention visualization
  - **Safety guardrails**: Output filtering, toxicity scoring, confidence thresholds, fallback policies
  - **A/B testing for models**: Integrate with `scoring.rs` Bayesian A/B, track conversion metrics
  - **Alert system**: Configurable thresholds, anomaly detection via `analytics.rs` CUSUM/Z-score

---

### Phase 6: Platform & Ecosystem Maturity

#### v41.0 — WASM AOT & WASI Runtime ✅
- ✅ WASM AOT target — compile `.sl` → standalone `.wasm` files (`wasm_aot.rs`)
- ✅ WASM-WASI support for file I/O and environment access in WebAssembly
- ✅ WASM component model integration for language interop
- ✅ Browser runtime shim and size optimization passes (DCE, tree shaking)

#### v42.0 — Package Registry & Distributed Build ✅
- ✅ Package registry server, dependency vulnerability scanning, lockfile pinning (`distributed_build.rs`)
- ✅ Distributed compilation across networked nodes with content-addressed shared cache
- ✅ Hermetic builds with sandboxed environments

#### v43.0 — Formal Verification & Advanced IDE ✅
- ✅ Contract-based programming (pre/postconditions, invariants, proof-carrying code) (`formal_verification.rs`)
- ✅ Symbolic execution engine for property checking
- ✅ LSP v4 features, IDE profiler integration, refactoring engine, code coverage reporting (`ide_features.rs`)

---

### Phase 7: Research Frontier

#### v44.0 — NAS, Continual & Federated Learning ✅ (Current Release)
- ✅ **Neural Architecture Search (NAS)**: Evolutionary + RL-based architecture optimization (`nas.rs`)
  - Extend `evolution_advanced.rs` NSGA-II + MAP-Elites for architecture space exploration
  - Network morphism operators (widen, deepen, skip) for efficient search
- ✅ **Neuro-symbolic integration**: Combine neural attention with `automata.rs` symbolic reasoning
- ✅ **Continual learning**: Elastic Weight Consolidation (EWC), progressive nets, memory replay (`continual_learning.rs`)
- ✅ **Self-evolving optimizer passes**: `optimizer.rs` passes that evolve themselves via `@evolvable`
- ✅ **Auto-vectorization**: Detect SIMD opportunities in IR, emit `simd_ops.rs` intrinsics
- ✅ **Effect polymorphism**: Row-polymorphic effects, algebraic subtyping with polar types
- ✅ **Capability-secure modules**: Object-capability model for AI safety sandboxing
- ✅ **Neuromorphic hardware targeting**: Compile SNN models from `neuromorphic.rs` to Intel Loihi / SpiNNaker
- ✅ **Federated learning**: Privacy-preserving distributed training with differential privacy guarantees (`federated_learning.rs`)
- ✅ **World models**: Learned environment simulators for model-based RL (MBRL)

---

## Architecture: How It All Connects

```
                          ┌─────────────────────────────────────────────────┐
                          │          VITALIS AI LANGUAGE STACK              │
                          │                                                 │
  ┌──────────┐           │  ┌─────────────────────────────────────────┐   │
  │ .sl code │──parse──▶ │  │  Compiler Pipeline (lexer→parser→IR)    │   │
  │ @evolvable│           │  │    + type_inference + effects + autograd│   │
  └──────────┘           │  │    + @differentiable shape-checking      │   │
                          │  └────────┬────────────────────────────────┘   │
                          │           │                                     │
                          │           ▼                                     │
                          │  ┌────────────────────────────────────────┐    │
                          │  │  Tensor Engine + SIMD Matmul            │    │
                          │  │  (tensor.rs + simd_ops.rs + numerical)  │    │
                          │  └────────┬───────────────────────────────┘    │
                          │           │                                     │
                          │           ▼                                     │
                          │  ┌────────────────────────────────────────┐    │
                          │  │  Autograd (reverse-mode AD tape)        │    │
                          │  │  + checkpointing + gradient clipping    │    │
                          │  └────────┬───────────────────────────────┘    │
                          │           │                                     │
                          │           ▼                                     │
                          │  ┌────────────────────────────────────────┐    │
                          │  │  Neural Layers + Transformer + Training │    │
                          │  │  (neural_net + transformer + training)  │    │
                          │  └────────┬───────────────────────────────┘    │
                          │           │                                     │
                          │     ┌─────┴─────────┐                          │
                          │     ▼               ▼                          │
                          │  ┌────────┐   ┌──────────────┐                │
                          │  │Inference│   │ LoRA / QLoRA │                │
                          │  │KV cache │   │ Fine-tuning  │                │
                          │  │Sampling │   │ Quantization │                │
                          │  └────────┘   └──────────────┘                │
                          │                                                 │
                          │  ┌────────────────────────────────────────┐    │
                          │  │  Self-Improvement Loop                  │    │
                          │  │  evolution.rs ←→ engine.rs              │    │
                          │  │  meta_evolution ←→ autonomous_agent     │    │
                          │  │  code_intelligence ←→ program_synthesis │    │
                          │  │  reward_model ←→ rl_framework           │    │
                          │  │  memory.rs (engram) ←→ profiler.rs      │    │
                          │  └────────────────────────────────────────┘    │
                          └─────────────────────────────────────────────────┘
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **f32 as default precision** | All modern AI uses f32 or lower; f64 is 2× slower and unnecessary for neural nets |
| **Tape-based autograd** (not source-transformation) | More flexible for dynamic computation graphs, easier to implement, covers all control flow |
| **Goto-algorithm SIMD matmul** | Best known single-threaded matmul performance without external dependencies (BLAS) |
| **Thompson sampling for compiler decisions** | Already proven in `optimizer.rs`; extend to all compiler heuristics |
| **RoPE over sinusoidal positional encoding** | RoPE generalizes to unseen sequence lengths and is now standard (LLaMA, Mistral, Gemma) |
| **SwiGLU over ReLU FFN** | ~1% accuracy gain at same compute; standard in all modern transformers |
| **QLoRA for fine-tuning** | Enables fine-tuning of large models on consumer hardware (4-bit quantization) |
| **CEGIS for program synthesis** | Sound verification loop ensures synthesized code is correct, not just plausible |
| **PPO for RL** | Most stable policy-gradient algorithm; used in RLHF, robotics, and game AI |

---

## Version History

| Version | Date | Modules | Tests | LOC | Key Feature |
|---------|------|---------|-------|-----|-------------|
| v0.1.0 | 2025-03-01 | 17 | 234 | ~13,500 | Initial compiler pipeline |
| v9.0.0 | 2025-03-01 | 31 | 470 | ~24,769 | 14 algorithm libraries |
| v10.0.0 | 2025-04-15 | 36 | ~550 | ~27,000 | ML, geometry, automata |
| v13.0.0 | 2025-05-01 | 41 | ~650 | ~30,000 | Quantum, bio, neuromorphic |
| v15.0.0 | 2025-05-20 | 41 | ~650 | ~31,000 | Closures, error handling |
| v19.0.0 | 2025-06-10 | 41 | ~650 | ~32,000 | Structs, modules, HTTP |
| v20.0.0 | 2025-06-20 | 41 | 741 | ~32,500 | Traits, type aliases, enums |
| v21.0.0 | 2025-07-05 | 47 | 870 | ~35,856 | Async, generics, WASM, GPU |
| v22.0.0 | 2025-07-19 | 58 | 1,043 | ~41,772 | Borrow checker, DAP, AOT |
| v23.0.0 | 2025-07-26 | 59 | 1,087 | ~43,095 | Non-Lexical Lifetimes |
| v24.0.0 | 2026-03-03 | 61 | 1,177 | ~45,703 | Effect handlers, pattern exhaustiveness |
| v25.0.0 | 2026-03-10 | 64 | 1,284 | ~47,743 | Formatter, linter, refinement types |
| v26.0.0 | 2026-03-17 | 67 | 1,458 | ~53,359 | Macros, const eval, iterators |
| v27.0.0 | 2026-03-24 | 70 | 1,586 | ~57,196 | Concurrency, type inference, documentation |
| v28.0.0 | 2026-03-31 | 76 | 1,765 | ~62,700 | Graphics engine, shaders, GUI, creative coding, visual nodes, charts |
| v29.0.0 | 2026-04-07 | 82 | 1,931 | ~68,200 | Profiler, memory pools, FFI bindgen, type classes, build system, benchmarks |
| v30.0.0 | 2026-04-14 | 88 | 2,108 | ~72,000 | Regex engine, serialization, property testing, data structures, networking, ECS |
| v31.0.0 | 2026-04-21 | 90 | 2,158 | ~76,000 | Tensor engine, SIMD matmul, autograd |
| v32.0.0 | 2026-04-28 | 92 | 2,213 | ~80,000 | Neural network layers, training engine |
| v33.0.0 | 2026-05-05 | 94 | 2,263 | ~84,000 | Transformer, tokenizer engine |
| v34.0.0 | 2026-05-12 | 97 | 2,328 | ~88,000 | Inference, model adaptation, quantization |
| v35.0.0 | 2026-05-19 | 100 | 2,368 | ~92,000 | Code intelligence, program synthesis, self-optimizer |
| v36.0.0 | 2026-05-26 | 102 | 2,408 | ~95,000 | Autonomous agent, reward model |
| v37.0.0 | 2026-06-02 | 104 | 2,458 | ~98,000 | Differentiable programming, probabilistic programming |
| v38.0.0 | 2026-06-09 | 106 | 2,508 | ~101,000 | RL framework, simulation environments |
| v39.0.0 | 2026-06-16 | 108 | 2,548 | ~104,000 | Data pipeline, experiment tracking |
| v40.0.0 | 2026-06-23 | 110 | 2,588 | ~107,000 | Model serving, AI observability |
| v41.0.0 | 2026-06-30 | 111 | 2,598 | ~108,000 | WASM AOT, WASI, component model |
| v42.0.0 | 2026-07-07 | 112 | 2,608 | ~109,000 | Package registry, distributed build |
| v43.0.0 | 2026-07-14 | 114 | 2,618 | ~109,500 | Formal verification, IDE features |
| v44.0.0 | 2026-07-21 | 117 | 2,627 | ~110,000 | NAS, continual learning, federated learning |
