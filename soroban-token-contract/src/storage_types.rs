use soroban_sdk::{contracttype, Address};

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub(crate) const BALANCE_LIFETIME_THRESHOLD: u32 = BALANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;
// Kontrat sabitlerini tanımlama
pub(crate) const REWARD_PRECISION: i128 = 10000; // Ödül hesaplamaları için hassasiyet faktörü
#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}
#[contracttype]
pub struct VestingSchedule {
    pub beneficiary: Address,       // Vesting planından faydalanacak kişi
    pub total_amount: i128,         // Toplam hak edilecek token miktarı
    pub claimed_amount: i128,       // Şu ana kadar çekilmiş miktar
    pub start_ledger: u32,          // Vesting'in başlayacağı ledger
    pub cliff_ledger: u32,          // Cliff süresi (0 ise cliff yok)
    pub end_ledger: u32,            // Vesting'in biteceği ledger
}
#[contracttype]
pub struct StakeInfo {
    pub amount: i128,           // Stake edilen token miktarı
    pub since_ledger: u32,      // Stake edildiği ledger numarası
    pub last_claim_ledger: u32, // Son ödül çekim ledger numarası
}
#[contracttype]
pub struct PoolInfo {
    pub token_id: Address,  
    pub reward_token_id: Address,    // Stake edilecek token adresi (bu, kontratın kendi tokeni olabilir)
    pub reward_rate: u32,       // Ödül oranı (her 10000 ledger başına birim başına ödül)
    pub total_staked: i128,     // Toplam stake edilen miktar
    pub min_stake_duration: u32, // Minimum stake süresi (ledger sayısı cinsinden)
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct LendingPool {
    pub total_supplied: i128,        // Toplam yatırılan miktar
    pub total_borrowed: i128,        // Toplam ödünç alınan miktar
    pub supply_rate: u32,            // Yıllık faiz oranı (baz puan olarak)
    pub borrow_rate: u32,            // Ödünç alma faiz oranı
    pub utilization_rate: u32,       // Kullanım oranı (%)
    pub reserve_factor: u32,         // Rezerv faktörü (%)
    pub last_update_ledger: u32,     // Son güncelleme ledger'ı
    pub collateral_factor: u32,      // Teminat faktörü (%)
}

// Kullanıcı Supply bilgisi
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct UserSupply {
    pub amount: i128,                // Yatırılan miktar
    pub last_update_ledger: u32,     // Son güncelleme ledger'ı
    pub accrued_interest: i128,      // Birikmiş faiz
}

// Kullanıcı Borrow bilgisi
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct UserBorrow {
    pub amount: i128,                // Ödünç alınan miktar
    pub last_update_ledger: u32,     // Son güncelleme ledger'ı
    pub accrued_interest: i128,      // Birikmiş faiz
    pub collateral_deposited: i128,  // Yatırılan teminat
}
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Allowance(AllowanceDataKey),
    Balance(Address),
    Nonce(Address),
    State(Address),
    Admin,
    Frozen(Address), 
    VestingSchedule(Address),
    StakeInfo(Address),
    PoolInfo,
    LendingPool,
    UserSupply,
    UserBorrow,
}