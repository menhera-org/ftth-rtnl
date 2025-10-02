#![allow(unreachable_patterns)]

use std::io::{self, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};
use futures::TryStreamExt;
use log::warn;
use netlink_packet_route::AddressFamily;
use netlink_packet_route::route::{
    RouteAddress, RouteAttribute, RouteMessage, RouteNextHop, RouteNextHopFlags, RouteType,
    RouteVia,
};
use rtnetlink::RouteMessageBuilder;

pub(crate) type Client = AsyncWorldClient<RtnlRouteRequest, RtnlRouteResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlRouteRequest, RtnlRouteResponse>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Route {
    pub if_id: Option<u32>,
    pub gateway: Option<IpAddr>,
    pub source: Option<Ipv4Addr>,
    pub metric: Option<u32>,
    pub table: Option<u32>,
    pub route: crate::Ipv4Net,
    pub nexthops: Vec<RouteNextHopInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv6Route {
    pub if_id: Option<u32>,
    pub gateway: Option<IpAddr>,
    pub source: Option<Ipv6Addr>,
    pub metric: Option<u32>,
    pub table: Option<u32>,
    pub route: crate::Ipv6Net,
    pub nexthops: Vec<RouteNextHopInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteNextHopInfo {
    pub if_id: Option<u32>,
    pub gateway: Option<IpAddr>,
    pub weight: u32,
    pub flags: RouteNextHopFlags,
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
    Ipv4RouteGetByPrefix(crate::Ipv4Net),
    Ipv6RouteGetByPrefix(crate::Ipv6Net),
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

    pub fn ipv4_route_get_by_prefix(&self, prefix: crate::Ipv4Net) -> io::Result<Ipv4Route> {
        match self
            .client
            .send_request(RtnlRouteRequest::Ipv4RouteGetByPrefix(prefix))?
        {
            RtnlRouteResponse::Ipv4Route(route) => Ok(route),
            RtnlRouteResponse::NotFound => {
                Err(io::Error::new(ErrorKind::NotFound, "Route not found"))
            }
            other => Err(io::Error::other(format!(
                "Unexpected response for IPv4 route get by prefix: {:?}",
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

    pub fn ipv6_route_get_by_prefix(&self, prefix: crate::Ipv6Net) -> io::Result<Ipv6Route> {
        match self
            .client
            .send_request(RtnlRouteRequest::Ipv6RouteGetByPrefix(prefix))?
        {
            RtnlRouteResponse::Ipv6Route(route) => Ok(route),
            RtnlRouteResponse::NotFound => {
                Err(io::Error::new(ErrorKind::NotFound, "Route not found"))
            }
            other => Err(io::Error::other(format!(
                "Unexpected response for IPv6 route get by prefix: {:?}",
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
            RtnlRouteRequest::Ipv4RouteGetByPrefix(prefix) => {
                get_route_v4_by_prefix(&handle, prefix).await
            }
            RtnlRouteRequest::Ipv6RouteGetByPrefix(prefix) => {
                get_route_v6_by_prefix(&handle, prefix).await
            }
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
    let target = destination;
    let message = build_route_message_v4(Some(destination), 32);
    lookup_route_v4(handle, message, move |route| route.route.contains(&target)).await
}

async fn get_route_v4_by_prefix(
    handle: &rtnetlink::RouteHandle,
    prefix: crate::Ipv4Net,
) -> RtnlRouteResponse {
    let target = prefix;
    let message = build_route_message_v4(
        if prefix.prefix_len() == 0 {
            None
        } else {
            Some(prefix.addr())
        },
        prefix.prefix_len(),
    );
    lookup_route_v4(handle, message, move |route| route.route == target).await
}

async fn get_route_v6(handle: &rtnetlink::RouteHandle, destination: Ipv6Addr) -> RtnlRouteResponse {
    let target = destination;
    let message = build_route_message_v6(Some(destination), 128);
    lookup_route_v6(handle, message, move |route| route.route.contains(&target)).await
}

async fn get_route_v6_by_prefix(
    handle: &rtnetlink::RouteHandle,
    prefix: crate::Ipv6Net,
) -> RtnlRouteResponse {
    let target = prefix;
    let message = build_route_message_v6(
        if prefix.prefix_len() == 0 {
            None
        } else {
            Some(prefix.addr())
        },
        prefix.prefix_len(),
    );
    lookup_route_v6(handle, message, move |route| route.route == target).await
}

async fn lookup_route<F>(
    handle: &rtnetlink::RouteHandle,
    message: RouteMessage,
    map: F,
) -> RtnlRouteResponse
where
    F: Fn(RouteMessage) -> Option<RtnlRouteResponse>,
{
    let stream = handle.get(message).execute();
    futures::pin_mut!(stream);
    loop {
        match stream.try_next().await {
            Ok(Some(msg)) => {
                if let Some(resp) = map(msg) {
                    return resp;
                }
            }
            Ok(None) => return RtnlRouteResponse::NotFound,
            Err(err) => {
                warn!("Failed to get route: {}", err);
                return RtnlRouteResponse::Failed;
            }
        }
    }
}

async fn lookup_route_v4<F>(
    handle: &rtnetlink::RouteHandle,
    message: RouteMessage,
    predicate: F,
) -> RtnlRouteResponse
where
    F: Fn(&Ipv4Route) -> bool,
{
    lookup_route(handle, message, |msg| {
        if matches!(msg.header.kind, RouteType::Local | RouteType::Broadcast) {
            return None;
        }
        decode_ipv4_route(msg).and_then(|route| {
            if predicate(&route) {
                Some(RtnlRouteResponse::Ipv4Route(route))
            } else {
                None
            }
        })
    })
    .await
}

async fn lookup_route_v6<F>(
    handle: &rtnetlink::RouteHandle,
    message: RouteMessage,
    predicate: F,
) -> RtnlRouteResponse
where
    F: Fn(&Ipv6Route) -> bool,
{
    lookup_route(handle, message, |msg| {
        if matches!(msg.header.kind, RouteType::Local | RouteType::Broadcast) {
            return None;
        }
        decode_ipv6_route(msg).and_then(|route| {
            if predicate(&route) {
                Some(RtnlRouteResponse::Ipv6Route(route))
            } else {
                None
            }
        })
    })
    .await
}

fn build_route_message_v4(destination: Option<Ipv4Addr>, prefix_len: u8) -> RouteMessage {
    let mut message = RouteMessage::default();
    message.header.address_family = AddressFamily::Inet;
    message.header.destination_prefix_length = prefix_len;
    if let Some(addr) = destination {
        message
            .attributes
            .push(RouteAttribute::Destination(RouteAddress::Inet(addr)));
    }
    message
}

fn build_route_message_v6(destination: Option<Ipv6Addr>, prefix_len: u8) -> RouteMessage {
    let mut message = RouteMessage::default();
    message.header.address_family = AddressFamily::Inet6;
    message.header.destination_prefix_length = prefix_len;
    if let Some(addr) = destination {
        message
            .attributes
            .push(RouteAttribute::Destination(RouteAddress::Inet6(addr)));
    }
    message
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
        match gw {
            IpAddr::V4(addr) => {
                builder = builder.gateway(addr);
            }
            IpAddr::V6(addr) => {
                builder
                    .get_mut()
                    .attributes
                    .push(RouteAttribute::Via(RouteVia::Inet6(addr)));
            }
        }
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

    if !route.nexthops.is_empty() {
        if let Some(multipath) = build_multipath_v4(&route.nexthops) {
            builder = builder.multipath(multipath);
        }
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
        match gw {
            IpAddr::V6(addr) => {
                builder = builder.gateway(addr);
            }
            IpAddr::V4(addr) => {
                builder
                    .get_mut()
                    .attributes
                    .push(RouteAttribute::Via(RouteVia::Inet(addr)));
            }
        }
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

    if !route.nexthops.is_empty() {
        if let Some(multipath) = build_multipath_v6(&route.nexthops) {
            builder = builder.multipath(multipath);
        }
    }

    builder.build()
}

fn decode_ipv4_route(message: RouteMessage) -> Option<Ipv4Route> {
    if message.header.address_family != AddressFamily::Inet {
        return None;
    }

    let header = message.header;
    let mut destination = None;
    let mut gateway: Option<IpAddr> = None;
    let mut source = None;
    let mut metric = None;
    let mut table = table_from_header(header.table);
    let mut oif = None;
    let mut nexthops = Vec::new();

    for attr in message.attributes {
        match attr {
            RouteAttribute::Destination(RouteAddress::Inet(addr)) => destination = Some(addr),
            RouteAttribute::Gateway(RouteAddress::Inet(addr)) => gateway = Some(IpAddr::V4(addr)),
            RouteAttribute::Gateway(RouteAddress::Inet6(addr)) => gateway = Some(IpAddr::V6(addr)),
            RouteAttribute::Via(RouteVia::Inet(addr)) => gateway = Some(IpAddr::V4(addr)),
            RouteAttribute::Via(RouteVia::Inet6(addr)) => gateway = Some(IpAddr::V6(addr)),
            RouteAttribute::PrefSource(RouteAddress::Inet(addr)) => source = Some(addr),
            RouteAttribute::Priority(value) => metric = Some(value),
            RouteAttribute::Oif(index) => oif = Some(index),
            RouteAttribute::Table(value) => table = Some(value),
            RouteAttribute::MultiPath(paths) => {
                nexthops.extend(convert_multipath(paths));
            }
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
        nexthops,
    })
}

fn decode_ipv6_route(message: RouteMessage) -> Option<Ipv6Route> {
    if message.header.address_family != AddressFamily::Inet6 {
        return None;
    }

    let header = message.header;
    let mut destination = None;
    let mut gateway: Option<IpAddr> = None;
    let mut source = None;
    let mut metric = None;
    let mut table = table_from_header(header.table);
    let mut oif = None;
    let mut nexthops = Vec::new();

    for attr in message.attributes {
        match attr {
            RouteAttribute::Destination(RouteAddress::Inet6(addr)) => destination = Some(addr),
            RouteAttribute::Gateway(RouteAddress::Inet(addr)) => gateway = Some(IpAddr::V4(addr)),
            RouteAttribute::Gateway(RouteAddress::Inet6(addr)) => gateway = Some(IpAddr::V6(addr)),
            RouteAttribute::Via(RouteVia::Inet(addr)) => gateway = Some(IpAddr::V4(addr)),
            RouteAttribute::Via(RouteVia::Inet6(addr)) => gateway = Some(IpAddr::V6(addr)),
            RouteAttribute::PrefSource(RouteAddress::Inet6(addr)) => source = Some(addr),
            RouteAttribute::Priority(value) => metric = Some(value),
            RouteAttribute::Oif(index) => oif = Some(index),
            RouteAttribute::Table(value) => table = Some(value),
            RouteAttribute::MultiPath(paths) => {
                nexthops.extend(convert_multipath(paths));
            }
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
        nexthops,
    })
}

fn table_from_header(value: u8) -> Option<u32> {
    if value == 0 { None } else { Some(value as u32) }
}

fn convert_multipath(paths: Vec<RouteNextHop>) -> Vec<RouteNextHopInfo> {
    let mut result = Vec::new();
    for path in paths {
        let mut gateway = None;
        for attr in path.attributes {
            match attr {
                RouteAttribute::Gateway(RouteAddress::Inet(addr)) => {
                    gateway = Some(IpAddr::V4(addr))
                }
                RouteAttribute::Gateway(RouteAddress::Inet6(addr)) => {
                    gateway = Some(IpAddr::V6(addr))
                }
                RouteAttribute::Via(RouteVia::Inet(addr)) => gateway = Some(IpAddr::V4(addr)),
                RouteAttribute::Via(RouteVia::Inet6(addr)) => gateway = Some(IpAddr::V6(addr)),
                RouteAttribute::NewDestination(_) => {}
                _ => {}
            }
        }

        let weight = u32::from(path.hops).saturating_add(1);
        let if_id = match path.interface_index {
            0 => None,
            index => Some(index),
        };

        result.push(RouteNextHopInfo {
            if_id,
            gateway,
            weight,
            flags: path.flags,
        });
    }
    result
}

fn build_multipath_v4(entries: &[RouteNextHopInfo]) -> Option<Vec<RouteNextHop>> {
    let mut nexthops = Vec::new();
    for entry in entries {
        let mut route_entry = RouteNextHop::default();
        route_entry.flags = entry.flags;
        route_entry.hops = weight_to_hops(entry.weight);
        route_entry.interface_index = entry.if_id.unwrap_or(0);
        route_entry.attributes = Vec::new();

        match entry.gateway {
            Some(IpAddr::V4(addr)) => {
                route_entry
                    .attributes
                    .push(RouteAttribute::Gateway(RouteAddress::Inet(addr)));
            }
            Some(IpAddr::V6(addr)) => {
                route_entry
                    .attributes
                    .push(RouteAttribute::Via(RouteVia::Inet6(addr)));
            }
            None => {}
        }

        nexthops.push(route_entry);
    }

    if nexthops.is_empty() {
        None
    } else {
        Some(nexthops)
    }
}

fn build_multipath_v6(entries: &[RouteNextHopInfo]) -> Option<Vec<RouteNextHop>> {
    let mut nexthops = Vec::new();
    for entry in entries {
        let mut route_entry = RouteNextHop::default();
        route_entry.flags = entry.flags;
        route_entry.hops = weight_to_hops(entry.weight);
        route_entry.interface_index = entry.if_id.unwrap_or(0);
        route_entry.attributes = Vec::new();

        match entry.gateway {
            Some(IpAddr::V6(addr)) => {
                route_entry
                    .attributes
                    .push(RouteAttribute::Gateway(RouteAddress::Inet6(addr)));
            }
            Some(IpAddr::V4(addr)) => {
                route_entry
                    .attributes
                    .push(RouteAttribute::Via(RouteVia::Inet(addr)));
            }
            None => {}
        }

        nexthops.push(route_entry);
    }

    if nexthops.is_empty() {
        None
    } else {
        Some(nexthops)
    }
}

fn weight_to_hops(weight: u32) -> u8 {
    weight.saturating_sub(1).min(u8::MAX as u32) as u8
}
