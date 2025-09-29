#![allow(unreachable_patterns)]

use std::net::{Ipv4Addr, Ipv6Addr};

use futures::TryStreamExt;

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};

pub(crate) type Client = AsyncWorldClient<RtnlAddressRequest, RtnlAddressResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlAddressRequest, RtnlAddressResponse>;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlAddressRequest {
    Ipv4AddrsGet {
        if_id: u32,
    },
    Ipv6AddrsGet {
        if_id: u32,
    },
    Ipv4AddrSet {
        prefix: crate::Ipv4Net,
        if_id: u32,
    },
    Ipv6AddrSet {
        prefix: crate::Ipv6Net,
        if_id: u32,
    },
    Ipv4AddrDel {
        prefix: crate::Ipv4Net,
        if_id: u32,
    },
    Ipv6AddrDel {
        prefix: crate::Ipv6Net,
        if_id: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlAddressResponse {
    Success,
    Failed,
    NotImplemented,
    NotFound,
    Ipv4Addrs(Vec<Ipv4Addr>),
    Ipv6Addrs(Vec<Ipv6Addr>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RtnlAddressClient {
    client: Client,
}

impl RtnlAddressClient {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
        }
    }

    pub fn ipv4_addrs_get(&self, if_id: Option<u32>) -> std::io::Result<Vec<Ipv4Addr>> {
        let res = self.client.send_request(RtnlAddressRequest::Ipv4AddrsGet { if_id: if_id.unwrap_or(0) })?;
        match res {
            RtnlAddressResponse::Ipv4Addrs(addrs) => {
                return Ok(addrs);
            },
            _ => {},
        }
        Err(std::io::Error::other("Failed to get IPv4 addresses"))
    }

    pub fn ipv6_addrs_get(&self, if_id: Option<u32>) -> std::io::Result<Vec<Ipv6Addr>> {
        let res = self.client.send_request(RtnlAddressRequest::Ipv6AddrsGet { if_id: if_id.unwrap_or(0) })?;
        match res {
            RtnlAddressResponse::Ipv6Addrs(addrs) => {
                return Ok(addrs);
            },
            _ => {},
        }
        Err(std::io::Error::other("Failed to get IPv6 addresses"))
    }
}

pub(crate) async fn run_server(mut server: Server, handle: rtnetlink::AddressHandle) {
    while let Some((req, respond)) = server.accept().await {
        match req {
            RtnlAddressRequest::Ipv4AddrsGet { if_id } => {
                let if_index = if_id;
                let mut addrs = Vec::new();
                let mut req = handle.get();
                if if_index != 0 {
                    req = req.set_link_index_filter(if_index);
                }
                let response = req.execute();

                futures::pin_mut!(response);
                while let Ok(Some(response)) = response.try_next().await {
                    if response.header.family != netlink_packet_route::AddressFamily::Inet {
                        continue;
                    }
                    for addr in response.attributes.iter() {
                        if let netlink_packet_route::address::AddressAttribute::Address(std::net::IpAddr::V4(addr)) = addr {
                            addrs.push(*addr);
                        }
                    }
                }
                respond(RtnlAddressResponse::Ipv4Addrs(addrs));
            },
            RtnlAddressRequest::Ipv6AddrsGet { if_id } => {
                let if_index = if_id;
                let mut addrs = Vec::new();
                let mut req = handle.get();
                if if_index != 0 {
                    req = req.set_link_index_filter(if_index);
                }
                let response = req.execute();

                futures::pin_mut!(response);
                while let Ok(Some(response)) = response.try_next().await {
                    if response.header.family != netlink_packet_route::AddressFamily::Inet6 {
                        continue;
                    }
                    for addr in response.attributes.iter() {
                        if let netlink_packet_route::address::AddressAttribute::Address(std::net::IpAddr::V6(addr)) = addr {
                            addrs.push(*addr);
                        }
                    }
                }
                respond(RtnlAddressResponse::Ipv6Addrs(addrs));
            },
            _ => respond(RtnlAddressResponse::NotImplemented),
        }
    }
}
