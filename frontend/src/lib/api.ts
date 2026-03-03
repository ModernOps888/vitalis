/**
 * Sentient ÔÇö API Client
 * Typed client for the FastAPI backend at localhost:8002
 */

export const API_BASE = "http://localhost:8002";

// ÔöÇÔöÇ Types ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface SystemStatus {
  status: string;
  uptime_seconds: number;
  modules_loaded: number;
  modules_failed: number;
  memory_count: number;
  active_goals: number;
  evolution_stats: {
    total_attempts: number;
    successes: number;
    failures: number;
    rollbacks: number;
  };
}

export interface ChatResponse {
  text: string;
  intent: string;
  emotion: string;
  latency_ms: number;
}

export interface Goal {
  id: string;
  description: string;
  priority: number;
  status: string;
  source: string;
  created_at: number;
  completed_at: number | null;
  progress_notes: string[];
}

export interface Module {
  name: string;
  purpose: string;
  version: string;
  capabilities: string[];
  is_ai_created: boolean;
  reload_count: number;
}

export interface EvolutionEntry {
  hash: string;
  message: string;
  date: string;
}

export interface Memory {
  content: string;
  memory_type: string;
  source: string;
  importance: number;
  timestamp: number;
  metadata: Record<string, unknown>;
}

// ÔöÇÔöÇ Swarm Types ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface ResearchResult {
  topic: string;
  summary: string;
  sources: string[];
  key_findings: string[];
  pages_fetched: number;
  duration_ms: number;
}

export interface ResearchStats {
  total_searches: number;
  total_pages_fetched: number;
  total_duration_ms: number;
  avg_duration_ms: number;
  evolution_researches: number;
  history_size: number;
}

export interface ResearchHistoryEntry {
  topic: string;
  summary: string;
  sources: string[];
  key_findings: string[];
  pages_fetched: number;
  duration_ms: number;
  timestamp: number;
  source: string;
}

// ÔöÇÔöÇ Swarm Types ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface SwarmStats {
  total_tasks: number;
  active_agents: number;
  max_concurrent: number;
  task_history_size: number;
  blackboard_entries: number;
}

export interface SwarmResult {
  result?: string;
  strategy?: string;
  agents_used?: number;
  execution_time?: number;
  consensus_score?: number;
  [key: string]: unknown;
}

// ÔöÇÔöÇ Plugin Types ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface PluginInfo {
  name: string;
  version: string;
  author: string;
  description: string;
  capabilities: string[];
  dependencies: string[];
  installed: boolean;
}

export interface PluginsData {
  installed: PluginInfo[];
  catalog: PluginInfo[];
  stats: { installed_count: number; enabled_count: number; catalog_size: number };
}

// ÔöÇÔöÇ Cluster Types ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface ClusterNode {
  node_id: string;
  host: string;
  port: number;
  state: string;
  role: string;
  alive: boolean;
  load_score: number;
  cpu_usage: number;
  memory_usage: number;
  active_requests: number;
}

export interface ClusterStatus {
  node_id: string;
  is_leader: boolean;
  cluster_size: number;
  healthy_nodes: number;
  nodes: ClusterNode[];
  recent_events: { type: string; node: string; details: string; timestamp: number }[];
}

// ÔöÇÔöÇ Rate Limiter Types ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface TokenBucket {
  available: number;
  capacity: number;
}

export interface CircuitBreaker {
  name: string;
  state: string;
  failure_count: number;
  success_count: number;
  total_calls: number;
}

export interface RateLimiterStats {
  total_requests: number;
  rejected_requests: number;
  rejection_rate: number;
  active_clients: number;
  token_buckets: Record<string, TokenBucket>;
  circuit_breakers: Record<string, CircuitBreaker>;
}

// ÔöÇÔöÇ Memory Persistence Types ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface MemoryPersistenceStats {
  consolidator: { total_consolidated: number; consolidation_cycles: number; avg_importance: number };
  graph: { total_relations: number; unique_memories: number };
  active_session: string | null;
}

// ÔöÇÔöÇ Distributed Evolution Types ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface DistributedEvoStats {
  instance_id: string;
  is_leader: boolean;
  rounds_completed: number;
  total_proposals: number;
  active_proposals: number;
  best_fitness: number;
  instances: { instance_id: string; host: string; port: number; health: number; evolution_count: number; fitness_avg: number }[];
}

// ÔöÇÔöÇ Fetch Helper ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (!res.ok) {
    const err = await res.text();
    throw new Error(`API ${res.status}: ${err}`);
  }
  return res.json();
}

