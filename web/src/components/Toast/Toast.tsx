"use client";

import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import styles from "./Toast.module.css";

export interface ToastOptions {
  message: string;
  /** Shows an Undo button. Undo is the only action a toast can have. */
  onUndo?: () => void | Promise<void>;
  /**
   * Runs when the toast closes without Undo (timeout, or replaced by another toast).
   * Used to defer an action until the Undo window has passed.
   */
  onCommit?: () => void | Promise<void>;
  duration?: number;
}

interface ActiveToast extends ToastOptions {
  id: number;
}

const ToastContext = createContext<{ show: (t: ToastOptions) => void } | null>(null);

export function useToast() {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used inside ToastProvider");
  return ctx;
}

const DURATION = 5000;

/** Holds one toast at a time. Lasts 5s and pauses while hovered or focused. */
export function ToastProvider({ children }: { children: ReactNode }) {
  const [toast, setToast] = useState<ActiveToast | null>(null);
  const current = useRef<ActiveToast | null>(null);
  const nextId = useRef(1);

  const close = useCallback((commit: boolean) => {
    const t = current.current;
    current.current = null;
    setToast(null);
    if (t && commit) void t.onCommit?.();
  }, []);

  const show = useCallback(
    (opts: ToastOptions) => {
      // A replaced toast commits its pending action right away.
      if (current.current) void current.current.onCommit?.();
      const t = { ...opts, id: nextId.current++ };
      current.current = t;
      setToast(t);
    },
    [],
  );

  // Commit anything pending if the page goes away.
  useEffect(() => {
    const flush = () => {
      if (current.current) close(true);
    };
    window.addEventListener("pagehide", flush);
    return () => window.removeEventListener("pagehide", flush);
  }, [close]);

  return (
    <ToastContext.Provider value={{ show }}>
      {children}
      <div className={styles.region} role="status" aria-live="polite">
        {toast ? <ToastView key={toast.id} toast={toast} onClose={close} /> : null}
      </div>
    </ToastContext.Provider>
  );
}

function ToastView({ toast, onClose }: { toast: ActiveToast; onClose: (commit: boolean) => void }) {
  const [paused, setPaused] = useState(false);
  const remaining = useRef(toast.duration ?? DURATION);

  useEffect(() => {
    if (paused) return;
    const started = Date.now();
    const timer = window.setTimeout(() => onClose(true), remaining.current);
    return () => {
      window.clearTimeout(timer);
      remaining.current -= Date.now() - started;
    };
  }, [paused, onClose]);

  async function undo() {
    onClose(false);
    await toast.onUndo?.();
  }

  return (
    <div
      className={styles.toast}
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
      onFocus={() => setPaused(true)}
      onBlur={() => setPaused(false)}
    >
      <span className={styles.message}>{toast.message}</span>
      {toast.onUndo ? (
        <button type="button" className={styles.undo} onClick={undo}>
          Undo
        </button>
      ) : null}
    </div>
  );
}
