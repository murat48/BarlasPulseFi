use crate::admin::{has_administrator, read_administrator, write_administrator};
use crate::allowance::{read_allowance, spend_allowance, write_allowance};
use crate::balance::{read_balance, receive_balance, spend_balance};
use crate::metadata::{read_decimal, read_name, read_symbol, write_metadata};
use crate::storage_types::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD};
use crate::storage_types::{DataKey, VestingSchedule,StakeInfo,PoolInfo};
use soroban_sdk::token::{self, Interface as _};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;
use soroban_sdk::{contract, contractimpl, Address, Env, String, Map};


///staking kodları

// Kontrat veri yapısı
#[contract]
pub struct StakingRewards;

// Verileri saklamak için kullanılacak anahtarlar
const POOL_INFO_KEY: &str = "pool_info";
const STAKES_KEY: &str = "stakes";
const ADMIN_KEY: &str = "admin";
const LENDING_POOL_KEY: &str = "lending_pool";

// Özel olayları yayınlamak için yardımcı fonksiyon
fn emit_event(e: &Env, event_type: &str, user: &Address, amount: i128) {
    e.events().publish((event_type, user.clone(), amount), ());
}
//////


fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
    }
}

// Bir hesabın dondurulup dondurulmadığını kontrol eden yardımcı fonksiyon
fn is_account_frozen(e: &Env, account: &Address) -> bool {
    let key = DataKey::Frozen(account.clone());
    e.storage().instance().get::<_, bool>(&key).unwrap_or(false)
}

// Özel olayları yayınlamak için yardımcı fonksiyon
fn emit_custom_event(e: &Env, event_type: &str, admin: Address, account: Address) {
    e.events().publish((event_type, admin, account), ());
}

// Vesting planı işlemleri için yardımcı fonksiyonlar
fn read_vesting_schedule(e: &Env, beneficiary: &Address) -> Option<VestingSchedule> {
    let key = DataKey::VestingSchedule(beneficiary.clone());
    e.storage().instance().get(&key)
}

fn write_vesting_schedule(e: &Env, beneficiary: &Address, schedule: &VestingSchedule) {
    let key = DataKey::VestingSchedule(beneficiary.clone());
    e.storage().instance().set(&key, schedule);
}

fn remove_vesting_schedule(e: &Env, beneficiary: &Address) {
    let key = DataKey::VestingSchedule(beneficiary.clone());
    e.storage().instance().remove(&key);
}

