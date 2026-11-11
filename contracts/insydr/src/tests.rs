use super::*;
use ed25519_dalek::{Signer, SigningKey};
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::testing_env;

const DAY_MS: u64 = 60_000;
const ONE: u128 = 1_000_000_000_000_000_000; // 1 token (18 decimals)
const MAX: u128 = 1_000_000_000 * ONE;

fn acc(s: &str) -> AccountId {
    s.parse().unwrap()
}

fn coin() -> AccountId {
    acc("coin.near")
}

struct T {
    c: Contract,
    oracles: Vec<SigningKey>,
    now_ms: u64,
    ts: u64,
    crowd: u32,
}

fn set_ctx(now_ms: u64, who: &AccountId, yocto: u128) {
    testing_env!(VMContextBuilder::new()
        .current_account_id(coin())
        .predecessor_account_id(who.clone())
        .signer_account_id(who.clone())
        .attached_deposit(NearToken::from_yoctonear(yocto))
        .block_timestamp(now_ms * 1_000_000)
        .build());
}

fn keypair(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn pk_of(k: &SigningKey) -> PublicKey {
    format!("ed25519:{}", bs58::encode(k.verifying_key().to_bytes()).into_string())
        .parse()
        .unwrap()
}

fn cfg() -> Config {
    Config {
        max_supply: U128(MAX),
        launch_bps: 3_500,
        pool_seed_bps: 500,
        milestones: vec![50, 100, 200],
        tranche_bps: vec![500, 700, 900],
        tranche_to_pool_bps: 8_000,
        contraction_days: 7,
        contraction_cut_bps: 1_000,
        quorum_min: 2,
        quorum_bps: 7_500,
        quorum_window_days: 7,
        daily_payout_bps: 50,
        miss_burn_bps: 25,
        age_full_days: 14,
        streak_cap: 30,
        streak_max_bonus_bps: 5_000,
        streak_miss_penalty: 3,
        balance_cap_bps: 5, // 0.05% of max supply = 500k tokens
        min_balance_to_check_in: U128(ONE),
        season_days: 30,
        season_growth_bps: 5_000,
        season_bonus_bps: 200,
        challenge_daily_cap_bps: 100,
        challenge_account_cap_bps: 10_000, // most tests: no per-account limit
        holder_min_balance: U128(ONE),
        holder_min_days: 14,
        tranche_cooldown_days: 0,
        creator_vest_days: 0,
        day_ms: U64(DAY_MS),
    }
}

impl T {
    /// Deploys without starting the ritual. `seeds` are the oracle keys, `threshold` of them must sign.
    fn deploy(config: Config, seeds: &[u8], threshold: u32) -> Self {
        let oracles: Vec<SigningKey> = seeds.iter().map(|s| keypair(*s)).collect();
        let now_ms = 1_000_000;
        set_ctx(now_ms, &acc("deployer.near"), 0);
        let c = Contract::new(
            "INSYDR Dog".into(),
            "IDOG".into(),
            None,
            config,
            acc("creator.near"),
            acc("launch.near"),
            oracles.iter().map(pk_of).collect(),
            threshold,
            vec![Link { label: "x".into(), url: "https://x.com/idog".into() }],
        );
        T { c, oracles, now_ms, ts: 0, crowd: 0 }
    }

    fn new_with(config: Config) -> Self {
        let mut t = Self::deploy(config, &[9], 1);
        let q = t.c.config.quorum_min;
        t.attest(q, vec![]); // start the ritual
        t
    }

    fn new() -> Self {
        Self::new_with(cfg())
    }
    fn ctx(&self, who: &AccountId, yocto: u128) {
        set_ctx(self.now_ms, who, yocto);
    }
    fn advance_days(&mut self, d: f64) {
        self.now_ms += (d * DAY_MS as f64) as u64;
        self.ctx(&acc("anyone.near"), 0);
    }
    fn register(&mut self, who: &str) {
        self.ctx(&acc(who), 0);
        self.c.ft.internal_register_account(&acc(who));
    }
    fn give(&mut self, who: &str, amount: u128) {
        if self.c.ft.accounts.get(&acc(who)).is_none() {
            self.register(who);
        }
        self.ctx(&acc("launch.near"), 1);
        self.c.ft_transfer(acc(who), U128(amount), None);
    }
    fn check_in(&mut self, who: &str) -> u128 {
        self.ctx(&acc(who), 0);
        self.c.check_in().0
    }
    fn claim(&mut self, who: &str) -> u128 {
        self.ctx(&acc(who), 0);
        self.c.claim().0
    }
    fn bal(&self, who: &str) -> u128 {
        self.c.ft.internal_unwrap_balance_of(&acc(who))
    }
    /// Make sure at least `n` accounts hold the coin on-chain (the contract refuses an
    /// attestation that reports more independent holders than exist).
    fn ensure_holders(&mut self, n: u32) {
        while self.c.get_holder_count() < n {
            let who = format!("crowd{}.near", self.crowd);
            self.crowd += 1;
            self.give(&who, ONE);
        }
    }
    fn payload(&mut self, holders: u32, clusters: &[(&str, &str, u32)]) -> String {
        self.ts += 1;
        let cl: Vec<String> = clusters
            .iter()
            .map(|(a, id, n)| format!(r#"{{"account_id":"{a}","cluster_id":"{id}","size":{n}}}"#))
            .collect();
        format!(r#"{{"ts":"{}","independent_holders":{},"clusters":[{}]}}"#, self.ts, holders, cl.join(","))
    }
    fn sigs(&self, payload: &str, signers: &[usize]) -> Vec<OracleSig> {
        signers
            .iter()
            .map(|i| OracleSig {
                index: *i as u32,
                signature: Base64VecU8(self.oracles[*i].sign(payload.as_bytes()).to_bytes().to_vec()),
            })
            .collect()
    }
    fn attest(&mut self, holders: u32, clusters: Vec<(&str, &str, u32)>) {
        self.ensure_holders(holders);
        let payload = self.payload(holders, &clusters);
        let all: Vec<usize> = (0..self.oracles.len()).collect();
        let sigs = self.sigs(&payload, &all);
        self.ctx(&acc("relayer.near"), 0);
        self.c.attest(payload, sigs);
    }
    fn challenge(&mut self, reward: u128, budget: u128) -> (u32, SigningKey) {
        let issuer = keypair(7);
        self.ctx(&acc("creator.near"), 0);
        let id = self.c.add_challenge("quest".into(), Proof::Signed { issuer_pk: pk_of(&issuer) }, U128(reward), 0, U128(budget));
        (id, issuer)
    }
    fn complete(&mut self, id: u32, issuer: &SigningKey, who: &str, nonce: &str) -> u128 {
        let msg = format!("insydr-challenge:coin.near:{id}:{who}:{nonce}");
        let sig = Base64VecU8(issuer.sign(msg.as_bytes()).to_bytes().to_vec());
        self.ctx(&acc(who), 0);
        self.c.claim_signed(id, nonce.into(), sig).0
    }
}

#[test]
fn launch_mints_only_launch_and_pool_seed() {
    let t = T::deploy(cfg(), &[9], 1);
    assert_eq!(t.c.ft.total_supply, MAX * 40 / 100);
    assert_eq!(t.bal("launch.near"), MAX * 35 / 100);
    assert_eq!(t.c.pool_balance(), MAX * 5 / 100);
    assert_eq!(t.c.reserve, MAX * 60 / 100);
}

#[test]
fn ritual_hit_pays_checkins_next_day() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    t.give("bob.near", 1_000 * ONE);
    t.advance_days(15.0); // full holding-age weight
    t.c.settle(); // days 0-14 had no check-ins and already burned part of the pool
    let pool_before = t.c.pool_balance();
    t.check_in("alice.near");
    t.check_in("bob.near");
    t.advance_days(1.0);
    t.c.settle();
    let claimable = t.c.get_claimable(acc("alice.near")).0;
    let paid_a = t.check_in("alice.near"); // pays yesterday
    assert_eq!(claimable, paid_a, "the view matches what a claim pays");
    let paid_b = t.claim("bob.near");
    let day = t.c.get_day(15).unwrap();
    assert!(day.hit);
    let expected = pool_before * 50 / 10_000;
    assert_eq!(day.payout.0, expected);
    assert!(paid_a > 0 && paid_b > 0);
    assert!(paid_a.abs_diff(paid_b) <= 1, "equal holders, equal pay");
    assert_eq!(t.c.community_streak, 1);
}

#[test]
#[should_panic(expected = "already checked in today")]
fn one_check_in_per_day() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    t.check_in("alice.near");
    t.check_in("alice.near");
}

#[test]
fn missed_quorum_burns_pool_and_resets_streak() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    t.advance_days(15.0);
    let supply_before = t.c.ft.total_supply;
    t.check_in("alice.near"); // quorum_min = 2, only one shows up
    t.advance_days(1.0);
    t.claim("alice.near");
    let day = t.c.get_day(15).unwrap();
    assert!(!day.hit);
    assert!(day.burned.0 > 0);
    assert!(t.c.ft.total_supply < supply_before);
    assert_eq!(t.c.community_streak, 0);
}

#[test]
fn empty_days_burn_too() {
    let mut t = T::new();
    t.advance_days(5.0);
    let before = t.c.pool_balance();
    t.ctx(&acc("anyone.near"), 0);
    t.c.settle();
    assert_eq!(t.c.next_settle_day, 5);
    assert!(t.c.pool_balance() < before);
}

#[test]
fn splitting_a_stack_earns_nothing_extra() {
    // one honest holder with 3,000 vs a farmer splitting 3,000 across 3 accounts (undetected)
    let mut t = T::new();
    t.give("honest.near", 3_000 * ONE);
    for f in ["f1.near", "f2.near", "f3.near"] {
        t.give(f, 1_000 * ONE);
    }
    t.advance_days(15.0);
    let w_honest = t.c.preview_weight(acc("honest.near")).0;
    let w_farm: u128 = ["f1.near", "f2.near", "f3.near"].iter().map(|f| t.c.preview_weight(acc(f)).0).sum();
    assert!(w_honest.abs_diff(w_farm) <= 3, "{w_honest} vs {w_farm}");
}

#[test]
fn detected_cluster_shares_one_weight() {
    let mut t = T::new();
    t.give("honest.near", 1_000 * ONE);
    for f in ["f1.near", "f2.near", "f3.near"] {
        t.give(f, 1_000 * ONE);
    }
    t.attest(10, vec![("f1.near", "farm", 3), ("f2.near", "farm", 3), ("f3.near", "farm", 3)]);
    t.advance_days(15.0);
    let w_honest = t.c.preview_weight(acc("honest.near")).0;
    let w_farm: u128 = ["f1.near", "f2.near", "f3.near"].iter().map(|f| t.c.preview_weight(acc(f)).0).sum();
    // the farm holds 3x the honest stack in total, yet the detected cluster weighs like one honest account
    assert!(w_farm.abs_diff(w_honest) <= 3, "{w_farm} vs {w_honest}");
    // and the cluster counts once toward the daily quorum
    for f in ["f1.near", "f2.near", "f3.near"] {
        t.check_in(f);
    }
    assert_eq!(t.c.get_day(15).unwrap().unique, 1);
    assert_eq!(t.c.get_day(15).unwrap().checkins, 3);
}

#[test]
fn new_holders_ramp_in() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    t.advance_days(1.0);
    let w1 = t.c.preview_weight(acc("alice.near")).0;
    t.advance_days(13.0);
    let w14 = t.c.preview_weight(acc("alice.near")).0;
    assert!(w1 * 10 < w14, "a 1-day holder weighs ~1/14 of a 14-day holder");
}

