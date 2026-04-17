use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeFlowError {
    /// Token allowance is insufficient for the requested transfer amount
    InsufficientAllowance,
    /// Contract has not been initialized
    NotInitialized,
    /// Invalid token address provided
    InvalidTokenAddress,
    /// Insufficient liquidity in the pool
    InsufficientLiquidity,
    /// Slippage protection triggered - output amount too low
    InsufficientOutputAmount,
    /// Received fewer liquidity shares than minimum expected
    InsufficientSharesReceived,
    /// Fee exceeds maximum allowed (10000 basis points = 100%)
    FeeTooHigh,
    /// Timelock period has not elapsed yet
    TimelockNotElapsed,
    /// No pending fee change found
    NoPendingFeeChange,
    /// Permit signature has expired
    PermitExpired,
    /// Permit owner mismatch
    PermitOwnerMismatch,
    /// Invalid nonce in permit
    InvalidNonce,
    /// Invalid permit signature
    InvalidPermitSignature,
    /// Flash loan is currently active - pool is locked
    FlashLoanActive,
    /// Trade size exceeds maximum allowed percentage
    TradeSizeExceedsMaximum,
}

impl TradeFlowError {
    pub fn as_str(&self) -> &'static str {
        match self {
            TradeFlowError::InsufficientAllowance => "Insufficient token allowance. Please approve the contract to spend your tokens.",
            TradeFlowError::NotInitialized => "Contract has not been initialized",
            TradeFlowError::InvalidTokenAddress => "Invalid token address provided",
            TradeFlowError::InsufficientLiquidity => "Insufficient liquidity in the pool",
            TradeFlowError::InsufficientOutputAmount => "Insufficient output amount - slippage protection triggered",
            TradeFlowError::InsufficientSharesReceived => "Insufficient liquidity shares received",
            TradeFlowError::FeeTooHigh => "Fee exceeds maximum allowed (100%)",
            TradeFlowError::TimelockNotElapsed => "Timelock period has not elapsed yet",
            TradeFlowError::NoPendingFeeChange => "No pending fee change found",
            TradeFlowError::PermitExpired => "Permit signature has expired",
            TradeFlowError::PermitOwnerMismatch => "Permit owner mismatch",
            TradeFlowError::InvalidNonce => "Invalid nonce in permit",
            TradeFlowError::InvalidPermitSignature => "Invalid permit signature",
            TradeFlowError::FlashLoanActive => "Flash loan is currently active - pool operations are locked",
            TradeFlowError::TradeSizeExceedsMaximum => "Trade size exceeds maximum allowed percentage",
        }
    }
}

pub fn check_and_panic_error(error: TradeFlowError) -> ! {
    panic!("{}", error.as_str());
}
