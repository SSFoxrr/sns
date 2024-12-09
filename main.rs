// We use borsh for efficient serialization and deserialization in Solana programs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    program_error::ProgramError,
    system_instruction,
    sysvar::{rent::Rent, Sysvar, clock::Clock},
    program::invoke,
};

// Constants to limit the size of names and records
const MAX_NAME_LENGTH: usize = 64;
const MAX_RECORD_SIZE: usize = 256;

// NameRecord struct with derive macros for Borsh serialization/deserialization
// and other useful traits for Solana program data structures
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default, PartialEq)]
pub struct NameRecord {
    pub name: String,
    pub owner: Pubkey,
    pub created_at: i64,
}

// Declare the program's entrypoint
entrypoint!(process_instruction);

// Main instruction processing function
// We use a lifetime parameter 'a to ensure all AccountInfo references have the same lifetime
pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    // Ensure we have some instruction data
    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse the instruction type from the first byte
    let instruction = instruction_data[0];
    let name_data = &instruction_data[1..];

    // Iterator for the accounts
    let accounts_iter = &mut accounts.iter();
    
    // Extract the required accounts
    let payer = next_account_info(accounts_iter)?;
    let name_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Route to the appropriate instruction handler
    match instruction {
        0 => register_name(program_id, payer, name_account, system_program, name_data),
        1 => resolve_name(name_account),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// Function to register a new name
// We use the same lifetime 'a for all AccountInfo references to satisfy the borrow checker
fn register_name<'a>(
    program_id: &Pubkey,
    payer: &'a AccountInfo<'a>,
    name_account: &'a AccountInfo<'a>,
    system_program: &'a AccountInfo<'a>,
    name_data: &[u8],
) -> ProgramResult {
    // Validate name length
    if name_data.is_empty() || name_data.len() > MAX_NAME_LENGTH {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Convert name bytes to string
    let name = String::from_utf8(name_data.to_vec())
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Calculate rent-exempt balance
    let rent = Rent::get()?;
    let space = MAX_RECORD_SIZE;
    let rent_lamports = rent.minimum_balance(space);

    // Create the name account
    invoke(
        &system_instruction::create_account(
            payer.key,
            name_account.key,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[payer.clone(), name_account.clone(), system_program.clone()],
    )?;

    // Create and serialize the NameRecord
    let record = NameRecord {
        name: name.clone(),
        owner: *payer.key,
        created_at: Clock::get()?.unix_timestamp,
    };

    // We use serialize directly as it's compatible with borsh 1.5.3
    record.serialize(&mut &mut name_account.data.borrow_mut()[..])?;

    msg!("Registered name: {}", name);
    Ok(())
}

// Function to resolve a name
fn resolve_name(name_account: &AccountInfo) -> ProgramResult {
    // Deserialize the NameRecord from the account data
    // We use deserialize directly as it's compatible with borsh 1.5.3
    let record = NameRecord::deserialize(&mut &name_account.data.borrow()[..])?;

    // Log the name details
    msg!("Name: {}", record.name);
    msg!("Owner: {}", record.owner);
    msg!("Created at: {}", record.created_at);

    Ok(())
}

// Test module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_record_serialization() {
        // Create a test record
        let record = NameRecord {
            name: "example.sol".to_string(),
            owner: Pubkey::new_unique(),
            created_at: 1234567890,
        };

        // Serialize the record
        let mut serialized = Vec::new();
        record.serialize(&mut serialized).unwrap();

        // Deserialize the record
        let decoded = NameRecord::deserialize(&mut &serialized[..]).unwrap();

        // Assert that the deserialized record matches the original
        assert_eq!(record.name, decoded.name);
        assert_eq!(record.owner, decoded.owner);
        assert_eq!(record.created_at, decoded.created_at);
    }

    #[test]
    fn test_record_size_limits() {
        // Create a record with a name longer than the maximum allowed length
        let long_name = "a".repeat(MAX_NAME_LENGTH + 1);
        let record = NameRecord {
            name: long_name,
            owner: Pubkey::new_unique(),
            created_at: 0,
        };

        // Assert that the name length exceeds the maximum allowed length
        assert!(record.name.len() > MAX_NAME_LENGTH);
    }
}