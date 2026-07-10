use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::Amount;
use crate::error::{TempestError, TempestResult};
use crate::ids::{AccountId, AssetId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AccountRole {
    Trader,
    Solver,
    Integrator,
    Reserve,
    Treasury,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    id: AccountId,
    role: AccountRole,
    balances: BTreeMap<AssetId, Amount>,
    exposure_limit: Amount,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AccountBook {
    accounts: BTreeMap<AccountId, Account>,
}

impl Account {
    pub fn new(id: impl Into<AccountId>, role: AccountRole, exposure_limit: Amount) -> Self {
        Self {
            id: id.into(),
            role,
            balances: BTreeMap::new(),
            exposure_limit,
        }
    }

    pub fn id(&self) -> &AccountId {
        &self.id
    }

    pub fn role(&self) -> &AccountRole {
        &self.role
    }

    pub fn balance(&self, asset: &AssetId) -> Amount {
        self.balances.get(asset).copied().unwrap_or(Amount::ZERO)
    }

    pub fn exposure_limit(&self) -> Amount {
        self.exposure_limit
    }

    pub fn credit(&mut self, asset: &AssetId, amount: Amount) -> TempestResult<()> {
        let current = self.balance(asset);
        self.balances
            .insert(asset.clone(), current.checked_add(amount)?);
        Ok(())
    }

    pub fn debit(&mut self, asset: &AssetId, amount: Amount) -> TempestResult<()> {
        let current = self.balance(asset);
        if current < amount {
            return Err(TempestError::InsufficientBalance {
                account: self.id.to_string(),
                asset: asset.to_string(),
            });
        }
        self.balances
            .insert(asset.clone(), current.checked_sub(amount)?);
        Ok(())
    }

    pub fn balances(&self) -> &BTreeMap<AssetId, Amount> {
        &self.balances
    }
}

impl AccountBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, account: Account) {
        self.accounts.insert(account.id.clone(), account);
    }

    pub fn get(&self, id: &AccountId) -> TempestResult<&Account> {
        self.accounts
            .get(id)
            .ok_or_else(|| TempestError::AccountNotFound(id.to_string()))
    }

    pub fn get_mut(&mut self, id: &AccountId) -> TempestResult<&mut Account> {
        self.accounts
            .get_mut(id)
            .ok_or_else(|| TempestError::AccountNotFound(id.to_string()))
    }

    pub fn credit(
        &mut self,
        account: &AccountId,
        asset: &AssetId,
        amount: Amount,
    ) -> TempestResult<()> {
        self.get_mut(account)?.credit(asset, amount)
    }

    pub fn debit(
        &mut self,
        account: &AccountId,
        asset: &AssetId,
        amount: Amount,
    ) -> TempestResult<()> {
        self.get_mut(account)?.debit(asset, amount)
    }

    pub fn transfer(
        &mut self,
        from: &AccountId,
        to: &AccountId,
        asset: &AssetId,
        amount: Amount,
    ) -> TempestResult<()> {
        self.debit(from, asset, amount)?;
        self.credit(to, asset, amount)?;
        Ok(())
    }

    pub fn balance(&self, account: &AccountId, asset: &AssetId) -> TempestResult<Amount> {
        Ok(self.get(account)?.balance(asset))
    }

    pub fn account_ids(&self) -> Vec<AccountId> {
        self.accounts.keys().cloned().collect()
    }

    pub fn balances_for_asset(&self, asset: &AssetId) -> BTreeMap<String, i128> {
        self.accounts
            .iter()
            .map(|(account_id, account)| (account_id.to_string(), account.balance(asset).raw()))
            .collect()
    }

    pub fn total_for_asset(&self, asset: &AssetId) -> Amount {
        self.accounts
            .values()
            .fold(Amount::ZERO, |acc, account| acc + account.balance(asset))
    }
}
