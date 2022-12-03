#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod course_reg {
    use ink_storage::Mapping;
    use ink_storage::traits::SpreadAllocate;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct CourseReg {
        /// The owner of the contract, the school leader
        owner: AccountId,
        /// the members of the school, <id, isTeacher>
        school_members: Mapping<AccountId, bool>
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
       InsufficientPermissions,
       InsufficientAllowance,
    }

    impl CourseReg {

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                let caller = Self::env().caller();
                contract.owner = caller;
                contract.school_members.insert(&caller, &true);
            })
        }

        /// Admits the account to school_members, as a Teacher
        #[ink(message)]
        pub fn admit_as_teacher(&mut self, account: AccountId) -> Result<(), Error> {
            if !self.is_owner() {
                return Err(Error::InsufficientPermissions);
            }
            self.school_members.insert(&account, &true);
            return Ok(());
        }

        /// Admits the account to school_members, as a student
        #[ink(message)]
        pub fn admit_as_student(&mut self, account: AccountId) -> Result<(), Error> {
            if !self.is_owner() {
                return Err(Error::InsufficientPermissions);
            }
            self.school_members.insert(&account, &false);
            return Ok(());
        }

        /// Returns if the account is a school_member
        #[ink(message)]
        pub fn is_scool_member(&self, account: AccountId) -> bool {
            self.school_members.get(&account) != None
        }

        /// Returns if the account is a teacher
        #[ink(message)]
        pub fn is_teacher(&self, account: AccountId) -> bool {
            self.school_members.get(&account).unwrap_or(false)
        }


        /// returns true if the caller is the owner of the contract
        fn is_owner(&self) -> bool {
            let caller = Self::env().caller();
            return caller == self.owner;
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

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let course_reg = CourseReg::default();
            //assert_eq!(course_reg.admit_as_teacher(AccountId::from([0x0; 32])), false);
        }
    }
}
