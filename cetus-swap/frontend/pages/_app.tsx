import '@/styles/globals.css';
import type { AppProps } from 'next/app';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SuiClientProvider, WalletProvider } from '@mysten/dapp-kit';
import { getFullnodeUrl } from '@mysten/sui/client';
import '@mysten/dapp-kit/dist/index.css';

// Create a client
const queryClient = new QueryClient();

// Configure network
const networks = {
  mainnet: { url: getFullnodeUrl('mainnet') },
  testnet: { url: getFullnodeUrl('testnet') },
};

export default function App({ Component, pageProps }: AppProps) {
  return (
    <QueryClientProvider client={queryClient}>
      <SuiClientProvider networks={networks} defaultNetwork="mainnet">
        <WalletProvider autoConnect>
          <Component {...pageProps} />
        </WalletProvider>
      </SuiClientProvider>
    </QueryClientProvider>
  );
}
