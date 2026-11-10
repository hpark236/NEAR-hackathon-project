//! Grassroots: a NEP-141 memecoin whose supply, treasury and rewards follow its *real* community.
//!
//! Every rule below is fixed at `new()` and there are no admin methods that change it. After
//! deploying, delete the account's full-access keys and the code is frozen too.
//!
//! * **Growth reserve:** most of the supply is not minted at launch. A tranche mints each time a
//!   signed Organic Score attestation reports that independent holders (wallet clusters counted
//!   once, held for the configured minimum) crossed the next milestone. 80% of each tranche
//!   (configurable) goes to the community pool and the rest to the creator earn-out. If holders
//!   stay below the last milestone for `contraction_days`, part of the unminted reserve is cancelled
//!   forever.
//! * **Daily ritual:** registered holders `check_in` once per day. If unique independent
//!   check-ins reach the quorum, a slice of the pool is paid to that day's check-ins by weight;
//!   if not, a slice of the pool burns and the community streak resets.
//! * **Anti-farm weight** for every payout (ritual and creator challenges):
//!   `independence share x holding-age ramp x streak bonus x capped stake`. A detected cluster of
//!   k wallets shares one weight, so splitting a stack across many accounts earns nothing extra.
//! * **Creator challenges:** the creator registers challenges whose completion is proven either by
//!   an ed25519 signature from the creator's own server, or by a NEAR contract implementing
//!   `verify(account_id, proof) -> bool`. Every challenge has a daily budget, and all challenges
//!   together are capped at a fixed share of the pool per day.
//! * **Seasons:** a season goal for independent-holder growth. Hit it and a bonus tranche goes to
//!   the pool; miss it and that tranche is cancelled.
//! * **Weekly bonfire:** voluntary burns; the biggest offering of the week holds the title.
//! * **Never blocks selling:** no transfer tax, no sell lock. Plain NEP-141 transfers always work.

use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::fungible_token::events::{FtBurn, FtMint};
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::resolver::FungibleTokenResolver;
use near_contract_standards::fungible_token::FungibleToken;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::json_types::{Base64VecU8, U128, U64};
use near_sdk::store::{IterableSet, LookupMap, LookupSet, Vector};
use near_sdk::{
    env, ext_contract, near, require, AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault,
    Promise, PromiseError, PromiseOrValue, PublicKey,
};

const SCALE: u128 = 1_000_000;
const BPS: u128 = 10_000;
const MAX_SETTLE_PER_CALL: u32 = 30;

#[derive(BorshStorageKey)]
#[near]
enum Key {
    Ft,
    Since,
    Members,
    Days,
    DayClusters,
    Clusters,
    Challenges,
    UsedNonces,
    ChallengeLast,
    WeekBurns,
    Holders,
    Ledger,
}

/// One line of the public supply ledger: every mint, burn, cancellation and pool payout.
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct LedgerEntry {
    pub day: u32,
    pub ts_ms: U64,
    /// "mint" | "burn" | "cancel" | "payout"
    pub kind: String,
    pub account: Option<AccountId>,
    pub amount: U128,
    pub memo: String,
}

/// Parameters fixed at launch. Readable by anyone through `get_config`.
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Config {
    pub max_supply: U128,
    /// Minted at launch to `launch_receiver` (auction / LP / distribution).
    pub launch_bps: u32,
    /// Minted at launch into the community pool.
    pub pool_seed_bps: u32,
    /// Independent-holder milestones (strictly increasing).
    pub milestones: Vec<u32>,
    /// Tranche size per milestone, in bps of max supply.
    pub tranche_bps: Vec<u32>,
    /// Share of each tranche that goes to the pool; the rest is the creator earn-out.
    pub tranche_to_pool_bps: u32,
    pub contraction_days: u32,
    pub contraction_cut_bps: u32,
    pub quorum_min: u32,
    pub quorum_bps: u32,
    pub daily_payout_bps: u32,
    pub miss_burn_bps: u32,
    pub age_full_days: u32,
    pub streak_cap: u32,
    pub streak_max_bonus_bps: u32,
    /// Stake weight grows linearly up to this balance (bps of max supply), then stays flat.
    pub balance_cap_bps: u32,
    /// Minimum balance to check in (anti-dust).
    pub min_balance_to_check_in: U128,
    pub season_days: u32,
    pub season_growth_bps: u32,
    pub season_bonus_bps: u32,
    /// All creator challenges together may pay at most this share of the free pool per day.
    pub challenge_daily_cap_bps: u32,
    /// Length of one "day" in milliseconds (86_400_000 on mainnet; shorter for demos).
    pub day_ms: U64,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub enum Proof {
    /// The creator's server signs `grassroots-challenge:{contract}:{id}:{account}:{nonce}`.
    Signed { issuer_pk: PublicKey },
    /// A NEAR contract answers `verify(account_id, proof) -> bool`.
    Contract { verifier_id: AccountId },
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Challenge {
    pub name: String,
    pub proof: Proof,
    pub reward: U128,
    pub cooldown_days: u32,
    pub daily_budget: U128,
    pub active: bool,
    pub spent_day: u32,
    pub spent: U128,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Default)]
pub struct Member {
    pub last_day: Option<u32>,
    pub streak: u32,
    /// (day, weight) check-ins not yet claimed.
    pub pending: Vec<(u32, U128)>,
    pub rewards_total: U128,
    pub burned_total: U128,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Default)]
