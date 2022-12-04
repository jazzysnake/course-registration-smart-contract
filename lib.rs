#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod course_reg {
    //use ink_env::debug_println;
    use ink_env::hash;
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
    #[derive(PackedLayout, SpreadLayout, scale::Encode, scale::Decode, PartialEq, Debug)]
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
        swaps: Mapping<[u8; 32], Vec<CourseRegistrationSwapProposal>>,
        /// the owned registration tokens <owner, tokens>
        registrations: Mapping<AccountId, Vec<CourseRegistration>>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
       InsufficientPermissions,
       NonexistentCourse,
       CourseCapacityFull,
       AlreadyRegistered,
       NoRegistrations,
       CourseAlreadyStarted,
       NoSwappableRegistrations,
       NoProposedSwap,
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

        /// registers the caller to the university course
        ///
        /// the caller must be an admitted member and can't
        /// register to the same course multiple times
        #[ink(message)]
        pub fn register_to_course(&mut self, course_id: [u8; 32]) -> Result<(), Error> {
            let caller = Self::env().caller();
            if !self.is_school_member_inner(caller) {
                return Err(Error::InsufficientPermissions);
            }
            if !self.courses.contains(course_id) {
                return Err(Error::NonexistentCourse);
            }
            let mut course = self.courses.get(course_id).unwrap();
            let alerady_registered = course.registrations.len();
            if alerady_registered >= course.capacity.try_into().unwrap() {
                return Err(Error::CourseCapacityFull);
            }
            if course.registrations.contains(&caller) {
                return Err(Error::AlreadyRegistered);
            }
            let current_time = Self::env().block_timestamp();
            if course.start_date <= current_time {
                return Err(Error::CourseAlreadyStarted);
            }
            course.registrations.push(caller);
            self.courses.insert(&course_id, &course);
            self.add_registration(course_id);
            Ok(())
        }

        /// creates a CourseRegistration token for the course with course_id
        /// and the caller becomes the owner of the token
        fn add_registration(&mut self, course_id: [u8;32]) {
            let caller = Self::env().caller();
            let course_reg = CourseRegistration { owner: caller, course_id};
            if !self.registrations.contains(caller) {
                let mut registrations = Vec::new();
                registrations.push(course_reg);
                self.registrations.insert(&caller, &registrations);
                return;
            }
            let mut registrations = self.registrations.get(caller).unwrap();
            registrations.push(course_reg);
            self.registrations.insert(&caller, &registrations);
        }

        /// Gets the caller's CourseRegistration tokens
        #[ink(message)]
        pub fn get_own_registrations(&self) -> Result<Vec<CourseRegistration>,Error> {
            let caller = Self::env().caller();
            if !self.registrations.contains(caller) || 
                self.registrations.get(caller).unwrap().len() == 0 {
                return Err(Error::NoRegistrations);
            }
            let registrations = self.registrations.get(caller).unwrap();
            Ok(registrations)
        }

        /// Gets the info of a university course
        #[ink(message)]
        pub fn get_course_info(&self, course_id: [u8; 32]) -> Result<Course,Error> {
            if !self.courses.contains(&course_id){
                return Err(Error::NonexistentCourse);
            }
            Ok(self.courses.get(course_id).unwrap())
        }

        /// Proposes a course registration swap
        #[ink(message)]
        pub fn propose_swap(&mut self, course_id: [u8; 32]) -> Result<(),Error> {
            let caller = Self::env().caller();
            if !self.registrations.contains(caller) {
                return Err(Error::NoSwappableRegistrations);
            }
            let mut registrations = self.registrations.get(caller).unwrap();
            let course = registrations.iter().position(|x| x.course_id == course_id);
            if course.is_none() {
                return Err(Error::NoSwappableRegistrations);
            }
            let course = course.unwrap();
            let course = registrations.remove(course);
            self.registrations.insert(&caller, &registrations);


            let proposal = CourseRegistrationSwapProposal {
                offer: course,
                counter_offers: Vec::default(),
            };
            self.add_proposal(course_id, proposal);
            Ok(())
        }

        /// creates a Swap proposal token for the given course
        /// and places it in the list of proposals for that course
        fn add_proposal(&mut self,course_id: [u8; 32], proposal: CourseRegistrationSwapProposal) {
            if !self.swaps.contains(course_id) {
                let mut swaps = Vec::new();
                swaps.push(proposal);
                self.swaps.insert(&course_id, &swaps);
                return;
            }
            let mut swaps = self.swaps.get(course_id).unwrap();
            swaps.push(proposal);
            self.swaps.insert(&course_id, &swaps);
        }

        /// retrieve swap proposals for a given course_id
        #[ink(message)]
        pub fn get_proposed_swaps(&self, course_id: [u8; 32]) -> Result<Vec<CourseRegistrationSwapProposal>, Error> {
            if !self.swaps.contains(course_id) {
                return Err(Error::NoProposedSwap);
            }
            Ok(self.swaps.get(course_id).unwrap())
        }

        /// Place a counter offer on a swap proposal
        #[ink(message)]
        pub fn counter_swap_proposal(&mut self, 
                                     course_id: [u8;32],
                                     offerer: AccountId,
                                     counter_course_id: [u8; 32]) -> Result<(), Error> {
            let caller = Self::env().caller();
            let caller_regs = self.get_own_registrations();

            if caller_regs.is_err() {
                return Err(Error::NoProposedSwap);
            }

            let mut caller_regs = caller_regs.unwrap();
            let exchange_course = caller_regs.iter().position(|reg| reg.course_id == counter_course_id);

            if exchange_course.is_none(){
                return Err(Error::NoProposedSwap);
            }
            let exchange_course = exchange_course.unwrap();
            let exchange_course = caller_regs.remove(exchange_course);
            self.registrations.insert(&caller, &caller_regs);

            if !self.swaps.contains(course_id) {
                return Err(Error::NoProposedSwap);
            }

            let proposals = self.swaps.get(course_id).unwrap();
            let mut found = false;

            for mut proposal in proposals {
                if proposal.offer.owner == offerer {
                    found = true;
                    proposal.counter_offers.push(exchange_course);
                    break;
                }
            }
            if !found {
                return Err(Error::NoProposedSwap);
            }

            Ok(())
        }

        /// returns true if the caller is the owner of the contract
        fn is_owner(&self) -> bool {
            let caller = Self::env().caller();
            return caller == self.owner;
        }
        /// returns teh Keccak256 hash of the input bytes
        pub fn hash_keccak_256(input: &[u8]) -> [u8; 32] {
            let mut output = <hash::Keccak256 as hash::HashOutput>::Type::default();
            ink_env::hash_bytes::<hash::Keccak256>(input, &mut output);
            output
        }
    }

    /// Unit tests
    #[cfg(test)]
    mod tests {
        use super::*;

        use ink_lang as ink;
        use ink_env;

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
            set_next_caller(teacher);
            assert_eq!(course_reg.create_course(course_id, course_cap, start_time), Ok(()));

            assert_ne!(course_reg.get_course_info(course_id), Err(Error::NonexistentCourse));
        }

        /// Course registration test
        #[ink::test]
        fn course_registration() {
            let owner = AccountId::from([0x0;32]);
            set_next_caller(owner);
            let mut course_reg = CourseReg::new(owner);
            let teacher = AccountId::from([0x1; 32]);
            let student = AccountId::from([0x2; 32]);
            let course_name = "test_course".as_bytes();
            let course_id = hash_keccak_256(course_name);
            let course_cap:u32 = 10;
            let start_time = get_current_time();

            assert_eq!(course_reg.admit_as_teacher(teacher), Ok(()));
            assert_eq!(course_reg.admit_as_student(student), Ok(()));
            assert_eq!(course_reg.is_school_member(teacher), true);
            assert_eq!(course_reg.is_teacher(teacher), true);
            assert_eq!(course_reg.is_school_member(student), true);
            set_next_caller(teacher);
            assert_eq!(course_reg.create_course(course_id, course_cap, start_time), Ok(()));
            assert_ne!(course_reg.get_course_info(course_id), Err(Error::NonexistentCourse));
            set_next_caller(student);

            assert_eq!(course_reg.register_to_course(course_id),Ok(()));
            assert_ne!(course_reg.get_own_registrations(), Err(Error::NoRegistrations));
        }

        /// Swap proposal creation test
        #[ink::test]
        fn swap_proposal_creation() {
            let owner = AccountId::from([0x0;32]);
            set_next_caller(owner);
            let mut course_reg = CourseReg::new(owner);
            let teacher = AccountId::from([0x1; 32]);
            let student = AccountId::from([0x2; 32]);
            let course_name = "test_course".as_bytes();
            let course_id = hash_keccak_256(course_name);
            let course_cap:u32 = 10;
            let start_time = get_current_time();

            assert_eq!(course_reg.admit_as_teacher(teacher), Ok(()));
            assert_eq!(course_reg.admit_as_student(student), Ok(()));
            assert_eq!(course_reg.is_school_member(teacher), true);
            assert_eq!(course_reg.is_teacher(teacher), true);
            assert_eq!(course_reg.is_school_member(student), true);
            set_next_caller(teacher);
            assert_eq!(course_reg.create_course(course_id, course_cap, start_time), Ok(()));
            assert_ne!(course_reg.get_course_info(course_id), Err(Error::NonexistentCourse));
            set_next_caller(student);
            assert_eq!(course_reg.register_to_course(course_id), Ok(()));
            assert_ne!(course_reg.get_own_registrations(), Err(Error::NoRegistrations));

            assert_eq!(course_reg.propose_swap(course_id), Ok(()));
            assert_eq!(course_reg.get_proposed_swaps(course_id).unwrap().len(),1); 
            assert_eq!(course_reg.get_own_registrations(), Err(Error::NoRegistrations));
        }
        //fn advance_block() {
        //    ink_env::test::advance_block::<ink_env::DefaultEnvironment>();
        //}
        /// Counter swap proposal test
        #[ink::test]
        fn swap_proposal_counter() {
            let owner = AccountId::from([0x0;32]);
            set_next_caller(owner);
            let mut course_reg = CourseReg::new(owner);
            let teacher = AccountId::from([0x1; 32]);
            let student1 = AccountId::from([0x2; 32]);
            let student2 = AccountId::from([0x3; 32]);
            let course_name1 = "test_course1".as_bytes();
            let course_id1 = hash_keccak_256(course_name1);
            let course_name2 = "test_course2".as_bytes();
            let course_id2 = hash_keccak_256(course_name2);
            let course_cap:u32 = 10;
            let start_time = get_current_time();

            assert_eq!(course_reg.admit_as_teacher(teacher), Ok(()));
            assert_eq!(course_reg.admit_as_student(student1), Ok(()));
            assert_eq!(course_reg.admit_as_student(student2), Ok(()));
            assert_eq!(course_reg.is_school_member(teacher), true);
            assert_eq!(course_reg.is_teacher(teacher), true);
            assert_eq!(course_reg.is_school_member(student1), true);
            assert_eq!(course_reg.is_school_member(student2), true);
            set_next_caller(teacher);
            assert_eq!(course_reg.create_course(course_id1, course_cap, start_time), Ok(()));
            assert_ne!(course_reg.get_course_info(course_id1), Err(Error::NonexistentCourse));
            assert_eq!(course_reg.create_course(course_id2, course_cap, start_time), Ok(()));
            assert_ne!(course_reg.get_course_info(course_id2), Err(Error::NonexistentCourse));
            set_next_caller(student1);
            assert_eq!(course_reg.register_to_course(course_id1), Ok(()));
            assert_ne!(course_reg.get_own_registrations(), Err(Error::NoRegistrations));
            assert_eq!(course_reg.propose_swap(course_id1), Ok(()));
            assert_eq!(course_reg.get_proposed_swaps(course_id1).unwrap().len(),1); 
            set_next_caller(student2);
            assert_eq!(course_reg.register_to_course(course_id2), Ok(()));
            assert_ne!(course_reg.get_own_registrations(), Err(Error::NoRegistrations));

            assert_eq!(course_reg.counter_swap_proposal(course_id1, student1, course_id2), Ok(()));  
            set_next_caller(student2);
            assert_eq!(course_reg.get_own_registrations(), Err(Error::NoRegistrations));
        }
    }
}
