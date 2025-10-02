#![allow(unreachable_patterns)]

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};

use futures::TryStreamExt;

use std::fmt::{Debug, Display};
use std::io::{self, ErrorKind};

use netlink_packet_route::link::LinkFlags;
use rtnetlink::{LinkMessageBuilder, LinkUnspec};

pub(crate) type Client = AsyncWorldClient<RtnlLinkRequest, RtnlLinkResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlLinkRequest, RtnlLinkResponse>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MacAddr {
    pub inner: [u8; 6],
}

impl MacAddr {
    pub const fn new(inner: [u8; 6]) -> Self {
        Self { inner }
    }
}

impl Default for MacAddr {
    fn default() -> Self {
        Self { inner: [0; 6] }
    }
}

impl Debug for MacAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("MacAddr({})", self))
    }
}

impl Display for MacAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.inner[0],
            self.inner[1],
            self.inner[2],
            self.inner[3],
            self.inner[4],
            self.inner[5],
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub if_name: String,
    pub if_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlLinkRequest {
    InterfaceList,
    InterfaceGet { if_id: u32 },
    InterfaceGetByName { if_name: String },
    MacAddrGet { if_id: u32 },
    MacAddrSet { if_id: u32, mac_addr: MacAddr },
    MtuGet { if_id: u32 },
    InterfaceSetAdmin { if_id: u32, up: bool },
    InterfaceSetPromisc { if_id: u32, enable: bool },
    InterfaceSetArp { if_id: u32, enable: bool },
    InterfaceSetMtu { if_id: u32, mtu: u32 },
    InterfaceRename { if_id: u32, if_name: String },
    InterfaceSetAllMulticast { if_id: u32, enable: bool },
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlLinkResponse {
    Success,
    Failed,
    NotImplemented,
    NotFound,
    InterfaceList(Vec<Interface>),
    Interface(Interface),
    MacAddr(MacAddr),
    Mtu(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RtnlLinkClient {
    client: Client,
}

impl RtnlLinkClient {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn interface_set_up(&self, if_id: u32) -> io::Result<()> {
        self.interface_set_admin_state(if_id, true)
    }

    pub fn interface_set_down(&self, if_id: u32) -> io::Result<()> {
        self.interface_set_admin_state(if_id, false)
    }

    pub fn interface_set_admin_state(&self, if_id: u32, up: bool) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::InterfaceSetAdmin { if_id, up })?;
        let op = if up {
            "Set interface up"
        } else {
            "Set interface down"
        };
        handle_status_response(op, res)
    }

    pub fn interface_set_promiscuous(&self, if_id: u32, enable: bool) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::InterfaceSetPromisc { if_id, enable })?;
        handle_status_response(
            if enable {
                "Enable promiscuous mode"
            } else {
                "Disable promiscuous mode"
            },
            res,
        )
    }

    pub fn interface_set_arp(&self, if_id: u32, enable: bool) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::InterfaceSetArp { if_id, enable })?;
        handle_status_response(if enable { "Enable ARP" } else { "Disable ARP" }, res)
    }

    pub fn interface_set_mtu(&self, if_id: u32, mtu: u32) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::InterfaceSetMtu { if_id, mtu })?;
        handle_status_response("Set MTU", res)
    }

    pub fn interface_rename(&self, if_id: u32, new_name: &str) -> io::Result<()> {
        let res = self.client.send_request(RtnlLinkRequest::InterfaceRename {
            if_id,
            if_name: new_name.to_owned(),
        })?;
        handle_status_response("Rename interface", res)
    }

    pub fn interface_get(&self, if_id: u32) -> io::Result<Interface> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::InterfaceGet { if_id })?;
        match res {
            RtnlLinkResponse::Interface(interface) => Ok(interface),
            RtnlLinkResponse::NotFound => {
                Err(io::Error::new(ErrorKind::NotFound, "Interface not found"))
            }
            _ => Err(io::Error::other("Failed to get interface")),
        }
    }

    pub fn interface_get_by_name(&self, name: &str) -> std::io::Result<Interface> {
        let name = name.to_owned();
        let res = self
            .client
            .send_request(RtnlLinkRequest::InterfaceGetByName { if_name: name })?;
        match res {
            RtnlLinkResponse::Interface(interface) => {
                return Ok(interface);
            }
            _ => {}
        }
        Err(std::io::Error::other("Not found"))
    }

    pub fn mac_addr_get(&self, if_id: u32) -> std::io::Result<Option<MacAddr>> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::MacAddrGet { if_id })?;
        match res {
            RtnlLinkResponse::MacAddr(addr) => {
                return Ok(Some(addr));
            }
            _ => {}
        }
        Ok(None)
    }

    pub fn mtu_get(&self, if_id: u32) -> io::Result<u32> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::MtuGet { if_id })?;
        match res {
            RtnlLinkResponse::Mtu(mtu) => Ok(mtu),
            RtnlLinkResponse::NotFound => {
                Err(io::Error::new(ErrorKind::NotFound, "Interface not found"))
            }
            _ => Err(io::Error::other("Failed to get MTU")),
        }
    }

    pub fn mac_addr_set(&self, if_id: u32, mac_addr: MacAddr) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::MacAddrSet { if_id, mac_addr })?;
        handle_status_response("Set MAC address", res)
    }

    pub fn interface_set_all_multicast(&self, if_id: u32, enable: bool) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlLinkRequest::InterfaceSetAllMulticast { if_id, enable })?;
        handle_status_response(
            if enable {
                "Enable all-multicast"
            } else {
                "Disable all-multicast"
            },
            res,
        )
    }

    pub fn interface_list(&self) -> std::io::Result<Vec<Interface>> {
        let res = self.client.send_request(RtnlLinkRequest::InterfaceList)?;
        match res {
            RtnlLinkResponse::InterfaceList(list) => {
                return Ok(list);
            }
            _ => {}
        }
        Err(std::io::Error::other("Unknown error"))
    }
}

