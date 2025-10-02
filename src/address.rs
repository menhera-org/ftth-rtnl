#![allow(unreachable_patterns)]

use std::io::{self, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use futures::TryStreamExt;

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};
use netlink_packet_route::{
    AddressFamily,
    address::{AddressAttribute, AddressMessage},
};

pub(crate) type Client = AsyncWorldClient<RtnlAddressRequest, RtnlAddressResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlAddressRequest, RtnlAddressResponse>;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlAddressRequest {
    Ipv4AddrsGet { if_id: u32 },
    Ipv6AddrsGet { if_id: u32 },
    Ipv4AddrSet { prefix: crate::Ipv4Net, if_id: u32 },
    Ipv6AddrSet { prefix: crate::Ipv6Net, if_id: u32 },
    Ipv4AddrDel { prefix: crate::Ipv4Net, if_id: u32 },
    Ipv6AddrDel { prefix: crate::Ipv6Net, if_id: u32 },
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
        Self { client }
    }

    pub fn ipv4_addrs_get(&self, if_id: Option<u32>) -> std::io::Result<Vec<Ipv4Addr>> {
        let res = self.client.send_request(RtnlAddressRequest::Ipv4AddrsGet {
            if_id: if_id.unwrap_or(0),
        })?;
        match res {
            RtnlAddressResponse::Ipv4Addrs(addrs) => {
                return Ok(addrs);
            }
            _ => {}
        }
        Err(std::io::Error::other("Failed to get IPv4 addresses"))
    }

    pub fn ipv6_addrs_get(&self, if_id: Option<u32>) -> std::io::Result<Vec<Ipv6Addr>> {
        let res = self.client.send_request(RtnlAddressRequest::Ipv6AddrsGet {
            if_id: if_id.unwrap_or(0),
        })?;
        match res {
            RtnlAddressResponse::Ipv6Addrs(addrs) => {
                return Ok(addrs);
            }
            _ => {}
        }
        Err(std::io::Error::other("Failed to get IPv6 addresses"))
    }

    pub fn ipv4_addr_set(&self, if_id: u32, prefix: crate::Ipv4Net) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlAddressRequest::Ipv4AddrSet { prefix, if_id })?;
        handle_basic_response("IPv4 address set", res, false)
    }

    pub fn ipv6_addr_set(&self, if_id: u32, prefix: crate::Ipv6Net) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlAddressRequest::Ipv6AddrSet { prefix, if_id })?;
        handle_basic_response("IPv6 address set", res, false)
    }

    pub fn ipv4_addr_del(&self, if_id: u32, prefix: crate::Ipv4Net) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlAddressRequest::Ipv4AddrDel { prefix, if_id })?;
        handle_basic_response("IPv4 address delete", res, true)
    }

    pub fn ipv6_addr_del(&self, if_id: u32, prefix: crate::Ipv6Net) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlAddressRequest::Ipv6AddrDel { prefix, if_id })?;
        handle_basic_response("IPv6 address delete", res, true)
    }
}

fn build_ipv4_address_message(prefix: &crate::Ipv4Net, if_id: u32) -> AddressMessage {
    let mut message = AddressMessage::default();
    message.header.family = AddressFamily::Inet;
    message.header.index = if_id;
    message.header.prefix_len = prefix.prefix_len();

    let addr = prefix.addr();
    if !addr.is_multicast() {
        message
            .attributes
            .push(AddressAttribute::Address(addr.into()));
        message
            .attributes
            .push(AddressAttribute::Local(addr.into()));

        let broadcast = if prefix.prefix_len() == 32 {
            addr
        } else {
            let host_bits = 0xffff_ffff_u32 >> u32::from(prefix.prefix_len());
            let ip_addr = u32::from(addr);
            Ipv4Addr::from(ip_addr | host_bits)
        };
        message
            .attributes
            .push(AddressAttribute::Broadcast(broadcast));
    }

    message
}

fn build_ipv6_address_message(prefix: &crate::Ipv6Net, if_id: u32) -> AddressMessage {
    let mut message = AddressMessage::default();
    message.header.family = AddressFamily::Inet6;
    message.header.index = if_id;
    message.header.prefix_len = prefix.prefix_len();

    let addr = prefix.addr();
    if addr.is_multicast() {
        message.attributes.push(AddressAttribute::Multicast(addr));
    } else {
        message
            .attributes
            .push(AddressAttribute::Address(addr.into()));
        message
            .attributes
            .push(AddressAttribute::Local(addr.into()));
    }

    message
}

