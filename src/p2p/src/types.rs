use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub topic: TopicId,
    pub endpoints: Vec<EndpointAddr>,
}

impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = postcard::to_stdvec(self).map_err(|_| fmt::Error)?;
        let text = bs58::encode(bytes).into_string();
        write!(f, "{}", text)
    }
}

impl FromStr for Ticket {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s).into_vec()?;
        let ticket = postcard::from_bytes(&bytes)?;
        Ok(ticket)
    }
}

pub struct Address(EndpointAddr);

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = postcard::to_stdvec(&self.0).map_err(|_| fmt::Error)?;
        let text = bs58::encode(bytes).into_string();
        write!(f, "{}", text)
    }
}

impl FromStr for Address {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s).into_vec()?;
        let addr = postcard::from_bytes(&bytes)?;
        Ok(Address(addr))
    }
}

impl From<EndpointAddr> for Address {
    fn from(addr: EndpointAddr) -> Self {
        Address(addr)
    }
}

impl From<Address> for EndpointAddr {
    fn from(address: Address) -> Self {
        address.0
    }
}