pub struct Day {
    pub unique: u32,
    pub checkins: u32,
    pub total_weight: U128,
    pub settled: bool,
    pub hit: bool,
    pub payout: U128,
    pub burned: U128,
    pub quorum: u32,
}

#[near(serializers = [json])]
pub struct ClusterEntry {
    pub account_id: AccountId,
    pub cluster_id: String,
    pub size: u32,
}

/// Signed by the Organic Score oracle. `clusters` may be partial; size 1 clears a flag.
#[near(serializers = [json])]
pub struct Attestation {
    pub ts: U64,
    pub independent_holders: u32,
    pub clusters: Vec<ClusterEntry>,
}

#[near(serializers = [borsh, json])]
#[derive(Clone)]
pub struct Cluster {
    pub id: String,
    pub size: u32,
}

#[near(serializers = [json])]
pub struct State {
    pub total_supply: U128,
    pub reserve: U128,
    pub pool_free: U128,
    pub pool_committed: U128,
    pub burned: U128,
    pub cancelled: U128,
    pub creator_unlocked: U128,
    pub creator_claimed: U128,
    pub milestone_idx: u32,
    pub independent_holders: u32,
    pub community_streak: u32,
    pub best_streak: u32,
    pub current_day: u32,
    pub next_settle_day: u32,
    pub season_start_holders: u32,
    pub seasons_won: u32,
    pub seasons_lost: u32,
    /// When the daily ritual started (None while the coin is still being distributed).
    pub ritual_started_ms: Option<U64>,
}

#[ext_contract(ext_verifier)]
pub trait ChallengeVerifier {
    fn verify(&self, account_id: AccountId, proof: String) -> bool;
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    ft: FungibleToken,
    metadata: FungibleTokenMetadata,
    config: Config,
    creator: AccountId,
    launch_receiver: AccountId,
    oracle_pk: PublicKey,
    links: Vec<Link>,
    /// Set when the oracle first reports `quorum_min` independent holders. Days, seasons and
    /// burns only run from then on, so a coin is not drained while it is still being distributed.
    genesis_ms: Option<u64>,

    since: LookupMap<AccountId, u64>,
    holders: IterableSet<AccountId>,
    members: LookupMap<AccountId, Member>,
    days: LookupMap<u32, Day>,
    day_clusters: LookupSet<String>,
    clusters: LookupMap<AccountId, Cluster>,
    challenges: Vec<Challenge>,
    used_nonces: LookupSet<String>,
    challenge_last: LookupMap<String, u32>,
    week_burns: LookupMap<String, u128>,
    week_top: Vec<(u32, AccountId, U128)>,
    ledger: Vector<LedgerEntry>,

    reserve: u128,
    pool_committed: u128,
    burned: u128,
    cancelled: u128,
    creator_unlocked: u128,
    creator_claimed: u128,
    milestone_idx: u32,
    independent_holders: u32,
    below_since_ms: Option<u64>,
    last_attest_ts: u64,
    community_streak: u32,
    best_streak: u32,
    next_settle_day: u32,
    season_start_holders: u32,
    seasons_won: u32,
    seasons_lost: u32,
    challenge_spent_day: u32,
    challenge_spent: u128,
}

fn bps(amount: u128, b: u32) -> u128 {
    amount * b as u128 / BPS
}

fn pk_bytes(pk: &PublicKey) -> [u8; 32] {
    let raw = pk.as_bytes();
    require!(raw.len() == 33 && raw[0] == 0, "ed25519 key required");
    raw[1..].try_into().unwrap()
}

