use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    sol,
};
use eyre::Result;
use std::str::FromStr;

const USDC_ADDRESS: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";

const SPENDER_ONE: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";
const SPENDER_TWO: &str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";
const SPENDER_THREE: &str = "0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296";

const MIN_MATIC_BALANCE: u64 = 1000;

const MAX_APPROVAL: U256 = U256::MAX;

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
    }
}

#[derive(Debug, Default)]
pub struct ApprovalResult {
    pub success: bool,
    pub error: Option<String>,
    pub approvals: Vec<Option<String>>,
}

#[derive(Debug, Default)]
pub struct ApprovalStatus {
    pub spender_one: bool,
    pub spender_two: bool,
    pub spender_three: bool,
}

impl ApprovalStatus {
    pub fn is_fully_approved(&self) -> bool {
        self.spender_one && self.spender_two && self.spender_three
    }
}

pub async fn check_approval_status<P: Provider + Clone>(
    provider: &P,
    user_address: Address,
) -> Result<ApprovalStatus> {
    let usdc = Address::from_str(USDC_ADDRESS)?;
    let spender_one = Address::from_str(SPENDER_ONE)?;
    let spender_two = Address::from_str(SPENDER_TWO)?;
    let spender_three = Address::from_str(SPENDER_THREE)?;

    let allowance_one = check_allowance(provider, usdc, user_address, spender_one).await?;
    let allowance_two = check_allowance(provider, usdc, user_address, spender_two).await?;
    let allowance_three = check_allowance(provider, usdc, user_address, spender_three).await?;

    Ok(ApprovalStatus {
        spender_one: allowance_one > U256::ZERO,
        spender_two: allowance_two > U256::ZERO,
        spender_three: allowance_three > U256::ZERO,
    })
}

pub async fn is_fully_approved<P: Provider + Clone>(
    provider: &P,
    user_address: Address,
) -> Result<bool> {
    let status = check_approval_status(provider, user_address).await?;
    Ok(status.is_fully_approved())
}

pub async fn check_allowance<P: Provider + Clone>(
    provider: &P,
    token_address: Address,
    owner: Address,
    spender: Address,
) -> Result<U256> {
    let contract = IERC20::new(token_address, provider.clone());
    let allowance = contract.allowance(owner, spender).call().await?;
    Ok(allowance)
}

pub async fn create_approve_tx<P: Provider + Clone>(
    provider: &P,
    token_address: Address,
    spender: Address,
    amount: U256,
) -> Result<String> {
    let contract = IERC20::new(token_address, provider.clone());

    let tx = contract.approve(spender, amount);
    let pending_tx = tx.send().await?;
    let receipt = pending_tx.get_receipt().await?;

    Ok(format!("{:?}", receipt.transaction_hash))
}

pub async fn approval_process<P: Provider + Clone>(
    provider: P,
    user_address: Address,
) -> Result<ApprovalResult> {
    let balance = provider.get_balance(user_address).await?;
    if balance < U256::from(MIN_MATIC_BALANCE) {
        return Ok(ApprovalResult {
            success: false,
            error: Some("Insufficient MATIC balance to process approvals.".to_string()),
            approvals: vec![],
        });
    }

    let usdc = Address::from_str(USDC_ADDRESS)?;
    let spender_one = Address::from_str(SPENDER_ONE)?;
    let spender_two = Address::from_str(SPENDER_TWO)?;
    let spender_three = Address::from_str(SPENDER_THREE)?;

    let mut approvals: Vec<Option<String>> = vec![None; 3];

    let allowance_one = check_allowance(&provider, usdc, user_address, spender_one).await?;
    let allowance_two = check_allowance(&provider, usdc, user_address, spender_two).await?;
    let allowance_three = check_allowance(&provider, usdc, user_address, spender_three).await?;

    if allowance_one == U256::ZERO {
        let hash = create_approve_tx(&provider, usdc, spender_one, MAX_APPROVAL).await?;
        approvals[0] = Some(hash);
    } else {
        println!("Allowance one already set: {}", allowance_one);
    }

    if allowance_two == U256::ZERO {
        let hash = create_approve_tx(&provider, usdc, spender_two, MAX_APPROVAL).await?;
        approvals[1] = Some(hash);
    } else {
        println!("Allowance two already set: {}", allowance_two);
    }

    if allowance_three == U256::ZERO {
        let hash = create_approve_tx(&provider, usdc, spender_three, MAX_APPROVAL).await?;
        approvals[2] = Some(hash);
    } else {
        println!("Allowance three already set: {}", allowance_three);
    }

    println!("Approvals: {:?}", approvals);

    Ok(ApprovalResult {
        success: true,
        error: None,
        approvals,
    })
}