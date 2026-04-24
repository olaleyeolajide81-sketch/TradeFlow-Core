 # Liquidity Pool Math Guide

This guide explains the constant-product formula, and how swaps work when a trading fee is introduced in TradeFlow-Core. It is written to be easy to follow for new developers. 

---

## 1. Explanation: The Constant Product Formula: x * y = k

Every pool keeps a constant balance between two tokens (when people trade) using this rule:

` x * y = k `


- \(**x**\) = reserve of token_A in the pool  (e.g., XLM)
- \(**y**\) = reserve of token_B in the pool  (e.g., USDC)
- \(**k**\) = constant value that doesn’t change during a trade (unless fees are added)

---

## 2. Fee Logic
A protocol fee is deducted from a user's input amount before a swap is executed.

* **Fee Rate:** 0.3%
* **Formula:** `amount_in * (1-Fee) = amount_in_scaled`

---

## 3. How To Calculate Swaps

When you put tokens in, the pool gives you some of the other token back.  
The math looks like this:


```
amount_out_scaled = (reserve_out_scaled * amount_in_scaled) / (reserve_in_scaled + amount_in_scaled)
```
- **Reserve_out:** Total balance of the token the user is taking out of the pool
- **Reserve_in:** Total balance of the token the user is depositing into the pool

Before you run the above formula, you need to deduct a **0.3% fee** from your input. 
That fee remains in the pool to reward liquidity providers.

---

## 4. Step-by-Step Example: 100 XLM → USDC

**Scenario:** Imagine the pool has:
- 10,000 XLM  
- 5,000 USDC  

You want to swap **100 XLM** for **USDC :** 

### **Step 1: Fee deduction**  


```100 XLM * (1 - 0.003) = 99.7 XLM```

This gives the `amount_in_scaled`.

### **Step 2: Apply `amount_out` formula**  

amount_out_scaled = (reserve_out_scaled * amount_in_scaled) / (reserve_in_scaled + amount_in_scaled) 
                               
```
amount_out = (5000 * 99.7) /(10000 + 99.7)
```

### **Step 3: Result**  
You get `amount_out` at approx **49.36 USDC**. 

--- 
## 5. Source Code Reference
The formula is defined in the following locations:
| Logic       | Code Reference |
| ----------- | -------------- |
| Source Code | [`contracts/amm_pool/src/llib.rs`](../contracts/amm_pool/src/lib.rs)|
| Associated Manifest | [`contracts/amm_pool/Cargo.toml`](../contracts/amm_pool/Cargo.toml)|

---
 