fn sig_bytes(sig: &Base64VecU8) -> [u8; 64] {
    sig.0.as_slice().try_into().unwrap_or_else(|_| env::panic_str("signature must be 64 bytes"))
}

#[near]
impl Contract {
    #[init]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        symbol: String,
        icon: Option<String>,
        config: Config,
        creator: AccountId,
        launch_receiver: AccountId,
        oracle_pk: PublicKey,
        links: Vec<Link>,
    ) -> Self {
        let c = &config;
        require!(c.milestones.len() == c.tranche_bps.len(), "milestones/tranches length mismatch");
        require!(c.milestones.windows(2).all(|w| w[0] < w[1]), "milestones must increase");
        let tranche_total: u32 = c.tranche_bps.iter().sum();
        require!(
            c.launch_bps + c.pool_seed_bps + tranche_total <= BPS as u32,
            "launch + seed + tranches exceed max supply"
        );
        require!(c.tranche_to_pool_bps <= BPS as u32 && c.quorum_bps <= BPS as u32, "bad bps");
        require!(c.day_ms.0 >= 1_000 && c.streak_cap > 0 && c.age_full_days > 0, "bad timing");
        require!(c.season_days > 0, "season_days must be > 0");
        pk_bytes(&oracle_pk);

        let max = c.max_supply.0;
        let mut this = Self {
            ft: FungibleToken::new(Key::Ft),
            metadata: FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name,
                symbol,
                icon,
                reference: None,
                reference_hash: None,
                decimals: 18,
            },
            config: config.clone(),
            creator: creator.clone(),
            launch_receiver: launch_receiver.clone(),
            oracle_pk,
            links,
            genesis_ms: None,
            since: LookupMap::new(Key::Since),
            holders: IterableSet::new(Key::Holders),
            members: LookupMap::new(Key::Members),
            days: LookupMap::new(Key::Days),
            day_clusters: LookupSet::new(Key::DayClusters),
            clusters: LookupMap::new(Key::Clusters),
            challenges: Vec::new(),
            used_nonces: LookupSet::new(Key::UsedNonces),
            challenge_last: LookupMap::new(Key::ChallengeLast),
            week_burns: LookupMap::new(Key::WeekBurns),
            week_top: Vec::new(),
            ledger: Vector::new(Key::Ledger),
            reserve: max - bps(max, config.launch_bps) - bps(max, config.pool_seed_bps),
            pool_committed: 0,
            burned: 0,
            cancelled: 0,
            creator_unlocked: 0,
            creator_claimed: 0,
            milestone_idx: 0,
            independent_holders: 0,
            below_since_ms: None,
            last_attest_ts: 0,
            community_streak: 0,
            best_streak: 0,
            next_settle_day: 0,
            season_start_holders: 0,
            seasons_won: 0,
            seasons_lost: 0,
            challenge_spent_day: 0,
            challenge_spent: 0,
        };
        let me = env::current_account_id();
        this.ft.internal_register_account(&me);
        for acct in [&creator, &launch_receiver] {
            if this.ft.accounts.get(acct).is_none() {
                this.ft.internal_register_account(acct);
            }
        }
        this.mint(&launch_receiver, bps(max, config.launch_bps), "launch");
        this.mint(&me, bps(max, config.pool_seed_bps), "pool seed");
        this.touch(&launch_receiver);
        this
    }

    // ------------------------------------------------------------------ ritual

    /// Once per day. Settles finished days, pays any claimable rewards, records today's check-in.
    pub fn check_in(&mut self) -> U128 {
        let who = env::predecessor_account_id();
        require!(self.ft.accounts.get(&who).is_some(), "register with storage_deposit first");
        let bal = self.ft.internal_unwrap_balance_of(&who);
        require!(bal >= self.config.min_balance_to_check_in.0, "balance below check-in minimum");
        require!(self.genesis_ms.is_some(), "the ritual has not started yet");
        self.settle();
        let today = self.current_day();
        let mut m = self.members.get(&who).cloned().unwrap_or_default();
        require!(m.last_day != Some(today), "already checked in today");
        m.streak = match m.last_day {
            Some(d) if d + 1 == today => m.streak + 1,
            _ => 1,
        };
        m.last_day = Some(today);

        let w = self.weight_of(&who, bal, m.streak);
        let mut day = self.days.get(&today).cloned().unwrap_or_default();
        day.checkins += 1;
        let unique = match self.clusters.get(&who) {
            Some(c) if c.size > 1 => self.day_clusters.insert(format!("{today}:{}", c.id)),
            _ => true,
        };
        if unique {
            day.unique += 1;
        }
        day.total_weight = U128(day.total_weight.0 + w);
        self.days.insert(today, day);
        m.pending.push((today, U128(w)));
        self.members.insert(who.clone(), m);
        let paid = self.internal_claim(&who);
        env::log_str(&format!("check_in {who} day {today} weight {w} paid {paid}"));
        U128(paid)
    }

    /// Settle finished days (anyone may call; also runs inside `check_in`).
    pub fn settle(&mut self) {
        if self.genesis_ms.is_none() {
            return;
        }
        let today = self.current_day();
        let mut n = 0;
        while self.next_settle_day < today && n < MAX_SETTLE_PER_CALL {
            self.settle_day(self.next_settle_day);
            self.next_settle_day += 1;
            n += 1;
        }
    }

    /// Claim rewards for settled days.
    pub fn claim(&mut self) -> U128 {
        self.settle();
        let who = env::predecessor_account_id();
        U128(self.internal_claim(&who))
    }

    /// Voluntary burn into this week's fire. The largest offering holds the weekly title.
    pub fn bonfire(&mut self, amount: U128) {
        let who = env::predecessor_account_id();
        require!(amount.0 > 0, "amount must be > 0");
        self.burn(&who, amount.0, "bonfire");
        self.touch(&who);
        let week = self.current_day() / 7;
        let key = format!("{week}:{who}");
        let total = self.week_burns.get(&key).copied().unwrap_or(0) + amount.0;
        self.week_burns.insert(key, total);
        let mut m = self.members.get(&who).cloned().unwrap_or_default();
        m.burned_total = U128(m.burned_total.0 + amount.0);
        self.members.insert(who.clone(), m);
        match self.week_top.iter_mut().find(|(w, _, _)| *w == week) {
            Some(top) if total > top.2 .0 => *top = (week, who, U128(total)),
            Some(_) => {}
            None => self.week_top.push((week, who, U128(total))),
        }
    }

    // ------------------------------------------------------------------ oracle

    /// Submit a signed Organic Score attestation (anyone may relay it).
    pub fn attest(&mut self, payload: String, signature: Base64VecU8) {
        require!(
            env::ed25519_verify(&sig_bytes(&signature), payload.as_bytes(), &pk_bytes(&self.oracle_pk)),
            "bad oracle signature"
        );
        let a: Attestation = serde_json::from_str(&payload)
            .unwrap_or_else(|_| env::panic_str("bad attestation payload"));
        require!(a.ts.0 > self.last_attest_ts, "stale attestation");
        self.last_attest_ts = a.ts.0;
        for c in a.clusters {
            if c.size <= 1 {
                self.clusters.remove(&c.account_id);
            } else {
                self.clusters.insert(c.account_id, Cluster { id: c.cluster_id, size: c.size });
            }
        }
        self.settle();
        self.independent_holders = a.independent_holders;
        if self.genesis_ms.is_none() && a.independent_holders >= self.config.quorum_min {
            self.genesis_ms = Some(env::block_timestamp_ms());
            self.season_start_holders = a.independent_holders;
            env::log_str(&format!("ritual started with {} independent holders", a.independent_holders));
        }
        self.apply_growth();
    }

    // ------------------------------------------------------------------ creator

    /// Creator earn-out: mint what the milestones have unlocked so far.
    pub fn claim_creator(&mut self) -> U128 {
        let amt = self.creator_unlocked - self.creator_claimed;
        require!(amt > 0, "nothing unlocked");
        self.creator_claimed += amt;
        let creator = self.creator.clone();
        self.mint(&creator, amt, "creator earn-out");
        self.touch(&creator);
        U128(amt)
    }

    /// Creator registers a challenge. Budgets are bounded by `challenge_daily_cap_bps` at payout.
    pub fn add_challenge(
        &mut self,
        name: String,
        proof: Proof,
        reward: U128,
        cooldown_days: u32,
        daily_budget: U128,
    ) -> u32 {
        require!(env::predecessor_account_id() == self.creator, "only the creator");
        if let Proof::Signed { issuer_pk } = &proof {
            pk_bytes(issuer_pk);
        }
        require!(reward.0 > 0 && daily_budget.0 >= reward.0, "bad reward/budget");
        self.challenges.push(Challenge {
            name,
            proof,
            reward,
            cooldown_days,
            daily_budget,
            active: true,
            spent_day: 0,
            spent: U128(0),
        });
        (self.challenges.len() - 1) as u32
    }

    /// Creator may switch a challenge off (never on again, never change its terms).
    pub fn retire_challenge(&mut self, id: u32) {
        require!(env::predecessor_account_id() == self.creator, "only the creator");
        self.challenges[id as usize].active = false;
    }

    /// Claim a challenge proven by a signature from the creator's server.
    pub fn claim_signed(&mut self, challenge_id: u32, nonce: String, signature: Base64VecU8) -> U128 {
        let who = env::predecessor_account_id();
        let ch = self.challenge(challenge_id);
        let pk = match &ch.proof {
            Proof::Signed { issuer_pk } => pk_bytes(issuer_pk),
            _ => env::panic_str("not a signed challenge"),
        };
        let msg = format!(
            "grassroots-challenge:{}:{}:{}:{}",
            env::current_account_id(),
            challenge_id,
            who,
            nonce
        );
        require!(env::ed25519_verify(&sig_bytes(&signature), msg.as_bytes(), &pk), "bad challenge signature");
        require!(self.used_nonces.insert(format!("{challenge_id}:{nonce}")), "nonce already used");
        U128(self.pay_challenge(&who, challenge_id))
    }

    /// Claim a challenge proven by the creator's verifier contract.
    pub fn claim_verified(&mut self, challenge_id: u32, proof: String) -> Promise {
        let who = env::predecessor_account_id();
        let verifier = match &self.challenge(challenge_id).proof {
            Proof::Contract { verifier_id } => verifier_id.clone(),
            _ => env::panic_str("not a contract-verified challenge"),
        };
        ext_verifier::ext(verifier)
            .with_static_gas(Gas::from_tgas(10))
            .verify(who.clone(), proof)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(20))
                    .on_verified(who, challenge_id),
            )
    }

    #[private]
    pub fn on_verified(
        &mut self,
        account_id: AccountId,
        challenge_id: u32,
        #[callback_result] result: Result<bool, PromiseError>,
    ) -> U128 {
        match result {
            Ok(true) => U128(self.pay_challenge(&account_id, challenge_id)),
            _ => {
                env::log_str("verifier rejected the proof");
                U128(0)
            }
        }
    }

    // ------------------------------------------------------------------ views

    pub fn get_config(&self) -> Config {
        self.config.clone()
    }

    pub fn get_links(&self) -> Vec<Link> {
        self.links.clone()
    }

    pub fn get_creator(&self) -> AccountId {
        self.creator.clone()
    }

    pub fn get_launch_receiver(&self) -> AccountId {
        self.launch_receiver.clone()
    }

    pub fn get_oracle_pk(&self) -> PublicKey {
        self.oracle_pk.clone()
    }

    pub fn get_state(&self) -> State {
        State {
            total_supply: U128(self.ft.total_supply),
            reserve: U128(self.reserve),
            pool_free: U128(self.pool_free()),
            pool_committed: U128(self.pool_committed),
            burned: U128(self.burned),
            cancelled: U128(self.cancelled),
            creator_unlocked: U128(self.creator_unlocked),
            creator_claimed: U128(self.creator_claimed),
            milestone_idx: self.milestone_idx,
            independent_holders: self.independent_holders,
            community_streak: self.community_streak,
            best_streak: self.best_streak,
            current_day: self.current_day(),
            next_settle_day: self.next_settle_day,
            season_start_holders: self.season_start_holders,
            seasons_won: self.seasons_won,
            seasons_lost: self.seasons_lost,
            ritual_started_ms: self.genesis_ms.map(U64),
        }
    }

    pub fn get_member(&self, account_id: AccountId) -> Option<Member> {
        self.members.get(&account_id).cloned()
    }

    pub fn get_day(&self, day: u32) -> Option<Day> {
        self.days.get(&day).cloned()
    }

    pub fn get_cluster(&self, account_id: AccountId) -> Option<Cluster> {
        self.clusters.get(&account_id).cloned()
    }

    pub fn get_holding_since(&self, account_id: AccountId) -> Option<U64> {
        self.since.get(&account_id).map(|t| U64(*t))
    }

    /// The weight `account_id` would get if it checked in now.
    pub fn preview_weight(&self, account_id: AccountId) -> U128 {
        let bal = self.ft.internal_unwrap_balance_of(&account_id);
        let today = self.current_day();
        let streak = match self.members.get(&account_id) {
            Some(m) => match m.last_day {
                Some(d) if d + 1 == today => m.streak + 1,
                Some(d) if d == today => m.streak,
                _ => 1,
            },
            None => 1,
        };
        U128(self.weight_of(&account_id, bal, streak))
    }

    /// Paginated holders with balance and holding-since time: everything the Organic Score
    /// oracle needs, readable from any public RPC.
    pub fn get_holders(&self, from_index: u32, limit: u32) -> Vec<(AccountId, U128, U64)> {
        self.holders
            .iter()
            .skip(from_index as usize)
            .take(limit.min(200) as usize)
            .map(|a| {
                (
                    a.clone(),
                    U128(self.ft.internal_unwrap_balance_of(a)),
                    U64(self.since.get(a).copied().unwrap_or(0)),
                )
            })
            .collect()
    }

    pub fn get_holder_count(&self) -> u32 {
        self.holders.len()
    }

    pub fn get_challenges(&self) -> Vec<Challenge> {
        self.challenges.clone()
    }

    /// The public supply ledger, oldest first. Page with `from_index` / `limit` (max 200).
    pub fn get_ledger(&self, from_index: u32, limit: u32) -> Vec<LedgerEntry> {
        let end = (from_index.saturating_add(limit.min(200))).min(self.ledger.len());
        (from_index..end).filter_map(|i| self.ledger.get(i).cloned()).collect()
    }

    pub fn get_ledger_len(&self) -> u32 {
        self.ledger.len()
    }

    pub fn get_week_top(&self) -> Vec<(u32, AccountId, U128)> {
        self.week_top.clone()
    }

    pub fn current_day(&self) -> u32 {
        match self.genesis_ms {
            Some(g) => ((env::block_timestamp_ms() - g) / self.config.day_ms.0) as u32,
            None => 0,
        }
    }
}

