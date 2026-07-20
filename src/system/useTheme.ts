import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Accent } from "./types";

/**
 * Follows the Windows appearance setting and pushes the system accent color
 * into CSS custom properties. Memora has no theme of its own.
 */
export function useSystemTheme(): void {
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () =>
      document.documentElement.setAttribute("data-theme", media.matches ? "dark" : "light");

    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, []);

  useEffect(() => {
    let disposed = false;
    invoke<Accent>("system_accent")
      .then((a) => {
        if (disposed) return;
        const root = document.documentElement;
        root.style.setProperty("--accent", a.accent);
        root.style.setProperty("--accent-light1", a.accentLight1);
        root.style.setProperty("--accent-light2", a.accentLight2);
        root.style.setProperty("--accent-dark1", a.accentDark1);
      })
      // A missing accent is not worth surfacing; the token default is the
      // Windows default accent anyway.
      .catch(() => {});
    return () => {
      disposed = true;
    };
  }, []);
}
