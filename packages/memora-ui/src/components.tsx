import {
  Children,
  cloneElement,
  isValidElement,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type HTMLAttributes,
  type Key,
  type ReactElement,
  type ReactNode,
  type SelectHTMLAttributes,
} from "react";
import { createPortal } from "react-dom";
import { useFluentPortalTarget } from "./theme";

function cx(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(" ");
}

export type ButtonVariant = "standard" | "primary" | "subtle" | "danger";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  /** Compatibility shortcut for Memora's original Button API. */
  accent?: boolean;
  /** Removes the press-scale treatment for controls where movement is undesirable. */
  static?: boolean;
}

export function Button({
  children,
  variant = "standard",
  accent = false,
  static: isStatic = false,
  className,
  type = "button",
  ...rest
}: ButtonProps) {
  const resolvedVariant = accent ? "primary" : variant;
  return (
    <button
      {...rest}
      type={type}
      className={cx(
        "memora-button",
        `memora-button--${resolvedVariant}`,
        isStatic && "memora-static",
        className,
      )}
    >
      {children}
    </button>
  );
}

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  variant?: ButtonVariant;
  static?: boolean;
}

export function IconButton({
  label,
  variant = "subtle",
  static: isStatic = false,
  className,
  type = "button",
  children,
  ...rest
}: IconButtonProps) {
  return (
    <button
      {...rest}
      type={type}
      aria-label={label}
      className={cx(
        "memora-icon-button",
        `memora-button--${variant}`,
        isStatic && "memora-static",
        className,
      )}
    >
      {children}
    </button>
  );
}

export function SectionHeader({ children, className, ...rest }: HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h2 {...rest} className={cx("memora-section-header", className)}>
      {children}
    </h2>
  );
}

export interface InfoRowProps extends HTMLAttributes<HTMLDivElement> {
  label: ReactNode;
  value: ReactNode;
  help?: string;
}

export function InfoRow({ label, value, help, className, ...rest }: InfoRowProps) {
  return (
    <div {...rest} className={cx("memora-info-row", className)}>
      <span className={cx("memora-info-row__label", help && "memora-help-text")} title={help}>
        {label}
      </span>
      <span className="memora-info-row__value memora-tabular">{value}</span>
    </div>
  );
}

export interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  label: string;
  className?: string;
}

export function ToggleSwitch({ checked, onChange, disabled = false, label, className }: ToggleSwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cx("memora-toggle", checked && "memora-toggle--checked", className)}
    >
      <span className="memora-toggle__track" aria-hidden="true">
        <span className="memora-toggle__thumb" />
      </span>
    </button>
  );
}

export interface ComboBoxOption<T extends string | number> {
  value: T;
  label: string;
  disabled?: boolean;
}

export interface ComboBoxProps<T extends string | number>
  extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "value" | "onChange"> {
  value: T;
  options: ComboBoxOption<T>[];
  onChange: (value: T) => void;
  label: string;
}

export function ComboBox<T extends string | number>({
  value,
  options,
  onChange,
  label,
  className,
  ...rest
}: ComboBoxProps<T>) {
  return (
    <select
      {...rest}
      aria-label={label}
      value={String(value)}
      onChange={(event) => {
        const picked = options.find((option) => String(option.value) === event.target.value);
        if (picked) onChange(picked.value);
      }}
      className={cx("memora-combo-box", className)}
    >
      {options.map((option) => (
        <option key={String(option.value)} value={String(option.value)} disabled={option.disabled}>
          {option.label}
        </option>
      ))}
    </select>
  );
}

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

export function NumberBox({ value, min, max, onChange, label, suffix, disabled, className }: NumberBoxProps) {
  return (
    <div className={cx("memora-number-box", className)}>
      <input
        type="number"
        aria-label={label}
        value={value}
        min={min}
        max={max}
        disabled={disabled}
        onChange={(event) => {
          const next = Number(event.target.value);
          if (Number.isFinite(next)) onChange(Math.min(max, Math.max(min, Math.round(next))));
        }}
      />
      {suffix && <span>{suffix}</span>}
    </div>
  );
}

export interface SettingsRowProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  title: ReactNode;
  description?: ReactNode;
  note?: ReactNode;
  control?: ReactNode;
}