// ---------------------------------------------------------------------- internals

impl Contract {
    fn pool_balance(&self) -> u128 {
        self.ft.internal_unwrap_balance_of(&env::current_account_id())
    }

    fn pool_free(&self) -> u128 {
        self.pool_balance().saturating_sub(self.pool_committed)
    }

    fn record(&mut self, kind: &str, account: Option<AccountId>, amount: u128, memo: &str) {
        if amount == 0 {
            return;
        }
        let day = self.current_day();
        self.ledger.push(LedgerEntry {
            day,
            ts_ms: U64(env::block_timestamp_ms()),
            kind: kind.to_string(),
            account,
            amount: U128(amount),
            memo: memo.to_string(),
        });
        self.ledger.flush();
    }

    fn mint(&mut self, to: &AccountId, amount: u128, memo: &str) {
        if amount == 0 {
            return;
        }
        self.ft.internal_deposit(to, amount);
        FtMint { owner_id: to, amount: U128(amount), memo: Some(memo) }.emit();
        self.record("mint", Some(to.clone()), amount, memo);
    }

    fn burn(&mut self, from: &AccountId, amount: u128, memo: &str) {
        if amount == 0 {
            return;
        }
        self.ft.internal_withdraw(from, amount);
        self.burned += amount;
        FtBurn { owner_id: from, amount: U128(amount), memo: Some(memo) }.emit();
        self.record("burn", Some(from.clone()), amount, memo);
    }

