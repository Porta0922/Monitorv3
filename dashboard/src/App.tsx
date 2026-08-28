import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useAuth } from './hooks/useAuth';
import { ErrorBoundary } from './components/ErrorBoundary';
import { LoginPage } from './pages/LoginPage';
import { DashboardPage } from './pages/DashboardPage';
import { ActivityPage } from './pages/ActivityPage';
import { InventoryPage } from './pages/InventoryPage';
import { USBPage } from './pages/USBPage';
import { AlertsPage } from './pages/AlertsPage';
import { SecurityPage } from './pages/SecurityPage';
import { DeviceDetailPage } from './pages/DeviceDetailPage';
import { MetricsPage } from './pages/MetricsPage';
import { WifiPage } from './pages/WifiPage';

function PrivateRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[#0a0e27] text-[#00d9ff]">
        <div className="cyber-card rounded-xl px-8 py-5 font-display text-lg">Syncing Nexus...</div>
      </div>
    );
  }

  return isAuthenticated ? <>{children}</> : <Navigate to="/login" />;
}

function App() {
  return (
    <ErrorBoundary>
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/dashboard"
          element={
            <PrivateRoute>
              <DashboardPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/devices/:deviceId"
          element={
            <PrivateRoute>
              <DeviceDetailPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/activity"
          element={
            <PrivateRoute>
              <ActivityPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/inventory"
          element={
            <PrivateRoute>
              <InventoryPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/usb"
          element={
            <PrivateRoute>
              <USBPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/security"
          element={
            <PrivateRoute>
              <SecurityPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/alerts"
          element={
            <PrivateRoute>
              <AlertsPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/metrics"
          element={
            <PrivateRoute>
              <MetricsPage />
            </PrivateRoute>
          }
        />
        <Route
          path="/wifi"
          element={
            <PrivateRoute>
              <WifiPage />
            </PrivateRoute>
          }
        />
        <Route path="/" element={<Navigate to="/dashboard" />} />
      </Routes>
    </BrowserRouter>
    </ErrorBoundary>
  );
}

export default App;