#[test]
fn growth_tranche_mints_on_milestone_and_feeds_creator() {
    let mut t = T::new();
    let reserve0 = t.c.reserve;
    let pool0 = t.c.pool_balance();
    t.attest(49, vec![]);
    assert_eq!(t.c.milestone_idx, 0);
    t.attest(120, vec![]); // past 50 and 100, but one tranche per attestation
    assert_eq!(t.c.milestone_idx, 1);
    t.attest(120, vec![]);
    assert_eq!(t.c.milestone_idx, 2);
    let minted = MAX * (500 + 700) / 10_000;
    assert_eq!(reserve0 - t.c.reserve, minted);
    assert_eq!(t.c.pool_balance() - pool0, minted * 8 / 10);
    assert_eq!(t.c.creator_unlocked, minted * 2 / 10);
    t.ctx(&acc("creator.near"), 0);
    assert_eq!(t.c.claim_creator().0, minted * 2 / 10);
    assert_eq!(t.bal("creator.near"), minted * 2 / 10);
}

#[test]
#[should_panic(expected = "nothing vested")]
fn creator_gets_nothing_without_real_growth() {
    let mut t = T::new();
    t.ctx(&acc("creator.near"), 0);
    t.c.claim_creator();
}

#[test]
fn shrinking_community_cancels_reserve() {
    let mut t = T::new();
    t.attest(120, vec![]);
    t.attest(120, vec![]);
    let r = t.c.reserve;
    t.attest(80, vec![]); // below last milestone (100): clock starts
    t.advance_days(7.0);
    t.attest(80, vec![]);
    assert!(t.c.cancelled > 0);
    assert_eq!(r - t.c.reserve, t.c.cancelled);
}