    /// Keep "holding since" in step with balances: set when an account starts holding, clear at zero.
    fn touch(&mut self, account: &AccountId) {
        if *account == env::current_account_id() {
            return; // the pool is not a holder
        }
        let bal = self.ft.accounts.get(account).unwrap_or(0);
        if bal == 0 {
            self.since.remove(account);
            self.holders.remove(account);
        } else {
            if !self.since.contains_key(account) {
                self.since.insert(account.clone(), env::block_timestamp_ms());
            }
            self.holders.insert(account.clone());
        }
    }

    fn weight_of(&self, who: &AccountId, bal: u128, streak: u32) -> u128 {
        let c = &self.config;
        let indep = match self.clusters.get(who) {
            Some(cl) if cl.size > 1 => SCALE / cl.size as u128,
            _ => SCALE,
        };
        let held_ms = self
            .since
            .get(who)
            .map(|t| env::block_timestamp_ms().saturating_sub(*t))
            .unwrap_or(0) as u128;
        let full_ms = c.age_full_days as u128 * c.day_ms.0 as u128;
        let age = (SCALE * held_ms / full_ms).min(SCALE);
        let s = streak.min(c.streak_cap) as u128;
        let streak_f = SCALE + SCALE * c.streak_max_bonus_bps as u128 / BPS * s / c.streak_cap as u128;
        let cap = bps(c.max_supply.0, c.balance_cap_bps).max(1);
        let stake = SCALE * bal.min(cap) / cap;
        indep * age / SCALE * streak_f / SCALE * stake / SCALE
    }

