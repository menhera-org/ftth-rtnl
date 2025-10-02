#![allow(unreachable_patterns)]

use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr};

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};
use futures::TryStreamExt;
use log::warn;
use netlink_packet_route::AddressFamily;
use netlink_packet_route::route::{RouteAddress, RouteAttribute, RouteMessage};
use rtnetlink::RouteMessageBuilder;

pub(crate) type Client = AsyncWorldClient<RtnlRouteRequest, RtnlRouteResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlRouteRequest, RtnlRouteResponse>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Route {
    pub if_id: Option<u32>,
    pub gateway: Option<Ipv4Addr>,
    pub source: Option<Ipv4Addr>,
    pub metric: Option<u32>,
    pub table: Option<u32>,
    pub route: crate::Ipv4Net,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv6Route {
    pub if_id: Option<u32>,
    pub gateway: Option<Ipv6Addr>,
    pub source: Option<Ipv6Addr>,
    pub metric: Option<u32>,
    pub table: Option<u32>,
    pub route: crate::Ipv6Net,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlRouteRequest {
    Ipv4RouteList,
    Ipv6RouteList,
    Ipv4RouteAdd(Ipv4Route),
    Ipv4RouteReplace(Ipv4Route),
    Ipv6RouteAdd(Ipv6Route),
    Ipv6RouteReplace(Ipv6Route),
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
        Self { client }
    }

    pub fn ipv4_route_add(&self, route: Ipv4Route) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlRouteRequest::Ipv4RouteAdd(route))?;
        handle_route_status("IPv4 route add", res)
    }

    pub fn ipv4_route_replace(&self, route: Ipv4Route) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlRouteRequest::Ipv4RouteReplace(route))?;
        handle_route_status("IPv4 route replace", res)
    }

    pub fn ipv4_route_del(&self, route: Ipv4Route) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlRouteRequest::Ipv4RouteDel(route))?;
        handle_route_status("IPv4 route delete", res)
    }

    pub fn ipv4_route_list(&self) -> io::Result<Vec<Ipv4Route>> {
        match self.client.send_request(RtnlRouteRequest::Ipv4RouteList)? {
            RtnlRouteResponse::Ipv4RouteList(routes) => Ok(routes),
            other => Err(io::Error::other(format!(
                "Unexpected response for IPv4 route list: {:?}",
                other
            ))),
        }
    }

    pub fn ipv4_route_get(&self, destination: Ipv4Addr) -> io::Result<Ipv4Route> {
        match self
            .client
            .send_request(RtnlRouteRequest::Ipv4RouteGet(destination))?
        {
            RtnlRouteResponse::Ipv4Route(route) => Ok(route),
            RtnlRouteResponse::NotFound => {
                Err(io::Error::new(ErrorKind::NotFound, "Route not found"))
            }
            other => Err(io::Error::other(format!(
                "Unexpected response for IPv4 route get: {:?}",
                other
            ))),
        }
    }

    pub fn ipv6_route_add(&self, route: Ipv6Route) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlRouteRequest::Ipv6RouteAdd(route))?;
        handle_route_status("IPv6 route add", res)
    }

    pub fn ipv6_route_replace(&self, route: Ipv6Route) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlRouteRequest::Ipv6RouteReplace(route))?;
        handle_route_status("IPv6 route replace", res)
    }

    pub fn ipv6_route_del(&self, route: Ipv6Route) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlRouteRequest::Ipv6RouteDel(route))?;
        handle_route_status("IPv6 route delete", res)
    }

    pub fn ipv6_route_list(&self) -> io::Result<Vec<Ipv6Route>> {
        match self.client.send_request(RtnlRouteRequest::Ipv6RouteList)? {
            RtnlRouteResponse::Ipv6RouteList(routes) => Ok(routes),
            other => Err(io::Error::other(format!(
                "Unexpected response for IPv6 route list: {:?}",
                other
            ))),
        }
    }

    pub fn ipv6_route_get(&self, destination: Ipv6Addr) -> io::Result<Ipv6Route> {
        match self
            .client
            .send_request(RtnlRouteRequest::Ipv6RouteGet(destination))?
        {
            RtnlRouteResponse::Ipv6Route(route) => Ok(route),
            RtnlRouteResponse::NotFound => {
                Err(io::Error::new(ErrorKind::NotFound, "Route not found"))
            }
            other => Err(io::Error::other(format!(
                "Unexpected response for IPv6 route get: {:?}",
                other
            ))),
        }
    }
}

