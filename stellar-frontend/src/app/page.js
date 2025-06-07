import ConnectWallet from '@/components/ConnectWallet';
import TokenOperations from '@/components/TokenOperations';
import TransactionHistory from '@/components/TransactionHistory';

export default function Home() {
  const contractId = "CB75F5E2YFS2SHR4XWRIT2NTP6LHT3UDXPLXNLCPPDEZ4QLGNLDO2ASF";

  return (
    <main className="container mx-auto p-4 max-w-2xl">
      {/* <div className="flex justify-between items-center mb-8">
        <h1 className="text-2xl font-bold">My Stellar Token</h1>
        <ConnectWallet />
      </div> */}

      <TokenOperations contractId={contractId} />
      {/* <TransactionHistory /> */}
    </main>
  );
}