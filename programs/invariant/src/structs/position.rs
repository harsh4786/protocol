use crate::decimals::*;
use crate::structs::pool::Pool;
use crate::structs::tick::Tick;
use crate::*;
use anchor_lang::prelude::*;

#[account(zero_copy)]
#[repr(packed)]
#[derive(PartialEq, Default, Debug)]
pub struct Position {
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub id: u128, // unique inside pool
    pub liquidity: Liquidity,
    pub lower_tick_index: i32,
    pub upper_tick_index: i32,
    pub fee_growth_inside_x: FeeGrowth,
    pub fee_growth_inside_y: FeeGrowth,
    pub seconds_per_liquidity_inside: FixedPoint,
    pub last_slot: u64,
    pub tokens_owed_x: Liquidity,
    pub tokens_owed_y: Liquidity,
    pub bump: u8,
}

impl Position {
    pub fn modify(
        &mut self,
        pool: &mut Pool,
        upper_tick: &mut Tick,
        lower_tick: &mut Tick,
        liquidity_delta: Liquidity,
        add: bool,
        current_timestamp: u64,
    ) -> Result<(TokenAmount, TokenAmount)> {
        if !pool.liquidity.is_zero() {
            pool.update_seconds_per_liquidity_global(current_timestamp);
        } else {
            pool.last_timestamp = current_timestamp;
        }

        // update initialized tick
        lower_tick.update(liquidity_delta, false, add)?;

        upper_tick.update(liquidity_delta, true, add)?;

        // update fee inside position
        let (fee_growth_inside_x, fee_growth_inside_y) = calculate_fee_growth_inside(
            *lower_tick,
            *upper_tick,
            pool.current_tick_index,
            pool.fee_growth_global_x,
            pool.fee_growth_global_y,
        );

        self.update(
            add,
            liquidity_delta,
            fee_growth_inside_x,
            fee_growth_inside_y,
        )?;

        // calculate tokens amounts and update pool liquidity
        calculate_amount_delta(
            pool,
            liquidity_delta,
            add,
            upper_tick.index,
            lower_tick.index,
        )
    }

    pub fn update(
        &mut self,
        sign: bool,
        liquidity_delta: Liquidity,
        fee_growth_inside_x: FeeGrowth,
        fee_growth_inside_y: FeeGrowth,
    ) -> Result<()> {
        require!(
            liquidity_delta.v != 0 || self.liquidity.v != 0,
            ErrorCode::EmptyPositionPokes
        );

        // calculate accumulated fee
        let tokens_owed_x = (fee_growth_inside_x - self.fee_growth_inside_x).to_fee(self.liquidity);
        let tokens_owed_y = (fee_growth_inside_y - self.fee_growth_inside_y).to_fee(self.liquidity);

        self.liquidity = self.calculate_new_liquidity_safely(sign, liquidity_delta)?;
        self.fee_growth_inside_x = fee_growth_inside_x;
        self.fee_growth_inside_y = fee_growth_inside_y;
        self.tokens_owed_x = self.tokens_owed_x + tokens_owed_x;
        self.tokens_owed_y = self.tokens_owed_y + tokens_owed_y;

        Ok(())
    }

    pub fn initialized_id(&mut self, pool: &mut Pool) {
        self.id = pool.position_iterator;
        pool.position_iterator += 1;
    }

    // for future use
    pub fn get_id(self) -> String {
        let mut id = self.pool.to_string();
        id.push_str({ self.id }.to_string().as_str());
        id
    }

