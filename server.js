const express = require('express');

require('dotenv').config();

const app = express();
const port = process.env.PORT || 3000;

// Security and middleware
app.use(helmet());
app.use(cors());
app.use(express.json());

// Mock data: A list of transactions to demonstrate pagination
const transactions = Array.from({ length: 100 }, (_, i) => ({
    id: `tx_${i + 1}`,
    type: i % 2 === 0 ? 'deposit' : 'withdrawal',
    amount: (Math.random() * 10000).toFixed(2),
    currency: 'USDC',
    timestamp: new Date(Date.now() - i * 3600000).toISOString(), // 1 hour apart
    status: 'completed'
}));

// Health check
app.get('/health', (req, res) => {
    res.json({ status: 'ok', timestamp: Date.now() });
});

// Contract info (from README_SERVER.md)
app.get('/api/contracts', (req, res) => {
    res.json({
        invoice_nft: process.env.INVOICE_NFT_ID || 'CCYU3LOQI34VHVN3ZOSEBHHKL4YK36FMTOEGLRYDUDRGS7JOLLRKCEQM',
        lending_pool: process.env.LENDING_POOL_ID || 'CDVJMVPLZJKXSJFDY5AWBOUIRN73BKU2SG674MQDH4GRE6BGBPQD33IQ'
    });
});

// Transactions endpoint with actual pagination logic (Issue #35)
app.get('/api/transactions', (req, res) => {
    // 1. Parse page and limit from query parameters (default to page 1, limit 10)
    const page = parseInt(req.query.page) || 1;
    const limit = parseInt(req.query.limit) || 10;

    // 2. Calculate startIndex and endIndex
    const startIndex = (page - 1) * limit;
    const endIndex = page * limit;

    // 3. Slice the array to get the requested chunk
    const paginatedData = transactions.slice(startIndex, endIndex);


});

app.listen(port, () => {
    console.log(`Server running on port ${port}`);
});