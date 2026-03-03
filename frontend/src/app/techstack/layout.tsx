import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Live Tech Stack Map — Infinity AI",
  description:
    "Interactive real-time architecture visualization of the Infinity autonomous AI system. 154k+ LOC across Rust (Vitalis v21 compiler, Cranelift JIT, 870 tests, async/await, generics, WASM, LSP, GPU compute), Python (FastAPI, 72 AI modules), and TypeScript (Next.js cyberpunk dashboard). Self-evolving code engine with Asimov's Laws safety.",
  alternates: { canonical: "/techstack" },
  keywords: [
    "AI architecture diagram",
    "live tech stack",
    "self-evolving AI system",
    "Vitalis compiler",
    "Cranelift JIT compiler",
    "autonomous code evolution",
    "multi-agent AI swarm",
    "FastAPI AI backend",
    "Rust AI compiler",
    "AI safety Asimov laws",
    "cyberpunk dashboard",
    "real-time AI monitoring",
    "custom programming language Rust",
    "AI code generation engine",
    "solo developer AI system",
  ],
  openGraph: {
    title: "Live Tech Stack Map — Infinity AI",
    description:
      "Watch an autonomous AI system in real-time. 72 modules, 142 API endpoints, custom Rust compiler, and self-evolving code — all on a $5/day budget.",
    url: "https://infinitytechstack.uk/techstack",
    type: "website",
  },
};

export default function TechStackLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <>{children}</>;
}
