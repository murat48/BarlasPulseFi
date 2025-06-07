'use client';
import { useSorobanReact } from '@soroban-react/core';

export default function ConnectWallet() {
    const { connect, disconnect, address } = useSorobanReact();

    // Fonksiyonları async olarak güncelle
    const handleConnect = async () => {
        try {
            await connect();
        } catch (error) {
            console.error('Connection error:', error);
        }
    };

    return (
        <div className="flex gap-4 items-center">
            {address ? (
                <>
                    <span className="text-sm">{`${address.slice(0, 6)}...${address.slice(-4)}`}</span>
                    <button
                        onClick={disconnect}
                        className="bg-red-500 text-white px-4 py-2 rounded hover:bg-red-600"
                    >
                        Disconnect
                    </button>
                </>
            ) : (
                <button
                    onClick={handleConnect}
                    className="bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600"
                >
                    Connect Wallet
                </button>
            )}
        </div>
    );
}