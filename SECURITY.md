# Security Policy

## Supported Versions

| Version | Supported |
|---------|----------|
| 3.0.x   | $([char]0x2705) Current |
| 2.x     | $([char]0x26A0)$([char]0xFE0F) Critical fixes only |
| < 2.0   | $([char]0x274C) End of life |

## Reporting a Vulnerability

If you discover a security vulnerability in Vitalis, please report it responsibly:

1. **Do NOT open a public issue.**
2. Email the maintainers or use [GitHub's private vulnerability reporting](https://github.com/ModernOps888/vitalis/security/advisories/new).
3. Include a description, steps to reproduce, and potential impact.

We aim to respond within 48 hours and will work to patch confirmed vulnerabilities promptly.

## Scope

Vitalis is a compiler and JIT runtime. Security-relevant areas include:

- **JIT code execution** $([char]0x2014) compiled code runs natively on the host CPU
- **FFI boundary** $([char]0x2014) C interop via `unsafe extern "C"` functions (122 exports)
- **Memory safety** $([char]0x2014) raw pointer handling in hotpath and bridge modules
- **Input validation** $([char]0x2014) malformed `.sl` source handling and sanitization
- **Evolved code sandboxing** $([char]0x2014) `@evolvable` functions run in capability-scoped contexts with resource quotas
- **GPU compute** $([char]0x2014) CUDA kernel dispatch and GPU memory pool management

## Security Model

Vitalis v3.0 enforces a **capability-based security model**:

- **Permission-scoped execution** $([char]0x2014) file I/O, network access, and system calls require explicit capability grants
- **Resource quotas** $([char]0x2014) hard limits on memory allocation, CPU cycles, recursion depth, and evolution generations
- **Sandboxed evolution** $([char]0x2014) evolved functions cannot escape their sandbox or access ambient authority
- **Input sanitization** $([char]0x2014) all external inputs (source files, network data, FFI arguments) are validated before processing