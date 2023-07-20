use cosmwasm_std::{dynamic_link, Addr, Contract};

#[derive(Contract)]
pub struct NumberContract {
    pub address: Addr,
}

#[dynamic_link(NumberContract)]
pub trait Number: Contract {
    fn add(&self, by: i32);
    fn sub(&self, by: i32);
    fn mul(&self, by: i32);
    fn number(&self) -> i32;
}
