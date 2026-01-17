import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { WalletProvider } from './contexts/WalletContext';
import { AppConfigProvider } from './contexts/AppConfigContext';
import { SessionExpiredModal } from './components/SessionExpiredModal';
import { Home } from './pages/Home';
import { MyAccount } from './pages/MyAccount';
import { AccountView } from './pages/AccountView';
import { Callback } from './pages/Callback';

function AppRoutes() {
  const { isAuthenticated, isLoading } = useAuth();

  // Wait for auth state to be loaded from localStorage before rendering routes
  if (isLoading) {
    return (
      <div className="min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  return (
    <Router>
      <Routes>
        {/* Public Routes - Home page accessible to everyone */}
        <Route path="/" element={<Home />} />

        {/* OAuth Callback - handles X login redirect */}
        <Route path="/callback" element={<Callback />} />

        {/* Public route - View any account by twitter_id */}
        <Route path="/account/:twitter_id" element={<AccountView />} />

        {/* Public route - View account tabs (activity, nfts) */}
        <Route path="/account/:twitter_id/:tab" element={<AccountView />} />

        {/* Public route - View transaction detail (opens modal on AccountView) */}
        <Route path="/account/:twitter_id/activities/tx/:digest" element={<AccountView />} />

        {/* Protected Routes - User's own account (requires X OAuth) */}
        <Route
          path="/profile"
          element={
            isAuthenticated ? <MyAccount /> : <Navigate to="/" replace />
          }
        />
        <Route
          path="/profile/:tab"
          element={
            isAuthenticated ? <MyAccount /> : <Navigate to="/" replace />
          }
        />

        {/* Catch all - redirect to home */}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Router>
  );
}

function App() {
  return (
    <AppConfigProvider>
      <AuthProvider>
        <WalletProvider>
          <AppRoutes />
          <SessionExpiredModal />
        </WalletProvider>
      </AuthProvider>
    </AppConfigProvider>
  );
}

export default App;