#[test]
#[should_panic(expected = "bad oracle signature")]
fn rejects_forged_attestation() {
    let mut t = T::new();
    let payload = r#"{"ts":"5","independent_holders":9999,"clusters":[]}"#.to_string();
    let sig = keypair(1).sign(payload.as_bytes()).to_bytes().to_vec();
    t.ctx(&acc("attacker.near"), 0);
    t.c.attest(payload, vec![OracleSig { index: 0, signature: Base64VecU8(sig) }]);
}

#[test]
#[should_panic(expected = "stale attestation")]
fn rejects_replayed_attestation() {
    let mut t = T::new();
    t.attest(10, vec![]);
    t.ts -= 1;
    t.attest(10, vec![]);
}

#[test]
fn season_missed_cancels_bonus() {
    let mut c = cfg();
    c.season_days = 3;
    let mut t = T::new_with(c);
    t.attest(20, vec![]);
    t.advance_days(3.5);
    t.ctx(&acc("anyone.near"), 0);
    t.c.settle(); // season 0 ends at day 2; start holders were 0, goal = quorum_min
    assert_eq!(t.c.seasons_won, 1);
    t.advance_days(3.0);
    t.c.settle(); // season 1: needs 20 * 1.5 = 30, still 20
    assert_eq!(t.c.seasons_lost, 1);
    assert_eq!(t.c.cancelled, MAX * 200 / 10_000);
}

const CAP: u128 = MAX * 5 / 10_000; // balance_cap_bps = 5: a full-weight stake

#[test]
fn signed_challenge_pays_with_anti_farm() {
    let mut t = T::new();
    t.attest(60, vec![("f1.near", "farm", 4)]); // fills the pool via a tranche
    t.give("alice.near", CAP);
    t.give("f1.near", CAP);
    let (id, issuer) = t.challenge(100 * ONE, 10_000 * ONE);
    t.advance_days(15.0);
    assert_eq!(t.complete(id, &issuer, "alice.near", "n1"), 100 * ONE);
    assert_eq!(t.complete(id, &issuer, "f1.near", "n2"), 25 * ONE, "cluster of 4 gets 1/4");
}

#[test]
fn challenge_rewards_scale_with_stake_so_splitting_is_neutral() {
    // 30 undetected wallets completing a challenge earn what one wallet with their combined stake earns
    let mut t = T::new();
    t.attest(60, vec![]);
    t.give("whole.near", 3_000 * ONE);
    for i in 0..30 {
        t.give(&format!("split{i}.near"), 100 * ONE);
    }
    let (id, issuer) = t.challenge(1_000 * ONE, 1_000_000 * ONE);
    t.advance_days(15.0);
    let whole = t.complete(id, &issuer, "whole.near", "w");
    let split: u128 = (0..30).map(|i| t.complete(id, &issuer, &format!("split{i}.near"), &format!("s{i}"))).sum();
    assert!(whole > 0);
    assert!(whole.abs_diff(split) <= 30, "{whole} vs {split}");
}

