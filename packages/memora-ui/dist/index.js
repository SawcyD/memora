import { jsx as e, jsxs as s, Fragment as T } from "react/jsx-runtime";
import { useState as E, useContext as H, createContext as I, Children as R, useId as C, useRef as x, useEffect as A, useLayoutEffect as F, isValidElement as P, cloneElement as q } from "react";
import { createPortal as M } from "react-dom";
const D = I(null);
function B() {
  return H(D);
}
function j({
  children: a,
  theme: r = "system",
  density: n = "compact",
  accentColor: i,
  className: t,
  style: l,
  ...m
}) {
  const [p, v] = E(null), c = {
    ...l,
    ...i ? { "--memora-accent": i } : {}
  };
  return /* @__PURE__ */ e(D.Provider, { value: p, children: /* @__PURE__ */ e(
    "div",
    {
      ...m,
      ref: v,
      className: ["memora-ui-root", t].filter(Boolean).join(" "),
      "data-memora-theme": r,
      "data-memora-density": n,
      style: c,
      children: a
    }
  ) });
}
function d(...a) {
  return a.filter(Boolean).join(" ");
}
function S({
  children: a,
  variant: r = "standard",
  accent: n = !1,
  static: i = !1,
  className: t,
  type: l = "button",
  ...m
}) {
  return /* @__PURE__ */ e(
    "button",
    {
      ...m,
      type: l,
      className: d(
        "memora-button",
        `memora-button--${n ? "primary" : r}`,
        i && "memora-static",
        t
      ),
      children: a
    }
  );
}
function k({
  label: a,
  variant: r = "subtle",
  static: n = !1,
  className: i,
  type: t = "button",
  children: l,
  ...m
}) {
  return /* @__PURE__ */ e(
    "button",
    {
      ...m,
      type: t,
      "aria-label": a,
      className: d(
        "memora-icon-button",
        `memora-button--${r}`,
        n && "memora-static",
        i
      ),
      children: l
    }
  );
}
function V({ children: a, className: r, ...n }) {
  return /* @__PURE__ */ e("h2", { ...n, className: d("memora-section-header", r), children: a });
}
function U({ label: a, value: r, help: n, className: i, ...t }) {
  return /* @__PURE__ */ s("div", { ...t, className: d("memora-info-row", i), children: [
    /* @__PURE__ */ e("span", { className: d("memora-info-row__label", n && "memora-help-text"), title: n, children: a }),
    /* @__PURE__ */ e("span", { className: "memora-info-row__value memora-tabular", children: r })
  ] });
}
function G({ checked: a, onChange: r, disabled: n = !1, label: i, className: t }) {
  return /* @__PURE__ */ e(
    "button",
    {
      type: "button",
      role: "switch",
      "aria-checked": a,
      "aria-label": i,
      disabled: n,
      onClick: () => r(!a),
      className: d("memora-toggle", a && "memora-toggle--checked", t),
      children: /* @__PURE__ */ e("span", { className: "memora-toggle__track", "aria-hidden": "true", children: /* @__PURE__ */ e("span", { className: "memora-toggle__thumb" }) })
    }
  );
}
function O({
  value: a,
  options: r,
  onChange: n,
  label: i,
  className: t,
  ...l
}) {
  return /* @__PURE__ */ e(
    "select",
    {
      ...l,
      "aria-label": i,
      value: String(a),
      onChange: (m) => {
        const p = r.find((v) => String(v.value) === m.target.value);
        p && n(p.value);
      },
      className: d("memora-combo-box", t),
      children: r.map((m) => /* @__PURE__ */ e("option", { value: String(m.value), disabled: m.disabled, children: m.label }, String(m.value)))
    }
  );
}
function W({ value: a, min: r, max: n, onChange: i, label: t, suffix: l, disabled: m, className: p }) {
  return /* @__PURE__ */ s("div", { className: d("memora-number-box", p), children: [
    /* @__PURE__ */ e(
      "input",
      {
        type: "number",
        "aria-label": t,
        value: a,
        min: r,
        max: n,
        disabled: m,
        onChange: (v) => {
          const c = Number(v.target.value);
          Number.isFinite(c) && i(Math.min(n, Math.max(r, Math.round(c))));
        }
      }
    ),
    l && /* @__PURE__ */ e("span", { children: l })
  ] });
}
function z({ title: a, description: r, note: n, control: i, className: t, ...l }) {
  return /* @__PURE__ */ s("div", { ...l, className: d("memora-settings-row", t), children: [
    /* @__PURE__ */ s("div", { className: "memora-settings-row__copy", children: [
      /* @__PURE__ */ e("div", { className: "memora-settings-row__title", children: a }),
      r && /* @__PURE__ */ e("div", { className: "memora-settings-row__description", children: r }),
      n && /* @__PURE__ */ e("div", { className: "memora-settings-row__note", children: n })
    ] }),
    i && /* @__PURE__ */ e("div", { className: "memora-settings-row__control", children: i })
  ] });
}
function J({ children: a, className: r, ...n }) {
  return /* @__PURE__ */ e("section", { ...n, className: d("memora-settings-section", r), children: a });
}
function Q({ value: a = 0, max: r = 100, label: n, indeterminate: i = !1, className: t, ...l }) {
  const m = r > 0 ? Math.min(100, Math.max(0, a / r * 100)) : 0;
  return /* @__PURE__ */ e(
    "div",
    {
      ...l,
      role: "progressbar",
      "aria-label": n,
      "aria-valuenow": i ? void 0 : a,
      "aria-valuemin": i ? void 0 : 0,
      "aria-valuemax": i ? void 0 : r,
      className: d("memora-progress", i && "memora-progress--indeterminate", t),
      children: /* @__PURE__ */ e("span", { className: "memora-progress__bar", style: i ? void 0 : { width: `${m}%` } })
    }
  );
}
function X({ title: a, message: r, tone: n = "info", action: i, onDismiss: t, className: l, ...m }) {
  return /* @__PURE__ */ s("div", { ...m, role: n === "error" ? "alert" : "status", className: d("memora-info-bar", `memora-info-bar--${n}`, l), children: [
    /* @__PURE__ */ e("span", { className: "memora-info-bar__mark", "aria-hidden": "true", children: "i" }),
    /* @__PURE__ */ s("div", { className: "memora-info-bar__copy", children: [
      /* @__PURE__ */ e("div", { className: "memora-info-bar__title", children: a }),
      r && /* @__PURE__ */ e("div", { className: "memora-info-bar__message", children: r })
    ] }),
    i,
    t && /* @__PURE__ */ e(k, { label: "Dismiss", onClick: t, children: "×" })
  ] });
}
function Y({ value: a, onChange: r, placeholder: n = "Search", label: i, className: t, ...l }) {
  return /* @__PURE__ */ s("div", { ...l, className: d("memora-search-box", t), children: [
    /* @__PURE__ */ s("svg", { viewBox: "0 0 16 16", "aria-hidden": "true", className: "memora-search-box__icon", children: [
      /* @__PURE__ */ e("circle", { cx: "7", cy: "7", r: "4.5" }),
      /* @__PURE__ */ e("path", { d: "m10.5 10.5 3 3" })
    ] }),
    /* @__PURE__ */ e(
      "input",
      {
        type: "search",
        "aria-label": i,
        value: a,
        placeholder: n,
        onChange: (m) => r(m.target.value)
      }
    )
  ] });
}
function Z({ children: a, className: r, ...n }) {
  return /* @__PURE__ */ e("div", { ...n, role: "toolbar", className: d("memora-command-bar", r), children: a });
}
function ee({ content: a, children: r, placement: n = "top" }) {
  const i = C(), t = P(r) ? q(r, { "aria-describedby": i }) : r;
  return /* @__PURE__ */ s("span", { className: "memora-tooltip-anchor", children: [
    t,
    /* @__PURE__ */ e("span", { id: i, role: "tooltip", className: d("memora-tooltip", `memora-tooltip--${n}`), children: a })
  ] });
}
function ae({
  open: a = !0,
  title: r,
  children: n,
  primaryText: i,
  cancelText: t = "Cancel",
  onPrimary: l,
  onCancel: m,
  destructive: p = !1
}) {
  const v = C(), c = x(null), u = B();
  return A(() => {
    if (!a) return;
    const f = document.activeElement;
    c.current?.focus();
    const y = (h) => {
      if (h.key === "Escape" && m(), h.key !== "Tab") return;
      const o = c.current?.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (!o?.length) return;
      const g = o[0], b = o[o.length - 1];
      h.shiftKey && document.activeElement === g ? (h.preventDefault(), b.focus()) : !h.shiftKey && document.activeElement === b && (h.preventDefault(), g.focus());
    };
    return window.addEventListener("keydown", y), () => {
      window.removeEventListener("keydown", y), f?.focus();
    };
  }, [m, a]), !a || typeof document > "u" ? null : M(
    /* @__PURE__ */ e("div", { className: "memora-dialog-backdrop", onMouseDown: (f) => f.target === f.currentTarget && m(), children: /* @__PURE__ */ s("div", { ref: c, role: "dialog", "aria-modal": "true", "aria-labelledby": v, tabIndex: -1, className: "memora-dialog", children: [
      /* @__PURE__ */ s("div", { className: "memora-dialog__body", children: [
        /* @__PURE__ */ e("h2", { id: v, children: r }),
        /* @__PURE__ */ e("div", { className: "memora-dialog__content", children: n })
      ] }),
      /* @__PURE__ */ s("div", { className: "memora-dialog__actions", children: [
        i && l && /* @__PURE__ */ e(S, { variant: p ? "danger" : "primary", onClick: l, children: i }),
        /* @__PURE__ */ e(S, { onClick: m, children: t })
      ] })
    ] }) }),
    u ?? document.body
  );
}
function re({ x: a, y: r, actions: n, onSelect: i, onDismiss: t }) {
  const l = x(null), [m, p] = E({ x: a, y: r }), v = B();
  return F(() => {
    const c = l.current;
    if (!c) return;
    const u = c.getBoundingClientRect();
    p({
      x: a + u.width > window.innerWidth ? Math.max(4, a - u.width) : a,
      y: r + u.height > window.innerHeight ? Math.max(4, r - u.height) : r
    }), c.querySelector('[role="menuitem"]:not(:disabled)')?.focus();
  }, [a, r]), A(() => {
    const c = (u) => {
      if (u.key === "Escape" && t(), !["ArrowDown", "ArrowUp", "Home", "End"].includes(u.key)) return;
      u.preventDefault();
      const f = Array.from(l.current?.querySelectorAll('[role="menuitem"]:not(:disabled)') ?? []);
      if (!f.length) return;
      const y = f.indexOf(document.activeElement), h = u.key === "Home" ? 0 : u.key === "End" ? f.length - 1 : u.key === "ArrowDown" ? (y + 1) % f.length : (y - 1 + f.length) % f.length;
      f[h].focus();
    };
    return window.addEventListener("keydown", c), () => window.removeEventListener("keydown", c);
  }, [t]), typeof document > "u" ? null : M(
    /* @__PURE__ */ s(T, { children: [
      /* @__PURE__ */ e("div", { className: "memora-menu-dismiss", onMouseDown: t, onContextMenu: (c) => {
        c.preventDefault(), t();
      } }),
      /* @__PURE__ */ e("div", { ref: l, role: "menu", className: "memora-context-menu", style: { left: m.x, top: m.y }, children: n.map((c) => /* @__PURE__ */ s("div", { children: [
        c.dividerBefore && /* @__PURE__ */ e("div", { role: "separator", className: "memora-menu-separator" }),
        /* @__PURE__ */ s(
          "button",
          {
            type: "button",
            role: "menuitem",
            disabled: c.disabled,
            className: d("memora-menu-item", c.danger && "memora-menu-item--danger"),
            onClick: () => {
              i(c.id), t();
            },
            children: [
              c.icon && /* @__PURE__ */ e("span", { className: "memora-menu-item__icon", children: c.icon }),
              /* @__PURE__ */ e("span", { children: c.label })
            ]
          }
        )
      ] }, c.id)) })
    ] }),
    v ?? document.body
  );
}
function ne({ title: a, children: r, onDismiss: n, action: i, className: t, ...l }) {
  return /* @__PURE__ */ s("aside", { ...l, className: d("memora-teaching-tip", t), children: [
    /* @__PURE__ */ s("div", { className: "memora-teaching-tip__header", children: [
      /* @__PURE__ */ e("strong", { children: a }),
      n && /* @__PURE__ */ e(k, { label: "Dismiss", onClick: n, children: "×" })
    ] }),
    /* @__PURE__ */ e("div", { className: "memora-teaching-tip__body", children: r }),
    i && /* @__PURE__ */ e("div", { className: "memora-teaching-tip__action", children: i })
  ] });
}
function te({ items: a, footerItems: r = [], selectedId: n, onSelect: i, collapsed: t = !1, onToggleCollapse: l, ariaLabel: m = "Main", className: p, ...v }) {
  const c = x(null), u = [...a, ...r].filter((h) => !h.disabled), f = (h) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(h.key)) return;
    h.preventDefault();
    const o = u.findIndex((w) => w.id === n), g = h.key === "Home" ? 0 : h.key === "End" ? u.length - 1 : Math.min(u.length - 1, Math.max(0, o + (h.key === "ArrowDown" ? 1 : -1))), b = u[g];
    b && (i(b.id), requestAnimationFrame(() => c.current?.querySelectorAll("[data-memora-nav-item]")[g]?.focus()));
  }, y = (h) => h.map((o) => /* @__PURE__ */ s(
    "button",
    {
      type: "button",
      role: "tab",
      "data-memora-nav-item": !0,
      "aria-selected": n === o.id,
      "aria-label": t ? o.label : void 0,
      title: t ? o.label : void 0,
      tabIndex: n === o.id ? 0 : -1,
      disabled: o.disabled,
      onClick: () => i(o.id),
      className: d("memora-nav-item", n === o.id && "memora-nav-item--selected", t && "memora-nav-item--collapsed"),
      children: [
        /* @__PURE__ */ e("span", { className: "memora-nav-item__indicator", "aria-hidden": "true" }),
        o.icon && /* @__PURE__ */ e("span", { className: "memora-nav-item__icon", children: o.icon }),
        !t && /* @__PURE__ */ e("span", { className: "memora-nav-item__label", children: o.label })
      ]
    },
    o.id
  ));
  return /* @__PURE__ */ s("nav", { ...v, ref: c, role: "tablist", "aria-orientation": "vertical", "aria-label": m, onKeyDown: f, className: d("memora-navigation", t && "memora-navigation--collapsed", p), children: [
    l && /* @__PURE__ */ e(k, { label: t ? "Expand navigation" : "Collapse navigation", "aria-expanded": !t, onClick: l, children: "☰" }),
    /* @__PURE__ */ e("div", { className: "memora-navigation__items", children: y(a) }),
    r.length > 0 && /* @__PURE__ */ e("div", { className: "memora-navigation__footer", children: y(r) })
  ] });
}
function ie({ rows: a, columns: r, rowKey: n, ariaLabel: i, sort: t, onSortChange: l, selectedKeys: m = [], onSelectionChange: p, onRowContextMenu: v, emptyMessage: c = "No items", className: u }) {
  const f = new Set(m.map(String)), y = (o, g) => {
    if (!p) return;
    const b = String(o), w = g ? new Set(f) : /* @__PURE__ */ new Set();
    f.has(b) && g ? w.delete(b) : w.add(b);
    const N = new Map(a.map((_) => [String(n(_)), n(_)]));
    p(Array.from(w, (_) => N.get(_) ?? _));
  }, h = (o, g) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(o.key)) return;
    o.preventDefault();
    const b = o.key === "Home" ? 0 : o.key === "End" ? a.length - 1 : Math.min(a.length - 1, Math.max(0, g + (o.key === "ArrowDown" ? 1 : -1)));
    o.currentTarget.parentElement?.querySelectorAll("tr[data-memora-grid-row]")[b]?.focus();
  };
  return /* @__PURE__ */ s("div", { className: d("memora-data-grid-wrap", u), children: [
    /* @__PURE__ */ s("table", { className: "memora-data-grid", "aria-label": i, children: [
      /* @__PURE__ */ e("thead", { children: /* @__PURE__ */ e("tr", { children: r.map((o) => {
        const g = t?.columnId === o.id, b = g ? t.direction : "none";
        return /* @__PURE__ */ e("th", { style: { width: o.width, textAlign: o.align }, "aria-sort": o.sortable ? b : void 0, children: o.sortable && l ? /* @__PURE__ */ s("button", { type: "button", className: "memora-grid-sort", onClick: () => l({ columnId: o.id, direction: g && t.direction === "ascending" ? "descending" : "ascending" }), children: [
          o.header,
          /* @__PURE__ */ e("span", { "aria-hidden": "true", children: g ? t.direction === "ascending" ? "↑" : "↓" : "" })
        ] }) : o.header }, o.id);
      }) }) }),
      /* @__PURE__ */ e("tbody", { children: a.map((o, g) => {
        const b = n(o), w = f.has(String(b));
        return /* @__PURE__ */ e("tr", { "data-memora-grid-row": !0, tabIndex: g === 0 ? 0 : -1, "aria-selected": w, onKeyDown: (N) => h(N, g), onClick: (N) => y(b, N.ctrlKey || N.metaKey), onContextMenu: (N) => v?.(o, N), children: r.map((N) => /* @__PURE__ */ e("td", { style: { textAlign: N.align }, children: N.render(o) }, N.id)) }, b);
      }) })
    ] }),
    a.length === 0 && /* @__PURE__ */ e("div", { className: "memora-data-grid__empty", children: c })
  ] });
}
function oe({ children: a, className: r, ...n }) {
  return /* @__PURE__ */ e("div", { ...n, className: d("memora-command-group", r), children: R.toArray(a) });
}
export {
  S as Button,
  O as ComboBox,
  Z as CommandBar,
  oe as CommandGroup,
  ae as ContentDialog,
  re as ContextMenu,
  ie as DataGrid,
  j as FluentProvider,
  k as IconButton,
  X as InfoBar,
  U as InfoRow,
  te as NavigationView,
  W as NumberBox,
  Q as ProgressBar,
  Y as SearchBox,
  V as SectionHeader,
  z as SettingsRow,
  J as SettingsSection,
  ne as TeachingTip,
  G as ToggleSwitch,
  ee as Tooltip,
  B as useFluentPortalTarget
};
