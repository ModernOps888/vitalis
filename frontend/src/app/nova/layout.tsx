import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Nova — Self-Evolving Native LLM Engine",
  description:
    "Nova is a from-scratch LLM training engine built entirely in Rust with CUDA GPU acceleration. 12,119 LOC, 57 source files, custom tensor library, BPE tokenizer, transformer architecture — no PyTorch, no Python.",
  keywords: [
    "Nova LLM",
    "Rust LLM",
    "CUDA training",
    "native LLM",
    "self-evolving AI",
    "transformer from scratch",
    "Rust machine learning",
    "GPU accelerated",
    "custom tensor library",
    "BPE tokenizer",
    "RTX 5060",
    "Infinity AI",
  ],
  openGraph: {
    title: "Nova — Self-Evolving Native LLM Engine",
    description:
      "A from-scratch LLM training engine in pure Rust + CUDA. Custom tensors, autograd, BPE tokenizer, transformer blocks, evolutionary self-improvement. No PyTorch dependency.",
    url: "https://infinitytechstack.uk/nova",
    siteName: "Infinity Tech Stack",
    locale: "en_GB",
    type: "website",
  },
};

export default function NovaLayout({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}