#[test]
#[should_panic(expected = "nonce already used")]
fn signed_challenge_nonce_cannot_replay() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    let (id, issuer) = t.challenge(ONE, 100 * ONE);
    t.advance_days(15.0);
    t.complete(id, &issuer, "alice.near", "n1");
    t.advance_days(1.0);
    t.complete(id, &issuer, "alice.near", "n1");
}

#[test]
#[should_panic(expected = "bad challenge signature")]
fn signed_challenge_rejects_other_account() {
    let mut t = T::new();
    t.give("mallory.near", 1_000 * ONE);
    let (id, issuer) = t.challenge(ONE, 100 * ONE);
    let msg = format!("insydr-challenge:coin.near:{id}:alice.near:n1"); // signed for alice
    let sig = Base64VecU8(issuer.sign(msg.as_bytes()).to_bytes().to_vec());
    t.ctx(&acc("mallory.near"), 0);
    t.c.claim_signed(id, "n1".into(), sig);
}

#[test]
fn challenges_cannot_drain_the_pool() {
    let mut t = T::new();
    // creator tries a huge reward and budget for a wallet of their own
    let (id, issuer) = t.challenge(MAX, MAX);
    t.give("creator2.near", CAP);
    t.advance_days(15.0);
    t.c.settle();
    let pool = t.c.pool_free();
    let paid = t.complete(id, &issuer, "creator2.near", "n");
    assert_eq!(paid, pool / 100, "capped at challenge_daily_cap_bps (1%) of the pool");
}

#[test]
fn one_account_cannot_take_the_whole_challenge_budget() {
    let mut c = cfg();
    c.challenge_account_cap_bps = 500; // one account: at most 5% of the day's challenge cap
    let mut t = T::new_with(c);
    let (id, issuer) = t.challenge(MAX, MAX);
    let (id2, _) = t.challenge(MAX, MAX);
    t.give("creator2.near", CAP);
    t.advance_days(15.0);
    t.c.settle();
    let day_cap = t.c.pool_free() / 100;
    assert_eq!(t.complete(id, &issuer, "creator2.near", "a"), day_cap * 5 / 100);
    // a second challenge the same day does not get around it
    t.ctx(&acc("creator2.near"), 0);
    let msg = format!("insydr-challenge:coin.near:{id2}:creator2.near:b");
    let sig = Base64VecU8(issuer.sign(msg.as_bytes()).to_bytes().to_vec());
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.c.claim_signed(id2, "b".into(), sig)));
    assert!(r.is_err(), "nothing more is payable to this account today");
}

#[test]
#[should_panic(expected = "only the creator")]
fn only_creator_adds_challenges() {
    let mut t = T::new();
    t.ctx(&acc("mallory.near"), 0);
    t.c.add_challenge("x".into(), Proof::Contract { verifier_id: acc("v.near") }, U128(1), 0, U128(1));
}

#[test]
fn flagged_insiders_can_always_sell() {
    let mut t = T::new();
    t.give("insider.near", 10_000 * ONE);
    t.attest(10, vec![("insider.near", "deployer", 30)]);
    t.register("pool.near");
    t.ctx(&acc("insider.near"), 1);
    t.c.ft_transfer(acc("pool.near"), U128(10_000 * ONE), None);
    assert_eq!(t.bal("pool.near"), 10_000 * ONE);
    assert!(t.c.get_holding_since(acc("insider.near")).is_none(), "holding clock resets at zero");
}

#[test]
fn bonfire_burns_and_tracks_weekly_top() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    t.give("bob.near", 1_000 * ONE);
    let s = t.c.ft.total_supply;
    t.ctx(&acc("alice.near"), 0);
    t.c.bonfire(U128(10 * ONE));
    t.ctx(&acc("bob.near"), 0);
    t.c.bonfire(U128(25 * ONE));
    assert_eq!(s - t.c.ft.total_supply, 35 * ONE);
    let top = t.c.get_week_top();
    assert_eq!(top[0].1, acc("bob.near"));
}

#[test]
fn holders_list_tracks_balances() {
    let mut t = T::new();
    t.give("alice.near", 5 * ONE);
    t.give("bob.near", 5 * ONE);
    assert_eq!(t.c.get_holder_count(), 4); // launch.near, one crowd account, alice, bob (the pool is excluded)
    t.ctx(&acc("alice.near"), 1);
    t.c.ft_transfer(acc("bob.near"), U128(5 * ONE), None);
    let h: Vec<String> = t.c.get_holders(0, 10).into_iter().map(|h| h.account_id.to_string()).collect();
    assert!(!h.contains(&"alice.near".to_string()));
    assert!(h.contains(&"bob.near".to_string()));
}

#[test]
fn ledger_records_every_mint_burn_and_cancel() {
    let mut t = T::new();
    // launch + pool seed are the first two mints
    let launch = t.c.get_ledger(0, 10);
    assert_eq!(launch.len(), 2);
    assert!(launch.iter().all(|e| e.kind == "mint"));
    t.attest(120, vec![]);
    t.attest(120, vec![]); // two growth tranches
    t.advance_days(3.0);
    t.ctx(&acc("anyone.near"), 0);
    t.c.settle(); // empty days burn
    let all = t.c.get_ledger(0, 200);
    assert_eq!(all.len() as u32, t.c.get_ledger_len());
    let minted: u128 = all.iter().filter(|e| e.kind == "mint").map(|e| e.amount.0).sum();
    let burned: u128 = all.iter().filter(|e| e.kind == "burn").map(|e| e.amount.0).sum();
    assert_eq!(minted - burned, t.c.ft.total_supply, "ledger reconciles with total supply");
    assert_eq!(all.iter().filter(|e| e.memo == "growth tranche").count(), 2);
    assert!(all.iter().any(|e| e.memo == "missed quorum"));
    // paging
    assert_eq!(t.c.get_ledger(2, 2).len(), 2);
    assert!(t.c.get_ledger(1_000, 5).is_empty());
}

