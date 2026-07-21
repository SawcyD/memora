import { type HTMLAttributes, type ReactNode } from "react";
export type MemoraTheme = "system" | "light" | "dark";
export type MemoraDensity = "compact" | "comfortable";
export interface FluentProviderProps extends HTMLAttributes<HTMLDivElement> {
    children: ReactNode;
    theme?: MemoraTheme;
    density?: MemoraDensity;
    /** Any valid CSS color, including a Windows system accent supplied at runtime. */
    accentColor?: string;
}
/** Used by transient surfaces so they keep the provider's theme variables. */
export declare function useFluentPortalTarget(): HTMLElement | null;
export declare function FluentProvider({ children, theme, density, accentColor, className, style, ...rest }: FluentProviderProps): import("react").JSX.Element;
//# sourceMappingURL=theme.d.ts.map