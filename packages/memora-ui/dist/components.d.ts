import { type ButtonHTMLAttributes, type HTMLAttributes, type Key, type ReactElement, type ReactNode, type SelectHTMLAttributes } from "react";
export type ButtonVariant = "standard" | "primary" | "subtle" | "danger";
export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
    variant?: ButtonVariant;
    /** Compatibility shortcut for Memora's original Button API. */
    accent?: boolean;
    /** Removes the press-scale treatment for controls where movement is undesirable. */
    static?: boolean;
}
export declare function Button({ children, variant, accent, static: isStatic, className, type, ...rest }: ButtonProps): import("react").JSX.Element;
export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
    label: string;
    variant?: ButtonVariant;
    static?: boolean;
}
export declare function IconButton({ label, variant, static: isStatic, className, type, children, ...rest }: IconButtonProps): import("react").JSX.Element;
export declare function SectionHeader({ children, className, ...rest }: HTMLAttributes<HTMLHeadingElement>): import("react").JSX.Element;
export interface InfoRowProps extends HTMLAttributes<HTMLDivElement> {
    label: ReactNode;
    value: ReactNode;
    help?: string;
}
export declare function InfoRow({ label, value, help, className, ...rest }: InfoRowProps): import("react").JSX.Element;
export interface ToggleSwitchProps {
    checked: boolean;
    onChange: (checked: boolean) => void;
    disabled?: boolean;
    label: string;
    className?: string;
}
export declare function ToggleSwitch({ checked, onChange, disabled, label, className }: ToggleSwitchProps): import("react").JSX.Element;
export interface ComboBoxOption<T extends string | number> {
    value: T;
    label: string;
    disabled?: boolean;
}
export interface ComboBoxProps<T extends string | number> extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "value" | "onChange"> {
    value: T;
    options: ComboBoxOption<T>[];
    onChange: (value: T) => void;
    label: string;
}
export declare function ComboBox<T extends string | number>({ value, options, onChange, label, className, ...rest }: ComboBoxProps<T>): import("react").JSX.Element;
export interface NumberBoxProps {
    value: number;
    min: number;
    max: number;
    onChange: (value: number) => void;
    label: string;
    suffix?: string;
    disabled?: boolean;
    className?: string;
}
export declare function NumberBox({ value, min, max, onChange, label, suffix, disabled, className }: NumberBoxProps): import("react").JSX.Element;
export interface SettingsRowProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
    title: ReactNode;
    description?: ReactNode;
    note?: ReactNode;
    control?: ReactNode;
}
export declare function SettingsRow({ title, description, note, control, className, ...rest }: SettingsRowProps): import("react").JSX.Element;
export declare function SettingsSection({ children, className, ...rest }: HTMLAttributes<HTMLDivElement>): import("react").JSX.Element;
export interface ProgressBarProps extends HTMLAttributes<HTMLDivElement> {
    value?: number;
    max?: number;
    label?: string;
    indeterminate?: boolean;
}
export declare function ProgressBar({ value, max, label, indeterminate, className, ...rest }: ProgressBarProps): import("react").JSX.Element;
export type InfoBarTone = "info" | "success" | "warning" | "error";
export interface InfoBarProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
    title: ReactNode;
    message?: ReactNode;
    tone?: InfoBarTone;
    action?: ReactNode;
    onDismiss?: () => void;
}
export declare function InfoBar({ title, message, tone, action, onDismiss, className, ...rest }: InfoBarProps): import("react").JSX.Element;
export interface SearchBoxProps extends Omit<HTMLAttributes<HTMLDivElement>, "onChange"> {
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
    label: string;
}
export declare function SearchBox({ value, onChange, placeholder, label, className, ...rest }: SearchBoxProps): import("react").JSX.Element;
export declare function CommandBar({ children, className, ...rest }: HTMLAttributes<HTMLDivElement>): import("react").JSX.Element;
export interface TooltipProps {
    content: ReactNode;
    children: ReactElement;
    placement?: "top" | "right" | "bottom" | "left";
}
export declare function Tooltip({ content, children, placement }: TooltipProps): import("react").JSX.Element;
export interface ContentDialogProps {
    open?: boolean;
    title: string;
    children: ReactNode;
    primaryText?: string;
    cancelText?: string;
    onPrimary?: () => void;
    onCancel: () => void;
    destructive?: boolean;
}
export declare function ContentDialog({ open, title, children, primaryText, cancelText, onPrimary, onCancel, destructive, }: ContentDialogProps): import("react").ReactPortal | null;
export interface MenuAction {
    id: string;
    label: ReactNode;
    icon?: ReactNode;
    danger?: boolean;
    disabled?: boolean;
    dividerBefore?: boolean;
}
export interface ContextMenuProps {
    x: number;
    y: number;
    actions: MenuAction[];
    onSelect: (id: string) => void;
    onDismiss: () => void;
}
export declare function ContextMenu({ x, y, actions, onSelect, onDismiss }: ContextMenuProps): import("react").ReactPortal | null;
export interface TeachingTipProps extends Omit<HTMLAttributes<HTMLElement>, "title"> {
    title: ReactNode;
    onDismiss?: () => void;
    action?: ReactNode;
}
export declare function TeachingTip({ title, children, onDismiss, action, className, ...rest }: TeachingTipProps): import("react").JSX.Element;
export interface NavigationItem {
    id: string;
    label: string;
    icon?: ReactNode;
    disabled?: boolean;
}
export interface NavigationViewProps extends Omit<HTMLAttributes<HTMLElement>, "onSelect"> {
    items: NavigationItem[];
    footerItems?: NavigationItem[];
    selectedId: string;
    onSelect: (id: string) => void;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
    ariaLabel?: string;
}
export declare function NavigationView({ items, footerItems, selectedId, onSelect, collapsed, onToggleCollapse, ariaLabel, className, ...rest }: NavigationViewProps): import("react").JSX.Element;
export type SortDirection = "ascending" | "descending";
export interface DataGridSort {
    columnId: string;
    direction: SortDirection;
}
export interface DataGridColumn<T> {
    id: string;
    header: ReactNode;
    render: (row: T) => ReactNode;
    sortable?: boolean;
    width?: string | number;
    align?: "start" | "center" | "end";
}
export interface DataGridProps<T> {
    rows: T[];
    columns: DataGridColumn<T>[];
    rowKey: (row: T) => Key;
    ariaLabel: string;
    sort?: DataGridSort;
    onSortChange?: (sort: DataGridSort) => void;
    selectedKeys?: Key[];
    onSelectionChange?: (keys: Key[]) => void;
    onRowContextMenu?: (row: T, event: React.MouseEvent) => void;
    emptyMessage?: ReactNode;
    className?: string;
}
export declare function DataGrid<T>({ rows, columns, rowKey, ariaLabel, sort, onSortChange, selectedKeys, onSelectionChange, onRowContextMenu, emptyMessage, className }: DataGridProps<T>): import("react").JSX.Element;
/** Visually groups toolbar children while preserving their own semantics. */
export declare function CommandGroup({ children, className, ...rest }: HTMLAttributes<HTMLDivElement>): import("react").JSX.Element;
//# sourceMappingURL=components.d.ts.map