#[test]
fn idle_coin_is_not_drained_before_the_ritual_starts() {
    let mut c = cfg();
    c.quorum_min = 5;
    c.season_days = 3;
    let mut t = T::deploy(c, &[9], 1);
    let pool0 = t.c.pool_balance();
    let reserve0 = t.c.reserve;
    t.advance_days(500.0); // a long, quiet distribution period
    t.c.settle();
    assert_eq!(t.c.pool_balance(), pool0, "no burns before start");
    assert_eq!(t.c.reserve, reserve0, "no lost seasons before start");
    assert_eq!(t.c.current_day(), 0);
    t.attest(4, vec![]); // below quorum_min: still not started
    assert!(t.c.get_state().ritual_started_ms.is_none());
    t.attest(5, vec![]);
    assert!(t.c.get_state().ritual_started_ms.is_some());
    t.give("alice.near", 1_000 * ONE);
    t.check_in("alice.near");
}

#[test]
#[should_panic(expected = "the ritual has not started yet")]
fn no_check_in_before_start() {
    let mut c = cfg();
    c.quorum_min = 5;
    let mut t = T::deploy(c, &[9], 1);
    t.give("alice.near", 1_000 * ONE);
    t.check_in("alice.near");
}

// ------------------------------------------------------------------ strength-test regressions

#[test]
fn dust_aged_wallet_gains_nothing_when_loaded() {
    // attack: park 1 token in a wallet for weeks, then move a big stack in and claim "old holder" weight
    let mut t = T::new();
    t.give("honest.near", 100_000 * ONE);
    t.give("sleeper.near", ONE);
    t.advance_days(30.0);
    t.give("sleeper.near", 100_000 * ONE - ONE); // loads the aged wallet
    let honest = t.c.preview_weight(acc("honest.near")).0;
    let sleeper = t.c.preview_weight(acc("sleeper.near")).0;
    assert!(sleeper * 1_000 < honest, "coin-age restarts with the stack: {sleeper} vs {honest}");
    // and it ramps like any new holder afterwards
    t.advance_days(7.0);
    let half = t.c.preview_weight(acc("sleeper.near")).0;
    assert!(half * 2 >= honest * 99 / 100 && half < honest, "about half way after 7 of 14 days");
}

#[test]
fn rotating_one_stack_through_wallets_does_not_multiply_it() {
    // attack: check in with wallet A, move the same coins to aged wallet B, check in again
    let mut t = T::new();
    t.give("a.near", 100_000 * ONE);
    t.give("b.near", ONE);
    t.give("other.near", 100_000 * ONE);
    t.advance_days(20.0);
    t.c.settle();
    t.check_in("a.near");
    t.ctx(&acc("a.near"), 1);
    t.c.ft_transfer(acc("b.near"), U128(100_000 * ONE), None);
    t.check_in("b.near");
    t.check_in("other.near");
    let day = t.c.get_day(20).unwrap();
    let a_and_b = t.c.get_member(acc("a.near")).unwrap().pending[0].1 .0 + t.c.get_member(acc("b.near")).unwrap().pending[0].1 .0;
    let other = t.c.get_member(acc("other.near")).unwrap().pending[0].1 .0;
    assert!(a_and_b <= other + other / 1_000, "one stack, one weight: {a_and_b} vs {other}");
    assert_eq!(day.checkins, 3);
}

#[test]
fn gifting_coins_cannot_lower_someone_elses_weight() {
    // below the cap, stake x coin-age is unchanged by an inflow, so "age griefing" does nothing
    let mut t = T::new();
    t.give("victim.near", 10_000 * ONE);
    t.advance_days(14.0);
    let before = t.c.preview_weight(acc("victim.near")).0;
    t.give("victim.near", 10_000 * ONE); // a griefer doubles the victim's balance
    let after = t.c.preview_weight(acc("victim.near")).0;
    assert!(after + 2 >= before, "{after} vs {before}");
}

#[test]
fn streak_decays_instead_of_resetting() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    for _ in 0..10 {
        t.check_in("alice.near");
        t.advance_days(1.0);
    }
    assert_eq!(t.c.get_member(acc("alice.near")).unwrap().streak, 10);
    t.advance_days(1.0); // misses exactly one day
    t.check_in("alice.near");
    assert_eq!(t.c.get_member(acc("alice.near")).unwrap().streak, 8, "10 - penalty 3 + 1");
    t.advance_days(30.0); // a long absence does run it down to zero
    t.check_in("alice.near");
    assert_eq!(t.c.get_member(acc("alice.near")).unwrap().streak, 1);
}

