const express = require('express');
const rateLimit = require('express-rate-limit');
const axios = require('axios');
const packageJson = require('./package.json');
const httpStatusCodes = require('./utils/httpStatusCodes');

require('dotenv').config();

function logWithTime(message) {
  console.log(`[${new Date().toISOString()}] ${message}`);
}

const app = express();
const PORT = process.env.PORT || 3000;

const limiter = rateLimit({
  windowMs: 15 * 60 * 1000,
  max: 100,
  message: {
    error: 'Too many requests from this IP, please try again after 15 minutes.'
  },
  standardHeaders: true,
  legacyHeaders: false,
});

const priceCache = {};
const CACHE_DURATION = 60 * 1000;
app.use(express.json());

app.get('/health', (req, res) => {
  res.json({
    status: 'OK',
    timestamp: new Date().toISOString()
  });
});

app.use('/api/v1/', limiter);

async function fetchPrices() {
  try {
    const response = await axios.get('https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum&vs_currencies=usd,eur');
    return response.data;
  } catch (error) {
    console.error('Error fetching prices:', error.message);
    throw error;
  }
}

app.get('/api/v1/prices', async (req, res) => {
  const now = Date.now();
  
  if (priceCache.data && (now - priceCache.timestamp) < CACHE_DURATION) {
    return res.json({
      ...priceCache.data,
      cached: true,
      timestamp: new Date(priceCache.timestamp).toISOString()
    });
  }

  try {
    const prices = await fetchPrices();
    priceCache.data = prices;
    priceCache.timestamp = now;
    
    res.json({
      ...prices,
      cached: false,
      timestamp: new Date(now).toISOString()
    });
  } catch (error) {
    res.status(httpStatusCodes.INTERNAL_SERVER_ERROR).json({
      error: 'Failed to fetch prices',
      message: error.message
    });
  }
});

app.get('/api/v1/test', (req, res) => {
  res.json({ message: 'Test endpoint working' });
});

app.get('/api/v1/version', (req, res) => {
  res.json({ version: packageJson.version });
});

app.get('/api/contracts', (req, res) => {
  res.json({
    invoice_nft: process.env.INVOICE_NFT_ID || 'CCYU3LOQI34VHVN3ZOSEBHHKL4YK36FMTOEGLRYDUDRGS7JOLLRKCEQM',
    lending_pool: process.env.LENDING_POOL_ID || 'CDVJMVPLZJKXSJFDY5AWBOUIRN73BKU2SG674MQDH4GRE6BGBPQD33IQ',
    network: 'testnet'
  });
});

app.listen(PORT, () => {
  logWithTime(`Server running on port ${PORT}`);
  logWithTime(`Health check available at: http://localhost:${PORT}/health`);
});
