import { useEffect } from "react"
import { CreateSessionModal } from "./components/CreateSessionModal"
import { Sidebar } from "./components/sidebar/Sidebar"
import { onBranchChanged, subscribeSnapshots } from "./ipc/events"
import { useCue } from "./store/useCue"

function App() {
  useEffect(() => {
    const { applySnapshot, applyBranchUpdate, setLastError } = useCue.getState()
    const stopSnapshots = subscribeSnapshots(applySnapshot, setLastError)
    const stopBranches = onBranchChanged(applyBranchUpdate)
    return () => {
      stopSnapshots()
      stopBranches()
    }
  }, [])
  const createModal = useCue((s) => s.createModal)

  return (
    <main className="h-full">
      <Sidebar />
      {createModal && <CreateSessionModal workspaceId={createModal.workspaceId} />}
    </main>
  )
}

export default App
