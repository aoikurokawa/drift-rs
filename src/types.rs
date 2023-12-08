use std::cmp::Ordering;

// re-export types in public API
pub use drift_program::{
    controller::position::PositionDirection,
    state::{
        order_params::{OrderParams, PostOnlyParam},
        user::{MarketType, Order, OrderType, PerpPosition, SpotPosition},
    },
};
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

use crate::constants::{perp_markets, spot_markets};

/// Drift program context
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Context {
    /// Target Devenet
    Dev,
    /// Target Mainnet
    Mainnet,
}

/// Id of a Drift market
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct MarketId {
    pub(crate) index: u16,
    pub(crate) kind: MarketType,
}

impl MarketId {
    /// Lookup a market id by context and symbol
    ///
    /// This operation is not free so lookups should be reused/cached by the caller
    ///
    /// Returns an error if symbol and context do not map to a known market
    pub fn lookup(context: Context, symbol: &str) -> Result<Self, ()> {
        let mut parts = symbol.split('-');
        match (parts.next(), parts.next()) {
            (Some(base), None) => {
                let markets = spot_markets(context);
                if let Some(market) = markets.iter().find(|m| m.symbol.eq_ignore_ascii_case(base)) {
                    return Ok(MarketId::spot(market.market_index));
                }
            }
            (Some(base), Some(perp)) => {
                if perp.eq_ignore_ascii_case("perp") {
                    let markets = perp_markets(context);
                    if let Some(market) = markets
                        .iter()
                        .find(|m| m.base_asset_symbol.eq_ignore_ascii_case(base))
                    {
                        return Ok(MarketId::perp(market.market_index));
                    }
                }
            }
            _ => (),
        }

        Err(())
    }
    /// Id of a perp market
    pub const fn perp(index: u16) -> Self {
        Self {
            index,
            kind: MarketType::Perp,
        }
    }
    /// Id of a spot market
    pub const fn spot(index: u16) -> Self {
        Self {
            index,
            kind: MarketType::Spot,
        }
    }
}

impl From<(u16, MarketType)> for MarketId {
    fn from(value: (u16, MarketType)) -> Self {
        Self {
            index: value.0,
            kind: value.1,
        }
    }
}

/// Provides builder API for Orders
#[derive(Default)]
pub struct NewOrder {
    order_type: OrderType,
    direction: PositionDirection,
    reduce_only: bool,
    market_id: MarketId,
    post_only: bool,
    ioc: bool,
    amount: u64,
    price: u64,
}

