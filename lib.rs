#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod course_reg {
    use ink_storage::Mapping;
    use ink_prelude::vec::Vec;
    use ink_storage::traits::{SpreadAllocate, PackedLayout, SpreadLayout};

    /// A university course created by a teacher
    #[derive(PackedLayout, SpreadLayout, scale::Encode, scale::Decode, PartialEq, Debug)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub struct Course {
        /// the teacher who created the course
        teacher: AccountId,
        /// the id of the course
        course_id: [u8; 32],
        /// the max number of students who can register
        capacity: u32,
        /// the registered students
        registrations: Vec<AccountId>,
        /// the starting time of the course
        start_date: Timestamp,
    }

    /// A course registration token
    #[derive(PackedLayout, SpreadLayout, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub struct CourseRegistration {
        /// the owner of the token
        owner: AccountId,
        /// the id of the course
        course_id: [u8; 32],
    }

    /// A course registration token swap proposal 
    #[derive(PackedLayout, SpreadLayout, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub struct CourseRegistrationSwapProposal {
        /// the offered token
        offer: CourseRegistration,
        /// the tokens offered in exchange
        counter_offers: Vec<CourseRegistration>
    }

    /// Contract storage
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct CourseReg {
        /// the owner of the contract, the school leader
        owner: AccountId,
        /// the members of the school, <id, isTeacher>
        school_members: Mapping<AccountId, bool>,
        /// the courses created by the teachers <CourseId, Course>
        courses: Mapping<[u8; 32], Course>,
        /// the proposed swaps <swapId, swapProposal>
        swaps: Mapping<[u8; 32], CourseRegistrationSwapProposal>,
        /// the owned registration tokens <owner, tokens>
        registrations: Mapping<AccountId, Vec<CourseRegistration>>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
       InsufficientPermissions,
       NonexistentCourse,
    }

    impl CourseReg {

        /// Default constructor that initializes the necessary values
        #[ink(constructor)]
        pub fn new(owner: AccountId) -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                contract.owner = owner;
                contract.school_members.insert(&owner, &true);
            })
        }

        /// Default constructor that initializes the necessary values
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
        pub fn is_school_member(&self, account: AccountId) -> bool {
            self.is_school_member_inner(account)
        }

        fn is_school_member_inner(&self, account: AccountId) -> bool {
            self.school_members.contains(account)
        }

        /// Returns if the account is a teacher
        #[ink(message)]
        pub fn is_teacher(&self, account: AccountId) -> bool {
            self.is_teacher_inner(account)
        }

        fn is_teacher_inner(&self, account: AccountId) -> bool {
            self.school_members.get(&account).unwrap_or(false)
        }


        /// Creates a university course
        #[ink(message)]
        pub fn create_course(&mut self,
                             course_id: [u8;32],
                             course_cap: u32,
                             course_start:Timestamp) -> Result<(),Error> {
            let caller = Self::env().caller();
            if !self.is_teacher_inner(caller) {
                return Err(Error::InsufficientPermissions);
            }
            let course = Course {
                teacher: caller,
                capacity: course_cap,
                course_id: course_id.clone(),
                start_date: course_start,
                registrations: Vec::default(),
            };
            self.courses.insert(&course_id, &course);
            return Ok(())
        }

        #[ink(message)]
        pub fn get_course_info(&self, course_id: [u8; 32]) -> Result<Course,Error> {
            if !self.courses.contains(&course_id){
                return Err(Error::NonexistentCourse);
            }
            Ok(self.courses.get(course_id).unwrap())
        }


        /// returns true if the caller is the owner of the contract
        fn is_owner(&self) -> bool {
            let caller = Self::env().caller();
            return caller == self.owner;
        }
    }

    /// Unit tests
    #[cfg(test)]
    mod tests {
        use super::*;

        use ink_lang as ink;
        use ink_env;
        use ink_env::hash;

        fn set_next_caller(caller: AccountId) {
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(caller);
        }

        fn get_current_time() -> Timestamp {
            let since_the_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");
            since_the_epoch.as_secs()
                + since_the_epoch.subsec_nanos() as u64 / 1_000_000_000 
        }

        pub fn hash_keccak_256(input: &[u8]) -> [u8; 32] {
            let mut output = <hash::Keccak256 as hash::HashOutput>::Type::default();
            ink_env::hash_bytes::<hash::Keccak256>(input, &mut output);
            output
        }

        /// Teacher admission test
        #[ink::test]
        fn teacher_admission() {
            let owner = AccountId::from([0x0;32]);
            set_next_caller(owner);
            let mut course_reg = CourseReg::new(owner);
            let teacher = AccountId::from([0x1; 32]);
            assert_eq!(course_reg.admit_as_teacher(teacher), Ok(()));
            assert_eq!(course_reg.is_school_member(teacher), true);
            assert_eq!(course_reg.is_teacher(teacher), true);
        }

        /// Student admission test
        #[ink::test]
        fn student_admission() {
            let owner = AccountId::from([0x0;32]);
            set_next_caller(owner);
            let mut course_reg = CourseReg::new(owner);
            let student = AccountId::from([0x1; 32]);
            assert_eq!(course_reg.admit_as_student(student), Ok(()));
            assert_eq!(course_reg.is_school_member(student), true);
            assert_eq!(course_reg.is_teacher(student), false);
        }

        /// Course creation test
        #[ink::test]
        fn course_creation() {
            let owner = AccountId::from([0x0;32]);
            set_next_caller(owner);
            let mut course_reg = CourseReg::new(owner);
            let teacher = AccountId::from([0x1; 32]);
            let course_name = "test_course".as_bytes();
            let course_id = hash_keccak_256(course_name);
            let course_cap:u32 = 10;
            let start_time = get_current_time();

            assert_eq!(course_reg.admit_as_teacher(teacher), Ok(()));
            assert_eq!(course_reg.is_school_member(teacher), true);
            assert_eq!(course_reg.is_teacher(teacher), true);
            assert_eq!(course_reg.create_course(course_id, course_cap, start_time), Ok(()));

            assert_ne!(course_reg.get_course_info(course_id), Err(Error::NonexistentCourse));
        }
    }
}