pub(crate) async fn run_server(mut server: Server, handle: rtnetlink::RouteHandle) {
    while let Some((req, respond)) = server.accept().await {
        let response = match req {
            RtnlRouteRequest::Ipv4RouteList => list_routes_v4(&handle).await,
            RtnlRouteRequest::Ipv6RouteList => list_routes_v6(&handle).await,
            RtnlRouteRequest::Ipv4RouteAdd(route) => add_route_v4(&handle, route, false).await,
            RtnlRouteRequest::Ipv4RouteReplace(route) => add_route_v4(&handle, route, true).await,
            RtnlRouteRequest::Ipv6RouteAdd(route) => add_route_v6(&handle, route, false).await,
            RtnlRouteRequest::Ipv6RouteReplace(route) => add_route_v6(&handle, route, true).await,
            RtnlRouteRequest::Ipv4RouteDel(route) => delete_route_v4(&handle, route).await,
            RtnlRouteRequest::Ipv6RouteDel(route) => delete_route_v6(&handle, route).await,
            RtnlRouteRequest::Ipv4RouteGet(destination) => get_route_v4(&handle, destination).await,
            RtnlRouteRequest::Ipv6RouteGet(destination) => get_route_v6(&handle, destination).await,
        };
        respond(response);
    }
}

fn handle_route_status(op: &str, response: RtnlRouteResponse) -> io::Result<()> {
    match response {
        RtnlRouteResponse::Success => Ok(()),
        RtnlRouteResponse::NotFound => Err(io::Error::new(
            ErrorKind::NotFound,
            format!("{}: route not found", op),
        )),
        RtnlRouteResponse::Failed => Err(io::Error::other(format!("{} failed", op))),
        RtnlRouteResponse::NotImplemented => Err(io::Error::new(
            ErrorKind::Unsupported,
            format!("{} not implemented", op),
        )),
        other => Err(io::Error::other(format!(
            "{} returned unexpected response: {:?}",
            op, other
        ))),
    }
}

async fn list_routes_v4(handle: &rtnetlink::RouteHandle) -> RtnlRouteResponse {
    let message = RouteMessageBuilder::<Ipv4Addr>::new().build();
    let stream = handle.get(message).execute();
    futures::pin_mut!(stream);
    let mut routes = Vec::new();
    loop {
        match stream.try_next().await {
            Ok(Some(msg)) => {
                if let Some(route) = decode_ipv4_route(msg) {
                    routes.push(route);
                }
            }
            Ok(None) => break,
            Err(err) => {
                warn!("Failed to list IPv4 routes: {}", err);
                return RtnlRouteResponse::Failed;
            }
        }
    }
    RtnlRouteResponse::Ipv4RouteList(routes)
}

async fn list_routes_v6(handle: &rtnetlink::RouteHandle) -> RtnlRouteResponse {
    let message = RouteMessageBuilder::<Ipv6Addr>::new().build();
    let stream = handle.get(message).execute();
    futures::pin_mut!(stream);
    let mut routes = Vec::new();
    loop {
        match stream.try_next().await {
            Ok(Some(msg)) => {
                if let Some(route) = decode_ipv6_route(msg) {
                    routes.push(route);
                }
            }
            Ok(None) => break,
            Err(err) => {
                warn!("Failed to list IPv6 routes: {}", err);
                return RtnlRouteResponse::Failed;
            }
        }
    }
    RtnlRouteResponse::Ipv6RouteList(routes)
}

