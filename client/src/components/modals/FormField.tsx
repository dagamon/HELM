import type { InputHTMLAttributes, TextareaHTMLAttributes } from "react";

const inputCls =
  "w-full bg-transparent border border-border rounded-lg px-3 py-2 text-sm text-text placeholder:text-text-tertiary focus:outline-none focus:border-border-hover";

export function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="block">
      <span className="text-xs text-text-tertiary mb-1 block">{label}</span>
      {children}
    </label>
  );
}

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} className={`${inputCls} ${props.className ?? ""}`} />;
}

export function TextArea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      {...props}
      className={`${inputCls} resize-none ${props.className ?? ""}`}
    />
  );
}

export function Select({
  options,
  ...props
}: InputHTMLAttributes<HTMLSelectElement> & { options: { value: string; label: string }[] }) {
  return (
    <select {...(props as React.SelectHTMLAttributes<HTMLSelectElement>)} className={inputCls}>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function Checkbox({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 cursor-pointer text-sm">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="rounded border-border"
      />
      {label}
    </label>
  );
}
