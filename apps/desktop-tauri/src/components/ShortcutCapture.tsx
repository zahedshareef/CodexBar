import { useCallback, useEffect, useRef, useState } from "react";
import { useLocale } from "../hooks/useLocale";

interface ShortcutCaptureProps {
  value: string;
  disabled?: boolean;
  onCommit: (accelerator: string) => void;
  onClear: () => void;
}

const MODIFIER_KEYS = new Set([
  "Control",
  "Shift",
  "Alt",
  "Meta",
  "OS",
  "AltGraph",
]);

function composeAccelerator(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  if (e.metaKey) parts.push("Super");
  if (parts.length === 0) return null;

  let key: string | null = null;
  const code = e.code;
  if (code.startsWith("Key") && code.length === 4) {
    key = code.slice(3); // KeyA → A
  } else if (code.startsWith("Digit") && code.length === 6) {
    key = code.slice(5); // Digit0 → 0
  } else if (/^F([1-9]|1[0-2])$/.test(code)) {
    key = code;
  } else {
    switch (code) {
      case "Space":
        key = "Space";
        break;
      case "Enter":
      case "NumpadEnter":
        key = "Enter";
        break;
      case "Tab":
        key = "Tab";
        break;
      case "Escape":
        // Reserved for cancel — never included as key.
        return null;
      default:
        return null;
    }
  }

  if (!key) return null;
  parts.push(key);
  return parts.join("+");
}

export function ShortcutCapture({
  value,
  disabled,
  onCommit,
  onClear,
}: ShortcutCaptureProps) {
  const { t } = useLocale();
  const [recording, setRecording] = useState(false);
  const chipRef = useRef<HTMLDivElement | null>(null);

  const stopRecording = useCallback(() => {
    setRecording(false);
  }, []);

  useEffect(() => {
    if (!recording) return;

    const handleKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        stopRecording();
        return;
      }
      if (e.key === "Backspace" && !e.ctrlKey && !e.altKey && !e.metaKey) {
        stopRecording();
        onClear();
        return;
      }

      const accel = composeAccelerator(e);
      if (accel) {
        stopRecording();
        onCommit(accel);
      }
    };

    window.addEventListener("keydown", handleKey, true);
    return () => {
      window.removeEventListener("keydown", handleKey, true);
    };
  }, [recording, stopRecording, onCommit, onClear]);

  useEffect(() => {
    if (recording) {
      chipRef.current?.focus();
    }
  }, [recording]);

  const chipText = recording
    ? t("ShortcutRecordingHint")
    : value || t("ShortcutEmptyPlaceholder");

  return (
    <div className="shortcut-capture">
      <div
        ref={chipRef}
        tabIndex={-1}
        className={
          "shortcut-capture__chip" +
          (recording ? " shortcut-capture__chip--recording" : "") +
          (!value && !recording ? " shortcut-capture__chip--empty" : "")
        }
        aria-live="polite"
      >
        {chipText}
      </div>
      <div className="shortcut-capture__actions">
        <button
          type="button"
          className="shortcut-capture__button"
          disabled={disabled || recording}
          onClick={() => setRecording(true)}
        >
          {recording ? t("ShortcutRecordingLabel") : t("ShortcutRecordButton")}
        </button>
        <button
          type="button"
          className="shortcut-capture__button shortcut-capture__button--ghost"
          disabled={disabled || recording || !value}
          onClick={onClear}
        >
          {t("ShortcutClearButton")}
        </button>
      </div>
    </div>
  );
}
