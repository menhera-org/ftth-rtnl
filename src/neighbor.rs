#![allow(unreachable_patterns)]

use std::io::{self, ErrorKind};
use std::net::IpAddr;

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};
use log::warn;
use netlink_packet_route::neighbour::{NeighbourAddress, NeighbourAttribute, NeighbourMessage};
use netlink_packet_route::{AddressFamily, route::RouteType};

pub use netlink_packet_route::neighbour::{NeighbourFlags, NeighbourState};

pub(crate) type Client = AsyncWorldClient<RtnlNeighborRequest, RtnlNeighborResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlNeighborRequest, RtnlNeighborResponse>;

#[derive(Debug, Clone, PartialEq)]
pub struct NeighborEntry {
    pub if_id: u32,
    pub destination: IpAddr,
    pub link_address: Option<Vec<u8>>,
    pub state: Option<NeighbourState>,
    pub flags: Option<NeighbourFlags>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeighborDelete {
    pub if_id: u32,
    pub destination: IpAddr,
    pub link_address: Option<Vec<u8>>,
    pub state: Option<NeighbourState>,
    pub flags: Option<NeighbourFlags>,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlNeighborRequest {
    Add(NeighborEntry),
    Change(NeighborEntry),
    Delete(NeighborDelete),
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlNeighborResponse {
    Success,
    Failed,
    NotImplemented,
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RtnlNeighborClient {
    client: Client,
}

impl RtnlNeighborClient {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn add(&self, entry: NeighborEntry) -> io::Result<()> {
        let res = self.client.send_request(RtnlNeighborRequest::Add(entry))?;
        handle_neighbor_response("Neighbor add", res, false)
    }

    pub fn change(&self, entry: NeighborEntry) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlNeighborRequest::Change(entry))?;
        handle_neighbor_response("Neighbor change", res, false)
    }

    pub fn delete(&self, entry: NeighborDelete) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlNeighborRequest::Delete(entry))?;
        handle_neighbor_response("Neighbor delete", res, false)
    }
}

pub(crate) async fn run_server(mut server: Server, handle: rtnetlink::NeighbourHandle) {
    while let Some((req, respond)) = server.accept().await {
        let response = match req {
            RtnlNeighborRequest::Add(entry) => add_or_change_neighbor(&handle, entry, false).await,
            RtnlNeighborRequest::Change(entry) => {
                add_or_change_neighbor(&handle, entry, true).await
            }
            RtnlNeighborRequest::Delete(entry) => delete_neighbor(&handle, entry).await,
        };
        respond(response);
    }
}

fn handle_neighbor_response(
    operation: &str,
    response: RtnlNeighborResponse,
    allow_not_found: bool,
) -> io::Result<()> {
    match response {
        RtnlNeighborResponse::Success => Ok(()),
        RtnlNeighborResponse::NotFound if allow_not_found => Ok(()),
        RtnlNeighborResponse::NotFound => Err(io::Error::new(
            ErrorKind::NotFound,
            format!("{}: entry not found", operation),
        )),
        RtnlNeighborResponse::Failed => Err(io::Error::other(format!("{} failed", operation))),
        RtnlNeighborResponse::NotImplemented => Err(io::Error::new(
            ErrorKind::Unsupported,
            format!("{} is not implemented", operation),
        )),
    }
}

async fn add_or_change_neighbor(
    handle: &rtnetlink::NeighbourHandle,
    entry: NeighborEntry,
    replace: bool,
) -> RtnlNeighborResponse {
    let mut request = handle.add(entry.if_id, entry.destination);

    if let Some(ref link_address) = entry.link_address {
        request = request.link_local_address(link_address);
    }

    if let Some(state) = entry.state {
        request = request.state(state);
    }

    if let Some(flags) = entry.flags {
        request = request.flags(flags);
    }

    if replace {
        request = request.replace();
    }

    match request.execute().await {
        Ok(()) => RtnlNeighborResponse::Success,
        Err(rtnetlink::Error::NetlinkError(err_msg)) => {
            let io_err = err_msg.to_io();
            match io_err.kind() {
                ErrorKind::NotFound => RtnlNeighborResponse::NotFound,
                ErrorKind::AlreadyExists => {
                    warn!("Neighbor operation failed (already exists): {}", io_err);
                    RtnlNeighborResponse::Failed
                }
                _ => {
                    warn!("Neighbor operation failed: {}", io_err);
                    RtnlNeighborResponse::Failed
                }
            }
        }
        Err(err) => {
            warn!("Neighbor operation failed: {}", err);
            RtnlNeighborResponse::Failed
        }
    }
}

async fn delete_neighbor(
    handle: &rtnetlink::NeighbourHandle,
    entry: NeighborDelete,
) -> RtnlNeighborResponse {
    let message = build_delete_message(&entry);
    match handle.del(message).execute().await {
        Ok(()) => RtnlNeighborResponse::Success,
        Err(rtnetlink::Error::NetlinkError(err_msg)) => {
            let io_err = err_msg.to_io();
            match io_err.kind() {
                ErrorKind::NotFound => RtnlNeighborResponse::NotFound,
                _ => {
                    warn!("Neighbor delete failed: {}", io_err);
                    RtnlNeighborResponse::Failed
                }
            }
        }
        Err(err) => {
            warn!("Neighbor delete failed: {}", err);
            RtnlNeighborResponse::Failed
        }
    }
}

fn build_delete_message(entry: &NeighborDelete) -> NeighbourMessage {
    let mut message = NeighbourMessage::default();
    message.header.family = match entry.destination {
        IpAddr::V4(_) => AddressFamily::Inet,
        IpAddr::V6(_) => AddressFamily::Inet6,
    };
    message.header.ifindex = entry.if_id;
    message.header.kind = RouteType::Unspec;

    if let Some(state) = entry.state {
        message.header.state = state;
    }

    if let Some(flags) = entry.flags {
        message.header.flags = flags;
    }

    let destination = match entry.destination {
        IpAddr::V4(addr) => NeighbourAddress::Inet(addr),
        IpAddr::V6(addr) => NeighbourAddress::Inet6(addr),
    };

    message
        .attributes
        .push(NeighbourAttribute::Destination(destination));

    if let Some(ref link_address) = entry.link_address {
        message
            .attributes
            .push(NeighbourAttribute::LinkLocalAddress(link_address.clone()));
    }

    message
}
