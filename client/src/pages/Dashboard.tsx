import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { FolderPlus, Plus, Search, X } from "lucide-react";
import type { Stack } from "@/api/types";
import { useServices } from "@/store/services";
import { useStacks } from "@/store/stacks";
import { ServiceCard } from "@/components/ServiceCard";
import { StackCard } from "@/components/StackCard";
import { GlobalMetricsBar } from "@/components/GlobalMetrics";
import { ServiceModal } from "@/components/modals/ServiceModal";
import { StackModal } from "@/components/modals/StackModal";
import { TagsDropdown } from "@/components/TagsDropdown";

const STATUS_OPTIONS = ["all", "running", "stopped", "crashed"] as const;

export function Dashboard() {
  const { services, loading, fetch, moveService, persistOrder, update } =
    useServices();
  const { stacks, fetch: fetchStacks } = useStacks();
  const [showAdd, setShowAdd] = useState(false);
  const [showAddStack, setShowAddStack] = useState(false);
  const [editingStack, setEditingStack] = useState<Stack | null>(null);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [dragId, setDragId] = useState<number | null>(null);

  useEffect(() => {
    fetch();
    fetchStacks();
  }, [fetch, fetchStacks]);

  const allTags = useMemo(() => {
    const set = new Set<string>();
    for (const svc of services) {
      svc.tags?.forEach((t) => set.add(t));
    }
    for (const st of stacks) {
      st.tags?.forEach((t) => set.add(t));
    }
    return [...set].sort();
  }, [services, stacks]);

  const statusCounts = useMemo(() => {
    const counts: Record<string, number> = { all: services.length };
    for (const svc of services) {
      counts[svc.status] = (counts[svc.status] || 0) + 1;
    }
    return counts;
  }, [services]);

  const filterActive =
    search !== "" || statusFilter !== "all" || selectedTags.length > 0;

  // With no filters: dashboard shows stacks + ungrouped services.
  // With filters: a flat, global search across all services wins over grouping.
  const filtered = useMemo(() => {
    return services.filter((svc) => {
      if (!filterActive && svc.stack_id != null) return false;
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
  }, [services, filterActive, search, statusFilter, selectedTags]);

  // A stack stays visible when its name matches the search, its aggregate
  // status is compatible with the status tab, and its own tags or any member
  // service's tags match the tag filter.
  const visibleStacks = useMemo(() => {
    if (!filterActive) return stacks;
    return stacks.filter((st) => {
      if (search && !st.name.toLowerCase().includes(search.toLowerCase()))
        return false;
      const members = services.filter((s) => s.stack_id === st.id);
      if (statusFilter !== "all") {
        if (statusFilter === "crashed") return false;
        const running = members.filter((s) => s.status === "running").length;
        if (statusFilter === "running" && running === 0) return false;
        if (statusFilter === "stopped" && running === members.length && members.length > 0)
          return false;
      }
      if (selectedTags.length > 0) {
        const tagPool = new Set([
          ...(st.tags ?? []),
          ...members.flatMap((m) => m.tags ?? []),
        ]);
        if (!selectedTags.some((t) => tagPool.has(t))) return false;
      }
      return true;
    });
  }, [stacks, services, filterActive, statusFilter, selectedTags, search]);

  // Manual drag-reorder only makes sense on the full, unfiltered list where the
  // visible order maps 1:1 to the persisted order.
  const dndEnabled = !filterActive;

  const toggleTag = (tag: string) => {
    setSelectedTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
    );
  };

  const hasAnything = services.length > 0 || stacks.length > 0;

  // Dropping a dragged service card onto a stack card adds it to that stack.
  const assignToStack = async (serviceId: number, stackId: number) => {
    setDragId(null);
    try {
      await update(serviceId, { stack_id: stackId });
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div className="max-w-6xl mx-auto">
      {/* Filter bar */}
      {hasAnything ? (
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

          {/* Add buttons — ghost style */}
          <button
            onClick={() => setShowAddStack(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
          >
            <FolderPlus className="w-3.5 h-3.5" />
            Add Stack
          </button>
          <button
            onClick={() => setShowAdd(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            Add Service
          </button>
        </div>
      ) : (
        <div className="flex justify-end gap-2.5 mb-6">
          <button
            onClick={() => setShowAddStack(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
          >
            <FolderPlus className="w-3.5 h-3.5" />
            Add Stack
          </button>
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
      {loading && !hasAnything ? (
        <div className="text-text-muted text-sm">Loading...</div>
      ) : !hasAnything ? (
        <div className="text-center py-20 text-text-muted">
          <p className="text-lg mb-2">No services yet</p>
          <p className="text-sm">Click "Add Service" to get started</p>
        </div>
      ) : (
        <>
          {/* Stacks */}
          {visibleStacks.length > 0 && (
            <>
              <div className="text-[11px] font-medium text-text-tertiary mb-2.5">
                Stacks
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-5 mb-8">
                {visibleStacks.map((st) => (
                  <StackCard
                    key={st.id}
                    stack={st}
                    onEdit={() => setEditingStack(st)}
                    dropActive={dragId != null}
                    onDropService={(sid) => assignToStack(sid, st.id)}
                  />
                ))}
              </div>
            </>
          )}

          {/* Services */}
          {visibleStacks.length > 0 && filtered.length > 0 && (
            <div className="text-[11px] font-medium text-text-tertiary mb-2.5">
              Services
            </div>
          )}
          {filtered.length === 0 && visibleStacks.length === 0 ? (
            <div className="text-center py-12 text-text-muted text-sm">
              No services match the current filters
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-5">
              {filtered.map((svc) => (
                <motion.div
                  key={svc.id}
                  layout
                  transition={{ type: "spring", stiffness: 400, damping: 32 }}
                >
                  <ServiceCard
                    service={svc}
                    drag={
                      dndEnabled
                        ? {
                            onDragStart: () => setDragId(svc.id),
                            // Live preview: reorder immediately while hovering,
                            // the layout spring animates cards into place.
                            onDragEnter: () => {
                              if (dragId != null && dragId !== svc.id)
                                moveService(dragId, svc.id);
                            },
                            onDrop: () => {},
                            onDragEnd: () => {
                              setDragId(null);
                              persistOrder();
                            },
                            dragging: dragId === svc.id,
                            dropTarget: false,
                          }
                        : undefined
                    }
                  />
                </motion.div>
              ))}
            </div>
          )}
        </>
      )}

      {showAdd && <ServiceModal onClose={() => setShowAdd(false)} />}
      {showAddStack && <StackModal onClose={() => setShowAddStack(false)} />}
      {editingStack && (
        <StackModal
          stack={editingStack}
          onClose={() => setEditingStack(null)}
        />
      )}
    </div>
  );
}
