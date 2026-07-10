import { useState, useEffect } from "react";
import { Info } from "lucide-react";

const PRESETS = [
  { label: "Every minute",      value: "* * * * *"   },
  { label: "Every 5 minutes",   value: "*/5 * * * *" },
  { label: "Every 15 minutes",  value: "*/15 * * * *"},
  { label: "Every 30 minutes",  value: "*/30 * * * *"},
  { label: "Hourly",            value: "0 * * * *"   },
  { label: "Every 2 hours",     value: "0 */2 * * *" },
  { label: "Every 6 hours",     value: "0 */6 * * *" },
  { label: "Daily at midnight", value: "0 0 * * *"   },
  { label: "Daily at noon",     value: "0 12 * * *"  },
  { label: "Weekdays at 9am",   value: "0 9 * * 1-5" },
  { label: "Every Monday 9am",  value: "0 9 * * 1"   },
  { label: "Weekly (Sunday)",   value: "0 0 * * 0"   },
  { label: "Monthly (1st)",     value: "0 0 1 * *"   },
  { label: "Custom",            value: "custom"       },
];

const FIELDS = [
  { key: "min",   label: "Min",     title: "Minute (0-59, */n)" },
  { key: "hour",  label: "Hour",    title: "Hour (0-23, */n)"   },
  { key: "day",   label: "Day",     title: "Day of month (1-31)"},
  { key: "month", label: "Month",   title: "Month (1-12)"       },
  { key: "dow",   label: "Weekday", title: "Day of week (0-7)"  },
];

type Fields = { min: string; hour: string; day: string; month: string; dow: string };

function exprToFields(expr: string): Fields {
  const parts = expr.trim().split(/\s+/);
  if (parts.length === 5) {
    return { min: parts[0], hour: parts[1], day: parts[2], month: parts[3], dow: parts[4] };
  }
  return { min: "*", hour: "*", day: "*", month: "*", dow: "*" };
}

function fieldsToExpr(f: Fields): string {
  return `${f.min} ${f.hour} ${f.day} ${f.month} ${f.dow}`;
}

function detectPreset(expr: string): string {
  const found = PRESETS.find((p) => p.value === expr);
  return found ? found.value : "custom";
}