fn get_claimable_amount(e: &Env, beneficiary: &Address) -> i128 {
    if let Some(schedule) = read_vesting_schedule(e, beneficiary) {
        let current_ledger = e.ledger().sequence();
        
        // Eğer henüz başlangıç zamanına gelmemişse veya cliff zamanından önceyse
        if current_ledger < schedule.start_ledger || 
           (schedule.cliff_ledger > 0 && current_ledger < schedule.cliff_ledger) {
            return 0;
        }
        
        // Eğer bitiş zamanını geçtiyse, kalan tüm miktar çekilebilir
        if current_ledger >= schedule.end_ledger {
            return schedule.total_amount - schedule.claimed_amount;
        }
        
        // Lineer vesting: Geçen zamana orantılı olarak token miktarı hesaplanır
        let total_vesting_time = schedule.end_ledger - schedule.start_ledger;
        let elapsed_time = current_ledger - schedule.start_ledger;
        
        let claimable_amount = schedule.total_amount * elapsed_time as i128 / total_vesting_time as i128;
        
        // Şimdiye kadar çekilen miktarı çıkaralım
        if claimable_amount <= schedule.claimed_amount {
            return 0;
        }
        
        return claimable_amount - schedule.claimed_amount;
    }
    
    0 // Vesting planı yoksa çekilebilir miktar 0
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    pub fn initialize(e: Env, admin: Address, decimal: u32, name: String, symbol: String) {
        if has_administrator(&e) {
            panic!("already initialized")
        }
        write_administrator(&e, &admin);
        if decimal > u8::MAX.into() {
            panic!("Decimal must fit in a u8");
        }

        write_metadata(
            &e,
            TokenMetadata {
                decimal,
                name,
                symbol,
            },
        )
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        let admin = read_administrator(&e);
        admin.require_auth();

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        receive_balance(&e, to.clone(), amount);
        TokenUtils::new(&e).events().mint(admin, to, amount);
    }

    pub fn set_admin(e: Env, new_admin: Address) {
        let admin = read_administrator(&e);
        admin.require_auth();

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_administrator(&e, &new_admin);
        TokenUtils::new(&e).events().set_admin(admin, new_admin);
    }

    // Bir hesabı dondur (sadece yönetici yapabilir)
    pub fn freeze_account(e: Env, account: Address) {
        // Sadece yönetici hesapları dondurabilir
        let admin = read_administrator(&e);
        admin.require_auth();

        // Kontrat örneğinin TTL süresini uzat
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Hesabı dondurulmuş olarak ayarla
        let key = DataKey::Frozen(account.clone());
        e.storage().instance().set(&key, &true);

       // Dondurma olayını yayınla
       emit_custom_event(&e, "freeze_account", admin, account);
    }

    // Bir hesabın dondurulmasını kaldır (sadece yönetici yapabilir)
    pub fn unfreeze_account(e: Env, account: Address) {
        // Sadece yönetici hesapların dondurulmasını kaldırabilir
        let admin = read_administrator(&e);
        admin.require_auth();

        // Kontrat örneğinin TTL süresini uzat
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Dondurulmuş durumu kaldır
        let key = DataKey::Frozen(account.clone());
        e.storage().instance().remove(&key);

        // Dondurma kaldırma olayını yayınla
        emit_custom_event(&e, "unfreeze_account", admin, account);
    }

    pub fn create_vesting(
        e: Env,
        beneficiary: Address,
        amount: i128,
        start_ledger: u32,
        cliff_ledger: u32,
        end_ledger: u32,
    ) {
        let admin = read_administrator(&e);
        admin.require_auth();
        
        check_nonnegative_amount(amount);
        
        // Parametrelerin mantıklı olduğunu kontrol et
        if end_ledger <= start_ledger {
            panic!("Bitiş ledger'ı başlangıç ledger'ından büyük olmalıdır");
        }
        
        if cliff_ledger > 0 && cliff_ledger < start_ledger {
            panic!("Cliff ledger'ı başlangıç ledger'ından küçük olamaz");
        }
        
        // Yöneticinin yeterli token'a sahip olduğunu kontrol et
        let admin_balance = read_balance(&e, admin.clone());
        if admin_balance < amount {
            panic!("Yönetici vesting için yeterli token'a sahip değil");
        }
        
        // TTL süresini uzat
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        // Vesting planını kaydet
        let schedule = VestingSchedule {
            beneficiary: beneficiary.clone(),
            total_amount: amount,
            claimed_amount: 0,
            start_ledger,
            cliff_ledger,
            end_ledger,
        };
        
        write_vesting_schedule(&e, &beneficiary, &schedule);
        
        // Vesting için ayrılan token'ları yöneticiden al (dondurulmuş olarak saklanacak)
        spend_balance(&e, admin.clone(), amount);
        receive_balance(&e, beneficiary.clone(), amount);
        
        // Hesabı otomatik olarak dondur
        let key = DataKey::Frozen(beneficiary.clone());
        e.storage().instance().set(&key, &true);
        
        // Vesting oluşturma olayını yayınla
        emit_custom_event(&e, "create_vesting", admin, beneficiary);
    }
    
    // Vesting planından token'ları talep et (sadece faydalanıcı yapabilir)
    pub fn claim_vesting(e: Env, beneficiary: Address) -> i128 {
        beneficiary.require_auth();
        
        // TTL süresini uzat
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        // Çekilebilir miktarı hesapla
        let claimable_amount = get_claimable_amount(&e, &beneficiary);
        
        if claimable_amount <= 0 {
            panic!("Çekilebilir token bulunmamaktadır");
        }
        
        // Vesting planını güncelle
        if let Some(mut schedule) = read_vesting_schedule(&e, &beneficiary) {
            schedule.claimed_amount += claimable_amount;
            
            // Eğer tüm tokenlar çekildiyse vesting planını kaldır ve hesabı çöz
            if schedule.claimed_amount >= schedule.total_amount {
                remove_vesting_schedule(&e, &beneficiary);
                
                // Hesabın dondurulmasını kaldır
                let key = DataKey::Frozen(beneficiary.clone());
                e.storage().instance().remove(&key);
            } else {
                write_vesting_schedule(&e, &beneficiary, &schedule);
            }
            
            // Donmuş token'ların çözülmesini sağla
            let admin = read_administrator(&e);
            
            // Özel bir olay yayınla
            emit_custom_event(&e, "claim_vesting", admin, beneficiary);
            
            return claimable_amount;
        }
        
        panic!("Vesting planı bulunamadı");
    }
    
    // Faydalanıcı için vesting planı bilgilerini getir
    pub fn get_vesting_info(e: Env, beneficiary: Address) -> Option<VestingSchedule> {
        // TTL süresini uzat
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        read_vesting_schedule(&e, &beneficiary)
    }
    
    // Çekilebilir vesting miktarını hesapla
    pub fn get_claimable_vesting(e: Env, beneficiary: Address) -> i128 {
        // TTL süresini uzat
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        get_claimable_amount(&e, &beneficiary)
    }
    
    // Bir vesting planını iptal et (sadece admin yapabilir)
    pub fn revoke_vesting(e: Env, beneficiary: Address) {
        let admin = read_administrator(&e);
        admin.require_auth();
        
        // TTL süresini uzat
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        if let Some(schedule) = read_vesting_schedule(&e, &beneficiary) {
            // Henüz çekilmemiş token'ları admin'e geri transfer et
            let unclaimed_amount = schedule.total_amount - schedule.claimed_amount;
            if unclaimed_amount > 0 {
                spend_balance(&e, beneficiary.clone(), unclaimed_amount);
                receive_balance(&e, admin.clone(), unclaimed_amount);
            }
            
            // Vesting planını kaldır
            remove_vesting_schedule(&e, &beneficiary);
            
            // Hesabın dondurulmasını kaldır
            let key = DataKey::Frozen(beneficiary.clone());
            e.storage().instance().remove(&key);
            
            // İptal etme olayını yayınla
            emit_custom_event(&e, "revoke_vesting", admin, beneficiary);
        } else {
            panic!("Vesting planı bulunamadı");
        }
    }

     
///Staking kodları
    pub fn initialize_staking(
        e: Env,
        admin: Address,
        token_id: Address,
        reward_token_id: Address,
        reward_rate: u32,
        min_stake_duration: u32,
    ) {
        // Kontratın sadece bir kez başlatılabilmesini sağla
        if e.storage().instance().has(&ADMIN_KEY) {
            panic!("Contract already initialized");
        }
        
        // Admin adresini kaydet
        e.storage().instance().set(&ADMIN_KEY, &admin);
        
        // Havuz bilgilerini kaydet
        let pool_info = PoolInfo {
            token_id,
            reward_token_id,
            reward_rate,
            total_staked: 0,
            min_stake_duration,
        };
        e.storage().instance().set(&POOL_INFO_KEY, &pool_info);
        
        // Boş bir stake haritası oluştur
        let stakes: Map<Address, StakeInfo> = Map::new(&e);
        e.storage().instance().set(&STAKES_KEY, &stakes);
        
        // TTL süresini uzat
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        // Başlatma olayını yayınla
        emit_event(&e, "initialize", &admin, 0);
    }
    
    // Ödül oranını güncelleme (sadece admin yapabilir)
    pub fn update_reward_rate(e: Env, new_rate: u32) {
        // Admin kontrolü
        let admin: Address = e.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();
        
        // Havuz bilgilerini al ve güncelle
        let mut pool_info: PoolInfo = e.storage().instance().get(&POOL_INFO_KEY).unwrap();
        pool_info.reward_rate = new_rate;
        e.storage().instance().set(&POOL_INFO_KEY, &pool_info);
        
        // TTL süresini uzat
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        // Oranı güncelleme olayını yayınla
        emit_event(&e, "update_rate", &admin, new_rate as i128);
    }
    
    // Minimum stake süresini güncelleme (sadece admin yapabilir)
    pub fn update_min_stake_duration(e: Env, new_duration: u32) {
        // Admin kontrolü
        let admin: Address = e.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();
        
        // Havuz bilgilerini al ve güncelle
        let mut pool_info: PoolInfo = e.storage().instance().get(&POOL_INFO_KEY).unwrap();
        pool_info.min_stake_duration = new_duration;
        e.storage().instance().set(&POOL_INFO_KEY, &pool_info);
        
        // TTL süresini uzat
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        // Minimum süreyi güncelleme olayını yayınla
        emit_event(&e, "update_min_duration", &admin, new_duration as i128);
    }
    
    // Tokenları stake etme fonksiyonu
    pub fn stake(e: Env, user: Address, amount: i128) {
        // Kullanıcının yetkilendirmesini kontrol et
        user.require_auth();
        
        // Negatif miktar kontrolü
        if amount <= 0 {
            panic!("Stake amount must be positive");
        }
        
        // Havuz bilgilerini al
        let mut pool_info: PoolInfo = e.storage().instance().get(&POOL_INFO_KEY).unwrap();
        
        // Kullanıcının bakiyesini kontrol et
        let user_balance = read_balance(&e, user.clone());
        if user_balance < amount {
            panic!("Insufficient balance");
        }
        
        // Token transferini doğrudan depolama işlemleriyle yap (re-entry önlemek için)
        spend_balance(&e, user.clone(), amount);
        receive_balance(&e, e.current_contract_address(), amount);
        
        // Mevcut stake bilgilerini al veya yeni oluştur
        let mut stakes: Map<Address, StakeInfo> = e.storage().instance().get(&STAKES_KEY).unwrap();
        
        let current_ledger = e.ledger().sequence();
        
        if let Some(mut stake_info) = stakes.get(user.clone()) {
            // Eğer kullanıcının mevcut stake'i varsa, önce bekleyen ödülleri hesapla ve stake'i güncelle
            let pending_reward = Self::calculate_reward(&e, &user, &stake_info, &pool_info);
            
            // Varsa ödülleri gönder - burada da doğrudan depolama kullan
            if pending_reward > 0 {
                // Token transferi yerine direkt bakiye güncelle
                spend_balance(&e, e.current_contract_address(), pending_reward);
                receive_balance(&e, user.clone(), pending_reward);
                
                // Ödül çekme olayını yayınla
                emit_event(&e, "claim_reward", &user, pending_reward);
            }
            
            // Stake bilgisini güncelle
            stake_info.amount += amount;
            stake_info.last_claim_ledger = current_ledger;
            stakes.set(user.clone(), stake_info);
        } else {
            // Yeni stake oluştur
            let stake_info = StakeInfo {
                amount,
                since_ledger: current_ledger,
                last_claim_ledger: current_ledger,
            };
            stakes.set(user.clone(), stake_info);
        }
        
        // Toplam stake miktarını güncelle
        pool_info.total_staked += amount;
        
        // Güncellenmiş bilgileri kaydet
        e.storage().instance().set(&POOL_INFO_KEY, &pool_info);
        e.storage().instance().set(&STAKES_KEY, &stakes);
        
        // TTL süresini uzat
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        
        // Stake olayını yayınla
        emit_event(&e, "stake", &user, amount);
    }
    
    // Ödül hesaplama (internal fonksiyon)
    fn calculate_reward(e: &Env, user: &Address, stake_info: &StakeInfo, pool_info: &PoolInfo) -> i128 {
        let current_ledger = e.ledger().sequence();
        
        // Son çekimden bu yana geçen ledger sayısı
        let ledgers_passed = current_ledger - stake_info.last_claim_ledger;
        
        // Ödülü hesapla: stake miktarı * ödül oranı * geçen ledger sayısı / 10000
        // (10000 bölmesi ödül oranını daha hassas ayarlamaya olanak tanır)
        (stake_info.amount * pool_info.reward_rate as i128 * ledgers_passed as i128) / 10000
    }
    
    // Ödül çekme fonksiyonu
    pub fn claim_rewards(e: Env, user: Address) -> i128 {
        // Kullanıcının yetkilendirmesini kontrol et
        user.require_auth();
        
        // Havuz ve stake bilgilerini al
        let pool_info: PoolInfo = e.storage().instance().get(&POOL_INFO_KEY).unwrap();
        let mut stakes: Map<Address, StakeInfo> = e.storage().instance().get(&STAKES_KEY).unwrap();
        
        // Kullanıcının stake bilgisini kontrol et
        if let Some(mut stake_info) = stakes.get(user.clone()) {
            // Bekleyen ödülü hesapla
            let reward = Self::calculate_reward(&e, &user, &stake_info, &pool_info);
            
            if reward <= 0 {
                panic!("No rewards to claim");
            }
            
            // Token::Client yerine doğrudan depolama işlemlerini kullan
            spend_balance(&e, e.current_contract_address(), reward);
            receive_balance(&e, user.clone(), reward);
            
            // Son çekim zamanını güncelle
            stake_info.last_claim_ledger = e.ledger().sequence();
            stakes.set(user.clone(), stake_info);
            
            // Güncellenmiş bilgileri kaydet
            e.storage().instance().set(&STAKES_KEY, &stakes);
            
            // TTL süresini uzat
            e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
            
            // Ödül çekme olayını yayınla
            emit_event(&e, "claim_reward", &user, reward);
            
            return reward;
        } else {
            panic!("No stake found for user");
        }
    }
    
    // Hesaplanabilir ödülü görüntüleme fonksiyonu (view fonksiyonu)
    pub fn get_pending_rewards(e: Env, user: Address) -> i128 {
        // Havuz ve stake bilgilerini al
        let pool_info: PoolInfo = e.storage().instance().get(&POOL_INFO_KEY).unwrap();
        let stakes: Map<Address, StakeInfo> = e.storage().instance().get(&STAKES_KEY).unwrap();
        
        // Kullanıcının stake bilgisini kontrol et
        if let Some(stake_info) = stakes.get(user.clone()) {
            // Bekleyen ödülü hesapla
            return Self::calculate_reward(&e, &user, &stake_info, &pool_info);
        } else {
            return 0;
        }
    }
    
    // Stake çekme fonksiyonu
pub fn unstake(e: Env, user: Address, amount: i128) -> i128 {
    // Kullanıcının yetkilendirmesini kontrol et
    user.require_auth();
    
    // Negatif miktar kontrolü
    if amount <= 0 {
        panic!("Unstake amount must be positive");
    }
    
    // Havuz ve stake bilgilerini al
    let mut pool_info: PoolInfo = e.storage().instance().get(&POOL_INFO_KEY).unwrap();
    let mut stakes: Map<Address, StakeInfo> = e.storage().instance().get(&STAKES_KEY).unwrap();
    
    // Kullanıcının stake bilgisini kontrol et
    if let Some(mut stake_info) = stakes.get(user.clone()) {
        // Miktarın kullanıcının toplam stake'inden az olduğunu kontrol et
        if amount > stake_info.amount {
            panic!("Unstake amount exceeds staked amount");
        }
        
        // Minimum stake süresinin geçip geçmediğini kontrol et
        let current_ledger = e.ledger().sequence();
        if current_ledger - stake_info.since_ledger < pool_info.min_stake_duration {
            panic!("Minimum stake duration not met");
        }
        
        // Önce bekleyen ödülleri hesapla
        let reward = Self::calculate_reward(&e, &user, &stake_info, &pool_info);
        
        // Varsa ödülleri gönder (token::Client yerine depolama işlemleri ile)
        if reward > 0 {
            spend_balance(&e, e.current_contract_address(), reward);
            receive_balance(&e, user.clone(), reward);
            
            // Ödül çekme olayını yayınla
            emit_event(&e, "claim_reward", &user, reward);
        }
        
        // Kullanıcıya tokenlarını geri gönder (token::Client yerine depolama işlemleri ile)
        spend_balance(&e, e.current_contract_address(), amount);
        receive_balance(&e, user.clone(), amount);
        
        // Stake miktarını ve toplam stake miktarını güncelle
        stake_info.amount -= amount;
        pool_info.total_staked -= amount;
        
        // Eğer kalan miktar 0 ise kaydı sil, değilse güncelle
        if stake_info.amount == 0 {
            stakes.remove(user.clone());
        } else {
            stake_info.last_claim_ledger = current_ledger;
            stakes.set(user.clone(), stake_info);
        }
        
        // Güncellenmiş bilgileri kaydet
        e.storage().instance().set(&POOL_INFO_KEY, &pool_info);
        e.storage().instance().set(&STAKES_KEY, &stakes);
        
        // TTL süresini uzat
       
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        // Unstake olayını yayınla
        emit_event(&e, "unstake", &user, amount);
        
        return amount;
    } else {
        panic!("No stake found for user");
    }
}
    
    // Kullanıcının stake bilgisini görüntüleme fonksiyonu
    pub fn get_stake_info(e: Env, user: Address) -> StakeInfo {
        let stakes: Map<Address, StakeInfo> = e.storage().instance().get(&STAKES_KEY).unwrap();
        
        if let Some(stake_info) = stakes.get(user) {
            return stake_info;
        } else {
            panic!("No stake found for user");
        }
    }
    
    // Havuz bilgilerini görüntüleme fonksiyonu
    pub fn get_pool_info(e: Env) -> PoolInfo {
        e.storage().instance().get(&POOL_INFO_KEY).unwrap()
    }
    
    // Acil durum fonksiyonu: Admin tüm ödül tokenlarını çekebilir (sadece acil durumlar için)
    pub fn emergency_withdraw_rewards(e: Env) -> i128 {
        // Admin kontrolü
        let admin: Address = e.storage().instance().get(&ADMIN_KEY).unwrap();
        admin.require_auth();
        
        // Havuz bilgilerini al
        let pool_info: PoolInfo = e.storage().instance().get(&POOL_INFO_KEY).unwrap();
        
        // Kontrattaki ödül token bakiyesini al - doğrudan depolama fonksiyonu kullan
        let balance = read_balance(&e, e.current_contract_address());
        
        // Tüm bakiyeyi admin'e transfer et
        if balance > 0 {
            // Token::Client yerine doğrudan depolama işlemlerini kullan
            spend_balance(&e, e.current_contract_address(), balance);
            receive_balance(&e, admin.clone(), balance);
            
            // Acil çekim olayını yayınla
            emit_event(&e, "emergency_withdraw", &admin, balance);
        }
        
        return balance;
    }
}
  

