'use client'; // Bu satırı eklediğinizden emin olun
import { SorobanReactProvider } from '@soroban-react/core';
import { freighter } from '@soroban-react/freighter';
import { futurenet, sandbox, standalone, testnet } from '@soroban-react/chains';

const chains = [testnet, futurenet, sandbox, standalone];
const connectors = [freighter()];

export function MySorobanProvider({ children }) {
    return (
        <SorobanReactProvider
            chains={chains}
            appName={"My Stellar App"}
            connectors={connectors}
            defaultChain={testnet}
            activeChain={testnet}
        >
            {children}
        </SorobanReactProvider>
    );
}