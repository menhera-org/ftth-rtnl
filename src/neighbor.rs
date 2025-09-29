#![allow(unreachable_patterns)]

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};

pub(crate) type Client = AsyncWorldClient<RtnlNeighborRequest, RtnlNeighborResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlNeighborRequest, RtnlNeighborResponse>;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlNeighborRequest {

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
        Self {
            client,
        }
    }
}

pub(crate) async fn run_server(mut server: Server, _handle: rtnetlink::NeighbourHandle) {
    while let Some((req, respond)) = server.accept().await {
        match req {
            _ => respond(RtnlNeighborResponse::NotImplemented),
        }
    }
}
