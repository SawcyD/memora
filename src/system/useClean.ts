import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CleanMethod, CleanProgress, CleanReport } from "./types";

export type CleanPhase = "idle" | "running" | "done";

export interface CleanState {
  phase: CleanPhase;
  progress: CleanProgress | null;
  report: CleanReport | null;
  /** Available-memory delta re-measured 30s after the run, or null until then. */
  settled: number | null;
  error: string | null;
  elevated: boolean;
  start: (methods: CleanMethod[], excluded?: number[]) => void;
  cancel: () => void;
  dismiss: () => void;
}

/**
 * Drives an optimization run. All the work happens on a Rust worker thread;
 * this hook only reflects the events it emits.
 */
export function useClean(): CleanState {
  const [phase, setPhase] = useState<CleanPhase>("idle");
  const [progress, setProgress] = useState<CleanProgress | null>(null);
  const [report, setReport] = useState<CleanReport | null>(null);
  const [settled, setSettled] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [elevated, setElevated] = useState(false);

  // Guards against a late `settled` event from a previous run overwriting the
  // current one's results.
  const runId = useRef(0);

  useEffect(() => {
    invoke<boolean>("is_elevated")
      .then(setElevated)
      .catch(() => setElevated(false));
  }, []);

  useEffect(() => {
    const subs = [
      listen<CleanProgress>("clean://progress", (e) => setProgress(e.payload)),
      listen<CleanReport>("clean://done", (e) => {
        setReport(e.payload);
        setPhase("done");
      }),
      listen<string>("clean://failed", (e) => {
        setError(e.payload);
        setPhase("idle");
      }),
      listen<number>("clean://settled", (e) => {
        const id = runId.current;
        setSettled((prev) => (id === runId.current ? e.payload : prev));
      }),
    ].map((p) => p.catch(() => () => {}));

    return () => {
      subs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  const start = useCallback((methods: CleanMethod[], excluded: number[] = []) => {
    runId.current += 1;
    setPhase("running");
    setProgress(null);
    setReport(null);
    setSettled(null);
    setError(null);

    invoke("start_optimization", { methods, excluded }).catch((e) => {
      setError(String(e));
      setPhase("idle");
    });
  }, []);

  const cancel = useCallback(() => {
    invoke("cancel_optimization").catch(() => {});
  }, []);

  const dismiss = useCallback(() => {
    setPhase("idle");
    setReport(null);
    setSettled(null);
  }, []);

  return { phase, progress, report, settled, error, elevated, start, cancel, dismiss };
}
