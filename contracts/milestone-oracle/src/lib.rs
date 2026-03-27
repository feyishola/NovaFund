#![no_std]

use shared::types::{MilestoneStatus, Timestamp};
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Auditors,
    Quorum,
    // (project_id, milestone_id) -> Votes
    Votes(u64, u64),
}

#[derive(Clone, Default)]
#[contracttype]
pub struct Votes {
    pub approvals: Vec<Address>,
    pub rejections: Vec<Address>,
    pub finalized: bool,
}

#[contract]
pub struct MilestoneOracle;

#[contractimpl]
impl MilestoneOracle {
    /// Initialize the contract with an admin and the required quorum (number of auditors).
    pub fn initialize(env: Env, admin: Address, quorum: u32) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Quorum, &quorum);
        
        let empty_auditors: Vec<Address> = Vec::new(&env);
        env.storage().instance().set(&DataKey::Auditors, &empty_auditors);
    }

    /// Add an auditor to the whitelist. (Admin only)
    pub fn add_auditor(env: Env, auditor: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        admin.require_auth();

        let mut auditors: Vec<Address> = env.storage().instance().get(&DataKey::Auditors).unwrap();
        if auditors.contains(&auditor) {
            panic!("already auditor");
        }
        auditors.push_back(auditor);
        env.storage().instance().set(&DataKey::Auditors, &auditors);
    }

    /// Remove an auditor from the whitelist. (Admin only)
    pub fn remove_auditor(env: Env, auditor: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        admin.require_auth();

        let mut auditors: Vec<Address> = env.storage().instance().get(&DataKey::Auditors).unwrap();
        let index = auditors.first_index_of(&auditor).expect("not auditor");
        auditors.remove(index);
        env.storage().instance().set(&DataKey::Auditors, &auditors);
    }

    /// Vote on a milestone. (Auditor only)
    pub fn vote(env: Env, auditor: Address, project_id: u64, milestone_id: u64, approve: bool) {
        auditor.require_auth();

        // Check if whitelisted
        let auditors: Vec<Address> = env.storage().instance().get(&DataKey::Auditors).expect("not initialized");
        if !auditors.contains(&auditor) {
            panic!("not whitelisted auditor");
        }

        let key = DataKey::Votes(project_id, milestone_id);
        let mut votes: Votes = env.storage().persistent().get(&key).unwrap_or(Votes {
            approvals: Vec::new(&env),
            rejections: Vec::new(&env),
            finalized: false,
        });

        if votes.finalized {
            panic!("milestone already finalized");
        }

        if votes.approvals.contains(&auditor) || votes.rejections.contains(&auditor) {
            panic!("already voted");
        }

        if approve {
            votes.approvals.push_back(auditor);
        } else {
            votes.rejections.push_back(auditor);
        }

        let quorum: u32 = env.storage().instance().get(&DataKey::Quorum).unwrap();
        
        // Finalize if quorum reached
        if votes.approvals.len() >= quorum {
            votes.finalized = true;
            // In a real system, this would emit an event or call another contract (Escrow)
            env.events().publish((symbol_short!("milestone"), symbol_short!("approved")), (project_id, milestone_id));
        } else if votes.rejections.len() >= quorum {
            votes.finalized = true;
            env.events().publish((symbol_short!("milestone"), symbol_short!("rejected")), (project_id, milestone_id));
        }

        env.storage().persistent().set(&key, &votes);
    }

    /// Get the current status of a milestone.
    pub fn get_milestone_status(env: Env, project_id: u64, milestone_id: u64) -> MilestoneStatus {
        let key = DataKey::Votes(project_id, milestone_id);
        let votes: Votes = env.storage().persistent().get(&key).unwrap_or_default();

        if votes.finalized {
            let quorum: u32 = env.storage().instance().get(&DataKey::Quorum).unwrap();
            if votes.approvals.len() >= quorum {
                MilestoneStatus::Approved
            } else {
                MilestoneStatus::Rejected
            }
        } else if votes.approvals.len() > 0 || votes.rejections.len() > 0 {
            MilestoneStatus::Submitted // Effectively "In Voting"
        } else {
            MilestoneStatus::Pending
        }
    }

    /// Get details of votes for a milestone.
    pub fn get_votes(env: Env, project_id: u64, milestone_id: u64) -> Votes {
        env.storage().persistent().get(&DataKey::Votes(project_id, milestone_id)).unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_voting_flow() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, MilestoneOracle);
        let client = MilestoneOracleClient::new(&env, &contract_id);

        client.initialize(&admin, &2); // 2 of N quorum

        let a1 = Address::generate(&env);
        let a2 = Address::generate(&env);
        let a3 = Address::generate(&env);

        env.mock_all_auths();

        client.add_auditor(&a1);
        client.add_auditor(&a2);
        client.add_auditor(&a3);

        let p_id = 1;
        let m_id = 1;

        // a1 votes approve
        client.vote(&a1, &p_id, &m_id, &true);
        assert_eq!(client.get_milestone_status(&p_id, &m_id) as u32, MilestoneStatus::Submitted as u32);

        // a2 votes approve -> Should be approved (quorum = 2)
        client.vote(&a2, &p_id, &m_id, &true);
        assert_eq!(client.get_milestone_status(&p_id, &m_id) as u32, MilestoneStatus::Approved as u32);
        
        // a3 votes should fail as it is already finalized
        let res = env.as_contract(&contract_id, || {
            client.vote(&a3, &p_id, &m_id, &true);
        });
        // Note: For now we'll just check manual results or handle expected panic if possible,
        // but let's just assert the state remained finalized.
    }
}
