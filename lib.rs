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

        /// Returns true if the account is a school_member
        #[ink(message)]
        pub fn is_school_member(&self, account: AccountId) -> bool {
            self.is_school_member_inner(account)
        }

        fn is_school_member_inner(&self, account: AccountId) -> bool {
            self.school_members.contains(account)
        }

        /// Returns true if the account is a teacher
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
                self.add_registration(course_id, caller);
                Ok(())
            }

            /// creates a CourseRegistration token for the course with course_id
            /// and the caller becomes the owner of the token
            fn add_registration(&mut self, course_id: [u8;32], owner: AccountId) {
                let course_reg = CourseRegistration { owner, course_id};
                if !self.registrations.contains(owner) {
                    let mut registrations = Vec::new();
                    registrations.push(course_reg);
                    self.registrations.insert(&owner, &registrations);
                    return;
                }
                let mut registrations = self.registrations.get(owner).unwrap();
                registrations.push(course_reg);
                self.registrations.insert(&owner, &registrations);
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
                let swaps = self.swaps.get(course_id);
                if swaps.as_ref().unwrap().len() == 0 {
                    return Err(Error::NoProposedSwap);
                }
                Ok(swaps.unwrap())
            }

            /// Place a counter offer on a swap proposal
            #[ink(message)]
            pub fn counter_swap_proposal(&mut self, 
                                         course_id: [u8;32],
                                         offerer: AccountId,
                                         counter_course_id: [u8; 32]) -> Result<(), Error> {
                let caller = Self::env().caller();
                // first we need to verify if the caller has the required
                // registration to swap
                let caller_regs = self.get_own_registrations();

                if caller_regs.is_err() {
                    return Err(Error::NoProposedSwap);
                }

                let mut caller_regs = caller_regs.unwrap();
                let exchange_course = caller_regs.iter().position(|reg| reg.course_id == counter_course_id);

                if exchange_course.is_none() {
                    return Err(Error::NoProposedSwap);
                }

                let exchange_course = exchange_course.unwrap();
                let exchange_course = caller_regs.remove(exchange_course);
                self.registrations.insert(&caller, &caller_regs); // caller's reg is removed

                // find the proposal the counter offer belongs to
                if !self.swaps.contains(course_id) {
                    return Err(Error::NoProposedSwap);
                }

                let mut proposals = self.swaps.get(course_id).unwrap();

                let found_prop = proposals.iter().position(|prop| prop.offer.owner == offerer);
                if found_prop.is_none() {
                    return Err(Error::NoProposedSwap);
                }
                let found_prop = found_prop.unwrap();
                let prop = &mut proposals[found_prop];
                prop.counter_offers.push(exchange_course);

                // result is saved
                self.swaps.insert(&course_id, &proposals);

                Ok(())
            }

            /// accepts a swap counter offer to a swap proposed by the caller
            #[ink(message)]
            pub fn accept_counter_offer(&mut self, 
                                        offered_course_id: [u8;32],
                                        accepted_course_id: [u8;32],
                                        accepted_owner: AccountId) -> Result<(), Error> {
                let caller = Self::env().caller();
                if !self.swaps.contains(offered_course_id) {
                    return Err(Error::NoProposedSwap)
                }

                // find the proposal of the caller
                let mut proposals = self.swaps.get(offered_course_id).unwrap();
                let found_prop = proposals.iter().position(|prop| prop.offer.owner == caller);

                if found_prop.is_none() {
                    return Err(Error::NoProposedSwap)
                }

                // remove the proposal from the active proposals
                let found_prop = found_prop.unwrap();
                let mut found_prop = proposals.remove(found_prop);
                self.swaps.insert(&offered_course_id, &proposals);
                
                // find the accepted counter offer
                let found_counter = found_prop.counter_offers.iter()
                                    .position(|counter_off| 
                                              counter_off.owner == accepted_owner
                                              && counter_off.course_id == accepted_course_id);
                if found_counter.is_none() {
                    return Err(Error::NoProposedSwap);
                }
                let found_counter = found_counter.unwrap();
                let found_counter = found_prop.counter_offers.remove(found_counter);
                if found_counter.owner != accepted_owner {
                    return Err(Error::NoProposedSwap);
                }

                // perform the token swap
                self.add_registration(accepted_course_id, caller);
                self.add_registration(offered_course_id, found_counter.owner);

                // change registrations in the course reg list
                let rep_res = self.replace_registration_in_reg_list(accepted_course_id, accepted_owner, caller);
                if rep_res.is_err() {
                    return rep_res;
                }
                self.replace_registration_in_reg_list(offered_course_id, caller, accepted_owner)
            }

            fn replace_registration_in_reg_list(&mut self, course_id: [u8;32], replace:AccountId, with:AccountId) -> 
                Result<(),Error> {
                    let replace_in = self.courses.get(course_id);
                    if replace_in.is_none() {
                        return Err(Error::NoProposedSwap);
                    }
                    let mut replace_in = replace_in.unwrap();
                    let reg = replace_in.registrations.iter().position(|acc_id| acc_id == &replace);
                    if reg.is_none() {
                        return Err(Error::NoProposedSwap);
                    }
                    replace_in.registrations[reg.unwrap()] = with;
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

        /// Full happy path test
        #[ink::test]
        fn accept_counter_offer() {
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

            set_next_caller(student1);
            assert_eq!(course_reg.accept_counter_offer(course_id1, course_id2, student2), Ok(()));
            let pos = course_reg.get_own_registrations().unwrap().iter().position(|course| course.course_id == course_id2);
            assert!(!pos.is_none());
        }
    }
}
