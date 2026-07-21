import { createContext, useContext, useState, type CSSProperties, type HTMLAttributes, type ReactNode } from "react";

export type MemoraTheme = "system" | "light" | "dark";
export type MemoraDensity = "compact" | "comfortable";

export interface FluentProviderProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  theme?: MemoraTheme;
  density?: MemoraDensity;
  /** Any valid CSS color, including a Windows system accent supplied at runtime. */
  accentColor?: string;
}

const PortalTargetContext = createContext<HTMLElement | null>(null);

/** Used by transient surfaces so they keep the provider's theme variables. */
export function useFluentPortalTarget() {
  return useContext(PortalTargetContext);
}

export function FluentProvider({
  children,
  theme = "system",
  density = "compact",
  accentColor,
  className,
  style,
  ...rest
}: FluentProviderProps) {
  const [portalTarget, setPortalTarget] = useState<HTMLDivElement | null>(null);
  const customStyle = {
    ...style,
    ...(accentColor ? { "--memora-accent": accentColor } : {}),
  } as CSSProperties;

  return (
    <PortalTargetContext.Provider value={portalTarget}>
      <div
        {...rest}
        ref={setPortalTarget}
        className={["memora-ui-root", className].filter(Boolean).join(" ")}
        data-memora-theme={theme}
        data-memora-density={density}
        style={customStyle}
      >
        {children}
      </div>
    </PortalTargetContext.Provider>
  );
}
