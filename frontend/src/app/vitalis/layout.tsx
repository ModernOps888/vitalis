import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Vitalis — AI-Native Programming Language",
  description:
    "Vitalis is a custom programming language built from scratch in Rust with Cranelift JIT compilation. v21 — 47 modules, 870 tests, 35,856 LOC. Async/await, generics, WASM target, LSP, GPU compute, package manager.",
  keywords: [
    "Vitalis language",
    "programming language",
    "Rust compiler",
    "Cranelift JIT",
    "AI native language",
    "custom compiler",
    "SSA IR",
    "SIMD optimization",
    "WebAssembly",
    "LSP server",
    "GPU compute",
    "async await",
    "generics",
    "Infinity AI",
  ],
  openGraph: {
    title: "Vitalis — AI-Native Programming Language",
    description:
      "A from-scratch programming language in Rust with Cranelift JIT, SIMD, generics, async/await, WASM, GPU compute, and a full LSP server. v21 — 870 tests passing.",
    url: "https://infinitytechstack.uk/vitalis",
    siteName: "Infinity Tech Stack",
    locale: "en_GB",
    type: "website",
  },
};

export default function VitalisLayout({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}