// ÔöÇÔöÇ API Functions ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getStatus(): Promise<SystemStatus> {
  return api("/status");
}

export async function sendChat(message: string): Promise<ChatResponse> {
  return api("/chat", {
    method: "POST",
    body: JSON.stringify({ message }),
  });
}

export async function getGoals(): Promise<{ active: Goal[]; all: Goal[]; stats: Record<string, number> }> {
  return api("/goals");
}

export async function createGoal(description: string, priority: number): Promise<Goal> {
  return api("/goals", {
    method: "POST",
    body: JSON.stringify({ description, priority }),
  });
}

export async function getEvolutionHistory(): Promise<{
  history: EvolutionEntry[];
  stats: Record<string, number>;
}> {
  return api("/evolution/history");
}

export async function getModules(): Promise<{ modules: Module[]; errors: string[] }> {
  return api("/modules");
}

export async function searchMemory(query: string, n = 10): Promise<{ results: Memory[] }> {
  return api(`/memory/search?q=${encodeURIComponent(query)}&n=${n}`);
}

export async function getMemoryStats(): Promise<Record<string, unknown>> {
  return api("/memory/stats");
}

export async function getLogs(lines = 100): Promise<{ lines: string[] }> {
  return api(`/logs?lines=${lines}`);
}

export async function sendControl(action: string): Promise<{ status: string }> {
  return api("/control", {
    method: "POST",
    body: JSON.stringify({ action }),
  });
}

export async function research(topic: string): Promise<ResearchResult> {
  return api(`/research?topic=${encodeURIComponent(topic)}`, { method: "POST" });
}

export async function getResearchStats(): Promise<ResearchStats> {
  return api("/research/stats");
}

export async function getResearchHistory(limit = 20): Promise<{ history: ResearchHistoryEntry[] }> {
  return api(`/research/history?limit=${limit}`);
}

// ÔöÇÔöÇ Hybrid LLM & Sub-Agents ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getHybridStats(): Promise<Record<string, unknown>> {
  return api("/hybrid/stats");
}

export async function getSubagentStats(): Promise<Record<string, unknown>> {
  return api("/subagents/stats");
}

export async function spawnAgent(
  role: string,
  prompt: string,
  tier = "smart",
): Promise<Record<string, unknown>> {
  return api(
    `/subagents/spawn?role=${encodeURIComponent(role)}&prompt=${encodeURIComponent(prompt)}&tier=${tier}`,
    { method: "POST" },
  );
}

export async function runTeam(
  roles: string[],
  prompt: string,
): Promise<Record<string, unknown>> {
  return api(
    `/subagents/team?roles=${encodeURIComponent(roles.join(","))}&prompt=${encodeURIComponent(prompt)}`,
    { method: "POST" },
  );
}

// ÔöÇÔöÇ Swarm ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getSwarmStats(): Promise<SwarmStats> {
  return api("/swarm/stats");
}

export async function executeSwarm(
  task: string,
  strategy = "parallel",
  agents: string[] = [],
): Promise<SwarmResult> {
  return api("/swarm/execute", {
    method: "POST",
    body: JSON.stringify({ task, strategy, agents }),
  });
}

// ÔöÇÔöÇ Plugins ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getPlugins(): Promise<PluginsData> {
  return api("/plugins");
}

export async function installPlugin(name: string): Promise<{ status: string }> {
  return api("/plugins/install", {
    method: "POST",
    body: JSON.stringify({ name }),
  });
}

// ÔöÇÔöÇ Cluster & Infrastructure ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getClusterStatus(): Promise<ClusterStatus> {
  return api("/cluster/status");
}

export async function getRateLimiterStats(): Promise<RateLimiterStats> {
  return api("/rate-limiter/stats");
}

export async function getMemoryPersistence(): Promise<MemoryPersistenceStats> {
  return api("/memory/persistence/stats");
}

export async function getDistributedEvolution(): Promise<DistributedEvoStats> {
  return api("/evolution/distributed/stats");
}

// ÔöÇÔöÇ Code Analysis ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function analyzeCode(file_path: string): Promise<Record<string, unknown>> {
  return api("/analyze/code", {
    method: "POST",
    body: JSON.stringify({ file_path }),
  });
}

// ÔöÇÔöÇ Evolution Detail ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface EvolutionDetail {
  hash: string;
  full_hash: string;
  message: string;
  date: string;
  author: string;
  files_changed: { path: string; change_type: string; insertions: number; deletions: number }[];
  diff: string;
  stats: Record<string, { insertions: number; deletions: number; lines: number }>;
}