const DOW_NAMES = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTH_NAMES = [
  "", "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

// Best-effort English rendering for the common cron shapes users actually type.
// Falls back to a neutral label rather than guessing wrong on exotic fields.
function describeCron(expr: string): string {
  const found = PRESETS.find((p) => p.value === expr);
  if (found && found.value !== "custom") return found.label;

  const [min, hour, day, month, dow] = expr.trim().split(/\s+/);
  if ([min, hour, day, month, dow].some((p) => p === undefined)) return "";

  const num = (s: string) => /^\d+$/.test(s);
  const step = (s: string) => /^\*\/\d+$/.test(s);
  const stepVal = (s: string) => s.slice(2);

  const parts: string[] = [];

  // Time of day
  if (num(min) && num(hour)) {
    parts.push(`at ${hour.padStart(2, "0")}:${min.padStart(2, "0")}`);
  } else if (step(min) && hour === "*") {
    parts.push(`every ${stepVal(min)} min`);
  } else if (min === "0" && step(hour)) {
    parts.push(`every ${stepVal(hour)} hours`);
  } else if (min === "*" && hour === "*") {
    parts.push("every minute");
  } else if (num(min) && hour === "*") {
    parts.push(`at :${min.padStart(2, "0")} every hour`);
  }

  // Day of week
  if (dow !== "*") {
    if (dow === "1-5") parts.push("on weekdays");
    else if (dow === "0,6" || dow === "6,0") parts.push("on weekends");
    else if (num(dow)) parts.push(`on ${DOW_NAMES[Number(dow) % 8] ?? dow}`);
    else parts.push(`on weekday ${dow}`);
  }

  // Day of month / month
  if (day !== "*" && num(day)) parts.push(`on day ${day}`);
  if (month !== "*" && num(month)) parts.push(`in ${MONTH_NAMES[Number(month)] ?? month}`);

  return parts.length ? parts.join(" ") : "Custom schedule";
}

// Shared select class
const selectCls =
  "w-full bg-transparent border border-border rounded-lg px-3 py-2 text-sm text-text focus:outline-none focus:border-border-hover";

// Compact field input
const fieldInputCls =
  "w-full bg-transparent border border-border rounded-md px-2 py-1.5 text-xs text-text font-mono text-center focus:outline-none focus:border-border-hover";

interface CronBuilderProps {
  value: string;
  onChange: (v: string) => void;
  onCommit?: (v: string) => void;
  compact?: boolean;
  error?: string | null;
}

export function CronBuilder({ value, onChange, onCommit, compact = false, error }: CronBuilderProps) {
  const [preset, setPreset] = useState<string>(() => detectPreset(value));
  const [fields, setFields] = useState<Fields>(() => exprToFields(value));
  const [customExpr, setCustomExpr] = useState<string>(value);

  // Sync when value changes externally
  useEffect(() => {
    const detected = detectPreset(value);
    setPreset(detected);
    setFields(exprToFields(value));
    setCustomExpr(value);
  }, [value]);

  const handlePresetChange = (newPreset: string) => {
    setPreset(newPreset);
    if (newPreset === "custom") {
      // Keep current fields/customExpr as-is
      return;
    }
    setFields(exprToFields(newPreset));
    setCustomExpr(newPreset);
    onChange(newPreset);
    onCommit?.(newPreset);
  };

  const handleFieldChange = (key: keyof Fields, val: string) => {
    const newFields = { ...fields, [key]: val || "*" };
    setFields(newFields);
    const expr = fieldsToExpr(newFields);
    setCustomExpr(expr);
    setPreset("custom");
    onChange(expr);
  };

  const handleFieldBlur = () => {
    const expr = fieldsToExpr(fields);
    onCommit?.(expr);
  };

  const handleCustomExprChange = (val: string) => {
    setCustomExpr(val);
    onChange(val);
  };

  const handleCustomExprBlur = () => {
    // Try to parse and sync fields
    const parts = customExpr.trim().split(/\s+/);
    if (parts.length === 5) {
      const newFields = { min: parts[0], hour: parts[1], day: parts[2], month: parts[3], dow: parts[4] };
      setFields(newFields);
    }
    onCommit?.(customExpr);
  };

  const description = preset === "custom" ? describeCron(customExpr) : describeCron(preset === "custom" ? "" : preset);

  if (compact) {
    return (
      <div className="space-y-1.5">
        <select
          value={preset}
          onChange={(e) => handlePresetChange(e.currentTarget.value)}
          className={selectCls}
        >
          {PRESETS.map((p) => (
            <option key={p.value} value={p.value}>
              {p.label}
            </option>
          ))}
        </select>

        {preset === "custom" && (
          <input
            type="text"
            value={customExpr}
            onChange={(e) => handleCustomExprChange(e.currentTarget.value)}
            onBlur={handleCustomExprBlur}
            placeholder="*/5 * * * *"
            className={`${selectCls} font-mono text-xs`}
          />
        )}

        {description && (
          <p className="flex items-center gap-1 text-xs text-text-muted">
            <Info className="w-3 h-3 shrink-0" />
            {description}
          </p>
        )}
        {error && <p className="text-xs text-red-400">{error}</p>}
      </div>
    );
  }

  // Full mode
  return (
    <div className="space-y-3">
      <div>
        <span className="text-xs text-text-tertiary mb-1 block">Preset</span>
        <select
          value={preset}
          onChange={(e) => handlePresetChange(e.currentTarget.value)}
          className={selectCls}
        >
          {PRESETS.map((p) => (
            <option key={p.value} value={p.value}>
              {p.label}
            </option>
          ))}
        </select>
      </div>

      {preset === "custom" ? (
        <div>
          <span className="text-xs text-text-tertiary mb-1 block">
            Expression <span className="font-mono">(min hour day month weekday)</span>
          </span>
          <div className="grid grid-cols-5 gap-2">
            {FIELDS.map((f) => (
              <div key={f.key} className="text-center">
                <span className="text-xs text-text-tertiary block mb-1">{f.label}</span>
                <input
                  type="text"
                  value={fields[f.key as keyof Fields]}
                  onChange={(e) => handleFieldChange(f.key as keyof Fields, e.currentTarget.value)}
                  onBlur={handleFieldBlur}
                  placeholder="*"
                  title={f.title}
                  className={fieldInputCls}
                />
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-5 gap-2">
          {FIELDS.map((f) => (
            <div key={f.key} className="text-center">
              <span className="text-xs text-text-tertiary block mb-1">{f.label}</span>
              <div className={`${fieldInputCls} text-text-tertiary bg-surface-hover cursor-default`}>
                {fields[f.key as keyof Fields]}
              </div>
            </div>
          ))}
        </div>
      )}

      {description && (
        <p className="flex items-center gap-1.5 text-xs text-text-muted">
          <Info className="w-3.5 h-3.5 shrink-0" />
          {description}
        </p>
      )}
      {error && <p className="text-xs text-red-400">{error}</p>}
    </div>
  );
}
