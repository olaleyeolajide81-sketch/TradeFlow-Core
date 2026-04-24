use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeFlowError {
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
    /// Contract is already initialized
    AlreadyInitialized = 19,
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
            TradeFlowError::AlreadyInitialized => "Contract already initialized",
        }
    }
}

pub fn check_and_panic_error(error: TradeFlowError) -> ! {
    panic!("{}", error.as_str());
}
