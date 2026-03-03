"use client";

// CRITICAL: These imports MUST use @/ prefix for Next.js path alias resolution.
// DO NOT remove the @/ prefix — it will break the build.
import { useEventStream } from "@/lib/useEventStream";
import type { SystemEvent } from "@/lib/api";
import { useEffect, useState, useRef, useCallback } from "react";

/**
 * EventFeed — real-time SSE event stream display.
 * Shows live system events pushed from the backend event bus.
 * Replaces polling-based updates with instant server-push.
 *
 * Features:
 * - Auto-refresh with configurable interval (default 5s)
 * - Last updated timestamp display with staleness coloring
 * - Pulsing dot indicator when auto-refresh is active
 * - Friendly empty state message with pulse animation
 * - Error banner with manual retry button
 * - Badge showing count of new events since last view
 * - "Last updated: Xs ago" with orange (>30s) and red (>60s) staleness indicators
 */

const EVENT_COLORS: Record<string, string> = {
  "evolution.success": "text-green-400",
  "evolution.failure": "text-red-400",
  "evolution.rollback": "text-yellow-400",
  "evolution.start": "text-blue-400",
  "goal.created": "text-purple-400",
  "goal.completed": "text-green-400",
  "memory.stored": "text-cyan-400",
  "fitness.benchmark": "text-orange-400",
  "system.status": "text-gray-400",
  "self.success": "text-green-300",
  "self.failure": "text-red-300",
  connected: "text-gray-500",
};

const EVENT_ICONS: Record<string, string> = {
  "evolution.success": "✅",
  "evolution.failure": "❌",
  "evolution.rollback": "↩️",
  "evolution.start": "🔄",
  "goal.created": "🎯",
  "goal.completed": "🏆",
  "memory.stored": "💾",
  "fitness.benchmark": "🧬",
  "system.status": "📡",
  "self.success": "💪",
  "self.failure": "⚠️",
  connected: "🔌",
};

const DEFAULT_REFRESH_INTERVAL = 5000;
const STALE_WARNING_THRESHOLD = 30;
const STALE_ERROR_THRESHOLD = 60;

function formatTime(ts: number | string): string {
  const n = typeof ts === "string" ? parseFloat(ts) || Date.now() / 1000 : ts;
  return new Date(n * 1000).toLocaleTimeString();
}