export function SettingsRow({ title, description, note, control, className, ...rest }: SettingsRowProps) {
  return (
    <div {...rest} className={cx("memora-settings-row", className)}>
      <div className="memora-settings-row__copy">
        <div className="memora-settings-row__title">{title}</div>
        {description && <div className="memora-settings-row__description">{description}</div>}
        {note && <div className="memora-settings-row__note">{note}</div>}
      </div>
      {control && <div className="memora-settings-row__control">{control}</div>}
    </div>
  );
}

export function SettingsSection({ children, className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <section {...rest} className={cx("memora-settings-section", className)}>
      {children}
    </section>
  );
}

export interface ProgressBarProps extends HTMLAttributes<HTMLDivElement> {
  value?: number;
  max?: number;
  label?: string;
  indeterminate?: boolean;
}

export function ProgressBar({ value = 0, max = 100, label, indeterminate = false, className, ...rest }: ProgressBarProps) {
  const percent = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0;
  return (
    <div
      {...rest}
      role="progressbar"
      aria-label={label}
      aria-valuenow={indeterminate ? undefined : value}
      aria-valuemin={indeterminate ? undefined : 0}
      aria-valuemax={indeterminate ? undefined : max}
      className={cx("memora-progress", indeterminate && "memora-progress--indeterminate", className)}
    >
      <span className="memora-progress__bar" style={indeterminate ? undefined : { width: `${percent}%` }} />
    </div>
  );
}

export type InfoBarTone = "info" | "success" | "warning" | "error";

export interface InfoBarProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  title: ReactNode;
  message?: ReactNode;
  tone?: InfoBarTone;
  action?: ReactNode;
  onDismiss?: () => void;
}

export function InfoBar({ title, message, tone = "info", action, onDismiss, className, ...rest }: InfoBarProps) {
  return (
    <div {...rest} role={tone === "error" ? "alert" : "status"} className={cx("memora-info-bar", `memora-info-bar--${tone}`, className)}>
      <span className="memora-info-bar__mark" aria-hidden="true">i</span>
      <div className="memora-info-bar__copy">
        <div className="memora-info-bar__title">{title}</div>
        {message && <div className="memora-info-bar__message">{message}</div>}
      </div>
      {action}
      {onDismiss && <IconButton label="Dismiss" onClick={onDismiss}>×</IconButton>}
    </div>
  );
}

export interface SearchBoxProps extends Omit<HTMLAttributes<HTMLDivElement>, "onChange"> {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  label: string;
}