#[test]
fn majority_source_shows_where_a_wallets_coins_came_from() {
    let mut t = T::new();
    t.give("farmer.near", 30_000 * ONE);
    for i in 0..3 {
        let w = format!("w{i}.near");
        t.register(&w);
        t.ctx(&acc("farmer.near"), 1);
        t.c.ft_transfer(acc(&w), U128(10_000 * ONE), None);
        assert_eq!(t.c.get_source(acc(&w)), Some(acc("farmer.near")));
    }
    // a buyer from the market, later tipped a little by a friend: the source stays the market
    t.give("buyer.near", 5_000 * ONE);
    t.give("friend.near", 500 * ONE);
    t.ctx(&acc("friend.near"), 1);
    t.c.ft_transfer(acc("buyer.near"), U128(100 * ONE), None);
    assert_eq!(t.c.get_source(acc("buyer.near")), Some(acc("launch.near")));
    // the view the oracle reads carries it
    let h = t.c.get_holders(0, 50);
    let w0 = h.iter().find(|x| x.account_id == acc("w0.near")).unwrap();
    assert_eq!(w0.source, Some(acc("farmer.near")));
    // emptying a wallet clears its record
    t.ctx(&acc("w0.near"), 1);
    t.c.ft_transfer(acc("farmer.near"), U128(10_000 * ONE), None);
    assert_eq!(t.c.get_source(acc("w0.near")), None);
}

#[test]
#[should_panic(expected = "not enough oracle signatures")]
fn one_oracle_alone_cannot_attest_when_two_are_required() {
    let mut t = T::deploy(cfg(), &[9, 10, 11], 2);
    t.ensure_holders(5);
    let p = t.payload(5, &[]);
    let sigs = t.sigs(&p, &[0]);
    t.ctx(&acc("relayer.near"), 0);
    t.c.attest(p, sigs);
}

#[test]
#[should_panic(expected = "duplicate oracle signature")]
fn the_same_oracle_cannot_sign_twice() {
    let mut t = T::deploy(cfg(), &[9, 10, 11], 2);
    t.ensure_holders(5);
    let p = t.payload(5, &[]);
    let sigs = t.sigs(&p, &[1, 1]);
    t.ctx(&acc("relayer.near"), 0);
    t.c.attest(p, sigs);
}

#[test]
fn two_of_three_oracles_can_attest() {
    let mut t = T::deploy(cfg(), &[9, 10, 11], 2);
    t.ensure_holders(5);
    let p = t.payload(5, &[]);
    let sigs = t.sigs(&p, &[2, 0]);
    t.ctx(&acc("relayer.near"), 0);
    t.c.attest(p, sigs);
    assert_eq!(t.c.independent_holders, 5);
    assert_eq!(t.c.get_oracle_threshold(), 2);
    assert_eq!(t.c.get_oracle_pks().len(), 3);
}

#[test]
#[should_panic(expected = "independent holders cannot exceed on-chain holders")]
fn oracle_cannot_report_more_holders_than_exist() {
    let mut t = T::new();
    let p = t.payload(1_000_000, &[]);
    let sigs = t.sigs(&p, &[0]);
    t.ctx(&acc("relayer.near"), 0);
    t.c.attest(p, sigs);
}

#[test]
fn a_compromised_oracle_cannot_mint_the_reserve_at_once() {
    let mut c = cfg();
    c.tranche_cooldown_days = 14;
    let mut t = T::new_with(c);
    let supply0 = t.c.ft.total_supply;
    for _ in 0..20 {
        t.attest(500, vec![]); // claims every milestone is passed, again and again
    }
    assert_eq!(t.c.milestone_idx, 1, "one tranche, then the cooldown");
    assert_eq!(t.c.ft.total_supply - supply0, MAX * 500 / 10_000 * 8 / 10);
    t.advance_days(14.0);
    t.attest(500, vec![]);
    assert_eq!(t.c.milestone_idx, 2);
}

#[test]
fn creator_cut_vests_linearly() {
    let mut c = cfg();
    c.creator_vest_days = 30;
    let mut t = T::new_with(c);
    t.attest(60, vec![]);
    let cut = MAX * 500 / 10_000 * 2 / 10;
    assert_eq!(t.c.get_state().creator_unlocked.0, cut);
    assert_eq!(t.c.get_state().creator_vested.0, 0);
    t.advance_days(15.0);
    t.ctx(&acc("creator.near"), 0);
    assert_eq!(t.c.claim_creator().0, cut / 2);
    t.advance_days(30.0);
    t.ctx(&acc("creator.near"), 0);
    assert_eq!(t.c.claim_creator().0, cut - cut / 2);
}

#[test]
#[should_panic(expected = "only the creator")]
fn a_stranger_cannot_trigger_the_creators_claim() {
    let mut t = T::new();
    t.attest(60, vec![]);
    t.ctx(&acc("mallory.near"), 0);
    t.c.claim_creator();
}

#[test]
fn contraction_claws_back_the_unvested_creator_cut() {
    // attack: fake a milestone with rented holders, wait for the creator cut, let the holders leave
    let mut c = cfg();
    c.creator_vest_days = 30;
    let mut t = T::new_with(c);
    t.attest(60, vec![]); // milestone 50 reached, creator cut starts vesting
    let cut = MAX * 500 / 10_000 * 2 / 10;
    t.advance_days(3.0);
    t.attest(10, vec![]); // the "holders" are gone: contraction clock starts
    t.advance_days(7.0);
    t.attest(10, vec![]); // contraction: 10 of 30 vesting days have passed
    let s = t.c.get_state();
    assert_eq!(s.creator_unlocked.0, cut / 3, "only the vested third survives");
    t.advance_days(60.0);
    t.ctx(&acc("creator.near"), 0);
    assert_eq!(t.c.claim_creator().0, cut / 3, "nothing more ever vests");
    assert!(t.c.get_ledger(0, 200).iter().any(|e| e.memo.contains("unvested creator earn-out")));
}