function formatSecondsAgo(seconds: number): string {
  if (seconds < 1) return "just now";
  if (seconds < 60) return `${Math.floor(seconds)}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${Math.floor(seconds % 60)}s ago`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m ago`;
}

function getStalenessColor(seconds: number): string {
  if (seconds >= STALE_ERROR_THRESHOLD) return "text-red-400";
  if (seconds >= STALE_WARNING_THRESHOLD) return "text-orange-400";
  return "text-gray-600";
}

function EventItem({ event, isNew }: { event: SystemEvent; isNew: boolean }) {
  const color = EVENT_COLORS[event.type] || "text-gray-400";
  const icon = EVENT_ICONS[event.type] || "📌";

  return (
    <div
      className={`flex items-start gap-2 py-1 text-xs transition-colors duration-700 ${
        isNew ? "bg-gray-700/40 rounded px-1 -mx-1" : ""
      }`}
    >
      <span>{icon}</span>
      <span className="text-gray-500 w-16 shrink-0">
        {formatTime(event.timestamp)}
      </span>
      <span className={`font-mono ${color}`}>{event.type}</span>
      {isNew && (
        <span className="text-[10px] bg-blue-500/20 text-blue-400 px-1 rounded shrink-0">
          NEW
        </span>
      )}
      <span className="text-gray-500 truncate">
        {Object.entries(event)
          .filter(([k]) => !["type", "timestamp", "source"].includes(k))
          .map(
            ([k, v]) =>
              `${k}=${typeof v === "object" ? JSON.stringify(v) : v}`
          )
          .join(" ")
          .slice(0, 100)}
      </span>
    </div>
  );
}

function PulsingDot({ active }: { active: boolean }) {
  if (!active) return null;
  return (
    <span className="relative flex h-2 w-2">
      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
      <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
    </span>
  );
}

function ErrorBanner({
  error,
  onRetry,
}: {
  error: string;
  onRetry: () => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2 bg-red-900/30 border border-red-700/50 rounded-lg px-3 py-2 mb-3">
      <div className="flex items-center gap-2 text-xs text-red-400">
        <span>⚠️</span>
        <span>{error}</span>
      </div>
      <button
        onClick={onRetry}
        className="text-xs px-2 py-1 rounded bg-red-800/50 text-red-300 hover:bg-red-700/50 transition-colors shrink-0 cursor-pointer"
      >
        Retry
      </button>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-12 gap-3">
      <span className="text-3xl animate-pulse">🌅</span>
      <div className="flex flex-col items-center gap-1">
        <p className="text-sm text-gray-400 font-medium animate-pulse">
          No events yet
        </p>
        <p className="text-xs text-gray-500 italic text-center">
          Sentient is warming up…
        </p>
        <p className="text-[10px] text-gray-600 text-center mt-1">
          Events will appear here in real-time once the system starts processing.
        </p>
      </div>
    </div>
  );
}

function NewEventsBadge({
  count,
  onClick,
}: {
  count: number;
  onClick: () => void;
}) {
  if (count <= 0) return null;
  return (
    <button
      onClick={onClick}
      className="text-[10px] px-2 py-0.5 rounded-full bg-blue-600/40 text-blue-300 hover:bg-blue-600/60 transition-colors cursor-pointer animate-pulse"
    >
      {count} new event{count !== 1 ? "s" : ""}
    </button>
  );
}

function LastUpdatedDisplay({ lastUpdated }: { lastUpdated: Date | null }) {
  const [secondsAgo, setSecondsAgo] = useState<number>(0);

  useEffect(() => {
    if (!lastUpdated) return;

    const updateSeconds = () => {
      const now = Date.now();
      const diff = (now - lastUpdated.getTime()) / 1000;
      setSecondsAgo(diff);
    };

    updateSeconds();
    const interval = setInterval(updateSeconds, 1000);
    return () => clearInterval(interval);
  }, [lastUpdated]);

  if (!lastUpdated) {
    return (
      <span className="text-[10px] text-gray-600">
        Last updated: Never
      </span>
    );
  }

  const stalenessColor = getStalenessColor(secondsAgo);
  const isStale = secondsAgo >= STALE_WARNING_THRESHOLD;

  return (
    <span className={`text-[10px] ${stalenessColor} transition-colors duration-300`}>
      Last updated: {formatSecondsAgo(secondsAgo)}
      {isStale && (
        <span className="ml-1 inline-block">
          {secondsAgo >= STALE_ERROR_THRESHOLD ? "🔴" : "🟠"}
        </span>
      )}
    </span>
  );
}

export default function EventFeed() {
  const { events, connected } = useEventStream({ maxEvents: 50 });

  const [autoRefreshActive, setAutoRefreshActive] = useState(true);
  const [refreshInterval, setRefreshInterval] = useState(DEFAULT_REFRESH_INTERVAL);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [newEventCount, setNewEventCount] = useState(0);
  const [lastViewedCount, setLastViewedCount] = useState(0);
  const [isUserScrolled, setIsUserScrolled] = useState(false);

  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const previousEventCountRef = useRef(0);

  // Track new events since last view
  useEffect(() => {
    const currentCount = events.length;
    if (currentCount > previousEventCountRef.current) {
      const diff = currentCount - previousEventCountRef.current;
      if (isUserScrolled || document.hidden) {
        setNewEventCount((prev) => prev + diff);
      }
      setLastUpdated(new Date());
    }
    previousEventCountRef.current = currentCount;
  }, [events.length, isUserScrolled]);

  // Auto-refresh heartbeat — updates the lastUpdated timestamp periodically
  // and can be used to trigger reconnection if SSE drops
  useEffect(() => {
    if (!autoRefreshActive) return;

    const interval = setInterval(() => {
      try {
        // Heartbeat: if we're connected, just update the timestamp
        if (connected) {
          setLastUpdated(new Date());
          setError(null);
        } else {
          // If disconnected, flag an error for the user
          setError("SSE connection lost. Attempting to reconnect…");
        }
      } catch (err) {
        setError(
          err instanceof Error
            ? err.message
            : "An unexpected error occurred during refresh."
        );
      }
    }, refreshInterval);

    return () => clearInterval(interval);
  }, [autoRefreshActive, refreshInterval, connected]);

  // Track scroll position to determine if user has scrolled up
  const handleScroll = useCallback(() => {
    const container = scrollContainerRef.current;
    if (!container) return;
    const { scrollTop } = container;
    // If scrolled to top (since we reverse the list, top = newest)
    setIsUserScrolled(scrollTop > 20);
  }, []);

  const handleMarkAsViewed = useCallback(() => {
    setNewEventCount(0);
    setLastViewedCount(events.length);
    // Scroll to top (newest events)
    scrollContainerRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  }, [events.length]);

  const handleRetry = useCallback(() => {
    setError(null);
    // Force a re-render / reconnection attempt by toggling auto-refresh
    setAutoRefreshActive(false);
    setTimeout(() => setAutoRefreshActive(true), 100);
  }, []);

  const toggleAutoRefresh = useCallback(() => {
    setAutoRefreshActive((prev) => !prev);
  }, []);

  const reversedEvents = [...events].reverse();

  // Determine which events are "new" (appeared after last viewed count)
  const newEventThreshold = events.length - lastViewedCount;

  return (
    <div className="bg-gray-800/60 rounded-xl p-4 border border-gray-700">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-semibold text-gray-300 uppercase tracking-wide">
            ⚡ Live Events
          </h3>
          <NewEventsBadge count={newEventCount} onClick={handleMarkAsViewed} />
        </div>
        <div className="flex items-center gap-2">
          {/* Auto-refresh indicator */}
          <button
            onClick={toggleAutoRefresh}
            className="flex items-center gap-1.5 text-[10px] text-gray-500 hover:text-gray-400 transition-colors cursor-pointer"
            title={
              autoRefreshActive
                ? `Auto-refresh every ${refreshInterval / 1000}s — click to pause`
                : "Auto-refresh paused — click to resume"
            }
          >
            <PulsingDot active={autoRefreshActive && connected} />
            <span>{autoRefreshActive ? "Auto" : "Paused"}</span>
          </button>

          {/* Connection status badge */}
          <span
            className={`text-xs px-2 py-0.5 rounded-full ${
              connected
                ? "bg-green-900/50 text-green-400"
                : "bg-red-900/50 text-red-400"
            }`}
          >
            {connected ? "SSE Connected" : "Disconnected"}
          </span>
        </div>
      </div>

      {/* Last updated timestamp with live seconds-ago counter */}
      <div className="flex items-center justify-between mb-2">
        <LastUpdatedDisplay lastUpdated={lastUpdated} />
        {events.length > 0 && (
          <span className="text-[10px] text-gray-600">
            {events.length} event{events.length !== 1 ? "s" : ""}
          </span>
        )}
      </div>

      {/* Error banner */}
      {error && <ErrorBanner error={error} onRetry={handleRetry} />}

      {/* Event list */}
      <div
        ref={scrollContainerRef}
        onScroll={handleScroll}
        className="max-h-64 overflow-y-auto space-y-0 scrollbar-thin scrollbar-thumb-gray-700"
      >
        {events.length === 0 ? (
          <EmptyState />
        ) : (
          reversedEvents.map((event, i) => (
            <EventItem
              key={`${event.timestamp}-${event.type}-${i}`}
              event={event}
              isNew={i < newEventThreshold && newEventCount > 0}
            />
          ))
        )}
      </div>

      {/* Refresh interval selector */}
      <div className="flex items-center justify-end mt-2 gap-1">
        <span className="text-[10px] text-gray-600">Refresh:</span>
        {[3000, 5000, 10000, 30000].map((interval) => (
          <button
            key={interval}
            onClick={() => setRefreshInterval(interval)}
            className={`text-[10px] px-1.5 py-0.5 rounded cursor-pointer transition-colors ${
              refreshInterval === interval
                ? "bg-gray-600/50 text-gray-300"
                : "text-gray-600 hover:text-gray-400"
            }`}
          >
            {interval / 1000}s
          </button>
        ))}
      </div>
    </div>
  );
}