export interface InternalEvolution {
  success: boolean;
  commit_hash: string | null;
  description: string;
  file_path: string;
  change_type: string;
  error: string | null;
  duration_ms: number;
  timestamp: number;
}

export interface JournalEntry {
  success: boolean;
  commit_hash: string | null;
  description: string;
  file_path: string;
  change_type: string;
  error: string | null;
  duration_ms: number;
  timestamp: number;
}

export async function getEvolutionDetail(hash: string): Promise<EvolutionDetail> {
  return api(`/evolution/detail/${hash}`);
}

export async function getEvolutionJournal(date?: string): Promise<{ date: string; entries: JournalEntry[] }> {
  const q = date ? `?date=${date}` : "";
  return api(`/evolution/journal${q}`);
}

// ÔöÇÔöÇ Memory Browsing ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface MemoryBrowseResult {
  memories: (Memory & { id: string })[];
  total: number;
  types: string[];
}

export interface Episode {
  timestamp: number;
  datetime: string;
  event_type: string;
  description: string;
  details: Record<string, unknown>;
}

export async function browseMemory(type?: string, limit?: number): Promise<MemoryBrowseResult> {
  const params = new URLSearchParams();
  if (type) params.set("memory_type", type);
  if (limit) params.set("limit", String(limit));
  return api(`/memory/browse?${params}`);
}

export async function deleteMemory(id: string): Promise<{ status: string }> {
  return api(`/memory/${id}`, { method: "DELETE" });
}

export async function getEpisodes(date?: string): Promise<{ date: string; episodes: Episode[] }> {
  const q = date ? `?date=${date}` : "";
  return api(`/memory/episodes${q}`);
}

// ÔöÇÔöÇ Goal Management ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function updateGoal(id: string, action: string, note?: string): Promise<Goal> {
  const params = new URLSearchParams({ action });
  if (note) params.set("note", note);
  return api(`/goals/${id}?${params}`, { method: "PUT" });
}

// ÔöÇÔöÇ Evolution Timeline ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface EvolutionTimelineDay {
  date: string;
  successes: number;
  failures: number;
  lines_added: number;
  modules_touched: number;
  rate: number;
}

export async function getEvolutionTimeline(days = 7): Promise<{ timeline: EvolutionTimelineDay[] }> {
  return api(`/evolution/timeline?days=${days}`);
}

// ÔöÇÔöÇ Streaming Chat ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export function streamChat(
  message: string,
  onToken: (token: string) => void,
  onDone: (data: { intent: string; latency_ms: number }) => void,
  onError: (err: string) => void,
): AbortController {
  const controller = new AbortController();
  fetch(`${API_BASE}/chat/stream`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ message }),
    signal: controller.signal,
  })
    .then((res) => {
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const reader = res.body?.getReader();
      if (!reader) throw new Error("No reader");
      const decoder = new TextDecoder();
      let buffer = "";
      const read = (): Promise<void> =>
        reader.read().then(({ done, value }) => {
          if (done) return;
          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split("\n");
          buffer = lines.pop() || "";
          for (const line of lines) {
            if (!line.startsWith("data: ")) continue;
            try {
              const payload = JSON.parse(line.slice(6));
              if (payload.done) onDone({ intent: payload.intent, latency_ms: payload.latency_ms });
              else if (payload.token) onToken(payload.token);
              else if (payload.error) onError(payload.error);
            } catch {}
          }
          return read();
        });
      return read();
    })
    .catch((err) => {
      if (err.name !== "AbortError") onError(String(err));
    });
  return controller;
}

// ÔöÇÔöÇ Self-Model & Reflection ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface SelfModel {
  identity: Record<string, unknown>;
  capabilities: Record<string, unknown>[];
  limitations: string[];
  emotional_state: Record<string, unknown>;
  growth_trajectory: Record<string, unknown>;
  [key: string]: unknown;
}

export interface Reflection {
  timestamp: number;
  insights: string[];
  mood: string;
  [key: string]: unknown;
}

export interface GuardianStats {
  approved: number;
  blocked: number;
  total_reviews: number;
  recent_decisions: { action: string; result: string; reason: string; timestamp: number }[];
  [key: string]: unknown;
}

export async function getSelfModel(): Promise<SelfModel> {
  return api("/self-model");
}

export async function getReflection(): Promise<Reflection> {
  return api("/reflection/latest");
}

export async function getGuardianStats(): Promise<GuardianStats> {
  return api("/guardian/stats");
}

export async function getAnalyticsMetrics(): Promise<Record<string, unknown>> {
  return api("/analytics/metrics");
}