#[test]
fn closing_an_account_with_a_balance_is_a_recorded_burn() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    t.ctx(&acc("alice.near"), 1);
    assert!(t.c.storage_unregister(Some(true)));
    assert_eq!(t.c.get_holder_count(), 2);
    let all = t.c.get_ledger(0, 200);
    let minted: u128 = all.iter().filter(|e| e.kind == "mint").map(|e| e.amount.0).sum();
    let burned: u128 = all.iter().filter(|e| e.kind == "burn").map(|e| e.amount.0).sum();
    assert_eq!(minted - burned, t.c.ft.total_supply);
}

/// Deterministic pseudo-random driver: thousands of valid operations in random order, with the
/// accounting invariants checked after every one of them.
#[test]
fn fuzz_accounting_invariants_hold() {
    struct Rng(u64);
    impl Rng {
        fn next(&mut self, n: u64) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (self.0 >> 33) % n
        }
    }
    // each seed runs in its own thread, because the mocked chain's storage is thread-local
    let runs: Vec<_> = [1u64, 7, 42, 2026].into_iter().map(|seed| std::thread::spawn(move || {
        let mut c = cfg();
        c.milestones = vec![4, 8, 12];
        c.season_days = 5;
        c.contraction_days = 2;
        c.creator_vest_days = 6;
        c.tranche_cooldown_days = 1;
        c.challenge_account_cap_bps = 2_000;
        let mut t = T::new_with(c);
        let mut r = Rng(seed);
        let names: Vec<String> = (0..14).map(|i| format!("u{i}.near")).collect();
        for n in &names {
            t.give(n, (1 + r.next(400_000)) as u128 * ONE);
        }
        let (ch, issuer) = t.challenge(50_000 * ONE, 500_000 * ONE);
        let mut nonce = 0u32;
        for step in 0..1_500 {
            let who = names[r.next(names.len() as u64) as usize].clone();
            let today = t.c.current_day();
            match r.next(9) {
                0 | 1 => {
                    let m = t.c.get_member(acc(&who));
                    if t.bal(&who) >= ONE && m.map(|m| m.last_day != Some(today)).unwrap_or(true) {
                        t.check_in(&who);
                    }
                }
                2 => {
                    t.claim(&who);
                }
                3 => {
                    let to = names[r.next(names.len() as u64) as usize].clone();
                    let bal = t.bal(&who);
                    if to != who && bal > 0 {
                        let amt = 1 + r.next(1_000) as u128 * bal / 1_000;
                        t.ctx(&acc(&who), 1);
                        t.c.ft_transfer(acc(&to), U128(amt.min(bal)), None);
                    }
                }
                4 => {
                    let n = r.next(t.c.get_holder_count() as u64 + 1) as u32;
                    let size = 1 + r.next(5) as u32;
                    t.ts += 1;
                    let p = format!(
                        r#"{{"ts":"{}","independent_holders":{},"clusters":[{{"account_id":"{}","cluster_id":"k","size":{}}}]}}"#,
                        t.ts, n, who, size
                    );
                    let sigs = t.sigs(&p, &[0]);
                    t.ctx(&acc("relayer.near"), 0);
                    t.c.attest(p, sigs);
                }
                5 => t.advance_days(r.next(30) as f64 / 10.0),
                6 => {
                    let bal = t.bal(&who);
                    if bal > 2 * ONE {
                        t.ctx(&acc(&who), 0);
                        t.c.bonfire(U128(bal / (2 + r.next(50) as u128)));
                    }
                }
                7 => {
                    let s = t.c.get_state();
                    if s.creator_vested.0 > s.creator_claimed.0 {
                        t.ctx(&acc("creator.near"), 0);
                        t.c.claim_creator();
                    }
                }
                _ => {
                    nonce += 1;
                    let msg = format!("insydr-challenge:coin.near:{ch}:{who}:{nonce}");
                    let sig = Base64VecU8(issuer.sign(msg.as_bytes()).to_bytes().to_vec());
                    t.ctx(&acc(&who), 0);
                    // a refused claim would revert on chain; here we only take the ones that pay
                    let c = &t.c;
                    let payable = {
                        let bal = t.bal(&who);
                        let cap = c.pool_free() / 100;
                        let spent = if c.challenge_spent_day == today { c.challenge_spent } else { 0 };
                        let earned = match c.challenge_earned.get(&acc(&who)) {
                            Some((d, n)) if *d == today => *n,
                            _ => 0,
                        };
                        let last_ok = c.challenge_last.get(&format!("{ch}:{who}")).map(|d| today > *d).unwrap_or(true);
                        let chs = c.get_challenges();
                        let budget_left = if chs[ch as usize].spent_day == today { 500_000 * ONE - chs[ch as usize].spent.0 } else { 500_000 * ONE };
                        last_ok
                            && 50_000 * ONE * c.base_weight(&acc(&who), bal) / 1_000_000 > 0
                            && cap > spent
                            && cap * 2_000 / 10_000 > earned
                            && budget_left > 0
                    };
                    if payable {
                        t.c.claim_signed(ch, nonce.to_string(), sig);
                    }
                }
            }
            // --- invariants
            let s = t.c.get_state();
            let max = t.c.config.max_supply.0;
            assert_eq!(
                s.total_supply.0 + s.burned.0 + s.reserve.0 + s.cancelled.0 + (s.creator_unlocked.0 - s.creator_claimed.0),
                max,
                "seed {seed} step {step}: every token is minted, burned, reserved, cancelled or owed to the creator"
            );
            assert!(s.total_supply.0 <= max);
            assert!(t.c.pool_committed <= t.c.pool_balance(), "seed {seed} step {step}: pool covers what it owes");
            assert!(s.creator_claimed.0 <= s.creator_vested.0 && s.creator_vested.0 <= s.creator_unlocked.0);
        }
        // the ledger tells the same story as the token
        let mut all = Vec::new();
        let n = t.c.get_ledger_len();
        let mut i = 0;
        while i < n {
            all.extend(t.c.get_ledger(i, 200));
            i += 200;
        }
        let minted: u128 = all.iter().filter(|e| e.kind == "mint").map(|e| e.amount.0).sum();
        let burned: u128 = all.iter().filter(|e| e.kind == "burn").map(|e| e.amount.0).sum();
        let cancelled: u128 = all.iter().filter(|e| e.kind == "cancel").map(|e| e.amount.0).sum();
        assert_eq!(minted - burned, t.c.ft.total_supply, "seed {seed}");
        assert_eq!(burned, t.c.burned, "seed {seed}");
        assert_eq!(cancelled, t.c.cancelled, "seed {seed}");
        let memos = |k: &str| all.iter().filter(|e| e.memo.contains(k)).count();
        println!(
            "FUZZ seed {seed}: {} ledger entries | tranches {} | ritual payouts {} | challenge payouts {} | missed-quorum burns {} | bonfires {} | season bonus {} | season cancels {} | contractions {} | creator clawbacks {} | creator claims {}",
            all.len(), memos("growth tranche"), memos("ritual"), memos("challenge"), memos("missed quorum"), memos("bonfire"),
            memos("season bonus"), memos("season missed"), memos("shrank"), memos("unvested"), memos("creator earn-out") - memos("unvested"),
        );
        (memos("ritual"), memos("challenge"), memos("missed quorum"), memos("shrank"))
    })).collect();
    let mut seen = (0, 0, 0, 0);
    for r in runs {
        let (a, b, c, d) = r.join().expect("a fuzz run failed");
        seen = (seen.0 + a, seen.1 + b, seen.2 + c, seen.3 + d);
    }
    assert!(seen.0 > 0 && seen.1 > 0 && seen.2 > 0 && seen.3 > 0, "the fuzz reached every branch: {seen:?}");
}

