# TradeFlow-Core API Server

This Express.js server provides a REST API for the TradeFlow-Core smart contracts and includes security headers via Helmet.js.

## Prerequisites

- Node.js 18+ and npm installed
- Clone the TradeFlow-Core repository

## Installation

1. Install dependencies:
```bash
npm install
```

2. Create environment file:
```bash
cp .env.example .env
```

3. Start the server:
```bash
# Development mode
npm run dev

# Production mode
npm start
```

## Security Features

This server implements the following security measures via Helmet.js:

- **XSS Protection**: Prevents cross-site scripting attacks
- **Content Security Policy**: Restricts resource loading
- **HSTS**: Enforces HTTPS connections
- **X-Frame-Options**: Prevents clickjacking
- **X-Content-Type-Options**: Prevents MIME type sniffing
- **Referrer Policy**: Controls referrer information

## API Endpoints

### Health Check
```
GET /health
```
Returns server status and timestamp.

### Contract Information
```
GET /api/contracts
```
Returns deployed contract IDs for Invoice NFT and Lending Pool contracts.

## Environment Variables

- `PORT`: Server port (default: 3000)
- `NODE_ENV`: Environment (development/production)

## Security Headers

All responses include security headers thanks to Helmet.js middleware. You can verify this by checking the response headers in your browser's developer tools.

## Development

The server is configured for development with:
- Hot reload via nodemon
- CORS enabled
- Comprehensive error handling
- Detailed logging