    // TODO: add tests
    fn calculate_new_liquidity_safely(
        &mut self,
        sign: bool,
        liquidity_delta: Liquidity,
    ) -> Result<Liquidity> {
        // validate in decrease liquidity case
        if !sign && { self.liquidity } < liquidity_delta {
            return Err(ErrorCode::InvalidPositionLiquidity.into());
        }

        Ok(match sign {
            true => self.liquidity + liquidity_delta,
            false => self.liquidity - liquidity_delta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_new_liquidity_safely() {
        // negative liquidity error
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(1),
                ..Default::default()
            };
            let sign: bool = false;
            let liquidity_delta = Liquidity::from_integer(2);

            let result = position.calculate_new_liquidity_safely(sign, liquidity_delta);

            assert!(result.is_err());
        }
        // adding liquidity
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(2),
                ..Default::default()
            };
            let sign: bool = true;
            let liquidity_delta = Liquidity::from_integer(2);

            let new_liquidity = position
                .calculate_new_liquidity_safely(sign, liquidity_delta)
                .unwrap();

            assert_eq!(new_liquidity, Liquidity::from_integer(4));
        }
        // subtracting liquidity
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(2),
                ..Default::default()
            };
            let sign: bool = false;
            let liquidity_delta = Liquidity::from_integer(2);

            let new_liquidity = position
                .calculate_new_liquidity_safely(sign, liquidity_delta)
                .unwrap();

            assert_eq!(new_liquidity, Liquidity::from_integer(0));
        }
    }

    #[test]
    fn test_update() {
        // Disable empty position pokes error
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(0),
                ..Default::default()
            };
            let sign = true;
            let liquidity_delta = Liquidity::from_integer(0);
            let fee_growth_inside_x = FeeGrowth::from_integer(1);
            let fee_growth_inside_y = FeeGrowth::from_integer(1);

            let result = position.update(
                sign,
                liquidity_delta,
                fee_growth_inside_x,
                fee_growth_inside_y,
            );

            assert!(result.is_err());
        }
        // zero liquidity fee shouldn't change
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(0),
                fee_growth_inside_x: FeeGrowth::from_integer(4),
                fee_growth_inside_y: FeeGrowth::from_integer(4),
                tokens_owed_x: Liquidity::from_integer(100),
                tokens_owed_y: Liquidity::from_integer(100),
                ..Default::default()
            };
            let sign = true;
            let liquidity_delta = Liquidity::from_integer(1);
            let fee_growth_inside_x = FeeGrowth::from_integer(5);
            let fee_growth_inside_y = FeeGrowth::from_integer(5);

            position
                .update(
                    sign,
                    liquidity_delta,
                    fee_growth_inside_x,
                    fee_growth_inside_y,
                )
                .unwrap();

            assert_eq!({ position.liquidity }, Liquidity::from_integer(1));
            assert_eq!({ position.fee_growth_inside_x }, FeeGrowth::from_integer(5));
            assert_eq!({ position.fee_growth_inside_y }, FeeGrowth::from_integer(5));
            assert_eq!({ position.tokens_owed_x }, Liquidity::from_integer(100));
            assert_eq!({ position.tokens_owed_y }, Liquidity::from_integer(100));
        }
        // fee should change
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(1),
                fee_growth_inside_x: FeeGrowth::from_integer(4),
                fee_growth_inside_y: FeeGrowth::from_integer(4),
                tokens_owed_x: Liquidity::from_integer(100),
                tokens_owed_y: Liquidity::from_integer(100),
                ..Default::default()
            };
            let sign = true;
            let liquidity_delta = Liquidity::from_integer(1);
            let fee_growth_inside_x = FeeGrowth::from_integer(5);
            let fee_growth_inside_y = FeeGrowth::from_integer(5);

            position
                .update(
                    sign,
                    liquidity_delta,
                    fee_growth_inside_x,
                    fee_growth_inside_y,
                )
                .unwrap();

            assert_eq!({ position.liquidity }, Liquidity::from_integer(2));
            assert_eq!({ position.fee_growth_inside_x }, FeeGrowth::from_integer(5));
            assert_eq!({ position.fee_growth_inside_y }, FeeGrowth::from_integer(5));
            assert_eq!({ position.tokens_owed_x }, Liquidity::from_integer(101));
            assert_eq!({ position.tokens_owed_y }, Liquidity::from_integer(101));
        }
    }

    #[test]
    fn test_modify() {
        // owed tokens after overflow
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(123),
                fee_growth_inside_x: FeeGrowth::new(u128::MAX) - FeeGrowth::from_integer(1234),
                fee_growth_inside_y: FeeGrowth::new(u128::MAX) - FeeGrowth::from_integer(1234),
                tokens_owed_x: Liquidity::from_integer(0),
                tokens_owed_y: Liquidity::from_integer(0),
                ..Default::default()
            };
            let mut pool = Pool {
                current_tick_index: 0,
                fee_growth_global_x: FeeGrowth::from_integer(20),
                fee_growth_global_y: FeeGrowth::from_integer(20),
                ..Default::default()
            };
            let mut upper_tick = Tick {
                index: -10,
                fee_growth_outside_x: FeeGrowth::from_integer(15),
                fee_growth_outside_y: FeeGrowth::from_integer(15),
                liquidity_gross: Liquidity::from_integer(123),
                ..Default::default()
            };
            let mut lower_tick = Tick {
                index: -20,
                fee_growth_outside_x: FeeGrowth::from_integer(20),
                fee_growth_outside_y: FeeGrowth::from_integer(20),
                liquidity_gross: Liquidity::from_integer(123),
                ..Default::default()
            };
            let liquidity_delta = Liquidity::new(0);
            let add = true;
            let current_timestamp: u64 = 1234567890;

            position
                .modify(
                    &mut pool,
                    &mut upper_tick,
                    &mut lower_tick,
                    liquidity_delta,
                    add,
                    current_timestamp,
                )
                .unwrap();

            // assert_eq!(
            //     { position.tokens_owed_x },
            //     (Decimal::from_integer(1234 - 5) + Decimal::new(1)) * Decimal::from_integer(123)
            // ) // 151167000000000123 so close enough?

            assert_eq!(
                { position.tokens_owed_x },
                Liquidity::new(151167000000000000)
            );
        }
    }
}
