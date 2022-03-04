/*!
Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

// const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAD8AAABACAMAAACa9V/5AAAAAXNSR0IB2cksfwAAAv1QTFRFAAAAOEszT3JMVHxYX4RbZIxdLj0oW4FXZI1hapdrcqBwdKJ2cKByc6V7ZpxwZpRqY4piZoxfSGlFV3lQd6iAcKB4cKBwaKB4WIBaaJhwYJBoYIhgWIhYWIBYYJBgaJhoWIBgUIBYR3hYSHBQSHdRQHBQUHhQa6F6eKh5gKh5cJhwUHhYUIBQUHBQQGhQYIhYZpt4apJgZJBkSGhIQGhAQGBAQFg4UHBIaJhgYaB3cKBwXYtgUIBgOFc4OFA3L1A5SGhAWIBQcJhoYJh4fqdwPFQ4WIhgMFAwMEgwOFhAUXhIcKBoeKhwcJ5qPFg7V4BbSHBIOEcwN0AvKUAwcKh4SG9LQFA4OUApMkAqOEQtOEApOkAoYIBVcZhgRmdDUHZRNDkmPj4oODUlNjclMDwoJy0eOjonaJBYaaBwRW1KSF9ASE0yMjsnNDQiPjspcqBgeKBoWZBgQEAtO0YuNTUiQGhIaJBggrB+RGVDTVhANjooQFc/aKCAcKF/eKBwaJhgQV5AQFxAYIhQRnFYcKiAaKiAcJxoNz4qRGlGMDglR1s/XopgYJhwgahwQ2ZITGBEPk42aI1dWHhRWlg4VXZTYZhocJBggZ1oXHBOQm9TcKhwVXhNW2BAR25JTHhYUGhESHBIVGxKgqBwaIhfOF9AUHdRMFA1WJBwQGBIQmBAQ1U4OGBHMFhASGhQYJBwLTwpK0g1SFA4aIBbQEkwLEAsJDgpW4VcWIhnWYNZUHhQYJBYaJBoNFQ8IDEgU1E2WJBoOFg+YIxgOmBASE4wXV09OzooYIBgeoBWOTsmQlU2QkoyQl09ME01QEAwYIhoW11AcIBXeHVNVFQ3bnhUc2ZIcHBQYmRCbnBIYWFGW1M4XIRYYHhTSlIyaGRAaHBQX2BAOFU9UHNQVIBccIhcaHhQYGhIeY1gXlZBRUIwQGNFVVI4aGhJXYNbSIBgUDQoaJBgWEhAYCwgUIhgVR0YZhwafTUwiIBogCUig1tNmGNblVBCaDw0pHBkiGhYaHRU6z3oKAAAAP90Uk5TABAYKDA4CEBojLzO4OzWqIdMIBD/////XP///////////////////////////////3Pc//z///////jo///////////tUP/////////uMN7//////7P/+OjQvv///17435VVMBgQ////wP/ogyjt/v//yEDQ////jP84/////8C2cP//////INRIOND//+L/gsj/1LP///////+d6PD//8jA////6vb//0Jo////////////yP+l//B+//+g////+PjO/8au//92KGDoqP//rv//j/////D/03OQ/9b///7u1v///////6Cbrv/T///w///////////////////g1gE0LAAACS5JREFUeJyVl3lcE2cax8FivFARMzNmQmaSDBmjAVMbI6HGhEQZiYPIZVQkXoBERY1ionJkqcRFcRRFUViPVg5FWEGhgkIV64V3FVRoUdG13m5327rVnp++QdEuCai/zyR/TN7v8zzvM888zxsXF0e5dvvArXt3N7cPurGc/Nq1uvXo2at3H3f3vn37uvfr199jgOd72Bg4oFefvmw2BATD4AsBV7/+gzzfjfbs2bsvG4I5KNfLiweE4cAGggATHm5vD2Jgzz5sNozyBUIhQXh7EyKCIEguDgwMRiD3Xm5vwbv3Fw8ZyhV4e0skpA9Jgo+vRDRMREqHQtBg5EPEfXi3rvBBH0EwVyDgCXxlI+Qj7ZLL/UgfEIOPAkb8gYGPe/foAh+Fo0o+jxwxerTqldQq1egAjQZsQgsjY/z9IfHY4Z1lIXAURgl4pBzA6nFBL2XnVbrxwICGhu0BQMETeg10iodMxDAeL1Q+UhUUFh4Rbr/CI4KCgAGdjiSJUCkND0YQJBLmODXgOskL8AFyfVjY5ClTo6Kipk6ZPC08PCwsDBiQR5MEaYD9/ZHBEKzFnRmYPuMVPnPW7JjYOFZcbMzsWTPnTAMWVCqdn4YIjaeNEOAhmsb7D+hoIXauF0qFztPPT1jw5iYrZtbCOXOmhal0AdHehGYRbPRHxOIhJg6+yKNj8mZgi8kA/ZLEDvdnL50zJ3yeWa6RePtaYCOCwOJlJhyh3Qf9fwTLSUwgWxG1wKWjkpZOS/bTmDUSwleqgMVISirbSkMwPeFvf62luDQBSs5b4oi7uHwyAw2Vhq6UpEukChwZbEuJtNLGVfTfF5l6D3i9KGO1kvKbv8YJnrkW5oh4flKBUMBT4EaIzR5ipZkUo9ZiWvemlkLWU7ysDU7wkI20FRUJSCkplGgUOBxpY4vFVg5ig+MN69aN7fmGz96U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";

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId, total_supply: U128) -> Self {
        Self::new(
            owner_id,
            total_supply,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "I don't know token".to_string(),
                symbol: "IKT".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 18,
            },
        )
    }

    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(
        owner_id: AccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
        };
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, total_supply.into());
        near_contract_standards::fungible_token::events::FtMint {
            owner_id: &owner_id,
            amount: &total_supply,
            memo: Some("Initial tokens supply is minted"),
        }
        .emit();
        this
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

    use super::*;

    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_default_meta(accounts(1).into(), TOTAL_SUPPLY.into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(2).into(), TOTAL_SUPPLY.into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