fn handle_status_response(op: &str, response: RtnlLinkResponse) -> io::Result<()> {
    match response {
        RtnlLinkResponse::Success => Ok(()),
        RtnlLinkResponse::NotFound => Err(io::Error::new(
            ErrorKind::NotFound,
            format!("{}: interface not found", op),
        )),
        RtnlLinkResponse::Failed => Err(io::Error::other(format!("{} failed", op))),
        RtnlLinkResponse::NotImplemented => Err(io::Error::new(
            ErrorKind::Unsupported,
            format!("{} not implemented", op),
        )),
        other => Err(io::Error::other(format!(
            "{} returned unexpected response: {:?}",
            op, other
        ))),
    }
}

async fn apply_link_set<F>(
    handle: &rtnetlink::LinkHandle,
    if_id: u32,
    op: F,
) -> Result<(), rtnetlink::Error>
where
    F: FnOnce(LinkMessageBuilder<LinkUnspec>) -> LinkMessageBuilder<LinkUnspec>,
{
    let builder = LinkMessageBuilder::<LinkUnspec>::new().index(if_id);
    let message = op(builder).build();
    handle.set(message).execute().await
}

fn map_link_result(result: Result<(), rtnetlink::Error>, op: &str, if_id: u32) -> RtnlLinkResponse {
    match result {
        Ok(()) => RtnlLinkResponse::Success,
        Err(rtnetlink::Error::NetlinkError(err_msg)) => {
            let io_err = err_msg.to_io();
            if io_err.kind() == ErrorKind::NotFound {
                RtnlLinkResponse::NotFound
            } else {
                log::warn!("Failed to {} for ifindex {}: {}", op, if_id, io_err);
                RtnlLinkResponse::Failed
            }
        }
        Err(err) => {
            log::warn!("Failed to {} for ifindex {}: {}", op, if_id, err);
            RtnlLinkResponse::Failed
        }
    }
}

