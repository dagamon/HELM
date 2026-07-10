import { useState } from "react";
import { Info } from "lucide-react";
import type { Stack, StackCreate } from "@/api/types";
import { useStacks } from "@/store/stacks";
import { SplitModal, type SplitSection } from "./SplitModal";
import { Field, Input, TextArea } from "./FormField";

interface Props {
  stack?: Stack;
  onClose: () => void;
  onCreated?: (stack: Stack) => void;
}

export function StackModal({ stack, onClose, onCreated }: Props) {
  const { create, update } = useStacks();
  const isEdit = !!stack;

  const [form, setForm] = useState({
    name: stack?.name ?? "",
    description: stack?.description ?? "",
    tags: stack?.tags?.join(", ") ?? "",
  });
  const [saving, setSaving] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);

    const data: StackCreate = {
      name: form.name,
      description: form.description || null,
      tags: form.tags
        ? form.tags.split(",").map((t) => t.trim()).filter(Boolean)
        : null,
    };

    try {
      if (isEdit) {
        await update(stack.id, data);
      } else {
        const created = await create(data);
        onCreated?.(created);
      }
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
      hint: "A stack groups services that start and stop together.",
      content: (
        <div className="space-y-4">
          <Field label="Name">
            <Input
              value={form.name}
              onChange={(e) => {
                const name = e.currentTarget.value;
                setForm((f) => ({ ...f, name }));
              }}
              placeholder="My App"
              required
              autoFocus
            />
          </Field>
          <Field label="Description">
            <TextArea
              value={form.description}
              onChange={(e) => {
                const description = e.currentTarget.value;
                setForm((f) => ({ ...f, description }));
              }}
              placeholder="Optional description"
              rows={3}
            />
          </Field>
          <Field label="Tags (comma-separated)">
            <Input
              value={form.tags}
              onChange={(e) => {
                const tags = e.currentTarget.value;
                setForm((f) => ({ ...f, tags }));
              }}
              placeholder="app, production"
            />
          </Field>
        </div>
      ),
    },
  ];

  return (
    <SplitModal
      title={isEdit ? "Edit Stack" : "Add Stack"}
      subtitle={isEdit ? stack?.name : "Group services into one unit"}
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
            disabled={saving || !form.name.trim()}
            className="px-4 py-2 text-sm rounded-lg bg-accent hover:bg-accent-hover text-white font-medium disabled:opacity-50"
          >
            {saving ? "Saving…" : isEdit ? "Save Changes" : "Create Stack"}
          </button>
        </>
      }
    />
  );
}
