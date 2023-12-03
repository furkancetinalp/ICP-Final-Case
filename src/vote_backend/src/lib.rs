//cargo build --target wasm32-unknown-unknown
use candid::{CandidType, Decode, Deserialize, Encode};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};
#[derive(CandidType, Deserialize)]
struct Proposal {
    description: String,
    approve: u32,
    reject: u32,
    pass: u32,
    is_active: bool,
    voted: Vec<candid::Principal>,
    owner: candid::Principal,
    total_vote :u32, //Adding total_vote variable to track total vote count and to write less line of code (to improve code efficiency)

}
#[derive(CandidType, Deserialize)]

struct CreateProposal {
    description: String,
    is_active: bool,
}


#[derive(CandidType, Deserialize)]
enum VoteTypes {
    Approve,
    Reject,
    Pass,
}
#[derive(CandidType, Deserialize)]
enum VoteError {
    AlreadyVoted,
    ProposalNotActive,
    Unauthorized,
    NoProposal,
    UpdateError,
    VoteFailed,
}
#[derive(CandidType, Deserialize)]
enum VoteStatus {
    Approved,
    Rejected,
    Passed,
    Undecided,

}
type Memory = VirtualMemory<DefaultMemoryImpl>;
const MAX_VALUE_SIZE: u32 = 1000;

impl Storable for Proposal {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Proposal {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static PROPOSAL_MAP: RefCell<StableBTreeMap<u64, Proposal, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );
}

#[ic_cdk_macros::query]
fn get_proposal(key:u64) ->Option<Proposal>{
    PROPOSAL_MAP.with(|p| p.borrow().get(&key))
}

#[ic_cdk_macros::query]
fn get_proposal_count() ->u64{
    PROPOSAL_MAP.with(|p| p.borrow().len())
}

#[ic_cdk_macros::update]
fn create_proposal(key:u64,proposal:CreateProposal) -> Option<Proposal>{
    let value = Proposal{
        description:proposal.description,
        approve: 0u32,
        reject: 0u32,
        pass: 0u32,
        is_active: proposal.is_active,
        voted:vec![],
        owner:ic_cdk::caller(),
        total_vote:0u32,
    };
    PROPOSAL_MAP.with(|p| p.borrow_mut().insert(key,value))
}
#[ic_cdk_macros::update]
fn edit_proposal(key: u64, proposal: CreateProposal) -> Result<(), VoteError> {
    PROPOSAL_MAP.with(|p| {
        let old_proposal = match p.borrow().get(&key) {
            Some(value) => value,
            None => return Err(VoteError::NoProposal),
        };
        if ic_cdk::caller() != old_proposal.owner {            
            return Err(VoteError::Unauthorized);
        }
        let value = Proposal {
            description: proposal.description,
            approve: old_proposal.approve,
            reject: old_proposal.reject,
            pass: old_proposal.pass,
            is_active: proposal.is_active,
            voted: old_proposal.voted,
            owner: old_proposal.owner, //preventing owner to be changed when each vote call is applied
            total_vote: old_proposal.total_vote,
        };
        let res = p.borrow_mut().insert(key, value);
        match res {
            Some(_) => Ok(()),
            None => Err(VoteError::UpdateError),
        }
    })
}

#[ic_cdk_macros::update]
fn end_proposal(key: u64) -> Result<(), VoteError> {
    PROPOSAL_MAP.with(|p| {
        let mut proposal = p.borrow_mut().get(&key).unwrap();
        if ic_cdk::caller() != proposal.owner {
            return Err(VoteError::Unauthorized);
        }
        proposal.is_active = false;
        let res = p.borrow_mut().insert(key, proposal);
        match res {
            Some(_) => Ok(()),
            None => Err(VoteError::UpdateError),
        }
    })
}
//
#[ic_cdk_macros::update]
fn vote(key: u64, choice: VoteTypes) -> Result<(), VoteError> {
    PROPOSAL_MAP.with(|p| {
        let mut proposal = p.borrow_mut().get(&key).unwrap();
        //Creating a new unique Principal id in order to pass the restriction of AlreadyVoted
        let caller = candid::Principal::self_authenticating(&ic_cdk::api::time().to_string());
        if proposal.voted.contains(&caller) {
            return Err(VoteError::AlreadyVoted);
        } else if !proposal.is_active {
            return Err(VoteError::ProposalNotActive);
        }
        match choice {
            VoteTypes::Approve => proposal.approve += 1,
            VoteTypes::Reject => proposal.reject += 1,
            VoteTypes::Pass => proposal.pass += 1,
        }
        proposal.total_vote +=1;
        proposal.voted.push(caller);
        let res = p.borrow_mut().insert(key, proposal);
        match res {
            Some(_) => Ok(()),
            None => Err(VoteError::VoteFailed),
        }
    })
}

#[ic_cdk_macros::query]
fn get_proposal_status(key:u64) ->Option<VoteStatus>{
    let proposal = PROPOSAL_MAP.with(|p| p.borrow().get(&key));//getting the selected vote from storage
    match proposal {
        Some(data) =>{
            // return Undecided if total_vote is less than 5
            if data.total_vote < 5 {
                return Some(VoteStatus::Undecided);
            }
            else{
                return Some(find_max_voted_type(&data)); //a new method that evaluates  result of the proposal
            }
        },
        None => None,
    }
}
//logic that finds the status of proposal
fn find_max_voted_type(prop: &Proposal) -> VoteStatus{
    
    if (prop.approve > prop.pass) && (prop.approve > prop.reject) && ((prop.approve as f32/prop.total_vote as f32)  >= 0.5) {
        return VoteStatus::Approved;
    }
    else if (prop.pass > prop.approve) && (prop.pass > prop.reject) && ((prop.pass as f32/prop.total_vote as f32)  >= 0.5) {
        return VoteStatus::Passed;
    }
    else if (prop.reject > prop.approve) && (prop.reject > prop.pass) && ((prop.reject as f32/prop.total_vote as f32)  >= 0.5) {
        return VoteStatus::Rejected;
    }
    else{
        return VoteStatus::Undecided;
    }
}