async fn add_route_v4(
    handle: &rtnetlink::RouteHandle,
    route: Ipv4Route,
    replace: bool,
) -> RtnlRouteResponse {
    let message = build_ipv4_route_message(&route);
    let request = handle.add(message);
    let request = if replace { request.replace() } else { request };
    map_route_result(
        request.execute().await,
        if replace {
            "replace IPv4 route"
        } else {
            "add IPv4 route"
        },
    )
}

async fn add_route_v6(
    handle: &rtnetlink::RouteHandle,
    route: Ipv6Route,
    replace: bool,
) -> RtnlRouteResponse {
    let message = build_ipv6_route_message(&route);
    let request = handle.add(message);
    let request = if replace { request.replace() } else { request };
    map_route_result(
        request.execute().await,
        if replace {
            "replace IPv6 route"
        } else {
            "add IPv6 route"
        },
    )
}

async fn delete_route_v4(handle: &rtnetlink::RouteHandle, route: Ipv4Route) -> RtnlRouteResponse {
    let message = build_ipv4_route_message(&route);
    map_route_result(handle.del(message).execute().await, "delete IPv4 route")
}

async fn delete_route_v6(handle: &rtnetlink::RouteHandle, route: Ipv6Route) -> RtnlRouteResponse {
    let message = build_ipv6_route_message(&route);
    map_route_result(handle.del(message).execute().await, "delete IPv6 route")
}

async fn get_route_v4(handle: &rtnetlink::RouteHandle, destination: Ipv4Addr) -> RtnlRouteResponse {
    let message = RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(destination, 32)
        .build();
    let stream = handle.get(message).execute();
    futures::pin_mut!(stream);
    match stream.try_next().await {
        Ok(Some(msg)) => decode_ipv4_route(msg)
            .map(RtnlRouteResponse::Ipv4Route)
            .unwrap_or(RtnlRouteResponse::NotFound),
        Ok(None) => RtnlRouteResponse::NotFound,
        Err(err) => {
            warn!("Failed to get IPv4 route: {}", err);
            RtnlRouteResponse::Failed
        }
    }
}

async fn get_route_v6(handle: &rtnetlink::RouteHandle, destination: Ipv6Addr) -> RtnlRouteResponse {
    let message = RouteMessageBuilder::<Ipv6Addr>::new()
        .destination_prefix(destination, 128)
        .build();
    let stream = handle.get(message).execute();
    futures::pin_mut!(stream);
    match stream.try_next().await {
        Ok(Some(msg)) => decode_ipv6_route(msg)
            .map(RtnlRouteResponse::Ipv6Route)
            .unwrap_or(RtnlRouteResponse::NotFound),
        Ok(None) => RtnlRouteResponse::NotFound,
        Err(err) => {
            warn!("Failed to get IPv6 route: {}", err);
            RtnlRouteResponse::Failed
        }
    }
}

fn map_route_result(result: Result<(), rtnetlink::Error>, op: &str) -> RtnlRouteResponse {
    match result {
        Ok(()) => RtnlRouteResponse::Success,
        Err(rtnetlink::Error::NetlinkError(err_msg)) => {
            let io_err = err_msg.to_io();
            match io_err.kind() {
                ErrorKind::NotFound => RtnlRouteResponse::NotFound,
                ErrorKind::AlreadyExists => {
                    warn!("Route operation failed (already exists): {}", io_err);
                    RtnlRouteResponse::Failed
                }
                _ => {
                    warn!("Route operation '{}' failed: {}", op, io_err);
                    RtnlRouteResponse::Failed
                }
            }
        }
        Err(err) => {
            warn!("Route operation '{}' failed: {}", op, err);
            RtnlRouteResponse::Failed
        }
    }
}

