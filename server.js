const express = require('express');
const helmet = require('helmet');
const cors = require('cors');
require('dotenv').config();

const app = express();

// Startup validation for required .env variables
if (!process.env.PORT) {
    console.warn('Warning: PORT is not defined in .env file. Falling back to default port 3000.');
}

const port = process.env.PORT || 3000;

// Contract version constant (#59)
const CONTRACT_VERSION = process.env.CONTRACT_VERSION || '1.0.0';

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
    timestamp: new Date(Date.now() - i * 3600000).toISOString(),
    status: 'completed'
}));

// Health check
app.get('/health', (req, res) => {
    res.json({ status: 'ok', timestamp: Date.now() });
});

// Version endpoint (#59)
app.get('/api/v1/version', (req, res) => {
    res.json({ version: CONTRACT_VERSION });
});

// Prices endpoint
app.get('/api/v1/prices', (req, res) => {
    res.json({
        USDC: '1.00',
        XLM: '0.11',
        timestamp: Date.now()
    });
});

// Rate limiting test endpoint
app.get('/api/v1/test', (req, res) => {
    res.json({ status: 'ok' });
});

// Contract info (from README_SERVER.md)
app.get('/api/contracts', (req, res) => {
    res.json({
        invoice_nft: process.env.INVOICE_NFT_ID || 'CCYU3LOQI34VHVN3ZOSEBHHKL4YK36FMTOEGLRYDUDRGS7JOLLRKCEQM',
        lending_pool: process.env.LENDING_POOL_ID || 'CDVJMVPLZJKXSJFDY5AWBOUIRN73BKU2SG674MQDH4GRE6BGBPQD33IQ'
    });
});

// Transactions endpoint with pagination (Issue #35)
app.get('/api/transactions', (req, res) => {
    const page = parseInt(req.query.page) || 1;
    const limit = parseInt(req.query.limit) || 10;
    const startIndex = (page - 1) * limit;
    const endIndex = page * limit;
    const paginatedData = transactions.slice(startIndex, endIndex);

    res.json({
        page,
        limit,
        total: transactions.length,
        data: paginatedData
    });
});

// Global 404 Not Found handler (FIXED)
app.use((req, res) => {
    res.status(404).json({ error: "Route not found" });
});

app.listen(port, () => {
    console.log(`Server running on port ${port}`);
});