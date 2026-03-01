# Vitalis — Open Source Release Setup
# Run this script to initialize the git repo and prepare for GitHub push.

# 1. Initialize git repo
git init

# 2. Add all files
git add .

# 3. Initial commit
git commit -m "v0.1.0 — Vitalis: JIT-compiled language with built-in code evolution

A compiled programming language built in Rust with Cranelift JIT backend.
Features @evolvable annotations for runtime code evolution, 98 stdlib functions,
44 native hot-path operations (7.6x faster than Python), and Python FFI integration.

Compiler pipeline: Source → Lexer → Parser → AST → TypeChecker → IR → Cranelift JIT → Native"

# 4. Create main branch
git branch -M main

# 5. Add remote (update URL with your GitHub username)
git remote add origin https://github.com/ModernOps888/vitalis.git

# 6. Push
git push -u origin main

# 7. Create initial tag
git tag -a v0.1.0 -m "Initial open-source release"
git push origin v0.1.0