#[contractimpl]
impl token::Interface for Token {
    fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_allowance(&e, from, spender).amount
    }

    fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        from.require_auth();

        check_nonnegative_amount(amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_allowance(&e, from.clone(), spender.clone(), amount, expiration_ledger);
        TokenUtils::new(&e)
            .events()
            .approve(from, spender, amount, expiration_ledger);
    }

    fn balance(e: Env, id: Address) -> i128 {
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_balance(&e, id)
    }

    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Göndericinin hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &from) {
            panic!("Hesap dondurulmuş ve token transfer edilemez");
        }

        // Transferi gerçekleştir
        spend_balance(&e, from.clone(), amount);
        receive_balance(&e, to.clone(), amount);
        TokenUtils::new(&e).events().transfer(from, to, amount);
    }

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Göndericinin hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &from) {
            panic!("Hesap dondurulmuş ve token transfer edilemez");
        }

         // Transferi gerçekleştir
        spend_allowance(&e, from.clone(), spender, amount);
        spend_balance(&e, from.clone(), amount);
        receive_balance(&e, to.clone(), amount);
        TokenUtils::new(&e).events().transfer(from, to, amount)
    }

    fn burn(e: Env, from: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Göndericinin hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &from) {
            panic!("Hesap dondurulmuş ve token yakılamaz");
        }

        // Yakma işlemini gerçekleştir
        spend_balance(&e, from.clone(), amount);
        TokenUtils::new(&e).events().burn(from, amount);
    }

    fn burn_from(e: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

         // Göndericinin hesabı dondurulmuş mu kontrol et
         if is_account_frozen(&e, &from) {
            panic!("Hesap dondurulmuş ve token yakılamaz");
        }

        // Yakma işlemini gerçekleştir
        spend_allowance(&e, from.clone(), spender, amount);
        spend_balance(&e, from.clone(), amount);
        TokenUtils::new(&e).events().burn(from, amount)
    }
 /// Lending havuzunu başlatma fonksiyonu (sadece admin)
    pub fn initialize_lending_pool(
        e: Env,
        supply_rate: u32,          // %5 için 500
        borrow_rate: u32,          // %8 için 800
        collateral_factor: u32,    // %75 için 7500
        reserve_factor: u32,       // %10 için 1000
    ) {
        let admin = read_administrator(&e);
        admin.require_auth();

        // Havuzun zaten başlatılmış olup olmadığını kontrol et
        let pool_key = DataKey::LendingPool;
        if e.storage().instance().has(&pool_key) {
            panic!("Lending pool already initialized");
        }

        let lending_pool = LendingPool {
            total_supplied: 0,
            total_borrowed: 0,
            supply_rate,
            borrow_rate,
            utilization_rate: 0,
            reserve_factor,
            last_update_ledger: e.ledger().sequence(),
            collateral_factor,
        };

        e.storage().instance().set(&pool_key, &lending_pool);

        // Liquidation parametrelerini ayarla
        let liquidation_threshold_key = DataKey::LiquidationThreshold;
        let liquidation_penalty_key = DataKey::LiquidationPenalty;
        
        e.storage().instance().set(&liquidation_threshold_key, &8000u32); // %80
        e.storage().instance().set(&liquidation_penalty_key, &500u32);   // %5

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "initialize_lending", &admin, 0);
    }

    /// Token yatırma (lending) fonksiyonu - faiz kazanmak için
    pub fn supply(e: Env, user: Address, amount: i128) {
        user.require_auth();
        check_nonnegative_amount(amount);

        // Kullanıcının hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &user) {
            panic!("Hesap dondurulmuş ve lending işlemi yapılamaz");
        }

        // Kullanıcının bakiyesini kontrol et
        let user_balance = read_balance(&e, user.clone());
        if user_balance < amount {
            panic!("Insufficient balance for supply");
        }

        // Lending havuz bilgilerini al ve güncelle
        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");
        
        // Faizi hesapla ve havuzu güncelle
        Self::accrue_lending_interest(&e, &mut lending_pool);

        // Kullanıcının mevcut supply bilgisini al
        let user_supply_key = DataKey::UserSupply(user.clone());
        let mut user_supply = e.storage().instance().get(&user_supply_key)
            .unwrap_or(UserSupply {
                amount: 0,
                last_update_ledger: e.ledger().sequence(),
                accrued_interest: 0,
            });

        // Önceki faizleri hesapla
        let interest_earned = Self::calculate_supply_interest(&e, &user_supply, &lending_pool);
        user_supply.accrued_interest += interest_earned;

        // Token transferi
        spend_balance(&e, user.clone(), amount);
        receive_balance(&e, e.current_contract_address(), amount);

        // Supply bilgilerini güncelle
        user_supply.amount += amount;
        user_supply.last_update_ledger = e.ledger().sequence();
        lending_pool.total_supplied += amount;

        // Kullanım oranını yeniden hesapla
        Self::update_utilization_rate(&mut lending_pool);

        // Güncellenmiş bilgileri kaydet
        e.storage().instance().set(&pool_key, &lending_pool);
        e.storage().instance().set(&user_supply_key, &user_supply);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "supply", &user, amount);
    }

    /// Token çekme (withdraw) fonksiyonu - yatırılan tokenları faizle birlikte çek
    pub fn withdraw(e: Env, user: Address, amount: i128) -> i128 {
        user.require_auth();
        check_nonnegative_amount(amount);

        // Kullanıcının hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &user) {
            panic!("Hesap dondurulmuş ve withdraw işlemi yapılamaz");
        }

        // Lending havuz bilgilerini al
        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");
        
        // Faizi hesapla ve havuzu güncelle
        Self::accrue_lending_interest(&e, &mut lending_pool);

        // Kullanıcının supply bilgisini al
        let user_supply_key = DataKey::UserSupply(user.clone());
        let mut user_supply: UserSupply = e.storage().instance().get(&user_supply_key)
            .expect("No supply found for user");

        // Faiz gelirini hesapla
        let interest_earned = Self::calculate_supply_interest(&e, &user_supply, &lending_pool);
        user_supply.accrued_interest += interest_earned;

        let available_amount = user_supply.amount + user_supply.accrued_interest;

        // Çekim miktarını kontrol et
        if amount > available_amount {
            panic!("Insufficient supplied amount");
        }

        // Havuzda yeterli likidite var mı kontrol et
        let available_liquidity = lending_pool.total_supplied - lending_pool.total_borrowed;
        if amount > available_liquidity {
            panic!("Insufficient liquidity in pool");
        }

        // Token transferi
        spend_balance(&e, e.current_contract_address(), amount);
        receive_balance(&e, user.clone(), amount);

        // Supply bilgilerini güncelle
        if amount <= user_supply.accrued_interest {
            user_supply.accrued_interest -= amount;
        } else {
            let remaining = amount - user_supply.accrued_interest;
            user_supply.accrued_interest = 0;
            user_supply.amount -= remaining;
        }

        user_supply.last_update_ledger = e.ledger().sequence();
        lending_pool.total_supplied -= amount;

        // Kullanım oranını yeniden hesapla
        Self::update_utilization_rate(&mut lending_pool);

        // Eğer kullanıcının hiç supply'ı kalmadıysa kaydı sil
        if user_supply.amount == 0 && user_supply.accrued_interest == 0 {
            e.storage().instance().remove(&user_supply_key);
        } else {
            e.storage().instance().set(&user_supply_key, &user_supply);
        }

        e.storage().instance().set(&pool_key, &lending_pool);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "withdraw", &user, amount);
        amount
    }

    /// Teminatlı borç alma fonksiyonu
    pub fn borrow(e: Env, user: Address, amount: i128, collateral_amount: i128) {
        user.require_auth();
        check_nonnegative_amount(amount);
        check_nonnegative_amount(collateral_amount);

        // Kullanıcının hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &user) {
            panic!("Hesap dondurulmuş ve borrow işlemi yapılamaz");
        }

        // Kullanıcının teminat için yeterli bakiyesi var mı kontrol et
        let user_balance = read_balance(&e, user.clone());
        if user_balance < collateral_amount {
            panic!("Insufficient balance for collateral");
        }

        // Lending havuz bilgilerini al
        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");
        
        // Faizi hesapla ve havuzu güncelle
        Self::accrue_lending_interest(&e, &mut lending_pool);

        // Havuzda yeterli likidite var mı kontrol et
        let available_liquidity = lending_pool.total_supplied - lending_pool.total_borrowed;
        if amount > available_liquidity {
            panic!("Insufficient liquidity for borrow");
        }

        // Kullanıcının mevcut borrow bilgisini al
        let user_borrow_key = DataKey::UserBorrow(user.clone());
        let mut user_borrow = e.storage().instance().get(&user_borrow_key)
            .unwrap_or(UserBorrow {
                amount: 0,
                last_update_ledger: e.ledger().sequence(),
                accrued_interest: 0,
                collateral_deposited: 0,
            });

        // Önceki faizleri hesapla
        let interest_owed = Self::calculate_borrow_interest(&e, &user_borrow, &lending_pool);
        user_borrow.accrued_interest += interest_owed;

        // Toplam borç ve teminat miktarlarını hesapla
        let total_debt = user_borrow.amount + user_borrow.accrued_interest + amount;
        let total_collateral = user_borrow.collateral_deposited + collateral_amount;

        // Teminat yeterliliğini kontrol et (teminat faktörü ile)
        let required_collateral = (total_debt * 10000) / lending_pool.collateral_factor as i128;
        if total_collateral < required_collateral {
            panic!("Insufficient collateral");
        }

        // Teminat transferi (kullanıcıdan kontrata)
        spend_balance(&e, user.clone(), collateral_amount);
        receive_balance(&e, e.current_contract_address(), collateral_amount);

        // Borç transferi (kontrattan kullanıcıya)
        spend_balance(&e, e.current_contract_address(), amount);
        receive_balance(&e, user.clone(), amount);

        // Borrow bilgilerini güncelle
        user_borrow.amount += amount;
        user_borrow.collateral_deposited += collateral_amount;
        user_borrow.last_update_ledger = e.ledger().sequence();
        lending_pool.total_borrowed += amount;

        // Kullanım oranını yeniden hesapla
        Self::update_utilization_rate(&mut lending_pool);

        // Güncellenmiş bilgileri kaydet
        e.storage().instance().set(&pool_key, &lending_pool);
        e.storage().instance().set(&user_borrow_key, &user_borrow);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "borrow", &user, amount);
    }

    /// Borç geri ödeme fonksiyonu
    pub fn repay(e: Env, user: Address, amount: i128) -> i128 {
        user.require_auth();
        check_nonnegative_amount(amount);

        // Kullanıcının hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &user) {
            panic!("Hesap dondurulmuş ve repay işlemi yapılamaz");
        }

        // Kullanıcının bakiyesini kontrol et
        let user_balance = read_balance(&e, user.clone());
        if user_balance < amount {
            panic!("Insufficient balance for repayment");
        }

        // Lending havuz bilgilerini al
        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");
        
        // Faizi hesapla ve havuzu güncelle
        Self::accrue_lending_interest(&e, &mut lending_pool);

        // Kullanıcının borrow bilgisini al
        let user_borrow_key = DataKey::UserBorrow(user.clone());
        let mut user_borrow: UserBorrow = e.storage().instance().get(&user_borrow_key)
            .expect("No borrow found for user");

        // Faiz borcunu hesapla
        let interest_owed = Self::calculate_borrow_interest(&e, &user_borrow, &lending_pool);
        user_borrow.accrued_interest += interest_owed;

        let total_debt = user_borrow.amount + user_borrow.accrued_interest;

        // Ödeme miktarını sınırla
        let repay_amount = if amount > total_debt { total_debt } else { amount };

        // Token transferi (kullanıcıdan kontrata)
        spend_balance(&e, user.clone(), repay_amount);
        receive_balance(&e, e.current_contract_address(), repay_amount);

        // Borç bilgilerini güncelle
        if repay_amount <= user_borrow.accrued_interest {
            user_borrow.accrued_interest -= repay_amount;
        } else {
            let remaining = repay_amount - user_borrow.accrued_interest;
            user_borrow.accrued_interest = 0;
            user_borrow.amount -= remaining;
        }

        user_borrow.last_update_ledger = e.ledger().sequence();
        lending_pool.total_borrowed -= repay_amount;

        // Kullanım oranını yeniden hesapla
        Self::update_utilization_rate(&mut lending_pool);

        // Eğer borç tamamen ödendiyse teminatı iade et
        if user_borrow.amount == 0 && user_borrow.accrued_interest == 0 {
            if user_borrow.collateral_deposited > 0 {
                spend_balance(&e, e.current_contract_address(), user_borrow.collateral_deposited);
                receive_balance(&e, user.clone(), user_borrow.collateral_deposited);
            }
            e.storage().instance().remove(&user_borrow_key);
        } else {
            e.storage().instance().set(&user_borrow_key, &user_borrow);
        }

        e.storage().instance().set(&pool_key, &lending_pool);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "repay", &user, repay_amount);
        repay_amount
    }

    /// Liquidation fonksiyonu - sağlıksız pozisyonları tasfiye et
    pub fn liquidate(e: Env, liquidator: Address, borrower: Address, repay_amount: i128) {
        liquidator.require_auth();
        check_nonnegative_amount(repay_amount);

        // Liquidator'ın hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &liquidator) {
            panic!("Liquidator hesabı dondurulmuş");
        }

        // Liquidator'ın bakiyesini kontrol et
        let liquidator_balance = read_balance(&e, liquidator.clone());
        if liquidator_balance < repay_amount {
            panic!("Insufficient balance for liquidation");
        }

        // Lending havuz bilgilerini al
        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");
        
        // Faizi hesapla ve havuzu güncelle
        Self::accrue_lending_interest(&e, &mut lending_pool);

        // Borrower'ın borrow bilgisini al
        let user_borrow_key = DataKey::UserBorrow(borrower.clone());
        let mut user_borrow: UserBorrow = e.storage().instance().get(&user_borrow_key)
            .expect("No borrow found for borrower");

        // Faiz borcunu hesapla
        let interest_owed = Self::calculate_borrow_interest(&e, &user_borrow, &lending_pool);
        user_borrow.accrued_interest += interest_owed;

        let total_debt = user_borrow.amount + user_borrow.accrued_interest;

        // Liquidation'ın gerekli olup olmadığını kontrol et
        let liquidation_threshold: u32 = e.storage().instance().get(&DataKey::LiquidationThreshold).unwrap();
        let health_factor = (user_borrow.collateral_deposited * liquidation_threshold as i128) / (total_debt * 10000);
        
        if health_factor >= 100 {
            panic!("Position is healthy, cannot liquidate");
        }

        // Liquidation miktarını sınırla (%50 max)
        let max_liquidation = total_debt / 2;
        let actual_repay = if repay_amount > max_liquidation { max_liquidation } else { repay_amount };

        // Liquidation penalty'sini al
        let liquidation_penalty: u32 = e.storage().instance().get(&DataKey::LiquidationPenalty).unwrap();
        let collateral_to_seize = actual_repay + (actual_repay * liquidation_penalty as i128 / 10000);

        if collateral_to_seize > user_borrow.collateral_deposited {
            panic!("Not enough collateral to seize");
        }

        // Token transferleri
        // Liquidator'dan kontrata (borç ödeme)
        spend_balance(&e, liquidator.clone(), actual_repay);
        receive_balance(&e, e.current_contract_address(), actual_repay);

        // Kontrattan liquidator'a (teminat)
        spend_balance(&e, e.current_contract_address(), collateral_to_seize);
        receive_balance(&e, liquidator.clone(), collateral_to_seize);

        // Borç bilgilerini güncelle
        if actual_repay <= user_borrow.accrued_interest {
            user_borrow.accrued_interest -= actual_repay;
        } else {
            let remaining = actual_repay - user_borrow.accrued_interest;
            user_borrow.accrued_interest = 0;
            user_borrow.amount -= remaining;
        }

        user_borrow.collateral_deposited -= collateral_to_seize;
        user_borrow.last_update_ledger = e.ledger().sequence();
        lending_pool.total_borrowed -= actual_repay;

        // Kullanım oranını yeniden hesapla
        Self::update_utilization_rate(&mut lending_pool);

        // Eğer borç tamamen ödendiyse kaydı sil
        if user_borrow.amount == 0 && user_borrow.accrued_interest == 0 {
            // Kalan teminatı iade et
            if user_borrow.collateral_deposited > 0 {
                spend_balance(&e, e.current_contract_address(), user_borrow.collateral_deposited);
                receive_balance(&e, borrower.clone(), user_borrow.collateral_deposited);
            }
            e.storage().instance().remove(&user_borrow_key);
        } else {
            e.storage().instance().set(&user_borrow_key, &user_borrow);
        }

        e.storage().instance().set(&pool_key, &lending_pool);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "liquidate", &liquidator, actual_repay);
    }

    /// Teminat ekleme fonksiyonu
    pub fn add_collateral(e: Env, user: Address, amount: i128) {
        user.require_auth();
        check_nonnegative_amount(amount);

        // Kullanıcının hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &user) {
            panic!("Hesap dondurulmuş ve collateral ekleme işlemi yapılamaz");
        }

        let user_balance = read_balance(&e, user.clone());
        if user_balance < amount {
            panic!("Insufficient balance for additional collateral");
        }

        let user_borrow_key = DataKey::UserBorrow(user.clone());
        let mut user_borrow: UserBorrow = e.storage().instance().get(&user_borrow_key)
            .expect("No existing borrow position");

        // Teminat transferi
        spend_balance(&e, user.clone(), amount);
        receive_balance(&e, e.current_contract_address(), amount);

        // Teminat miktarını güncelle
        user_borrow.collateral_deposited += amount;
        e.storage().instance().set(&user_borrow_key, &user_borrow);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "add_collateral", &user, amount);
    }

    /// Kısmi teminat çekme
    pub fn remove_collateral(e: Env, user: Address, amount: i128) {
        user.require_auth();
        check_nonnegative_amount(amount);

        // Kullanıcının hesabı dondurulmuş mu kontrol et
        if is_account_frozen(&e, &user) {
            panic!("Hesap dondurulmuş ve collateral çekme işlemi yapılamaz");
        }

        let pool_key = DataKey::LendingPool;
        let lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");

        let user_borrow_key = DataKey::UserBorrow(user.clone());
        let mut user_borrow: UserBorrow = e.storage().instance().get(&user_borrow_key)
            .expect("No existing borrow position");

        // Faiz hesapla
        let interest_owed = Self::calculate_borrow_interest(&e, &user_borrow, &lending_pool);
        let total_debt = user_borrow.amount + user_borrow.accrued_interest + interest_owed;

        // Teminat çekildikten sonra pozisyonun sağlıklı kalacağını kontrol et
        let remaining_collateral = user_borrow.collateral_deposited - amount;
        let required_collateral = (total_debt * 10000) / lending_pool.collateral_factor as i128;

        if remaining_collateral < required_collateral {
            panic!("Removing collateral would make position unhealthy");
        }

        // Teminat transferi
        spend_balance(&e, e.current_contract_address(), amount);
        receive_balance(&e, user.clone(), amount);

        // Teminat miktarını güncelle
        user_borrow.collateral_deposited -= amount;
        e.storage().instance().set(&user_borrow_key, &user_borrow);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "remove_collateral", &user, amount);
    }

    // ===============================
    // YARDIMCI FONKSİYONLAR
    // ===============================

    fn accrue_lending_interest(e: &Env, lending_pool: &mut LendingPool) {
        let current_ledger = e.ledger().sequence();
        let ledgers_passed = current_ledger - lending_pool.last_update_ledger;
        
        if ledgers_passed > 0 {
            // Basit faiz hesaplama (yıllık oranı ledger başına dönüştür)
            // Varsayım: 1 yıl = 365 * 24 * 60 * 12 ledger (5 saniyelik ledger'lar)
            let ledgers_per_year = 365 * 24 * 60 * 12;
            
            let borrow_interest = (lending_pool.total_borrowed * lending_pool.borrow_rate as i128 * ledgers_passed as i128) / (10000 * ledgers_per_year);
            lending_pool.total_borrowed += borrow_interest;
            
            let supply_interest = (lending_pool.total_supplied * lending_pool.supply_rate as i128 * ledgers_passed as i128) / (10000 * ledgers_per_year);
            lending_pool.total_supplied += supply_interest;
            
            lending_pool.last_update_ledger = current_ledger;
        }
    }

    fn calculate_supply_interest(e: &Env, user_supply: &UserSupply, lending_pool: &LendingPool) -> i128 {
        let current_ledger = e.ledger().sequence();
        let ledgers_passed = current_ledger - user_supply.last_update_ledger;
        
        if ledgers_passed == 0 {
            return 0;
        }
        
        let ledgers_per_year = 365 * 24 * 60 * 12;
        (user_supply.amount * lending_pool.supply_rate as i128 * ledgers_passed as i128) / (10000 * ledgers_per_year)
    }

    fn calculate_borrow_interest(e: &Env, user_borrow: &UserBorrow, lending_pool: &LendingPool) -> i128 {
        let current_ledger = e.ledger().sequence();
        let ledgers_passed = current_ledger - user_borrow.last_update_ledger;
        
        if ledgers_passed == 0 {
            return 0;
        }
        
        let ledgers_per_year = 365 * 24 * 60 * 12;
        (user_borrow.amount * lending_pool.borrow_rate as i128 * ledgers_passed as i128) / (10000 * ledgers_per_year)
    }

    fn update_utilization_rate(lending_pool: &mut LendingPool) {
        if lending_pool.total_supplied == 0 {
            lending_pool.utilization_rate = 0;
        } else {
            lending_pool.utilization_rate = ((lending_pool.total_borrowed * 10000) / lending_pool.total_supplied) as u32;
        }
    }

    // ===============================
    // VIEW FONKSİYONLARI
    // ===============================

    /// Lending havuz bilgilerini görüntüle
    pub fn get_lending_pool_info(e: Env) -> LendingPool {
        let pool_key = DataKey::LendingPool;
        e.storage().instance().get(&pool_key).expect("Lending pool not initialized")
    }

    /// Kullanıcının supply bilgilerini görüntüle
    pub fn get_user_supply_info(e: Env, user: Address) -> Option<UserSupply> {
        let user_supply_key = DataKey::UserSupply(user);
        e.storage().instance().get(&user_supply_key)
    }

    /// Kullanıcının borrow bilgilerini görüntüle
    pub fn get_user_borrow_info(e: Env, user: Address) -> Option<UserBorrow> {
        let user_borrow_key = DataKey::UserBorrow(user);
        e.storage().instance().get(&user_borrow_key)
    }

    /// Kullanıcının sağlık faktörünü hesapla
    pub fn get_user_health_factor(e: Env, user: Address) -> i128 {
        let user_borrow_key = DataKey::UserBorrow(user.clone());
        if let Some(user_borrow) = e.storage().instance().get::<_, UserBorrow>(&user_borrow_key) {
            let pool_key = DataKey::LendingPool;
            let lending_pool: LendingPool = e.storage().instance().get(&pool_key)
                .expect("Lending pool not initialized");
            
            let interest_owed = Self::calculate_borrow_interest(&e, &user_borrow, &lending_pool);
            let total_debt = user_borrow.amount + user_borrow.accrued_interest + interest_owed;
            
            if total_debt == 0 {
                return i128::MAX; // Sonsuz sağlık faktörü
            }
            
            let liquidation_threshold: u32 = e.storage().instance().get(&DataKey::LiquidationThreshold).unwrap();
            (user_borrow.collateral_deposited * liquidation_threshold as i128) / (total_debt * 100)
        } else {
            i128::MAX // Borcu yoksa sağlık faktörü sonsuz
        }
    }

    /// Kullanıcının birikmiş supply faizini hesapla
    pub fn get_pending_supply_interest(e: Env, user: Address) -> i128 {
        let user_supply_key = DataKey::UserSupply(user);
        if let Some(user_supply) = e.storage().instance().get::<_, UserSupply>(&user_supply_key) {
            let pool_key = DataKey::LendingPool;
            let lending_pool: LendingPool = e.storage().instance().get(&pool_key)
                .expect("Lending pool not initialized");
            
            Self::calculate_supply_interest(&e, &user_supply, &lending_pool)
        } else {
            0
        }
    }

    /// Kullanıcının birikmiş borrow faizini hesapla
    pub fn get_pending_borrow_interest(e: Env, user: Address) -> i128 {
        let user_borrow_key = DataKey::UserBorrow(user);
        if let Some(user_borrow) = e.storage().instance().get::<_, UserBorrow>(&user_borrow_key) {
            let pool_key = DataKey::LendingPool;
            let lending_pool: LendingPool = e.storage().instance().get(&pool_key)
                .expect("Lending pool not initialized");
            
            Self::calculate_borrow_interest(&e, &user_borrow, &lending_pool)
        } else {
            0
        }
    }

    /// Kullanıcı pozisyon özeti (supply, borrow, collateral, health factor)
    pub fn get_user_position_summary(e: Env, user: Address) -> (i128, i128, i128, i128) {
        let user_supply_key = DataKey::UserSupply(user.clone());
        let user_borrow_key = DataKey::UserBorrow(user.clone());

        let supply_info = e.storage().instance().get::<_, UserSupply>(&user_supply_key);
        let borrow_info = e.storage().instance().get::<_, UserBorrow>(&user_borrow_key);

        let total_supplied = supply_info.map_or(0, |s| s.amount + s.accrued_interest);
        let total_borrowed = borrow_info.as_ref().map_or(0, |b| b.amount + b.accrued_interest);
        let total_collateral = borrow_info.map_or(0, |b| b.collateral_deposited);
        let health_factor = Self::get_user_health_factor(e, user);

        (total_supplied, total_borrowed, total_collateral, health_factor)
    }

    // ===============================
    // ADMIN FONKSİYONLARI
    // ===============================

    /// Lending faiz oranlarını güncelle (sadece admin)
    pub fn update_lending_rates(e: Env, new_supply_rate: u32, new_borrow_rate: u32) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");
        
        lending_pool.supply_rate = new_supply_rate;
        lending_pool.borrow_rate = new_borrow_rate;
        
        e.storage().instance().set(&pool_key, &lending_pool);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "update_lending_rates", &admin, new_supply_rate as i128);
    }

    /// Liquidation parametrelerini güncelle (sadece admin)
    pub fn update_liquidation_params(e: Env, threshold: u32, penalty: u32) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let threshold_key = DataKey::LiquidationThreshold;
        let penalty_key = DataKey::LiquidationPenalty;
        
        e.storage().instance().set(&threshold_key, &threshold);
        e.storage().instance().set(&penalty_key, &penalty);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "update_liquidation_params", &admin, threshold as i128);
    }

    /// Collateral faktörünü güncelle (sadece admin)
    pub fn update_collateral_factor(e: Env, new_factor: u32) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");
        
        lending_pool.collateral_factor = new_factor;
        e.storage().instance().set(&pool_key, &lending_pool);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "update_collateral_factor", &admin, new_factor as i128);
    }

    /// Dinamik faiz oranı hesaplama ve güncelleme (sadece admin)
    pub fn update_dynamic_rates(e: Env) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");

        // Kullanım oranına göre dinamik faiz hesapla
        let utilization = lending_pool.utilization_rate;

        // Jump Rate Model:
        // Utilization < 80%: Linear artış
        // Utilization >= 80%: Exponential artış
        let optimal_utilization = 8000; // %80
        let base_rate = 200; // %2
        let multiplier = 500; // %5
        let jump_multiplier = 10000; // %100

        let new_borrow_rate = if utilization <= optimal_utilization {
            base_rate + (utilization * multiplier / 10000)
        } else {
            let excess_utilization = utilization - optimal_utilization;
            base_rate + (optimal_utilization * multiplier / 10000) + 
            (excess_utilization * jump_multiplier / 10000)
        };

        // Supply rate = borrow rate * utilization * (1 - reserve factor)
        let new_supply_rate = (new_borrow_rate * utilization / 10000) * 
                             (10000 - lending_pool.reserve_factor) / 10000;

        lending_pool.borrow_rate = new_borrow_rate;
        lending_pool.supply_rate = new_supply_rate;

        e.storage().instance().set(&pool_key, &lending_pool);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "dynamic_rate_update", &admin, new_borrow_rate as i128);
    }

    /// Protokol rezervlerini çek (sadece admin)
    pub fn withdraw_reserves(e: Env, amount: i128) {
        let admin = read_administrator(&e);
        admin.require_auth();
        check_nonnegative_amount(amount);

        let pool_key = DataKey::LendingPool;
        let lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");

        // Rezerv miktarını hesapla
        let total_interest_earned = if lending_pool.total_supplied > lending_pool.total_borrowed {
            lending_pool.total_supplied - lending_pool.total_borrowed
        } else {
            0
        };

        let available_reserves = (total_interest_earned * lending_pool.reserve_factor as i128) / 10000;

        if amount > available_reserves {
            panic!("Insufficient reserves");
        }

        // Rezervleri admin'e transfer et
        spend_balance(&e, e.current_contract_address(), amount);
        receive_balance(&e, admin.clone(), amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "withdraw_reserves", &admin, amount);
    }

    /// Risk analizi metrikleri (sadece admin)
    pub fn get_protocol_risk_metrics(e: Env) -> (i128, i128, u32, u32) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let pool_key = DataKey::LendingPool;
        let lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");

        let total_value_locked = lending_pool.total_supplied;
        let total_debt = lending_pool.total_borrowed;
        let utilization_rate = lending_pool.utilization_rate;
        
        // Risk skoru hesapla (utilization rate bazlı)
        let risk_score = if utilization_rate > 9000 {
            100 // Yüksek risk
        } else if utilization_rate > 8000 {
            75  // Orta-yüksek risk
        } else if utilization_rate > 6000 {
            50  // Orta risk
        } else {
            25  // Düşük risk
        };

        (total_value_locked, total_debt, utilization_rate, risk_score)
    }

    /// Acil durum lending pool çekimi (sadece admin)
    pub fn emergency_withdraw_lending_pool(e: Env) -> i128 {
        let admin = read_administrator(&e);
        admin.require_auth();

        // Kontrattaki toplam bakiyeyi al
        let balance = read_balance(&e, e.current_contract_address());

        // Tüm bakiyeyi admin'e transfer et
        if balance > 0 {
            spend_balance(&e, e.current_contract_address(), balance);
            receive_balance(&e, admin.clone(), balance);

            // Lending pool'u sıfırla
            let pool_key = DataKey::LendingPool;
            if let Ok(mut lending_pool) = e.storage().instance().get::<_, LendingPool>(&pool_key) {
                lending_pool.total_supplied = 0;
                lending_pool.total_borrowed = 0;
                lending_pool.utilization_rate = 0;
                e.storage().instance().set(&pool_key, &lending_pool);
            }

            emit_event(&e, "emergency_withdraw_lending", &admin, balance);
        }

        balance
    }

    // ===============================
    // GELİŞMİŞ FONKSİYONLAR
    // ===============================

    /// Toplu liquidation (birden fazla pozisyonu aynı anda tasfiye et)
    pub fn batch_liquidate(e: Env, liquidator: Address, targets: Vec<(Address, i128)>) {
        liquidator.require_auth();

        if targets.len() > 10 {
            panic!("Too many targets, maximum 10 allowed");
        }

        let mut total_repaid = 0i128;

        for target in targets.iter() {
            let borrower = target.0.clone();
            let amount = target.1;

            // Her liquidation için health factor kontrol et
            let health_factor = Self::get_user_health_factor(e.clone(), borrower.clone());
            
            if health_factor >= 100 {
                continue; // Sağlıklı pozisyon, atla
            }

            // Liquidation işlemini gerçekleştir
            Self::liquidate(e.clone(), liquidator.clone(), borrower, amount);
            total_repaid += amount;
        }

        emit_event(&e, "batch_liquidate", &liquidator, total_repaid);
    }

    /// Sağlıksız pozisyonları tespit et
    pub fn find_liquidatable_positions(e: Env, users: Vec<Address>) -> Vec<Address> {
        let admin = read_administrator(&e);
        admin.require_auth();

        let mut liquidatable_users = Vec::new(&e);

        for user in users.iter() {
            let health_factor = Self::get_user_health_factor(e.clone(), user.clone());
            
            // Sağlık faktörü 100'ün altındaysa (pozisyon sağlıksız)
            if health_factor < 100 {
                liquidatable_users.push_back(user.clone());
            }
        }

        liquidatable_users
    }

    /// Lending havuzu manuel faiz güncelleme
    pub fn accrue_lending_interest_manual(e: Env) {
        let pool_key = DataKey::LendingPool;
        let mut lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");
        
        Self::accrue_lending_interest(&e, &mut lending_pool);
        e.storage().instance().set(&pool_key, &lending_pool);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        emit_event(&e, "manual_interest_accrual", &e.current_contract_address(), 0);
    }

    /// Kullanıcının maksimum borçlanabileceği miktarı hesapla
    pub fn get_max_borrowable_amount(e: Env, user: Address, collateral_amount: i128) -> i128 {
        let pool_key = DataKey::LendingPool;
        let lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");

        // Mevcut borrow pozisyonunu al
        let user_borrow_key = DataKey::UserBorrow(user.clone());
        let current_debt = if let Some(user_borrow) = e.storage().instance().get::<_, UserBorrow>(&user_borrow_key) {
            let interest_owed = Self::calculate_borrow_interest(&e, &user_borrow, &lending_pool);
            user_borrow.amount + user_borrow.accrued_interest + interest_owed
        } else {
            0
        };

        // Maksimum borçlanabilir miktar = (collateral * collateral_factor / 10000) - current_debt
        let max_total_debt = (collateral_amount * lending_pool.collateral_factor as i128) / 10000;
        
        if max_total_debt > current_debt {
            max_total_debt - current_debt
        } else {
            0
        }
    }

    /// Havuzdaki mevcut likiditeyi kontrol et
    pub fn get_available_liquidity(e: Env) -> i128 {
        let pool_key = DataKey::LendingPool;
        let lending_pool: LendingPool = e.storage().instance().get(&pool_key)
            .expect("Lending pool not initialized");

        lending_pool.total_supplied - lending_pool.total_borrowed
    }
    fn decimals(e: Env) -> u32 {
        read_decimal(&e)
    }

    fn name(e: Env) -> String {
        read_name(&e)
    }

    fn symbol(e: Env) -> String {
        read_symbol(&e)
    }
}