import { useState } from "react";
import { Info, SquareTerminal, CalendarClock } from "lucide-react";
import type { Script, ScriptCreate } from "@/api/types";
import { useScripts } from "@/store/scripts";
import { SplitModal, type SplitSection } from "./SplitModal";
import { Field, Input, TextArea, Select, Checkbox } from "./FormField";
import { CronBuilder } from "./CronBuilder";

const PLATFORM_OPTIONS = [
  { value: "all", label: "All" },
  { value: "windows", label: "Windows" },
  { value: "linux", label: "Linux" },
];

const RUN_MODE_OPTIONS = [
  { value: "exec", label: "Executable + args" },
  { value: "shell", label: "Terminal command" },
];

interface Props {
  script?: Script;
  onClose: () => void;
}

function isValidCron(expr: string): boolean {
  return expr.trim().split(/\s+/).length === 5;
}

export function ScriptModal({ script, onClose }: Props) {
  const { create, update } = useScripts();
  const isEdit = !!script;

  const [form, setForm] = useState({
    name: script?.name ?? "",
    description: script?.description ?? "",
    command: script?.command ?? "",
    run_mode: script?.run_mode ?? "exec",
    cwd: script?.cwd ?? "",
    args: script?.args?.join(" ") ?? "",
    platform: script?.platform ?? "all",
    tags: script?.tags?.join(", ") ?? "",
    cron_enabled: script?.cron_enabled ?? false,
    cron_schedule: script?.cron_schedule ?? "",
  });

  const [saving, setSaving] = useState(false);
  const [cronError, setCronError] = useState<string | null>(null);

  const setField = (key: string, value: string | boolean) =>
    setForm((f) => ({ ...f, [key]: value }));

  const cronInvalid =
    form.cron_enabled && !!form.cron_schedule && !isValidCron(form.cron_schedule);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (cronInvalid) {
      setCronError("Must be a 5-part cron expression (e.g. */5 * * * *)");
      return;
    }
    setCronError(null);
    setSaving(true);

    const data: ScriptCreate = {
      name: form.name,
      description: form.description || null,
      command: form.command,
      run_mode: form.run_mode as "exec" | "shell",
      cwd: form.cwd || null,
      args:
        form.run_mode === "exec" && form.args
          ? form.args.split(/\s+/).filter(Boolean)
          : null,
      platform: form.platform,
      tags: form.tags ? form.tags.split(",").map((t) => t.trim()).filter(Boolean) : null,
      cron_enabled: form.cron_enabled,
      cron_schedule: form.cron_enabled ? form.cron_schedule || null : null,
    };

    try {
      if (isEdit) await update(script.id, data);
      else await create(data);
      onClose();
    } catch (err) {
      console.error(err);
      setSaving(false);
    }
  };

  const sections: SplitSection[] = [
    {
      id: "general",
      label: "General",
      icon: Info,
      invalid: !form.name.trim(),
      hint: "What this script is and how it's tagged.",
      content: (
        <div className="space-y-4">
          <Field label="Name">
            <Input
              value={form.name}
              onChange={(e) => setField("name", e.currentTarget.value)}
              placeholder="Backup script"
              required
              autoFocus
            />
          </Field>
          <Field label="Description">
            <TextArea
              value={form.description}
              onChange={(e) => setField("description", e.currentTarget.value)}
              placeholder="Optional description"
              rows={3}
            />
          </Field>
          <div className="grid grid-cols-2 gap-4">
            <Field label="Platform">
              <Select
                value={form.platform}
                onChange={(e) => setField("platform", e.currentTarget.value)}
                options={PLATFORM_OPTIONS}
              />
            </Field>
            <Field label="Tags (comma-separated)">
              <Input
                value={form.tags}
                onChange={(e) => setField("tags", e.currentTarget.value)}
                placeholder="backup, util"
              />
            </Field>
          </div>
        </div>
      ),
    },
    {
      id: "command",
      label: "Command",
      icon: SquareTerminal,
      invalid: !form.command.trim(),
      hint: "What runs when the script is invoked.",
      content: (
        <div className="space-y-4">
          <Field label="Run Mode">
            <Select
              value={form.run_mode}
              onChange={(e) => setField("run_mode", e.currentTarget.value)}
              options={RUN_MODE_OPTIONS}
            />
          </Field>
          <Field label="Command">
            <Input
              value={form.command}
              onChange={(e) => setField("command", e.currentTarget.value)}
              placeholder={form.run_mode === "shell" ? "ssh pi@192.168.1.50" : "python"}
              required
            />
          </Field>
          {form.run_mode === "shell" && (
            <p className="text-xs text-text-muted -mt-1">
              Enter the full terminal command on one line (for example:{" "}
              <span className="font-mono">ssh pi@192.168.1.50</span>).
            </p>
          )}
          <div className="grid grid-cols-2 gap-4">
            <Field label="Working Directory">
              <Input
                value={form.cwd}
                onChange={(e) => setField("cwd", e.currentTarget.value)}
                placeholder="/path/to/dir"
              />
            </Field>
            {form.run_mode === "exec" && (
              <Field label="Arguments (space-separated)">
                <Input
                  value={form.args}
                  onChange={(e) => setField("args", e.currentTarget.value)}
                  placeholder="script.py --verbose"
                />
              </Field>
            )}
          </div>
        </div>
      ),
    },
    {
      id: "schedule",
      label: "Schedule",
      icon: CalendarClock,
      invalid: cronInvalid,
      hint: "Optionally run the script automatically on a cron schedule.",
      content: (
        <div className="space-y-3">
          <Checkbox
            label="Enable cron schedule"
            checked={form.cron_enabled}
            onChange={(v) => setField("cron_enabled", v)}
          />
          {form.cron_enabled && (
            <CronBuilder
              value={form.cron_schedule}
              onChange={(v) => {
                setField("cron_schedule", v);
                setCronError(null);
              }}
              error={cronError}
            />
          )}
        </div>
      ),
    },
  ];

  return (
    <SplitModal
      title={isEdit ? "Edit Script" : "Add Script"}
      subtitle={isEdit ? script?.name : "Configure a runnable script"}
      sections={sections}
      onClose={onClose}
      footer={
        <>
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-lg hover:bg-surface-hover text-text-muted transition-colors"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSubmit}
            disabled={saving || !form.name.trim() || !form.command.trim()}
            className="px-4 py-2 text-sm rounded-lg bg-accent hover:bg-accent-hover text-white font-medium disabled:opacity-50"
          >
            {saving ? "Saving…" : isEdit ? "Save Changes" : "Create Script"}
          </button>
        </>
      }
    />
  );
}
