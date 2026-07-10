import { useEffect } from "react";
import { Routes, Route } from "react-router-dom";
import { Layout } from "@/components/Layout";
import { Dashboard } from "@/pages/Dashboard";
import { ServiceDetail } from "@/pages/ServiceDetail";
import { StackDetail } from "@/pages/StackDetail";
import { Scripts } from "@/pages/Scripts";
import { Diagnostics } from "@/pages/Diagnostics";
import { FAQ } from "@/pages/FAQ";
import { Settings } from "@/pages/Settings";
import { useStatusStream } from "@/hooks/useStatusStream";
import { useThemeStore } from "@/store/theme";

export function App() {
  useStatusStream();

  // Pull any server-stored theme (set here or via MCP) once on load.
  useEffect(() => {
    useThemeStore.getState().hydrateFromServer();
  }, []);

  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Dashboard />} />
        <Route path="services/:id" element={<ServiceDetail />} />
        <Route path="stacks/:id" element={<StackDetail />} />
        <Route path="scripts" element={<Scripts />} />
        <Route path="diagnostics" element={<Diagnostics />} />
        <Route path="faq" element={<FAQ />} />
        <Route path="settings" element={<Settings />} />
      </Route>
    </Routes>
  );
}
