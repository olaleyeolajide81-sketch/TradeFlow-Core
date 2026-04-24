use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Token allowance is insufficient for the requested transfer amount
    InsufficientAllowance = 1,
    /// Contract has not been initialized
    NotInitialized = 2,
    /// Invalid token address provided
    InvalidTokenAddress = 3,
    /// Insufficient liquidity in the pool
    InsufficientLiquidity = 4,
    /// Slippage protection triggered - output amount too low
    InsufficientOutputAmount = 5,
    /// Received fewer liquidity shares than minimum expected
    InsufficientSharesReceived = 6,
    /// Fee exceeds maximum allowed (10000 basis points = 100%)
    FeeTooHigh = 7,
    /// Timelock period has not elapsed yet
    TimelockNotElapsed = 8,
    /// No pending fee change found
    NoPendingFeeChange = 9,
    /// Permit signature has expired
    PermitExpired = 10,
    /// Permit owner mismatch
    PermitOwnerMismatch = 11,
    /// Invalid nonce in permit
    InvalidNonce = 12,
    /// Invalid permit signature
    InvalidPermitSignature = 13,
    /// Flash loan is currently active - pool is locked
    FlashLoanActive = 14,
    /// Trade size exceeds maximum allowed percentage
    TradeSizeExceedsMaximum = 15,
    /// Factory is paused - all operations halted
    FactoryPaused = 16,
    /// Insufficient balance for requested operation
    InsufficientBalance = 17,
    /// Slippage exceeded during swap
    SlippageExceeded = 18,
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
            Error::FactoryPaused => "Factory is paused - all operations are halted",
            Error::InsufficientBalance => "Insufficient balance for requested operation",
            Error::SlippageExceeded => "Slippage exceeded during swap",
        }
    }
}

pub fn check_and_panic_error(error: Error) -> ! {
    panic!("{}", error.as_str());
}
