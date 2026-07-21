import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

export interface SettingsState {
  settings: Settings | null;
  error: string | null;
  /** Applies a partial change; the backend sanitizes and returns the result. */
  update: (patch: Partial<Settings>) => Promise<void>;
}

export function useSettings(): SettingsState {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Settings>("get_settings")
      .then(setSettings)
      .catch((e) => setError(String(e)));
  }, []);

  const update = useCallback(
    async (patch: Partial<Settings>) => {
      if (!settings) return;
      const next = { ...settings, ...patch };

      // Show the change immediately, then reconcile with what the backend
      // actually stored — it clamps values and can reject a registry write.
      setSettings(next);
      try {
        const saved = await invoke<Settings>("update_settings", { settings: next });
        setSettings(saved);
        setError(null);
      } catch (e) {
        setSettings(settings); // roll back to the last known-good state
        setError(String(e));
      }
    },
    [settings],
  );

  return { settings, error, update };
}
