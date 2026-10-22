import { useEffect, useRef, useState } from "react";

/**
 * Blocking progress dialog for long file operations (backup, restore, delete all).
 * A modal <dialog> makes the rest of the window inert, so the user can't switch
 * pages or start something else halfway through. Escape is ignored.
 *
 * ponytail: indeterminate bar + elapsed time. Add a percentage via backend
 * progress events if backups get big enough that people want an ETA.
 */
export function BusyDialog({ title, detail }: { title: string; detail: string }) {
  const ref = useRef<HTMLDialogElement>(null);
  const [seconds, setSeconds] = useState(0);

  useEffect(() => {
    ref.current?.showModal();
    const started = Date.now();
    const timer = setInterval(() => setSeconds(Math.floor((Date.now() - started) / 1000)), 1000);
    return () => clearInterval(timer);
  }, []);

  const elapsed = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;

  return (
    <dialog ref={ref} className="modal" aria-labelledby="busy-title" onCancel={(e) => e.preventDefault()}>
      <header className="modal__head">
        <div>
          <h3 id="busy-title">{title}</h3>
          <p>{detail}</p>
        </div>
      </header>
      <div className="modal__body">
        <div className="wiz-stack" aria-live="polite">
          <div className="modal__status">
            <span className="spinner" />
            <span className="modal__status-text">Working… don't close CAMPP</span>
            <span className="modal__pct mono">{elapsed}</span>
          </div>
          <div className="progress-container wiz-progress" role="progressbar" aria-label={title}>
            <div className="progress-bar-animated" />
          </div>
        </div>
      </div>
    </dialog>
  );
}
