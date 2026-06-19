const VARIANTS: Record<string, string> = {
  running: "text-success",
  stopped: "text-text-tertiary",
  crashed: "text-danger",
  error: "text-danger",
  success: "text-success",
};

export function StatusBadge({ status }: { status: string }) {
  const cls = VARIANTS[status] ?? VARIANTS.stopped;
  return (
    <span className={`inline-flex items-center gap-1.5 text-xs font-medium ${cls}`}>
      <span className="w-1.5 h-1.5 rounded-full bg-current" />
      {status}
    </span>
  );
}