impl NewOrder {
    /// Create a market order
    pub fn market(market_id: MarketId) -> Self {
        Self {
            order_type: OrderType::Market,
            market_id,
            ..Default::default()
        }
    }
    /// Create a limit order
    pub fn limit(market_id: MarketId) -> Self {
        Self {
            order_type: OrderType::Limit,
            market_id,
            ..Default::default()
        }
    }
    /// Set order amount
    ///
    /// A sub-zero amount indicates a short
    pub fn amount(mut self, amount: i64) -> Self {
        self.direction = if amount >= 0 {
            PositionDirection::Long
        } else {
            PositionDirection::Short
        };
        self.amount = amount.unsigned_abs();

        self
    }
    /// Set order price
    pub fn price(mut self, price: u64) -> Self {
        self.price = price;
        self
    }
    /// Set reduce only (default: false)
    pub fn reduce_only(mut self, flag: bool) -> Self {
        self.reduce_only = flag;
        self
    }
    /// Set immediate or cancel (default: false)
    pub fn ioc(mut self, flag: bool) -> Self {
        self.ioc = flag;
        self
    }
    /// Set post-only (default: false)
    pub fn post_only(mut self, flag: bool) -> Self {
        self.post_only = flag; // TODO: map the other variants
        self
    }
    /// Call to complete building the Order
    pub fn build(self) -> OrderParams {
        OrderParams {
            order_type: self.order_type,
            market_index: self.market_id.index,
            market_type: self.market_id.kind,
            price: self.price,
            base_asset_amount: self.amount,
            reduce_only: self.reduce_only,
            direction: self.direction,
            immediate_or_cancel: self.ioc,
            post_only: if self.post_only {
                PostOnlyParam::TryPostOnly
            } else {
                PostOnlyParam::None
            },
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub enum SdkError {
    Rpc(solana_client::client_error::ClientError),
    Ws(solana_client::nonblocking::pubsub_client::PubsubClientError),
    InvalidAccount,
}

impl From<solana_client::client_error::ClientError> for SdkError {
    fn from(value: solana_client::client_error::ClientError) -> Self {
        Self::Rpc(value)
    }
}

impl From<solana_client::nonblocking::pubsub_client::PubsubClientError> for SdkError {
    fn from(value: solana_client::nonblocking::pubsub_client::PubsubClientError) -> Self {
        Self::Ws(value)
    }
}

#[cfg(test)]
mod test {
    use super::{Context, MarketId};

    #[test]
    fn market_id_lookups() {
        for (context, symbol, expected) in &[
            (Context::Dev, "wBTC", MarketId::spot(2)),
            (Context::Dev, "SOL", MarketId::spot(1)),
            (Context::Dev, "sol-perp", MarketId::perp(0)),
            (Context::Mainnet, "wbtc", MarketId::spot(3)),
            (Context::Mainnet, "SOL", MarketId::spot(1)),
            (Context::Mainnet, "sol-perp", MarketId::perp(0)),
            (Context::Mainnet, "eth-perp", MarketId::perp(2)),
        ] {
            dbg!(context, symbol);
            assert_eq!(MarketId::lookup(*context, symbol).unwrap(), *expected,);
        }

        for (context, symbol) in &[
            (Context::Mainnet, "market404"),
            (Context::Mainnet, "market404-perp"),
            (Context::Mainnet, "market404-something"),
        ] {
            assert!(MarketId::lookup(*context, symbol).is_err())
        }
    }
}

/// Helper type for Accounts included in drift instructions
///
/// Provides sorting implementation matching drift program
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord)]
#[repr(u8)]
pub(crate) enum AccountType {
    Oracle { pubkey: Pubkey },
    Spot { pubkey: Pubkey, writable: bool },
    Perp { pubkey: Pubkey, writable: bool },
}

impl AccountType {
    fn pubkey(&self) -> &Pubkey {
        match self {
            Self::Oracle { pubkey } => pubkey,
            Self::Spot { pubkey, .. } => pubkey,
            Self::Perp { pubkey, .. } => pubkey,
        }
    }
    fn parts(self) -> (Pubkey, bool) {
        match self {
            Self::Oracle { pubkey } => (pubkey, false),
            Self::Spot {
                pubkey, writable, ..
            } => (pubkey, writable),
            Self::Perp {
                pubkey, writable, ..
            } => (pubkey, writable),
        }
    }
    fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
}

impl PartialOrd for AccountType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let type_order = self.discriminant().cmp(&other.discriminant());
        if let Ordering::Equal = type_order {
            self.pubkey().partial_cmp(other.pubkey())
        } else {
            Some(type_order)
        }
    }
}

impl From<AccountType> for AccountMeta {
    fn from(value: AccountType) -> Self {
        let (pubkey, is_writable) = value.parts();
        AccountMeta {
            pubkey,
            is_writable,
            is_signer: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::pubkey::Pubkey;

    use super::AccountType;

    #[test]
    fn account_type_sorting() {
        let mut accounts = vec![
            AccountType::Perp {
                pubkey: Pubkey::new_from_array([4_u8; 32]),
                writable: false,
            },
            AccountType::Oracle {
                pubkey: Pubkey::new_from_array([2_u8; 32]),
            },
            AccountType::Oracle {
                pubkey: Pubkey::new_from_array([1_u8; 32]),
            },
            AccountType::Spot {
                pubkey: Pubkey::new_from_array([3_u8; 32]),
                writable: true,
            },
        ];
        accounts.sort();

        assert_eq!(
            accounts,
            vec![
                AccountType::Oracle {
                    pubkey: Pubkey::new_from_array([1_u8; 32])
                },
                AccountType::Oracle {
                    pubkey: Pubkey::new_from_array([2_u8; 32])
                },
                AccountType::Spot {
                    pubkey: Pubkey::new_from_array([3_u8; 32]),
                    writable: true
                },
                AccountType::Perp {
                    pubkey: Pubkey::new_from_array([4_u8; 32]),
                    writable: false
                },
            ]
        )
    }
}