    fn settle_day(&mut self, d: u32) {
        let mut day = self.days.get(&d).cloned().unwrap_or_default();
        let quorum = self
            .config
            .quorum_min
            .max((self.independent_holders as u128 * self.config.quorum_bps as u128 / BPS) as u32);
        day.quorum = quorum;
        day.settled = true;
        if day.unique >= quorum && day.total_weight.0 > 0 {
            day.hit = true;
            let payout = bps(self.pool_free(), self.config.daily_payout_bps);
            day.payout = U128(payout);
            self.pool_committed += payout;
            self.record("payout", None, payout, &format!("day {d} ritual: quorum hit, paid to that day's check-ins"));
            self.community_streak += 1;
            self.best_streak = self.best_streak.max(self.community_streak);
        } else {
            let amt = bps(self.pool_free(), self.config.miss_burn_bps);
            let me = env::current_account_id();
            self.burn(&me, amt, "missed quorum");
            day.burned = U128(amt);
            self.community_streak = 0;
        }
        self.days.insert(d, day);
        if (d + 1) % self.config.season_days == 0 {
            self.end_season();
        }
    }

    fn end_season(&mut self) {
        let goal = (self.season_start_holders as u128
            * (BPS + self.config.season_growth_bps as u128)
            / BPS)
            .max(self.config.quorum_min as u128) as u32;
        let bonus = bps(self.config.max_supply.0, self.config.season_bonus_bps).min(self.reserve);
        self.reserve -= bonus;
        if self.independent_holders >= goal {
            let me = env::current_account_id();
            self.mint(&me, bonus, "season bonus");
            self.seasons_won += 1;
        } else {
            self.cancelled += bonus;
            self.record("cancel", None, bonus, "season missed: bonus cancelled from reserve");
            self.seasons_lost += 1;
            env::log_str(&format!("season missed: {} < {goal}, {bonus} cancelled", self.independent_holders));
        }
        self.season_start_holders = self.independent_holders;
    }

