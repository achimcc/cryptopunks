#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod cryptopunks {
    use ink_storage::lazy::Mapping;

    #[ink(storage)]
    #[derive(Default, ink_storage::traits::SpreadAllocate)]
    pub struct Cryptopunks {
        owner: AccountId,
        total_supply: u32,
        punks_remaining_to_assign: u32,
        number_of_punks_to_reserve: u32,
        number_of_punks_reserved: u32,
        next_punk_index_to_assign: u32,
        punk_index_to_address: Mapping<u32, AccountId>,
        punks_offered_for_sale: Mapping<u32, Offer>,
        pending_withdrawals: Mapping<AccountId, u128>,
        balance_of: Mapping<AccountId, u32>,
    }

    #[derive(
        Default,
        scale::Encode,
        scale::Decode,
        ink_storage::traits::PackedLayout,
        ink_storage::traits::SpreadLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct Offer {
        is_for_sale: bool,
        punk_index: u32,
        seller: AccountId,
        min_value: Balance,
        only_sell_to: Option<AccountId>,
    }

    #[ink(event)]
    pub struct PunkNoLongerForSale {
        #[ink(topic)]
        punk_index: u32,
    }

    #[ink(event)]
    pub struct PunkOffered {
        #[ink(topic)]
        punk_index: u32,
        min_sale_price: Balance,
        #[ink(topic)]
        address: Option<AccountId>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        value: u128,
    }

    #[ink(event)]
    pub struct PunkTransfer {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        punk_index: u32,
    }

    #[ink(event)]
    pub struct Assign {
        #[ink(topic)]
        to: AccountId,
        punk_index: u32,
    }

    impl Cryptopunks {
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::codegen::initialize_contract(|contract: &mut Self| {
                contract.owner = Self::env().caller();
                contract.total_supply = 1000;
                contract.punks_remaining_to_assign = 1000;
                contract.number_of_punks_to_reserve = 1000;
                contract.number_of_punks_reserved = 0;
                contract.next_punk_index_to_assign = 0;
            })
        }

        #[ink(message)]
        pub fn reserve_punks_for_owner(&mut self, max_for_this_run: u32) {
            assert_eq!(self.env().caller(), self.owner, "Caller is not owner!");
            assert!(
                self.number_of_punks_reserved <= self.number_of_punks_to_reserve,
                "Already all reservable punks reserved!"
            );
            let mut number_punks_reserved_this_run: u32 = 0;
            while number_punks_reserved_this_run < self.number_of_punks_to_reserve
                && number_punks_reserved_this_run < max_for_this_run
            {
                self.punk_index_to_address
                    .insert(self.next_punk_index_to_assign, &self.env().caller());
                self.env().emit_event(Assign {
                    to: self.env().caller(),
                    punk_index: self.next_punk_index_to_assign,
                });
                number_punks_reserved_this_run += 1;
                self.next_punk_index_to_assign += 1;
            }
            self.punks_remaining_to_assign -= number_punks_reserved_this_run;
            self.number_of_punks_reserved += number_punks_reserved_this_run;
            let previous_balance = self.balance_of.get(self.env().caller()).unwrap_or(0);
            self.balance_of.insert(
                self.env().caller(),
                &(previous_balance + number_punks_reserved_this_run),
            );
        }

        #[ink(message)]
        pub fn get_punk(&mut self, punk_index: u32) {
            assert!(self.punks_remaining_to_assign > 0);
            assert_eq!(self.punk_index_to_address.get(punk_index), None);
            self.punk_index_to_address
                .insert(punk_index, &self.env().caller());
            let amount = self.balance_of.get(self.env().caller()).unwrap_or(0);
            self.balance_of.insert(self.env().caller(), &(amount + 1));
            self.punks_remaining_to_assign -= 1;
            self.env().emit_event(Assign {
                to: self.env().caller(),
                punk_index,
            });
        }

        #[ink(message)]
        pub fn transfer_punk(&mut self, to: AccountId, punk_index: u32) {
            let owner = self
                .punk_index_to_address
                .get(punk_index)
                .expect("Punk is not assigned");
            assert_eq!(owner, self.env().caller());
            self.punk_index_to_address.insert(punk_index, &to);
            let holder_balance = self
                .balance_of
                .get(self.env().caller())
                .expect("Holder has at least 1 punk");
            self.balance_of
                .insert(self.env().caller(), &(holder_balance - 1));
            let receiver_balance = self.balance_of.get(to).unwrap_or(0);
            self.balance_of.insert(to, &(receiver_balance + 1));
            self.env().emit_event(Transfer {
                from: self.env().caller(),
                to,
                value: 1,
            });
            self.env().emit_event(PunkTransfer {
                from: self.env().caller(),
                to,
                punk_index,
            });
        }

        #[ink(message)]
        pub fn offer_punk_for_sale(
            &mut self,
            punk_index: u32,
            min_sale_price: Balance,
            address: Option<AccountId>,
        ) {
            assert_eq!(
                self.punk_index_to_address.get(punk_index),
                Some(self.env().caller())
            );
            let offer = Offer {
                is_for_sale: true,
                punk_index,
                seller: self.env().caller(),
                min_value: min_sale_price,
                only_sell_to: address,
            };
            self.punks_offered_for_sale.insert(punk_index, &offer);
            self.env().emit_event(PunkOffered {
                punk_index,
                min_sale_price,
                address,
            });
        }

        #[ink(message, payable)]
        pub fn buy_punk(&mut self, punk_index: u32) {
            let balance = self.env().transferred_balance();
            let offer = self
                .punks_offered_for_sale
                .get(punk_index)
                .expect("Punk doesn't exist!");
            assert!(offer.is_for_sale, "Punk isn't for sale!");
            if offer.only_sell_to.is_some() {
                assert_eq!(
                    offer.only_sell_to,
                    Some(self.env().caller()),
                    "Punk is reserved for other buyer!"
                );
            };

            assert!(balance >= offer.min_value, "Offer for punk is to low!");
            assert_eq!(
                self.punk_index_to_address.get(punk_index),
                Some(offer.seller),
                "Seller is no longer owner of the punk!"
            );

            Self::env().emit_event(Transfer {
                from: offer.seller,
                to: self.env().caller(),
                value: balance,
            });

            self.pending_withdrawals.insert(offer.seller, &balance);

            self.no_longer_for_sale(punk_index);
        }

        fn no_longer_for_sale(&mut self, punk_index: u32) {
            let offer = Offer {
                is_for_sale: false,
                punk_index,
                seller: self.env().caller(),
                min_value: 0,
                only_sell_to: None,
            };
            self.punks_offered_for_sale.insert(punk_index, &offer);
            Self::env().emit_event(PunkNoLongerForSale { punk_index });
        }

        #[ink(message)]
        pub fn withdraw(&mut self) {
            let amount = self
                .pending_withdrawals
                .get(self.env().caller())
                .expect("No pending withdrawals for caller");
            assert!(amount > 0, "No remaining balance to withdraw!");
            self.pending_withdrawals.insert(self.env().caller(), &0);
            self.env()
                .transfer(self.env().caller(), amount)
                .expect("Transfer failed");
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// Imports `ink_lang` so we can use `#[ink::test]`.
        use ink_lang as ink;

        fn set_sender(sender: AccountId, amount: Balance) {
            ink_env::test::push_execution_context::<Environment>(
                sender,
                ink_env::account_id::<Environment>(),
                1000000,
                amount,
                ink_env::test::CallData::new(ink_env::call::Selector::new([0x00; 4])), /* dummy */
            );
        }

        /// We test if the default constructor does its job.
        #[ink::test]
        fn new_works() {
            // Constructor works.
            let _cryptopunks = Cryptopunks::new();

            //  // Transfer event triggered during initial construction.
            //  let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            //  assert_eq!(1, emitted_events.len());
        }

        #[ink::test]
        fn get_works() {
            // Constructor works.
            let mut cryptopunks = Cryptopunks::new();

            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                .expect("Cannot get accounts");
            let _balance =
                ink_env::test::get_account_balance::<ink_env::DefaultEnvironment>(accounts.alice)
                    .expect("Alice has no Account Balance");
            cryptopunks.get_punk(0);
        }

        #[ink::test]
        fn sale_works() {
            // Constructor works.
            let mut cryptopunks = Cryptopunks::new();

            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                .expect("Cannot get accounts");
            let _balance =
                ink_env::test::get_account_balance::<ink_env::DefaultEnvironment>(accounts.alice)
                    .expect("Alice has no Account Balance");

            set_sender(accounts.alice, 0);

            cryptopunks.get_punk(0);

            cryptopunks.offer_punk_for_sale(0, 100000, None);

            set_sender(accounts.charlie, 100000);

            cryptopunks.buy_punk(0);

            set_sender(accounts.alice, 0);

            cryptopunks.withdraw();
        }
    }
}
