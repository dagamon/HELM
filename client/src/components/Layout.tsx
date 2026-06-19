import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { LayoutDashboard, ScrollText, Menu, X, Settings, HelpCircle, Anchor, Bot } from "lucide-react";

const NAV_MAIN = [
  { to: "/", icon: LayoutDashboard, label: "Dashboard" },
  { to: "/scripts", icon: ScrollText, label: "Scripts" },
  { to: "/agents", icon: Bot, label: "Agents" },
] as const;

const NAV_BOTTOM = [
  { to: "/faq", icon: HelpCircle, label: "FAQ" },
  { to: "/settings", icon: Settings, label: "Настройки" },
] as const;

export function Layout() {
  const [sidebarOpen, setSidebarOpen] = useState(false);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setSidebarOpen(false);
    };
    if (sidebarOpen) {
      window.addEventListener("keydown", handler);
      return () => window.removeEventListener("keydown", handler);
    }
  }, [sidebarOpen]);

  return (
    <div className="flex h-screen">
      {/* Sidebar — pushes content */}
      <aside
        className={`shrink-0 bg-surface/80 backdrop-blur-md border-r border-border flex flex-col overflow-hidden transition-all duration-200 ease-out ${
          sidebarOpen ? "w-56" : "w-0"
        }`}
      >
        <div className="h-12 flex items-center justify-between px-4 border-b border-border w-56">
          <div className="flex items-center gap-2">
            <Anchor className="w-4 h-4 text-accent" />
            <span className="font-semibold text-sm tracking-tight">HELM</span>
          </div>
          <button
            onClick={() => setSidebarOpen(false)}
            className="p-1.5 rounded-md hover:bg-surface-hover text-text-muted transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <nav className="flex-1 py-3 px-3 space-y-1 w-56">
          {NAV_MAIN.map(({ to, icon: Icon, label }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              onClick={() => setSidebarOpen(false)}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
                  isActive
                    ? "bg-surface-hover text-text font-medium"
                    : "text-text-muted hover:bg-surface-hover hover:text-text"
                }`
              }
            >
              <Icon className="w-4 h-4" />
              {label}
            </NavLink>
          ))}
        </nav>

        <div className="border-t border-border py-3 px-3 space-y-1 w-56">
          {NAV_BOTTOM.map(({ to, icon: Icon, label }) => (
            <NavLink
              key={to}
              to={to}
              onClick={() => setSidebarOpen(false)}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
                  isActive
                    ? "bg-surface-hover text-text font-medium"
                    : "text-text-muted hover:bg-surface-hover hover:text-text"
                }`
              }
            >
              <Icon className="w-4 h-4" />
              {label}
            </NavLink>
          ))}
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto px-10 py-6">
        {/* Hamburger — always rendered to preserve layout */}
        <div className="mb-5">
          <button
            onClick={() => setSidebarOpen(true)}
            className={`p-1.5 rounded-md hover:bg-surface-hover text-text-muted transition-all duration-200 ${
              sidebarOpen ? "opacity-0 pointer-events-none" : "opacity-100"
            }`}
          >
            <Menu className="w-5 h-5" />
          </button>
        </div>
        <Outlet />
      </main>
    </div>
  );
}