export function SearchBox({ value, onChange, placeholder = "Search", label, className, ...rest }: SearchBoxProps) {
  return (
    <div {...rest} className={cx("memora-search-box", className)}>
      <svg viewBox="0 0 16 16" aria-hidden="true" className="memora-search-box__icon">
        <circle cx="7" cy="7" r="4.5" />
        <path d="m10.5 10.5 3 3" />
      </svg>
      <input
        type="search"
        aria-label={label}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}

export function CommandBar({ children, className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div {...rest} role="toolbar" className={cx("memora-command-bar", className)}>
      {children}
    </div>
  );
}

export interface TooltipProps {
  content: ReactNode;
  children: ReactElement;
  placement?: "top" | "right" | "bottom" | "left";
}

export function Tooltip({ content, children, placement = "top" }: TooltipProps) {
  const id = useId();
  const child = isValidElement(children)
    ? cloneElement(children as ReactElement<{ "aria-describedby"?: string }>, { "aria-describedby": id })
    : children;
  return (
    <span className="memora-tooltip-anchor">
      {child}
      <span id={id} role="tooltip" className={cx("memora-tooltip", `memora-tooltip--${placement}`)}>
        {content}
      </span>
    </span>
  );
}

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

export function ContentDialog({
  open = true,
  title,
  children,
  primaryText,
  cancelText = "Cancel",
  onPrimary,
  onCancel,
  destructive = false,
}: ContentDialogProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const providerTarget = useFluentPortalTarget();

  useEffect(() => {
    if (!open) return;
    const previousFocus = document.activeElement as HTMLElement | null;
    panelRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
      if (event.key !== "Tab") return;
      const focusable = panelRef.current?.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (!focusable?.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      previousFocus?.focus();
    };
  }, [onCancel, open]);

  if (!open || typeof document === "undefined") return null;
  return createPortal(
    <div className="memora-dialog-backdrop" onMouseDown={(event) => event.target === event.currentTarget && onCancel()}>
      <div ref={panelRef} role="dialog" aria-modal="true" aria-labelledby={titleId} tabIndex={-1} className="memora-dialog">
        <div className="memora-dialog__body">
          <h2 id={titleId}>{title}</h2>
          <div className="memora-dialog__content">{children}</div>
        </div>
        <div className="memora-dialog__actions">
          {primaryText && onPrimary && (
            <Button variant={destructive ? "danger" : "primary"} onClick={onPrimary}>{primaryText}</Button>
          )}
          <Button onClick={onCancel}>{cancelText}</Button>
        </div>
      </div>
    </div>,
    providerTarget ?? document.body,
  );
}

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

export function ContextMenu({ x, y, actions, onSelect, onDismiss }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ x, y });
  const providerTarget = useFluentPortalTarget();

  useLayoutEffect(() => {
    const menu = menuRef.current;
    if (!menu) return;
    const bounds = menu.getBoundingClientRect();
    setPosition({
      x: x + bounds.width > window.innerWidth ? Math.max(4, x - bounds.width) : x,
      y: y + bounds.height > window.innerHeight ? Math.max(4, y - bounds.height) : y,
    });
    menu.querySelector<HTMLElement>('[role="menuitem"]:not(:disabled)')?.focus();
  }, [x, y]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onDismiss();
      if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      const items = Array.from(menuRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)') ?? []);
      if (!items.length) return;
      const current = items.indexOf(document.activeElement as HTMLElement);
      const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : event.key === "ArrowDown" ? (current + 1) % items.length : (current - 1 + items.length) % items.length;
      items[next].focus();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onDismiss]);

  if (typeof document === "undefined") return null;
  return createPortal(
    <>
      <div className="memora-menu-dismiss" onMouseDown={onDismiss} onContextMenu={(event) => { event.preventDefault(); onDismiss(); }} />
      <div ref={menuRef} role="menu" className="memora-context-menu" style={{ left: position.x, top: position.y }}>
        {actions.map((action) => (
          <div key={action.id}>
            {action.dividerBefore && <div role="separator" className="memora-menu-separator" />}
            <button
              type="button"
              role="menuitem"
              disabled={action.disabled}
              className={cx("memora-menu-item", action.danger && "memora-menu-item--danger")}
              onClick={() => { onSelect(action.id); onDismiss(); }}
            >
              {action.icon && <span className="memora-menu-item__icon">{action.icon}</span>}
              <span>{action.label}</span>
            </button>
          </div>
        ))}
      </div>
    </>,
    providerTarget ?? document.body,
  );
}

export interface TeachingTipProps extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  title: ReactNode;
  onDismiss?: () => void;
  action?: ReactNode;
}

export function TeachingTip({ title, children, onDismiss, action, className, ...rest }: TeachingTipProps) {
  return (
    <aside {...rest} className={cx("memora-teaching-tip", className)}>
      <div className="memora-teaching-tip__header">
        <strong>{title}</strong>
        {onDismiss && <IconButton label="Dismiss" onClick={onDismiss}>×</IconButton>}
      </div>
      <div className="memora-teaching-tip__body">{children}</div>
      {action && <div className="memora-teaching-tip__action">{action}</div>}
    </aside>
  );
}

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