fn build_ipv4_route_message(route: &Ipv4Route) -> RouteMessage {
    let mut builder = RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(route.route.addr(), route.route.prefix_len());

    if let Some(if_id) = route.if_id.filter(|id| *id != 0) {
        builder = builder.output_interface(if_id);
    }

    if let Some(gw) = route.gateway {
        builder = builder.gateway(gw);
    }

    if let Some(src) = route.source {
        builder = builder.pref_source(src);
    }

    if let Some(metric) = route.metric {
        builder = builder.priority(metric);
    }

    if let Some(table) = route.table {
        builder = builder.table_id(table);
    }

    builder.build()
}

fn build_ipv6_route_message(route: &Ipv6Route) -> RouteMessage {
    let mut builder = RouteMessageBuilder::<Ipv6Addr>::new()
        .destination_prefix(route.route.addr(), route.route.prefix_len());

    if let Some(if_id) = route.if_id.filter(|id| *id != 0) {
        builder = builder.output_interface(if_id);
    }

    if let Some(gw) = route.gateway {
        builder = builder.gateway(gw);
    }

    if let Some(src) = route.source {
        builder = builder.pref_source(src);
    }

    if let Some(metric) = route.metric {
        builder = builder.priority(metric);
    }

    if let Some(table) = route.table {
        builder = builder.table_id(table);
    }

    builder.build()
}

fn decode_ipv4_route(message: RouteMessage) -> Option<Ipv4Route> {
    if message.header.address_family != AddressFamily::Inet {
        return None;
    }

    let header = message.header;
    let mut destination = None;
    let mut gateway = None;
    let mut source = None;
    let mut metric = None;
    let mut table = table_from_header(header.table);
    let mut oif = None;

    for attr in message.attributes {
        match attr {
            RouteAttribute::Destination(RouteAddress::Inet(addr)) => destination = Some(addr),
            RouteAttribute::Gateway(RouteAddress::Inet(addr)) => gateway = Some(addr),
            RouteAttribute::PrefSource(RouteAddress::Inet(addr)) => source = Some(addr),
            RouteAttribute::Priority(value) => metric = Some(value),
            RouteAttribute::Oif(index) => oif = Some(index),
            RouteAttribute::Table(value) => table = Some(value),
            _ => {}
        }
    }

    let addr = destination.unwrap_or(Ipv4Addr::UNSPECIFIED);
    let net = crate::Ipv4Net::new(addr, header.destination_prefix_length).ok()?;

    Some(Ipv4Route {
        if_id: oif.filter(|id| *id != 0),
        gateway,
        source,
        metric,
        table,
        route: net,
    })
}

fn decode_ipv6_route(message: RouteMessage) -> Option<Ipv6Route> {
    if message.header.address_family != AddressFamily::Inet6 {
        return None;
    }

    let header = message.header;
    let mut destination = None;
    let mut gateway = None;
    let mut source = None;
    let mut metric = None;
    let mut table = table_from_header(header.table);
    let mut oif = None;

    for attr in message.attributes {
        match attr {
            RouteAttribute::Destination(RouteAddress::Inet6(addr)) => destination = Some(addr),
            RouteAttribute::Gateway(RouteAddress::Inet6(addr)) => gateway = Some(addr),
            RouteAttribute::PrefSource(RouteAddress::Inet6(addr)) => source = Some(addr),
            RouteAttribute::Priority(value) => metric = Some(value),
            RouteAttribute::Oif(index) => oif = Some(index),
            RouteAttribute::Table(value) => table = Some(value),
            _ => {}
        }
    }

    let addr = destination.unwrap_or(Ipv6Addr::UNSPECIFIED);
    let net = crate::Ipv6Net::new(addr, header.destination_prefix_length).ok()?;

    Some(Ipv6Route {
        if_id: oif.filter(|id| *id != 0),
        gateway,
        source,
        metric,
        table,
        route: net,
    })
}

fn table_from_header(value: u8) -> Option<u32> {
    if value == 0 { None } else { Some(value as u32) }
}
