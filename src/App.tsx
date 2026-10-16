import { BrowserRouter, Routes, Route } from 'react-router-dom'
import { AppShell } from './components/layout'
import { SnapshotProvider } from './hooks/useMonitor'
import Dashboard from './pages/Dashboard'
import Slots from './pages/Slots'
import Performance from './pages/Performance'
import Hardware from './pages/Hardware'
import Settings from './pages/Settings'

function App() {
  return (
    <BrowserRouter>
      <SnapshotProvider>
        <AppShell>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/slots" element={<Slots />} />
            <Route path="/performance" element={<Performance />} />
            <Route path="/hardware" element={<Hardware />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </AppShell>
      </SnapshotProvider>
    </BrowserRouter>
  )
}

export default App
