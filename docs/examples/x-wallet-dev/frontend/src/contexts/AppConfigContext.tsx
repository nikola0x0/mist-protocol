import React, { createContext, useContext, useState, useEffect } from 'react';
import { API_BASE_URL } from '../utils/constants';

interface AppConfig {
  sponsorEnabled: boolean;
  loading: boolean;
}

interface AppConfigContextType extends AppConfig {
  refetch: () => Promise<void>;
}

const AppConfigContext = createContext<AppConfigContextType | undefined>(undefined);

export const AppConfigProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [config, setConfig] = useState<AppConfig>({
    sponsorEnabled: true, // Default while loading
    loading: true,
  });

  const fetchConfig = async () => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/config`);
      if (response.ok) {
        const data = await response.json();
        const sponsorEnabled = typeof data.sponsor_enabled === 'boolean'
          ? data.sponsor_enabled
          : true; // fallback if invalid type
        setConfig({ sponsorEnabled, loading: false });
      } else {
        // Fallback to default on error
        setConfig({ sponsorEnabled: true, loading: false });
      }
    } catch {
      setConfig({ sponsorEnabled: true, loading: false });
    }
  };

  useEffect(() => {
    fetchConfig();
  }, []);

  return (
    <AppConfigContext.Provider value={{ ...config, refetch: fetchConfig }}>
      {children}
    </AppConfigContext.Provider>
  );
};

export const useAppConfig = () => {
  const context = useContext(AppConfigContext);
  if (!context) {
    throw new Error('useAppConfig must be used within an AppConfigProvider');
  }
  return context;
};
