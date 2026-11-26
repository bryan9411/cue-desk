import { useEffect } from "react";
import { CreateSessionModal } from "./components/CreateSessionModal";
import { Sidebar } from "./components/sidebar/Sidebar";
import { subscribeSnapshots } from "./ipc/events";
import { useCue } from "./store/useCue";

function App() {
  useEffect(() => {
    const { applySnapshot, setLastError } = useCue.getState();
    return subscribeSnapshots(applySnapshot, setLastError);
  }, []);
  const createModal = useCue((s) => s.createModal);

  return (
    <main className="h-full">
      <Sidebar />
      {createModal && <CreateSessionModal workspaceId={createModal.workspaceId} />}
    </main>
  );
}

export default App;
