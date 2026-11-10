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
    oracle: SigningKey,
    now_ms: u64,
    ts: u64,
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
        quorum_bps: 5_000,
        daily_payout_bps: 50,
        miss_burn_bps: 25,
        age_full_days: 14,
        streak_cap: 30,
        streak_max_bonus_bps: 5_000,
        balance_cap_bps: 5, // 0.05% of max supply = 500k tokens
        min_balance_to_check_in: U128(ONE),
        season_days: 30,
        season_growth_bps: 5_000,
        season_bonus_bps: 200,
        challenge_daily_cap_bps: 100,
        day_ms: U64(DAY_MS),
    }
}

impl T {
    fn new_with(config: Config) -> Self {
        let oracle = keypair(9);
        let now_ms = 1_000_000;
        set_ctx(now_ms, &acc("deployer.near"), 0);
        let c = Contract::new(
            "Grassroots Dog".into(),
            "GDOG".into(),
            None,
            config,
            acc("creator.near"),
            acc("launch.near"),
            pk_of(&oracle),
            vec![Link { label: "x".into(), url: "https://x.com/gdog".into() }],
        );
        let mut t = T { c, oracle, now_ms, ts: 0 };
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
    fn attest(&mut self, holders: u32, clusters: Vec<(&str, &str, u32)>) {
        self.ts += 1;
        let cl: Vec<String> = clusters
            .iter()
            .map(|(a, id, n)| format!(r#"{{"account_id":"{a}","cluster_id":"{id}","size":{n}}}"#))
            .collect();
        let payload = format!(
            r#"{{"ts":"{}","independent_holders":{},"clusters":[{}]}}"#,
            self.ts,
            holders,
            cl.join(",")
        );
        let sig = self.oracle.sign(payload.as_bytes()).to_bytes().to_vec();
        self.ctx(&acc("relayer.near"), 0);
        self.c.attest(payload, Base64VecU8(sig));
    }
}

#[test]
fn launch_mints_only_launch_and_pool_seed() {
    let t = T::new();
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
    let paid_a = t.check_in("alice.near"); // settles yesterday and pays it
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
    t.attest(120, vec![]); // crosses 50 and 100 at once
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
#[should_panic(expected = "nothing unlocked")]
fn creator_gets_nothing_without_real_growth() {
    let mut t = T::new();
    t.ctx(&acc("creator.near"), 0);
    t.c.claim_creator();
}

#[test]
fn shrinking_community_cancels_reserve() {
    let mut t = T::new();
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
    t.c.attest(payload, Base64VecU8(sig));
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

#[test]
fn signed_challenge_pays_once_with_anti_farm() {
    let mut t = T::new();
    t.attest(60, vec![("f1.near", "farm", 4)]); // fills the pool via a tranche
    t.give("alice.near", 1_000 * ONE);
    t.give("f1.near", 1_000 * ONE);
    let issuer = keypair(7);
    t.ctx(&acc("creator.near"), 0);
    let id = t.c.add_challenge(
        "reach level 10".into(),
        Proof::Signed { issuer_pk: pk_of(&issuer) },
        U128(100 * ONE),
        1,
        U128(10_000 * ONE),
    );
    t.advance_days(15.0);
    let sign = |who: &str, nonce: &str| {
        let msg = format!("grassroots-challenge:coin.near:{id}:{who}:{nonce}");
        Base64VecU8(issuer.sign(msg.as_bytes()).to_bytes().to_vec())
    };
    t.ctx(&acc("alice.near"), 0);
    assert_eq!(t.c.claim_signed(id, "n1".into(), sign("alice.near", "n1")).0, 100 * ONE);
    t.ctx(&acc("f1.near"), 0);
    assert_eq!(t.c.claim_signed(id, "n2".into(), sign("f1.near", "n2")).0, 25 * ONE, "cluster of 4 gets 1/4");
}

#[test]
#[should_panic(expected = "nonce already used")]
fn signed_challenge_nonce_cannot_replay() {
    let mut t = T::new();
    t.give("alice.near", 1_000 * ONE);
    let issuer = keypair(7);
    t.ctx(&acc("creator.near"), 0);
    let id = t.c.add_challenge("x".into(), Proof::Signed { issuer_pk: pk_of(&issuer) }, U128(ONE), 0, U128(100 * ONE));
    t.advance_days(15.0);
    let msg = format!("grassroots-challenge:coin.near:{id}:alice.near:n1");
    let sig = Base64VecU8(issuer.sign(msg.as_bytes()).to_bytes().to_vec());
    t.ctx(&acc("alice.near"), 0);
    t.c.claim_signed(id, "n1".into(), sig.clone());
    t.advance_days(1.0);
    t.ctx(&acc("alice.near"), 0);
    t.c.claim_signed(id, "n1".into(), sig);
}

#[test]
#[should_panic(expected = "bad challenge signature")]
fn signed_challenge_rejects_other_account() {
    let mut t = T::new();
    t.give("mallory.near", 1_000 * ONE);
    let issuer = keypair(7);
    t.ctx(&acc("creator.near"), 0);
    let id = t.c.add_challenge("x".into(), Proof::Signed { issuer_pk: pk_of(&issuer) }, U128(ONE), 0, U128(100 * ONE));
    let msg = format!("grassroots-challenge:coin.near:{id}:alice.near:n1"); // signed for alice
    let sig = Base64VecU8(issuer.sign(msg.as_bytes()).to_bytes().to_vec());
    t.ctx(&acc("mallory.near"), 0);
    t.c.claim_signed(id, "n1".into(), sig);
}

#[test]
fn challenges_cannot_drain_the_pool() {
    let mut t = T::new();
    let issuer = keypair(7);
    t.ctx(&acc("creator.near"), 0);
    // creator tries a huge reward and budget
    let id = t.c.add_challenge("drain".into(), Proof::Signed { issuer_pk: pk_of(&issuer) }, U128(MAX), 0, U128(MAX));
    t.give("creator2.near", 1_000 * ONE);
    t.advance_days(15.0);
    let pool = t.c.pool_free();
    let msg = format!("grassroots-challenge:coin.near:{id}:creator2.near:n");
    let sig = Base64VecU8(issuer.sign(msg.as_bytes()).to_bytes().to_vec());
    t.ctx(&acc("creator2.near"), 0);
    let paid = t.c.claim_signed(id, "n".into(), sig).0;
    assert_eq!(paid, pool / 100, "capped at challenge_daily_cap_bps (1%) of the pool");
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
    assert_eq!(t.c.get_holder_count(), 3); // launch.near, alice, bob (the pool is excluded)
    t.ctx(&acc("alice.near"), 1);
    t.c.ft_transfer(acc("bob.near"), U128(5 * ONE), None);
    let h: Vec<String> = t.c.get_holders(0, 10).into_iter().map(|(a, _, _)| a.to_string()).collect();
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
    let oracle = keypair(9);
    set_ctx(1_000_000, &acc("deployer.near"), 0);
    let contract = Contract::new("G".into(), "G".into(), None, c, acc("creator.near"), acc("launch.near"), pk_of(&oracle), vec![]);
    let mut t = T { c: contract, oracle, now_ms: 1_000_000, ts: 0 };
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
    let oracle = keypair(9);
    set_ctx(1_000_000, &acc("deployer.near"), 0);
    let contract = Contract::new("G".into(), "G".into(), None, c, acc("creator.near"), acc("launch.near"), pk_of(&oracle), vec![]);
    let mut t = T { c: contract, oracle, now_ms: 1_000_000, ts: 0 };
    t.give("alice.near", 1_000 * ONE);
    t.check_in("alice.near");
}
