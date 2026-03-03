"use client";

import { useState, useMemo } from "react";

interface SentientAvatarProps {
  state?: string | { state: string; setState?: any };
  compact?: boolean;
  size?: number;
  showLabel?: boolean;
  evolutions?: number;
}

export function SentientAvatar(_props: SentientAvatarProps) {
  return null;
}

export function useAvatarState(_status?: any): string {
  const [state] = useState("idle");
  const derived = useMemo(() => {
    if (!_status) return "idle";
    if (_status?.status === "running") return "active";
    return state;
  }, [_status, state]);
  return derived;
}
