import { Routes, Route } from "react-router-dom";
import { Layout } from "@/components/Layout";
import { Dashboard } from "@/pages/Dashboard";
import { ServiceDetail } from "@/pages/ServiceDetail";
import { Scripts } from "@/pages/Scripts";
import { FAQ } from "@/pages/FAQ";
import { Settings } from "@/pages/Settings";
import { Agents } from "@/pages/Agents";
import { useStatusStream } from "@/hooks/useStatusStream";

export function App() {
  useStatusStream();

  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Dashboard />} />
        <Route path="services/:id" element={<ServiceDetail />} />
        <Route path="scripts" element={<Scripts />} />
        <Route path="agents" element={<Agents />} />
        <Route path="faq" element={<FAQ />} />
        <Route path="settings" element={<Settings />} />
      </Route>
    </Routes>
  );
}
