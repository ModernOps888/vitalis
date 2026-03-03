# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 23.x    | ✅ |
| 22.x    | ✅ |
| < 22.0  | ❌ |

## Reporting a Vulnerability

If you discover a security vulnerability in Vitalis, please report it responsibly:

1. **Do NOT open a public issue.**
2. Email the maintainers or use GitHub's private vulnerability reporting.
3. Include a description, steps to reproduce, and potential impact.

We aim to respond within 48 hours and will work to patch confirmed vulnerabilities promptly.

## Scope

Vitalis is a compiler and JIT runtime. Security-relevant areas include:

- **JIT code execution** — compiled code runs natively
- **FFI boundary** — C interop via `unsafe extern "C"` functions
- **Memory safety** — raw pointer handling in hotpath and bridge modules
- **Input validation** — malformed `.sl` source handling
