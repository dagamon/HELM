import { useEffect, useMemo, useState } from "react";
import { Plus, Search, X } from "lucide-react";
import { useServices } from "@/store/services";
import { ServiceCard } from "@/components/ServiceCard";
import { GlobalMetricsBar } from "@/components/GlobalMetrics";
import { ServiceModal } from "@/components/modals/ServiceModal";
import { TagsDropdown } from "@/components/TagsDropdown";

const STATUS_OPTIONS = ["all", "running", "stopped", "crashed"] as const;

export function Dashboard() {
  const { services, loading, fetch } = useServices();
  const [showAdd, setShowAdd] = useState(false);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);

  useEffect(() => {
    fetch();
  }, [fetch]);

  const allTags = useMemo(() => {
    const set = new Set<string>();
    for (const svc of services) {
      svc.tags?.forEach((t) => set.add(t));
    }
    return [...set].sort();
  }, [services]);

  const statusCounts = useMemo(() => {
    const counts: Record<string, number> = { all: services.length };
    for (const svc of services) {
      counts[svc.status] = (counts[svc.status] || 0) + 1;
    }
    return counts;
  }, [services]);

  const filtered = useMemo(() => {
    return services.filter((svc) => {
      if (search && !svc.name.toLowerCase().includes(search.toLowerCase()))
        return false;
      if (statusFilter !== "all" && svc.status !== statusFilter) return false;
      if (
        selectedTags.length > 0 &&
        !selectedTags.some((t) => svc.tags?.includes(t))
      )
        return false;
      return true;
    });
  }, [services, search, statusFilter, selectedTags]);

  const toggleTag = (tag: string) => {
    setSelectedTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
    );
  };

  return (
    <div className="max-w-6xl mx-auto">
      {/* Filter bar */}
      {services.length > 0 ? (
        <div className="flex flex-wrap items-center gap-2.5 mb-6">
          {/* Search */}
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-tertiary" />
            <input
              type="text"
              placeholder="Filter services..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-8 pr-7 py-1.5 text-sm bg-transparent border border-border rounded-lg focus:outline-none focus:border-border-hover w-48 placeholder:text-text-tertiary"
            />
            {search && (
              <button
                onClick={() => setSearch("")}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-text-tertiary hover:text-text"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            )}
          </div>

          {/* Status tabs */}
          <div className="flex bg-surface rounded-lg p-0.5 border border-border">
            {STATUS_OPTIONS.map((s) => {
              const label =
                s === "all" ? "All" : s.charAt(0).toUpperCase() + s.slice(1);
              const count = statusCounts[s] || 0;
              return (
                <button
                  key={s}
                  onClick={() => setStatusFilter(s)}
                  className={`px-2.5 py-1 text-xs rounded-md transition-colors ${
                    statusFilter === s
                      ? "bg-surface-hover text-text"
                      : "text-text-tertiary hover:text-text-muted"
                  }`}
                >
                  {label}
                  {count > 0 && (
                    <span className="text-text-tertiary ml-1">{count}</span>
                  )}
                </button>
              );
            })}
          </div>

          {/* Tags dropdown */}
          {allTags.length > 0 && (
            <TagsDropdown
              tags={allTags}
              selected={selectedTags}
              onToggle={toggleTag}
            />
          )}

          {/* Metrics inline */}
          <GlobalMetricsBar services={services} />

          <div className="flex-1" />

          {/* Add button — ghost style */}
          <button
            onClick={() => setShowAdd(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            Add Service
          </button>
        </div>
      ) : (
        <div className="flex justify-end mb-6">
          <button
            onClick={() => setShowAdd(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            Add Service
          </button>
        </div>
      )}

      {/* Content */}
      {loading && services.length === 0 ? (
        <div className="text-text-muted text-sm">Loading...</div>
      ) : services.length === 0 ? (
        <div className="text-center py-20 text-text-muted">
          <p className="text-lg mb-2">No services yet</p>
          <p className="text-sm">Click "Add Service" to get started</p>
        </div>
      ) : filtered.length === 0 ? (
        <div className="text-center py-12 text-text-muted text-sm">
          No services match the current filters
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-5">
          {filtered.map((svc) => (
            <ServiceCard key={svc.id} service={svc} />
          ))}
        </div>
      )}

      {showAdd && <ServiceModal onClose={() => setShowAdd(false)} />}
    </div>
  );
}
