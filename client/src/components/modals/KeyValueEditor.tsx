import { Plus, Trash2 } from "lucide-react";

interface Props {
  value: Record<string, string>;
  onChange: (env: Record<string, string>) => void;
}

export function KeyValueEditor({ value, onChange }: Props) {
  const entries = Object.entries(value);

  const update = (oldKey: string, newKey: string, newVal: string) => {
    const next: Record<string, string> = {};
    for (const [k, v] of entries) {
      if (k === oldKey) {
        if (newKey) next[newKey] = newVal;
      } else {
        next[k] = v;
      }
    }
    onChange(next);
  };

  const add = () => {
    let key = "NEW_VAR";
    let i = 1;
    while (key in value) key = `NEW_VAR_${i++}`;
    onChange({ ...value, [key]: "" });
  };

  const remove = (key: string) => {
    const next = { ...value };
    delete next[key];
    onChange(next);
  };

  return (
    <div className="space-y-1.5">
      {entries.map(([k, v]) => (
        <div key={k} className="flex gap-2">
          <input
            className="w-[40%] bg-bg border border-border rounded-lg px-2.5 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono"
            value={k}
            placeholder="KEY"
            onChange={(e) => update(k, e.target.value, v)}
          />
          <input
            className="flex-1 bg-bg border border-border rounded-lg px-2.5 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono"
            value={v}
            placeholder="value"
            onChange={(e) => update(k, k, e.target.value)}
          />
          <button
            type="button"
            onClick={() => remove(k)}
            className="p-1.5 text-text-muted hover:text-danger rounded-md hover:bg-surface-hover transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      ))}
      <button
        type="button"
        onClick={add}
        className="flex items-center gap-1.5 text-xs text-accent hover:text-accent-hover transition-colors pt-1"
      >
        <Plus className="w-3.5 h-3.5" />
        Add variable
      </button>
    </div>
  );
}
