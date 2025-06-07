# TokenNexus

<img src="/mr.png" alt="Profil Resmi" width="200"/>

## About Me

My name is Murat Keskin. I graduated from Kyiv Polytechnic University in 2015 with a degree in Business Administration and later from Anadolu University in 2025 with a degree in Computer Programming. I started investing in crypto in 2024, which sparked my interest in web3 technologies. Six months ago, I wrote my first code for the Internet Computer Protocol (ICP) and have been building ever since. I’m passionate about gaming, travel, and creating new opportunities through blockchain technology.

## Description

TokenNexus is a powerful token ecosystem built on the Scroll blockchain using Soroban smart contracts. It goes beyond basic token features to offer a complete financial system with strong security. Key features include secure token transfers, burning, approvals, and a flexible vesting system with cliffs and claims. It also offers customizable staking with adjustable rewards and lock periods. For extra safety, TokenNexus allows account freezing and strict authorization checks. Admins can manage settings and activate emergency functions when needed. Developed in Rust with Soroban SDK, it ensures efficient storage and full event tracking. TokenNexus is perfect for token projects, DeFi apps, and institutions needing a secure, flexible token solution with advanced distribution and reward tools.

## Vision

At TokenNexus, we aim to build the future of token infrastructure on the Stellar ecosystem. Our vision is to empower users and institutions with secure, transparent, and flexible tools that make complex finance simple. By combining enterprise-level security with easy-to-use features, we encourage long-term participation and help grow the Stellar network. TokenNexus bridges traditional and crypto finance, giving more people access to powerful financial tools. We strive to set a new industry standard by uniting the best practices of blockchain into one strong, smart token system for the future.

## Project RoadMap / Future Plans

1. Smart Contract Architecture Design

Define key modules: Token Operations, Vesting, Staking, Admin Controls.
Design data structures: user balances, vesting schedules, staking pools, frozen accounts.
Outline events: transfers, staking updates, vesting claims, account freezes.

2. Core Smart Contract Development (Rust + Soroban SDK)

Implement token functions: `transfer`, `burn`, `approve`, `freeze_account`, `unfreeze_account`.
Build vesting system: `create_vesting`, `claim_vested_tokens`, `check_vesting_schedule`.
Develop staking mechanism: `stake_tokens`, `unstake_tokens`, `calculate_rewards`.
Add admin features: `update_parameters`, `trigger_emergency`, `authorization checks`.

3. Smart Contract Testing & Security Review

Write unit tests for each function using Soroban's testing framework.
Perform security audits: validate authorization, emergency scenarios, storage management.

4. Front-End Interface Development

Build a responsive web app with user dashboards for token management, staking, vesting claims.
Integrate smart contract interaction (e.g., using Soroban.js or Web3 library).
Implement wallet connection, transaction signing, and event notifications.

5. Final System Testing (Integration + User Testing)

Connect front-end to smart contracts in a testnet environment.
Run full workflow tests: minting, transferring, staking, vesting, freezing.
Collect feedback and fine-tune UX/UI.

6. Deployment to Scroll Mainnet

Deploy smart contracts after final audit.
Launch front-end with mainnet connection.
Prepare documentation and community launch materials.

## Programming Language

Rust & Web3

 ## Contract Address 
 
- CDRPGQHJMHSLTXRN44KYLAGCOBH7GU4DLDIN5BPF7FZFWPYWAXH66ZDJ

## Setup Enviroment

Prerequisites

Rust (recommended version 1.70+)
Soroban CLI
Stellar Account with testnet funds

### Installation

 #### Install Rust (if not already installed)
- `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

 #### Clone the repository
 - `git clone https://github.com/murat48/stellar-token-contract.git`

 #### Add Wasm target for Rust
- `rustup target add wasm32-unknown-unknown`

 #### Install Soroban CLI
 - `cargo install --locked soroban-cli`

#### Build the contract
 - `cargo build --target wasm32-unknown-unknown --release`

#### Ledger
 - sudo apt install jq --- $(curl -s "https://horizon-testnet.stellar.org/" | jq '.core_latest_ledger')
   
# Deployment

 - `stellar keys generate --global alice --network testnet --fund (This you will be generate global Secret Key) `
 - `stellar keys address alice (This you will be show your secret key address) `

# Deploy the contract
   `contract deploy \
  --wasm target/wasm32-unknown-unknown/release/stellar_token_contract.wasm \
  --source alice \
  --network testnet `
This will output a contract ID that you should save for interacting with the contract.

# Usage
#### Initialize Token

- ` soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin \
  --network testnet \
  -- \
  initialize \
  --admin <ADMIN_ADDRESS> \
  --decimal 7 \
  --name "MyToken" \
  --symbol "MRT" `
  
#### Mint

- `soroban contract invoke \
  --id <CONTRACT_ID> \
  --source-account admin \
  --network testnet \
  -- \
  mint \
  --to <ADMIN_ADDRESS> \
  --amount 1000000000000000`
  
#### Create Vesting

- ` soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin \
  --network testnet \
  -- \
  create_vesting \
  --beneficiary ADDRESS \
  --amount 10000000 \
  --start_ledger $(curl -s "https://horizon-testnet.stellar.org/" | jq '.core_latest_ledger') \
  --cliff_ledger $(curl -s "https://horizon-testnet.stellar.org/" | jq '.core_latest_ledger + 17280') \
  --end_ledger $(curl -s "https://horizon-testnet.stellar.org/" | jq '.core_latest_ledger + 518400') `

  #### initialize staking
 
- ` soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin \
  --network testnet \
  -- \
  initialize_staking \
  --admin ADDRESS \
  --token_id <CONTRACT_ID2>  \
  --reward_token_id <CONTRACT_ID2>  \
  --reward_rate 100 \
  --min_stake_duration 17280 `

`
# Soroban Token Contract

This project is a standard token contract developed on the Soroban platform. Soroban is a smart contract platform that runs on the Stellar blockchain.

- Skipping install because wasm already installed
- Using wasm hash 906c93394fae9e56114d45e4603fac33bd385099fb5e5be02f14eba40c228fb6
- Simulating deploy transaction…
- Transaction hash is e2d96061e5b5d59023351edbb7ca9cca3178c64f329e15a7ebd1e91b32352aac
- https://stellar.expert/explorer/testnet/tx/e2d96061e5b5d59023351edbb7ca9cca3178c64f329e15a7ebd1e91b32352aac
- Signing transaction: e2d96061e5b5d59023351edbb7ca9cca3178c64f329e15a7ebd1e91b32352aac
- Submitting deploy transaction…
- https://stellar.expert/explorer/testnet/contract/CDRPGQHJMHSLTXRN44KYLAGCOBH7GU4DLDIN5BPF7FZFWPYWAXH66ZDJ
- Deployed!
CDRPGQHJMHSLTXRN44KYLAGCOBH7GU4DLDIN5BPF7FZFWPYWAXH66ZDJ
  # Result
  <img src="/contract.png" alt="Profil Resmi" width="400"/>
