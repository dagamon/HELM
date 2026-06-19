import { useState } from "react";
import type { Service, ServiceCreate } from "@/api/types";
import { useServices } from "@/store/services";
import { Modal } from "./Modal";
import { Field, Input, TextArea, Select, Checkbox } from "./FormField";
import { KeyValueEditor } from "./KeyValueEditor";

const TYPE_OPTIONS = [
  { value: "custom", label: "Custom" },
  { value: "python", label: "Python" },
  { value: "node", label: "Node.js" },
  { value: "rust", label: "Rust" },
  { value: "exe", label: "Executable" },
  { value: "url", label: "URL Monitor" },
];

const PLATFORM_OPTIONS = [
  { value: "all", label: "All" },
  { value: "windows", label: "Windows" },
  { value: "linux", label: "Linux" },
];

interface Props {
  service?: Service;
  onClose: () => void;
}

export function ServiceModal({ service, onClose }: Props) {
  const { services, create, update } = useServices();
  const isEdit = !!service;

  const [form, setForm] = useState({
    name: service?.name ?? "",
    description: service?.description ?? "",
    type: service?.type ?? "custom",
    command: service?.command ?? "",
    cwd: service?.cwd ?? "",
    venv_path: service?.venv_path ?? "",
    args: service?.args?.join(" ") ?? "",
    env: service?.env ?? ({} as Record<string, string>),
    url: service?.url ?? "",
    platform: service?.platform ?? "all",
    auto_start: service?.auto_start ?? false,
    restart_on_crash: service?.restart_on_crash ?? false,
    tags: service?.tags?.join(", ") ?? "",
    depends_on: service?.depends_on ?? ([] as number[]),
    webhook_url: service?.webhook_url ?? "",
    manifest_path: service?.manifest_path ?? "",
    binary_path: service?.binary_path ?? "",
    cargo_profile: service?.cargo_profile ?? "release",
    cargo_features: service?.cargo_features ?? "",
    prebuild: service?.prebuild ?? false,
  });

  const [saving, setSaving] = useState(false);

  const set = (key: string, value: string | boolean) =>
    setForm((f) => ({ ...f, [key]: value }));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);

    const data: ServiceCreate = {
      name: form.name,
      description: form.description || null,
      type: form.type,
      command: form.command || null,
      cwd: form.cwd || null,
      venv_path: form.venv_path || null,
      args: form.args ? form.args.split(/\s+/).filter(Boolean) : null,
      env: Object.keys(form.env).length > 0 ? form.env : null,
      url: form.url || null,
      platform: form.platform,
      auto_start: form.auto_start,
      restart_on_crash: form.restart_on_crash,
      tags: form.tags ? form.tags.split(",").map((t) => t.trim()).filter(Boolean) : null,
      depends_on: form.depends_on.length > 0 ? form.depends_on : null,
      webhook_url: form.webhook_url || null,
      manifest_path: form.type === "rust" ? form.manifest_path || null : null,
      binary_path: form.type === "rust" ? form.binary_path || null : null,
      cargo_profile: form.type === "rust" ? form.cargo_profile || "release" : null,
      cargo_features: form.type === "rust" ? form.cargo_features || null : null,
      prebuild: form.type === "rust" ? form.prebuild : false,
    };

    try {
      if (isEdit) {
        await update(service.id, data);
      } else {
        await create(data);
      }
      onClose();
    } catch (err) {
      console.error(err);
      setSaving(false);
    }
  };

  return (
    <Modal title={isEdit ? "Edit Service" : "Add Service"} onClose={onClose}>
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <Field label="Name">
            <Input
              value={form.name}
              onChange={(e) => set("name", e.currentTarget.value)}
              placeholder="My Service"
              required
            />
          </Field>
          <Field label="Type">
            <Select
              value={form.type}
              onChange={(e) => set("type", e.currentTarget.value)}
              options={TYPE_OPTIONS}
            />
          </Field>
        </div>

        <Field label="Description">
          <TextArea
            value={form.description}
            onChange={(e) => set("description", e.currentTarget.value)}
            placeholder="Optional description"
            rows={2}
          />
        </Field>

        <Field label="Command">
          <Input
            value={form.command}
            onChange={(e) => set("command", e.currentTarget.value)}
            placeholder="python app.py"
          />
        </Field>

        <div className="grid grid-cols-2 gap-4">
          <Field label="Working Directory">
            <Input
              value={form.cwd}
              onChange={(e) => set("cwd", e.currentTarget.value)}
              placeholder="/path/to/dir"
            />
          </Field>
          <Field label="Virtual Environment">
            <Input
              value={form.venv_path}
              onChange={(e) => set("venv_path", e.currentTarget.value)}
              placeholder="C:\project\.venv"
            />
          </Field>
        </div>

        <Field label="Arguments (space-separated)">
          <Input
            value={form.args}
            onChange={(e) => set("args", e.currentTarget.value)}
            placeholder="--port 8080"
          />
        </Field>

        <Field label="Environment Variables">
          <KeyValueEditor
            value={form.env}
            onChange={(env) => setForm((f) => ({ ...f, env }))}
          />
        </Field>

        <div className="grid grid-cols-2 gap-4">
          <Field label="URL (for monitoring)">
            <Input
              value={form.url}
              onChange={(e) => set("url", e.currentTarget.value)}
              placeholder="http://localhost:3000"
            />
          </Field>
          <Field label="Platform">
            <Select
              value={form.platform}
              onChange={(e) => set("platform", e.currentTarget.value)}
              options={PLATFORM_OPTIONS}
            />
          </Field>
        </div>

        <Field label="Tags (comma-separated)">
          <Input
            value={form.tags}
            onChange={(e) => set("tags", e.currentTarget.value)}
            placeholder="backend, api"
          />
        </Field>

        <Field label="Dependencies (start only after these are running)">
          <div className="flex flex-wrap gap-2">
            {services
              .filter((s) => s.id !== service?.id)
              .map((s) => {
                const selected = form.depends_on.includes(s.id);
                return (
                  <button
                    key={s.id}
                    type="button"
                    onClick={() =>
                      setForm((f) => ({
                        ...f,
                        depends_on: selected
                          ? f.depends_on.filter((id) => id !== s.id)
                          : [...f.depends_on, s.id],
                      }))
                    }
                    className={`px-2.5 py-1 text-xs rounded-md transition-colors ${
                      selected
                        ? "bg-accent/20 text-accent border border-accent/40"
                        : "bg-bg border border-border text-text-muted hover:text-text"
                    }`}
                  >
                    {s.name}
                  </button>
                );
              })}
            {services.filter((s) => s.id !== service?.id).length === 0 && (
              <span className="text-xs text-text-muted">No other services</span>
            )}
          </div>
        </Field>

        {form.type === "rust" && (
          <div className="space-y-4 rounded-lg border border-border bg-bg/40 p-4">
            <div className="text-xs uppercase tracking-wide text-text-muted">Rust runtime</div>
            <div className="grid grid-cols-2 gap-4">
              <Field label="Manifest Path (Cargo.toml)">
                <Input
                  value={form.manifest_path}
                  onChange={(e) => set("manifest_path", e.currentTarget.value)}
                  placeholder="/path/to/Cargo.toml"
                />
              </Field>
              <Field label="Binary Path (skip cargo if exists)">
                <Input
                  value={form.binary_path}
                  onChange={(e) => set("binary_path", e.currentTarget.value)}
                  placeholder="/path/to/target/release/app"
                />
              </Field>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <Field label="Cargo Profile">
                <Input
                  value={form.cargo_profile}
                  onChange={(e) => set("cargo_profile", e.currentTarget.value)}
                  placeholder="release"
                />
              </Field>
              <Field label="Cargo Features (comma-separated)">
                <Input
                  value={form.cargo_features}
                  onChange={(e) => set("cargo_features", e.currentTarget.value)}
                  placeholder="feature1,feature2"
                />
              </Field>
            </div>
            <Checkbox
              label="Prebuild (cargo build before start)"
              checked={form.prebuild}
              onChange={(v) => set("prebuild", v)}
            />
          </div>
        )}

        <Field label="Webhook URL (notified on crash)">
          <Input
            value={form.webhook_url}
            onChange={(e) => set("webhook_url", e.currentTarget.value)}
            placeholder="https://hooks.example.com/crash"
          />
        </Field>

        <div className="flex gap-6">
          <Checkbox
            label="Auto Start"
            checked={form.auto_start}
            onChange={(v) => set("auto_start", v)}
          />
          <Checkbox
            label="Restart on Crash"
            checked={form.restart_on_crash}
            onChange={(v) => set("restart_on_crash", v)}
          />
        </div>

        <div className="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-lg hover:bg-surface-hover text-text-muted"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={saving || !form.name}
            className="px-4 py-2 text-sm rounded-lg bg-accent hover:bg-accent-hover text-white font-medium disabled:opacity-50"
          >
            {saving ? "Saving..." : isEdit ? "Save Changes" : "Create Service"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
