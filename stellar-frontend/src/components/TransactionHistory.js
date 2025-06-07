'use client';
import { useEffect, useState } from 'react';
import { useSorobanReact } from '@soroban-react/core';
import { Horizon } from '@stellar/stellar-sdk';

export default function TransactionHistory() {
    const { address } = useSorobanReact();
    const [transactions, setTransactions] = useState([]);

    useEffect(() => {
        const fetchTransactions = async () => {
            if (!address) return;

            try {
                const horizon = new Horizon.Server('https://horizon-testnet.stellar.org');
                const response = await horizon
                    .transactions()
                    .forAccount(address)
                    .order('desc')
                    .limit(10)
                    .call();

                setTransactions(response.records);
            } catch (error) {
                console.error('Transaction history error:', error);
            }
        };

        fetchTransactions();
    }, [address]);

    return (
        <div className="mt-8 p-6 bg-white rounded-xl shadow-lg">
            <h3 className="text-xl font-bold mb-4 text-gray-800">Latest Transactions</h3>
            <div className="space-y-4">
                {transactions.map((tx) => (
                    <div
                        key={tx.id}
                        className="p-4 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors border border-gray-200"
                    >
                        <div className="flex justify-between items-start">
                            <div>
                                <p className="text-sm font-mono text-blue-600 break-all">
                                    {tx.id.slice(0, 12)}...{tx.id.slice(-6)}
                                </p>
                                <p className="text-xs text-gray-500 mt-1">
                                    {new Date(tx.created_at).toLocaleString()}
                                </p>
                            </div>
                            <span className="text-sm font-medium text-green-600">
                                {tx.successful ? '✅ Successful' : '❌ Unsuccessful'}
                            </span>
                        </div>
                        <div className="mt-2 text-sm text-gray-700">
                            <p>İşlem Ücreti: {(parseInt(tx.fee_charged) / 10000000).toFixed(7)} XLM</p>
                            <p className="mt-1">Transaction Hash: {tx.hash.slice(0, 10)}...</p>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}