#[test]
fn the_daily_target_follows_the_communitys_own_turnout() {
    let mut t = T::new();
    let names: Vec<String> = (0..10).map(|i| format!("m{i}.near")).collect();
    for n in &names {
        t.give(n, CAP); // full-weight stakes, so daily rewards do not change anyone's weight
    }
    t.advance_days(15.0);
    t.c.settle();
    // a week of full turnout sets the bar
    for _ in 0..7 {
        for n in &names {
            t.check_in(n);
        }
        t.advance_days(1.0);
    }
    t.c.settle();
    let s = t.c.get_state();
    assert!(s.community_streak >= 6, "a steady community keeps its streak: {}", s.community_streak);
    assert!(s.turnout_target.0 > 0);
    // 8 of 10 show up: above 75% of the usual turnout, the streak lives
    for n in &names[..8] {
        t.check_in(n);
    }
    t.advance_days(1.0);
    t.c.settle();
    let d = t.c.get_day(t.c.next_settle_day - 1).unwrap();
    assert!(d.hit, "turnout {} vs target {}", d.turnout.0, d.target.0);
    // only 4 of 10 show up: well under the bar, the pool burns and the streak resets
    for n in &names[..4] {
        t.check_in(n);
    }
    t.advance_days(1.0);
    t.c.settle();
    let d = t.c.get_day(t.c.next_settle_day - 1).unwrap();
    assert!(!d.hit && d.burned.0 > 0, "hit {} turnout {} target {} unique {} burned {}", d.hit, d.turnout.0, d.target.0, d.unique, d.burned.0);
    assert_eq!(t.c.community_streak, 0);
    // ...and if that is the new normal, the bar comes down to meet it within a couple of weeks
    let mut days_to_recover = 0;
    loop {
        for n in &names[..4] {
            t.check_in(n);
        }
        t.advance_days(1.0);
        t.c.settle();
        days_to_recover += 1;
        if t.c.get_day(t.c.next_settle_day - 1).unwrap().hit {
            break;
        }
        assert!(days_to_recover < 30);
    }
    assert!(days_to_recover <= 16, "retargeted after {days_to_recover} days");
}

#[test]
fn dust_wallets_cannot_move_the_daily_target() {
    // 500 dust wallets checking in add almost nothing to turnout, so they can neither carry a
    // day nor inflate the target and then walk away to force burns
    let mut t = T::new();
    for i in 0..5 {
        t.give(&format!("real{i}.near"), 100_000 * ONE);
    }
    for i in 0..60 {
        t.give(&format!("dust{i}.near"), ONE);
    }
    t.advance_days(15.0);
    t.c.settle();
    for i in 0..5 {
        t.check_in(&format!("real{i}.near"));
    }
    let real = t.c.get_day(15).unwrap().turnout.0;
    for i in 0..60 {
        t.check_in(&format!("dust{i}.near"));
    }
    let with_dust = t.c.get_day(15).unwrap().turnout.0;
    assert!(with_dust - real <= real / 1_000, "60 dust wallets moved turnout by {} of {}", with_dust - real, real);
}
