import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { MemorySnapshot } from "./types";

/** One hour of 1 Hz samples — the longest range the Memory page offers. */
const MAX_SAMPLES = 3600;

export interface MemoryState {
  current: MemorySnapshot | null;
  history: MemorySnapshot[];
  error: string | null;
}

/**
 * Subscribes to the Rust sampler. The backend owns the sampling cadence so the
 * graph, the readouts and (later) the tray meter all share one series.
 */
export function useMemory(): MemoryState {
  const [current, setCurrent] = useState<MemorySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  // History lives in a ref and is mirrored into state so appending a sample
  // does not reallocate a 3600-element array on every tick.
  const historyRef = useRef<MemorySnapshot[]>([]);
  const [history, setHistory] = useState<MemorySnapshot[]>([]);

  useEffect(() => {
    let disposed = false;

    const push = (snap: MemorySnapshot) => {
      if (disposed) return;
      const next = historyRef.current;
      next.push(snap);
      if (next.length > MAX_SAMPLES) next.splice(0, next.length - MAX_SAMPLES);
      setCurrent(snap);
      setHistory(next.slice());
    };

    invoke<MemorySnapshot>("memory_snapshot")
      .then(push)
      .catch((e) => !disposed && setError(String(e)));

    const unlisten = listen<MemorySnapshot>("memory://sample", (e) => push(e.payload)).catch(
      () => () => {},
    );

    return () => {
      disposed = true;
      unlisten.then((f) => f());
    };
  }, []);

  return { current, history, error };
}
