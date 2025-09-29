#![allow(unreachable_patterns)]

use std::net::{Ipv4Addr, Ipv6Addr};

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};

pub(crate) type Client = AsyncWorldClient<RtnlRouteRequest, RtnlRouteResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlRouteRequest, RtnlRouteResponse>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Route {
    if_id: Option<u32>, // 0 is the same as None
    gateway: Option<Ipv4Addr>,
    route: crate::Ipv4Net,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv6Route {
    if_id: Option<u32>, // 0 is the same as None
    gateway: Option<Ipv6Addr>,
    route: crate::Ipv6Net,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlRouteRequest {
    Ipv4RouteList,
    Ipv6RouteList,
    Ipv4RouteAdd(Ipv4Route),
    Ipv6RouteAdd(Ipv6Route),
    Ipv4RouteDel(Ipv4Route),
    Ipv6RouteDel(Ipv6Route),
    Ipv4RouteGet(Ipv4Addr),
    Ipv6RouteGet(Ipv6Addr),
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlRouteResponse {
    Success,
    Failed,
    NotImplemented,
    NotFound,
    Ipv4RouteList(Vec<Ipv4Route>),
    Ipv6RouteList(Vec<Ipv6Route>),
    Ipv4Route(Ipv4Route),
    Ipv6Route(Ipv6Route),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RtnlRouteClient {
    client: Client,
}

impl RtnlRouteClient {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
        }
    }
}

pub(crate) async fn run_server(mut server: Server, _handle: rtnetlink::RouteHandle) {
    while let Some((req, respond)) = server.accept().await {
        match req {
            _ => respond(RtnlRouteResponse::NotImplemented),
        }
    }
}