// ÔöÇÔöÇ Health & Diagnostics ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export interface DeepHealth {
  status: string;
  checks: Record<string, { status: string; latency_ms?: number; details?: string }>;
  [key: string]: unknown;
}

export async function getDeepHealth(): Promise<DeepHealth> {
  return api("/health/deep");
}

export async function getCircuitBreakers(): Promise<Record<string, unknown>> {
  return api("/health/circuits");
}

export async function getCacheStats(): Promise<Record<string, unknown>> {
  return api("/health/caches");
}

export async function getCacheStatsDetailed(): Promise<Record<string, unknown>> {
  return api("/cache/stats");
}

export async function flushCache(): Promise<{ status: string }> {
  return api("/cache/flush", { method: "DELETE" });
}

// ÔöÇÔöÇ Voice ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getVoiceStatus(): Promise<Record<string, unknown>> {
  return api("/voice/status");
}

export async function voiceSpeak(text: string): Promise<Record<string, unknown>> {
  return api(`/voice/speak?text=${encodeURIComponent(text)}`, { method: "POST" });
}

// ÔöÇÔöÇ Multimodal ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getMultimodalStats(): Promise<Record<string, unknown>> {
  return api("/multimodal/stats");
}

// ÔöÇÔöÇ Fine-tuning ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getFinetuneStats(): Promise<Record<string, unknown>> {
  return api("/finetune/stats");
}

export async function getFinetuneDatasets(): Promise<Record<string, unknown>> {
  return api("/finetune/datasets");
}

// ÔöÇÔöÇ Federated Learning ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getFederatedStats(): Promise<Record<string, unknown>> {
  return api("/federated/stats");
}

export async function getFederatedKnowledge(): Promise<Record<string, unknown>> {
  return api("/federated/knowledge");
}

// ÔöÇÔöÇ Evolution Controls ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function pauseEvolution(): Promise<{ status: string }> {
  return api("/evolution/pause", { method: "POST" });
}

export async function resumeEvolution(): Promise<{ status: string }> {
  return api("/evolution/resume", { method: "POST" });
}

export async function rollbackEvolution(): Promise<{ status: string }> {
  return api("/evolution/rollback", { method: "POST" });
}

// ÔöÇÔöÇ Goals Suggest ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getGoalSuggestions(): Promise<{ suggestions: string[] }> {
  return api("/goals/suggest");
}

// ÔöÇÔöÇ Module Stats ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getModuleStats(): Promise<Record<string, unknown>> {
  return api("/modules/stats");
}

// ÔöÇÔöÇ Server Settings ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getServerSettings(): Promise<Record<string, unknown>> {
  return api("/settings");
}

// ÔöÇÔöÇ Memory Store ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function storeMemory(text: string, memoryType = "manual"): Promise<Record<string, unknown>> {
  return api(`/memory/store?text=${encodeURIComponent(text)}&memory_type=${memoryType}`, { method: "POST" });
}

// ÔöÇÔöÇ Memory Sessions ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getMemorySessions(): Promise<Record<string, unknown>> {
  return api("/memory/sessions");
}

// ÔöÇÔöÇ Logs ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getLogsTail(lines = 50): Promise<Record<string, unknown>> {
  return api(`/logs/tail?lines=${lines}`);
}

// ÔöÇÔöÇ Infra Status ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export async function getInfraStatus(): Promise<Record<string, unknown>> {
  return api("/infra/status");
}

// ÔöÇÔöÇ WebSocket ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

export function connectWebSocket(
  onMessage: (data: unknown) => void,
  onClose?: () => void,
): WebSocket {
  const ws = new WebSocket("ws://localhost:8002/ws/stream");
  ws.onmessage = (event) => {
    try {
      onMessage(JSON.parse(event.data));
    } catch {
      onMessage(event.data);
    }
  };
  ws.onclose = () => onClose?.();
  return ws;
}

// --- Stub types and functions for modules that reference them ---

export type SystemEvent = {
  id?: string;
  type: string;
  message: string;
  timestamp?: number | string;
  level?: string;
};

export async function getFitnessTrend(_days?: number): Promise<{ trend: any[]; latest: any | null }> {
  return { trend: [], latest: null };
}

export async function runFitnessBenchmark(): Promise<any> {
  return { score: 0, details: [] };
}

export type FitnessBenchmark = {
  score: number;
  fitness_score: number;
  label: string;
  timestamp: number;
  details?: any[];
  avg_latency_ms?: number;
  categories?: any[];
  passed_tests?: number;
  total_tests?: number;
};
