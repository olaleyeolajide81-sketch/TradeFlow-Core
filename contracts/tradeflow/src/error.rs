use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
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

impl Error {
    pub fn as_str(&self) -> &'static str {
        match self {
            Error::InsufficientAllowance => "Insufficient token allowance. Please approve the contract to spend your tokens.",
            Error::NotInitialized => "Contract has not been initialized",
            Error::InvalidTokenAddress => "Invalid token address provided",
            Error::InsufficientLiquidity => "Insufficient liquidity in the pool",
            Error::InsufficientOutputAmount => "Insufficient output amount - slippage protection triggered",
            Error::InsufficientSharesReceived => "Insufficient liquidity shares received",
            Error::FeeTooHigh => "Fee exceeds maximum allowed (100%)",
            Error::TimelockNotElapsed => "Timelock period has not elapsed yet",
            Error::NoPendingFeeChange => "No pending fee change found",
            Error::PermitExpired => "Permit signature has expired",
            Error::PermitOwnerMismatch => "Permit owner mismatch",
            Error::InvalidNonce => "Invalid nonce in permit",
            Error::InvalidPermitSignature => "Invalid permit signature",
            Error::FlashLoanActive => "Flash loan is currently active - pool operations are locked",
            Error::TradeSizeExceedsMaximum => "Trade size exceeds maximum allowed percentage",
        }
    }
}

pub fn check_and_panic_error(error: Error) -> ! {
    panic!("{}", error.as_str());
}
