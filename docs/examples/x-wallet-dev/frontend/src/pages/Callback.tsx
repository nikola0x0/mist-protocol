import { useEffect, useState, useRef } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useXAuth } from '../hooks/useXAuth';
import { useAuth } from '../contexts/AuthContext';
import { CheckCircle, XCircle, Loader2 } from 'lucide-react';

export const Callback: React.FC = () => {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { handleCallback, error: authError } = useXAuth();
  const { login } = useAuth();
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<'processing' | 'success' | 'error'>('processing');

  // Prevent double execution in StrictMode
  const hasProcessed = useRef(false);

  useEffect(() => {
    const processCallback = async () => {
      // Skip if already processed (StrictMode double-render protection)
      if (hasProcessed.current) return;
      hasProcessed.current = true;
      // Get OAuth parameters from URL
      const code = searchParams.get('code');
      const state = searchParams.get('state');
      const errorParam = searchParams.get('error');
      const errorDescription = searchParams.get('error_description');

      // Handle X OAuth errors
      if (errorParam) {
        setError(errorDescription || `OAuth error: ${errorParam}`);
        setStatus('error');
        return;
      }

      // Validate required parameters
      if (!code || !state) {
        setError('Missing authorization code or state parameter');
        setStatus('error');
        return;
      }

      try {
        // Exchange code for token and get user info
        const result = await handleCallback(code, state);

        // Update auth context with user info and tokens
        login(
          {
            twitterUserId: result.user.id,
            twitterHandle: result.user.username,
            avatarUrl: result.user.profile_image_url || null,
            suiObjectId: result.xwalletAccount?.sui_object_id || null,
            linkedWalletAddress: result.xwalletAccount?.owner_address || null,
          },
          {
            accessToken: result.accessToken,
            refreshToken: result.refreshToken,
          }
        );

        setStatus('success');

        // Redirect to dashboard after short delay
        setTimeout(() => {
          navigate('/dashboard', { replace: true });
        }, 1500);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Authentication failed');
        setStatus('error');
      }
    };

    processCallback();
  }, [searchParams, handleCallback, login, navigate]);

  return (
    <div className="min-h-screen flex items-center justify-center p-4">
      <div className="glass-strong rounded-2xl p-8 max-w-md w-full text-center">
        {status === 'processing' && (
          <>
            <div className="w-16 h-16 mx-auto mb-6 rounded-full bg-sui-500/20 flex items-center justify-center">
              <Loader2 className="w-8 h-8 text-sui-400 animate-spin" />
            </div>
            <h2 className="text-xl font-semibold text-white mb-2">
              Completing sign in...
            </h2>
            <p className="text-gray-400">
              Please wait while we verify your X account.
            </p>
          </>
        )}

        {status === 'success' && (
          <>
            <div className="w-16 h-16 mx-auto mb-6 rounded-full bg-cyber-green/20 flex items-center justify-center">
              <CheckCircle className="w-8 h-8 text-cyber-green" />
            </div>
            <h2 className="text-xl font-semibold text-white mb-2">
              Successfully signed in!
            </h2>
            <p className="text-gray-400">
              Redirecting to dashboard...
            </p>
            <div className="mt-4 flex justify-center">
              <div className="flex space-x-1">
                {[0, 1, 2].map((i) => (
                  <div
                    key={i}
                    className="w-2 h-2 rounded-full bg-cyber-green animate-pulse"
                    style={{ animationDelay: `${i * 0.2}s` }}
                  />
                ))}
              </div>
            </div>
          </>
        )}

        {status === 'error' && (
          <>
            <div className="w-16 h-16 mx-auto mb-6 rounded-full bg-red-500/20 flex items-center justify-center">
              <XCircle className="w-8 h-8 text-red-400" />
            </div>
            <h2 className="text-xl font-semibold text-white mb-2">
              Sign In Failed
            </h2>
            <p className="text-red-400 mb-6">
              {error || authError || 'An unknown error occurred'}
            </p>
            <button
              onClick={() => navigate('/', { replace: true })}
              className="btn-sui w-full"
            >
              Try Again
            </button>
          </>
        )}
      </div>
    </div>
  );
};
