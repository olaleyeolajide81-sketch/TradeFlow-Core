import { Request, Response, NextFunction } from 'express';

/**
 * Middleware to verify the admin API key for protected routes.
 * It checks for the 'x-api-key' header and compares it to the ADMIN_API_KEY env variable.
 */
export const verifyApiKey = (req: Request, res: Response, next: NextFunction) => {
  const apiKey = req.headers['x-api-key'];
  const adminApiKey = process.env.ADMIN_API_KEY;

  // Verify that the API key exists and matches the environment variable
  if (!adminApiKey || apiKey !== adminApiKey) {
    return res.status(401).json({ 
      error: 'Unauthorized', 
      message: 'Invalid or missing API key.' 
    });
  }

  next();
};