fn handle_basic_response(
    operation: &str,
    response: RtnlAddressResponse,
    is_delete: bool,
) -> io::Result<()> {
    match response {
        RtnlAddressResponse::Success => Ok(()),
        RtnlAddressResponse::Failed => {
            Err(io::Error::other(format!("{} request failed", operation)))
        }
        RtnlAddressResponse::NotImplemented => Err(io::Error::new(
            ErrorKind::Unsupported,
            format!("{} request is not implemented", operation),
        )),
        RtnlAddressResponse::NotFound => Err(io::Error::new(
            ErrorKind::NotFound,
            if is_delete {
                format!("{} target not found", operation)
            } else {
                format!("{} not found", operation)
            },
        )),
        unexpected => Err(io::Error::other(format!(
            "{} returned unexpected response: {:?}",
            operation, unexpected
        ))),
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
                        if let netlink_packet_route::address::AddressAttribute::Address(
                            std::net::IpAddr::V4(addr),
                        ) = addr
                        {
                            addrs.push(*addr);
                        }
                    }
                }
                respond(RtnlAddressResponse::Ipv4Addrs(addrs));
            }
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
                        if let netlink_packet_route::address::AddressAttribute::Address(
                            std::net::IpAddr::V6(addr),
                        ) = addr
                        {
                            addrs.push(*addr);
                        }
                    }
                }
                respond(RtnlAddressResponse::Ipv6Addrs(addrs));
            }
            RtnlAddressRequest::Ipv4AddrSet { prefix, if_id } => {
                if if_id == 0 {
                    respond(RtnlAddressResponse::Failed);
                    continue;
                }

                let addr = prefix.addr();
                let prefix_len = prefix.prefix_len();
                let result = handle
                    .add(if_id, IpAddr::V4(addr), prefix_len)
                    .execute()
                    .await;

                match result {
                    Ok(()) => respond(RtnlAddressResponse::Success),
                    Err(rtnetlink::Error::NetlinkError(err_msg))
                        if err_msg.to_io().kind() == ErrorKind::AlreadyExists =>
                    {
                        respond(RtnlAddressResponse::Success);
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to add IPv4 address {}/{} on ifindex {}: {}",
                            addr,
                            prefix_len,
                            if_id,
                            err,
                        );
                        respond(RtnlAddressResponse::Failed);
                    }
                }
            }
            RtnlAddressRequest::Ipv6AddrSet { prefix, if_id } => {
                if if_id == 0 {
                    respond(RtnlAddressResponse::Failed);
                    continue;
                }

                let addr = prefix.addr();
                let prefix_len = prefix.prefix_len();
                let result = handle
                    .add(if_id, IpAddr::V6(addr), prefix_len)
                    .execute()
                    .await;

                match result {
                    Ok(()) => respond(RtnlAddressResponse::Success),
                    Err(rtnetlink::Error::NetlinkError(err_msg))
                        if err_msg.to_io().kind() == ErrorKind::AlreadyExists =>
                    {
                        respond(RtnlAddressResponse::Success);
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to add IPv6 address {}/{} on ifindex {}: {}",
                            addr,
                            prefix_len,
                            if_id,
                            err,
                        );
                        respond(RtnlAddressResponse::Failed);
                    }
                }
            }
            RtnlAddressRequest::Ipv4AddrDel { prefix, if_id } => {
                if if_id == 0 {
                    respond(RtnlAddressResponse::Failed);
                    continue;
                }

                let addr = prefix.addr();
                let prefix_len = prefix.prefix_len();
                let message = build_ipv4_address_message(&prefix, if_id);

                let result = handle.del(message).execute().await;

                match result {
                    Ok(()) => respond(RtnlAddressResponse::Success),
                    Err(rtnetlink::Error::NetlinkError(err_msg)) => {
                        let io_err = err_msg.to_io();
                        if matches!(
                            io_err.kind(),
                            ErrorKind::AddrNotAvailable | ErrorKind::NotFound
                        ) {
                            respond(RtnlAddressResponse::NotFound);
                        } else {
                            log::warn!(
                                "Failed to delete IPv4 address {}/{} on ifindex {}: {}",
                                addr,
                                prefix_len,
                                if_id,
                                err_msg,
                            );
                            respond(RtnlAddressResponse::Failed);
                        }
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to delete IPv4 address {}/{} on ifindex {}: {}",
                            addr,
                            prefix_len,
                            if_id,
                            err,
                        );
                        respond(RtnlAddressResponse::Failed);
                    }
                }
            }
            RtnlAddressRequest::Ipv6AddrDel { prefix, if_id } => {
                if if_id == 0 {
                    respond(RtnlAddressResponse::Failed);
                    continue;
                }

                let addr = prefix.addr();
                let prefix_len = prefix.prefix_len();
                let message = build_ipv6_address_message(&prefix, if_id);

                let result = handle.del(message).execute().await;

                match result {
                    Ok(()) => respond(RtnlAddressResponse::Success),
                    Err(rtnetlink::Error::NetlinkError(err_msg)) => {
                        let io_err = err_msg.to_io();
                        if matches!(
                            io_err.kind(),
                            ErrorKind::AddrNotAvailable | ErrorKind::NotFound
                        ) {
                            respond(RtnlAddressResponse::NotFound);
                        } else {
                            log::warn!(
                                "Failed to delete IPv6 address {}/{} on ifindex {}: {}",
                                addr,
                                prefix_len,
                                if_id,
                                err_msg,
                            );
                            respond(RtnlAddressResponse::Failed);
                        }
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to delete IPv6 address {}/{} on ifindex {}: {}",
                            addr,
                            prefix_len,
                            if_id,
                            err,
                        );
                        respond(RtnlAddressResponse::Failed);
                    }
                }
            }
            _ => respond(RtnlAddressResponse::NotImplemented),
        }
    }
}
