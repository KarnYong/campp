import { useState, type InputHTMLAttributes } from "react";
import { Icon } from "./icons";

/** Password field with an eye button that toggles visibility. */
export function PasswordInput({ className = "", ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  const [visible, setVisible] = useState(false);
  return (
    <span className={`pw-input ${className}`}>
      <input {...rest} type={visible ? "text" : "password"} className="input" />
      <button
        type="button"
        className="pw-input__toggle"
        onClick={() => setVisible((v) => !v)}
        disabled={rest.disabled}
        aria-label={visible ? "Hide password" : "Show password"}
        aria-pressed={visible}
        title={visible ? "Hide password" : "Show password"}
      >
        <Icon name={visible ? "eye-off" : "eye"} size={15} />
      </button>
    </span>
  );
}
