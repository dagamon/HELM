import { createContext, useContext } from "react";

interface SidebarContextValue {
  openSidebar: () => void;
}

export const SidebarContext = createContext<SidebarContextValue>({
  openSidebar: () => {},
});

export const useSidebar = () => useContext(SidebarContext);
