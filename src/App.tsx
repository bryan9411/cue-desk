import { useEffect } from "react";
import { Sidebar } from "./components/sidebar/Sidebar";
import { subscribeSnapshots } from "./ipc/events";
import { useCue } from "./store/useCue";

function App() {
  useEffect(() => {
    const { applySnapshot, setLastError } = useCue.getState();
    return subscribeSnapshots(applySnapshot, setLastError);
  }, []);

  return (
    <main className="h-full">
      <Sidebar />
    </main>
  );
}

export default App;
