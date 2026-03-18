pub const ONE_TRTH: u128 = 1_000_000_000;

pub const TX_SIGN_CONTEXT: &[u8] = b"truthlinked-transaction-v1";
pub const GENERIC_SIGN_CONTEXT: &[u8] = b"truthlinked-generic-v1";

pub const GAS_TRANSFER: u64 = 1_000;

pub const STORAGE_RENT_LIFETIME_FEE: u128 = ONE_TRTH;

pub const MAX_CALLDATA_SIZE: usize = 262_144;
pub const MAX_RETURN_DATA_SIZE: usize = MAX_CALLDATA_SIZE;
pub const MAX_CALL_CHAIN_CALLS: usize = 64;
pub const MAX_CALL_CHAIN_TOTAL_CALLDATA: usize = MAX_CALLDATA_SIZE;
pub const MAX_BATCH_TRANSFER_RECIPIENTS: usize = 64;

pub const MIN_URL_PROPOSAL_BOND: u128 = ONE_TRTH;

pub const MAX_AIRDROP_AMOUNT: u128 = 50 * ONE_TRTH;