pub(crate) async fn run_server(mut server: Server, mut handle: rtnetlink::LinkHandle) {
    'reqloop: while let Some((req, respond)) = server.accept().await {
        match req {
            RtnlLinkRequest::InterfaceGet { if_id } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let response = handle.get().match_index(if_id).execute();
                futures::pin_mut!(response);
                while let Ok(Some(response)) = response.try_next().await {
                    let mut if_name = None;
                    for attr in response.attributes.iter() {
                        if let netlink_packet_route::link::LinkAttribute::IfName(name) = attr {
                            if_name = Some(name.clone());
                        }
                    }

                    if let Some(name) = if_name {
                        respond(RtnlLinkResponse::Interface(Interface {
                            if_id,
                            if_name: name,
                        }));
                        continue 'reqloop;
                    }
                }
                respond(RtnlLinkResponse::NotFound);
            }
            RtnlLinkRequest::InterfaceGetByName { if_name } => {
                let response = handle.get().match_name(if_name.to_owned()).execute();
                futures::pin_mut!(response);
                while let Ok(Some(response)) = response.try_next().await {
                    let if_index = response.header.index;
                    if if_index == 0 {
                        continue;
                    }

                    respond(RtnlLinkResponse::Interface(Interface {
                        if_id: if_index,
                        if_name: if_name.to_owned(),
                    }));
                    continue 'reqloop;
                }
                respond(RtnlLinkResponse::NotFound);
            }
            RtnlLinkRequest::MacAddrGet { if_id } => {
                let if_index = if_id;
                if if_index == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }
                let response = handle.get().match_index(if_index).execute();
                futures::pin_mut!(response);
                while let Ok(Some(response)) = response.try_next().await {
                    for link in response.attributes.iter() {
                        match link {
                            netlink_packet_route::link::LinkAttribute::Address(addr) => {
                                respond(RtnlLinkResponse::MacAddr(MacAddr::new(
                                    addr[0..6].try_into().unwrap_or([0; 6]),
                                )));
                                continue 'reqloop;
                            }
                            _ => {}
                        }
                    }
                }
                respond(RtnlLinkResponse::NotFound);
            }
            RtnlLinkRequest::MtuGet { if_id } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let response = handle.get().match_index(if_id).execute();
                futures::pin_mut!(response);
                while let Ok(Some(response)) = response.try_next().await {
                    for link in response.attributes.iter() {
                        if let netlink_packet_route::link::LinkAttribute::Mtu(mtu) = link {
                            respond(RtnlLinkResponse::Mtu(*mtu));
                            continue 'reqloop;
                        }
                    }
                }
                respond(RtnlLinkResponse::NotFound);
            }
            RtnlLinkRequest::InterfaceList => {
                let mut interfaces = Vec::new();
                let response = handle.get().execute();
                futures::pin_mut!(response);
                while let Ok(Some(response)) = response.try_next().await {
                    let if_index = response.header.index;
                    let mut if_name = None;
                    for link in response.attributes.iter() {
                        match link {
                            netlink_packet_route::link::LinkAttribute::IfName(name) => {
                                if_name = Some(name.clone());
                            }
                            _ => {}
                        }
                    }

                    if if_index == 0 || if_name.is_none() {
                        continue;
                    }

                    interfaces.push(Interface {
                        if_id: if_index,
                        if_name: if_name.unwrap(),
                    });
                }
                respond(RtnlLinkResponse::InterfaceList(interfaces));
            }
            RtnlLinkRequest::MacAddrSet { if_id, mac_addr } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let mac_bytes = mac_addr.inner.to_vec();
                let result =
                    apply_link_set(&handle, if_id, |builder| builder.address(mac_bytes)).await;
                respond(map_link_result(result, "set MAC address", if_id));
            }
            RtnlLinkRequest::InterfaceSetAdmin { if_id, up } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let op_desc = if up {
                    "set interface up"
                } else {
                    "set interface down"
                };
                let result = apply_link_set(&handle, if_id, |builder| {
                    if up { builder.up() } else { builder.down() }
                })
                .await;

                respond(map_link_result(result, op_desc, if_id));
            }
            RtnlLinkRequest::InterfaceSetPromisc { if_id, enable } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let op_desc = if enable {
                    "enable promiscuous mode"
                } else {
                    "disable promiscuous mode"
                };
                let result =
                    apply_link_set(&handle, if_id, |builder| builder.promiscuous(enable)).await;

                respond(map_link_result(result, op_desc, if_id));
            }
            RtnlLinkRequest::InterfaceSetArp { if_id, enable } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let op_desc = if enable { "enable ARP" } else { "disable ARP" };
                let result = apply_link_set(&handle, if_id, |builder| builder.arp(enable)).await;

                respond(map_link_result(result, op_desc, if_id));
            }
            RtnlLinkRequest::InterfaceSetMtu { if_id, mtu } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let result = apply_link_set(&handle, if_id, |builder| builder.mtu(mtu)).await;
                respond(map_link_result(result, "set MTU", if_id));
            }
            RtnlLinkRequest::InterfaceRename { if_id, if_name } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let new_name = if_name.clone();
                let result = apply_link_set(&handle, if_id, |builder| builder.name(new_name)).await;
                let op_desc = format!("rename interface to {}", if_name);
                respond(map_link_result(result, &op_desc, if_id));
            }
            RtnlLinkRequest::InterfaceSetAllMulticast { if_id, enable } => {
                if if_id == 0 {
                    respond(RtnlLinkResponse::NotFound);
                    continue 'reqloop;
                }

                let op_desc = if enable {
                    "enable all-multicast mode"
                } else {
                    "disable all-multicast mode"
                };

                let mut message = LinkMessageBuilder::<LinkUnspec>::new().index(if_id).build();
                if enable {
                    message.header.flags |= LinkFlags::Allmulti;
                } else {
                    message.header.flags.remove(LinkFlags::Allmulti);
                }
                message.header.change_mask |= LinkFlags::Allmulti;

                let result = handle.set(message).execute().await;

                respond(map_link_result(result, op_desc, if_id));
            }
            _ => respond(RtnlLinkResponse::NotImplemented),
        }
    }
}