export function NavigationView({ items, footerItems = [], selectedId, onSelect, collapsed = false, onToggleCollapse, ariaLabel = "Main", className, ...rest }: NavigationViewProps) {
  const navRef = useRef<HTMLElement>(null);
  const allItems = [...items, ...footerItems].filter((item) => !item.disabled);
  const onKeyDown = (event: React.KeyboardEvent<HTMLElement>) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const current = allItems.findIndex((item) => item.id === selectedId);
    const next = event.key === "Home" ? 0 : event.key === "End" ? allItems.length - 1 : Math.min(allItems.length - 1, Math.max(0, current + (event.key === "ArrowDown" ? 1 : -1)));
    const item = allItems[next];
    if (!item) return;
    onSelect(item.id);
    requestAnimationFrame(() => navRef.current?.querySelectorAll<HTMLElement>("[data-memora-nav-item]")[next]?.focus());
  };
  const renderItems = (group: NavigationItem[]) => group.map((item) => (
    <button
      key={item.id}
      type="button"
      role="tab"
      data-memora-nav-item
      aria-selected={selectedId === item.id}
      aria-label={collapsed ? item.label : undefined}
      title={collapsed ? item.label : undefined}
      tabIndex={selectedId === item.id ? 0 : -1}
      disabled={item.disabled}
      onClick={() => onSelect(item.id)}
      className={cx("memora-nav-item", selectedId === item.id && "memora-nav-item--selected", collapsed && "memora-nav-item--collapsed")}
    >
      <span className="memora-nav-item__indicator" aria-hidden="true" />
      {item.icon && <span className="memora-nav-item__icon">{item.icon}</span>}
      {!collapsed && <span className="memora-nav-item__label">{item.label}</span>}
    </button>
  ));
  return (
    <nav {...rest} ref={navRef} role="tablist" aria-orientation="vertical" aria-label={ariaLabel} onKeyDown={onKeyDown} className={cx("memora-navigation", collapsed && "memora-navigation--collapsed", className)}>
      {onToggleCollapse && <IconButton label={collapsed ? "Expand navigation" : "Collapse navigation"} aria-expanded={!collapsed} onClick={onToggleCollapse}>☰</IconButton>}
      <div className="memora-navigation__items">{renderItems(items)}</div>
      {footerItems.length > 0 && <div className="memora-navigation__footer">{renderItems(footerItems)}</div>}
    </nav>
  );
}

export type SortDirection = "ascending" | "descending";
export interface DataGridSort { columnId: string; direction: SortDirection }
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

export function DataGrid<T>({ rows, columns, rowKey, ariaLabel, sort, onSortChange, selectedKeys = [], onSelectionChange, onRowContextMenu, emptyMessage = "No items", className }: DataGridProps<T>) {
  const selected = new Set(selectedKeys.map(String));
  const toggleRow = (key: Key, additive: boolean) => {
    if (!onSelectionChange) return;
    const value = String(key);
    const next = additive ? new Set(selected) : new Set<string>();
    if (selected.has(value) && additive) next.delete(value); else next.add(value);
    const keyMap = new Map(rows.map((row) => [String(rowKey(row)), rowKey(row)]));
    onSelectionChange(Array.from(next, (item) => keyMap.get(item) ?? item));
  };
  const moveFocus = (event: React.KeyboardEvent<HTMLTableRowElement>, index: number) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const next = event.key === "Home" ? 0 : event.key === "End" ? rows.length - 1 : Math.min(rows.length - 1, Math.max(0, index + (event.key === "ArrowDown" ? 1 : -1)));
    event.currentTarget.parentElement?.querySelectorAll<HTMLElement>("tr[data-memora-grid-row]")[next]?.focus();
  };
  return (
    <div className={cx("memora-data-grid-wrap", className)}>
      <table className="memora-data-grid" aria-label={ariaLabel}>
        <thead><tr>{columns.map((column) => {
          const active = sort?.columnId === column.id;
          const ariaSort = active ? sort.direction : "none";
          return <th key={column.id} style={{ width: column.width, textAlign: column.align }} aria-sort={column.sortable ? ariaSort : undefined}>
            {column.sortable && onSortChange ? <button type="button" className="memora-grid-sort" onClick={() => onSortChange({ columnId: column.id, direction: active && sort.direction === "ascending" ? "descending" : "ascending" })}>{column.header}<span aria-hidden="true">{active ? (sort.direction === "ascending" ? "↑" : "↓") : ""}</span></button> : column.header}
          </th>;
        })}</tr></thead>
        <tbody>
          {rows.map((row, index) => {
            const key = rowKey(row);
            const isSelected = selected.has(String(key));
            return <tr key={key} data-memora-grid-row tabIndex={index === 0 ? 0 : -1} aria-selected={isSelected} onKeyDown={(event) => moveFocus(event, index)} onClick={(event) => toggleRow(key, event.ctrlKey || event.metaKey)} onContextMenu={(event) => onRowContextMenu?.(row, event)}>
              {columns.map((column) => <td key={column.id} style={{ textAlign: column.align }}>{column.render(row)}</td>)}
            </tr>;
          })}
        </tbody>
      </table>
      {rows.length === 0 && <div className="memora-data-grid__empty">{emptyMessage}</div>}
    </div>
  );
}

/** Visually groups toolbar children while preserving their own semantics. */
export function CommandGroup({ children, className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div {...rest} className={cx("memora-command-group", className)}>{Children.toArray(children)}</div>;
}
