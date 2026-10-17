import { useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";

interface DialogOptions {
  title: string;
  message: string;
  /** Confirm dialogs get a Cancel button; alerts only an OK button. */
  kind?: "alert" | "confirm";
  confirmLabel?: string;
  /** Style the confirm button as destructive/risky. */
  danger?: boolean;
}

function ThemedDialog({ title, message, kind = "alert", confirmLabel, danger, onDone }: DialogOptions & { onDone: (ok: boolean) => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => ref.current?.showModal(), []);

  return (
    <dialog
      ref={ref}
      className="modal"
      aria-labelledby="themed-dialog-title"
      onCancel={(e) => {
        e.preventDefault();
        onDone(false);
      }}
    >
      <header className="modal__head">
        <div>
          <h3 id="themed-dialog-title">{title}</h3>
        </div>
      </header>
      <div className="modal__body">
        <p className="modal__message">{message}</p>
      </div>
      <footer className="modal__foot">
        {kind === "confirm" && (
          <button type="button" className="btn-secondary btn-sm" onClick={() => onDone(false)}>
            Cancel
          </button>
        )}
        <button
          type="button"
          className={danger ? "btn-danger-solid btn-sm" : "btn-primary btn-sm"}
          onClick={() => onDone(true)}
          autoFocus
        >
          {confirmLabel ?? "OK"}
        </button>
      </footer>
    </dialog>
  );
}

/** Themed replacement for window.alert/confirm. Resolves true on confirm, false on cancel/Esc. */
export function showDialog(opts: DialogOptions): Promise<boolean> {
  return new Promise((resolve) => {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    root.render(
      <ThemedDialog
        {...opts}
        onDone={(ok) => {
          root.unmount();
          host.remove();
          resolve(ok);
        }}
      />,
    );
  });
}

export const showAlert = (title: string, message: string) => showDialog({ title, message }).then(() => undefined);

export const showConfirm = (title: string, message: string, confirmLabel: string, danger = false) =>
  showDialog({ title, message, kind: "confirm", confirmLabel, danger });