    fn apply_growth(&mut self) {
        let max = self.config.max_supply.0;
        let n = self.independent_holders;
        let ms = self.config.milestones.clone();
        let mut grew = false;
        while (self.milestone_idx as usize) < ms.len() && n >= ms[self.milestone_idx as usize] {
            let amt = bps(max, self.config.tranche_bps[self.milestone_idx as usize]).min(self.reserve);
            self.reserve -= amt;
            let to_pool = bps(amt, self.config.tranche_to_pool_bps);
            let me = env::current_account_id();
            self.mint(&me, to_pool, "growth tranche");
            self.creator_unlocked += amt - to_pool;
            self.milestone_idx += 1;
            grew = true;
        }
        if grew {
            self.below_since_ms = None;
            return;
        }
        let now = env::block_timestamp_ms();
        if self.milestone_idx > 0 && n < ms[self.milestone_idx as usize - 1] {
            match self.below_since_ms {
                None => self.below_since_ms = Some(now),
                Some(t) if now - t >= self.config.contraction_days as u64 * self.config.day_ms.0 => {
                    let cut = bps(self.reserve, self.config.contraction_cut_bps);
                    self.reserve -= cut;
                    self.cancelled += cut;
                    self.record("cancel", None, cut, "contraction: community shrank below last milestone");
                    self.below_since_ms = Some(now);
                    env::log_str(&format!("contraction: {cut} of reserve cancelled"));
                }
                _ => {}
            }
        } else {
            self.below_since_ms = None;
        }
    }

    fn internal_claim(&mut self, who: &AccountId) -> u128 {
        let Some(mut m) = self.members.get(who).cloned() else { return 0 };
        let mut paid = 0u128;
        let mut keep = Vec::new();
        for (d, w) in m.pending.drain(..) {
            if d >= self.next_settle_day {
                keep.push((d, w));
                continue;
            }
            let day = self.days.get(&d).cloned().unwrap_or_default();
            if day.hit && day.total_weight.0 > 0 {
                paid += day.payout.0 * w.0 / day.total_weight.0;
            }
        }
        m.pending = keep;
        if paid > 0 {
            let paid_now = paid.min(self.pool_committed);
            self.pool_committed -= paid_now;
            let me = env::current_account_id();
            self.ft.internal_transfer(&me, who, paid_now, Some("ritual reward".into()));
            self.touch(who);
            m.rewards_total = U128(m.rewards_total.0 + paid_now);
            paid = paid_now;
        }
        self.members.insert(who.clone(), m);
        paid
    }

