import { VercelRequest, VercelResponse } from '@vercel/node'
import PRICES_DATA from '../data/prices.json'

export default function (_req: VercelRequest, res: VercelResponse) {
  // @ts-expect-error
  res.setHeader('Access-Control-Allow-Credentials', true)
  res.setHeader('Access-Control-Allow-Origin', '*')
  // res.setHeader('Access-Control-Allow-Origin', req.headers.origin);
  res.setHeader('Access-Control-Allow-Methods', 'GET,OPTIONS,PATCH,DELETE,POST,PUT')
  res.setHeader(
    'Access-Control-Allow-Headers',
    'X-CSRF-Token, X-Requested-With, Accept, Accept-Version, Content-Length, Content-MD5, Content-Type, Date, X-Api-Version'
  )

  const weekData = Object.entries(PRICES_DATA).reduce((acc, [id, snaps]) => {
    const weekPrices = {}

    const now = Date.now()

    Object.entries(snaps as Object).forEach(([time, price]) => {
      if ((now - +time) / (1000 * 60 * 60 * 24) < 8) {
        weekPrices[time] = price
      }
    })
    return {
      ...acc,
      [id]: weekPrices
    }
  }, {})

  res.json(weekData)
}
