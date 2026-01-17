import React from 'react';
import { AlertCircle } from 'lucide-react';
import { useAuth } from '../contexts/AuthContext';

export const SessionExpiredModal: React.FC = () => {
  const { sessionExpired, confirmSessionExpired } = useAuth();

  if (!sessionExpired) return null;

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[200]">
      <div className="glass-strong rounded-2xl w-full max-w-sm mx-4 overflow-hidden">
        {/* Header */}
        <div className="p-6 text-center">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-red-500/20 text-red-400 mb-4">
            <AlertCircle className="w-8 h-8" />
          </div>
          <h3 className="text-xl font-semibold text-white mb-2">Session Expired</h3>
          <p className="text-gray-400 text-sm">
            Your session has expired. Please log in again to continue.
          </p>
        </div>

        {/* Action */}
        <div className="px-6 pb-6">
          <button
            onClick={confirmSessionExpired}
            className="w-full btn-sui py-3 text-sm font-medium"
          >
            OK
          </button>
        </div>
      </div>
    </div>
  );
};