    fn challenge(&self, id: u32) -> Challenge {
        let ch = self
            .challenges
            .get(id as usize)
            .cloned()
            .unwrap_or_else(|| env::panic_str("no such challenge"));
        require!(ch.active, "challenge retired");
        ch
    }

    /// Pays a completed challenge, applying the same anti-farm factors and every budget cap.
    fn pay_challenge(&mut self, who: &AccountId, id: u32) -> u128 {
        require!(self.ft.accounts.get(who).is_some(), "register with storage_deposit first");
        let today = self.current_day();
        let last_key = format!("{id}:{who}");
        let mut ch = self.challenge(id);
        if let Some(last) = self.challenge_last.get(&last_key) {
            require!(today >= last + ch.cooldown_days.max(1), "challenge on cooldown");
        }
        // anti-farm: independence share x holding-age ramp (no streak or stake factor here)
        let c = &self.config;
        let indep = match self.clusters.get(who) {
            Some(cl) if cl.size > 1 => SCALE / cl.size as u128,
            _ => SCALE,
        };
        let held_ms = self
            .since
            .get(who)
            .map(|t| env::block_timestamp_ms().saturating_sub(*t))
            .unwrap_or(0) as u128;
        let age = (SCALE * held_ms / (c.age_full_days as u128 * c.day_ms.0 as u128)).min(SCALE);
        let mut amount = ch.reward.0 * indep / SCALE * age / SCALE;

        if ch.spent_day != today {
            ch.spent_day = today;
            ch.spent = U128(0);
        }
        if self.challenge_spent_day != today {
            self.challenge_spent_day = today;
            self.challenge_spent = 0;
        }
        let global_cap = bps(self.pool_free(), c.challenge_daily_cap_bps);
        amount = amount
            .min(ch.daily_budget.0.saturating_sub(ch.spent.0))
            .min(global_cap.saturating_sub(self.challenge_spent));
        require!(amount > 0, "nothing payable (new holder, cluster share, or budget exhausted)");
        ch.spent = U128(ch.spent.0 + amount);
        self.challenge_spent += amount;
        self.challenges[id as usize] = ch;
        self.challenge_last.insert(last_key, today);
        let me = env::current_account_id();
        self.ft.internal_transfer(&me, who, amount, Some(format!("challenge {id}")));
        let name = self.challenges[id as usize].name.clone();
        self.record("payout", Some(who.clone()), amount, &format!("challenge {id}: {name}"));
        self.touch(who);
        let mut m = self.members.get(who).cloned().unwrap_or_default();
        m.rewards_total = U128(m.rewards_total.0 + amount);
        self.members.insert(who.clone(), m);
        amount
    }
}

// ---------------------------------------------------------------------- NEP-141 / NEP-145

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        let sender = env::predecessor_account_id();
        self.ft.ft_transfer(receiver_id.clone(), amount, memo);
        self.touch(&sender);
        self.touch(&receiver_id);
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let sender = env::predecessor_account_id();
        let r = self.ft.ft_transfer_call(receiver_id.clone(), amount, memo, msg);
        self.touch(&sender);
        self.touch(&receiver_id);
        r
    }

    fn ft_total_supply(&self) -> U128 {
        self.ft.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.ft.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(&mut self, sender_id: AccountId, receiver_id: AccountId, amount: U128) -> U128 {
        let (used, burned) = self.ft.internal_ft_resolve_transfer(&sender_id, receiver_id.clone(), amount);
        if burned > 0 {
            self.burned += burned;
        }
        self.touch(&sender_id);
        self.touch(&receiver_id);
        used.into()
    }
}

#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(&mut self, account_id: Option<AccountId>, registration_only: Option<bool>) -> StorageBalance {
        self.ft.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.ft.storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        let who = env::predecessor_account_id();
        let r = self.ft.internal_storage_unregister(force);
        if let Some((_, burned)) = r {
            self.burned += burned;
            self.since.remove(&who);
            return true;
        }
        false
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.ft.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.ft.storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.clone()
    }
}

#[cfg(test)]
mod tests;
