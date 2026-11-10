//! INSYDR demo tap: lets a judge try the coin without a wallet.
//!
//! `drip()` registers the caller with the token and sends them a fixed amount, once per account.
//! It exists only for the public testnet demo. The amount is set below the coin's
//! `holder_min_balance`, so tap accounts can check in and earn but never count as independent
//! holders and can never unlock supply. A real coin has no tap.
use near_sdk::json_types::U128;
use near_sdk::store::LookupSet;
use near_sdk::{env, near, require, serde_json::json, AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault, Promise};

#[derive(BorshStorageKey)]
#[near]
enum Key {
    Served,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Tap {
    token: AccountId,
    amount: U128,
    served: LookupSet<AccountId>,
    count: u32,
}

#[near]
impl Tap {
    #[init]
    pub fn new(token: AccountId, amount: U128) -> Self {
        Self { token, amount, served: LookupSet::new(Key::Served), count: 0 }
    }

    /// Once per account: register the caller with the token and send them `amount`.
    pub fn drip(&mut self) -> Promise {
        let who = env::predecessor_account_id();
        require!(self.served.insert(who.clone()), "this account already used the tap");
        self.count += 1;
        Promise::new(self.token.clone())
            .function_call(
                "storage_deposit".to_string(),
                json!({ "account_id": who, "registration_only": true }).to_string().into_bytes(),
                NearToken::from_millinear(3), // unused part is refunded by the token
                Gas::from_tgas(15),
            )
            .function_call(
                "ft_transfer".to_string(),
                json!({ "receiver_id": who, "amount": self.amount, "memo": "demo tap" }).to_string().into_bytes(),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(25),
            )
    }

    pub fn info(&self) -> (AccountId, U128, u32) {
        (self.token.clone(), self.amount, self.count)
    }

    pub fn served(&self, account_id: AccountId) -> bool {
        self.served.contains(&account_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn ctx(who: &str) {
        testing_env!(VMContextBuilder::new()
            .current_account_id("tap.near".parse().unwrap())
            .predecessor_account_id(who.parse().unwrap())
            .build());
    }

    #[test]
    fn serves_each_account_once() {
        ctx("owner.near");
        let mut t = Tap::new("coin.near".parse().unwrap(), U128(50));
        ctx("guest.near");
        t.drip();
        assert!(t.served("guest.near".parse().unwrap()));
        assert_eq!(t.info().2, 1);
    }

    #[test]
    #[should_panic(expected = "already used the tap")]
    fn refuses_a_second_drip() {
        ctx("owner.near");
        let mut t = Tap::new("coin.near".parse().unwrap(), U128(50));
        ctx("guest.near");
        t.drip();
        t.drip();
    }
}
