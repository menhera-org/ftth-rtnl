#![allow(unreachable_patterns)]

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};

use futures::TryStreamExt;

use std::fmt::{Debug, Display};

pub(crate) type Client = AsyncWorldClient<RtnlLinkRequest, RtnlLinkResponse>;
pub(crate) type Server = AsyncWorldServer<RtnlLinkRequest, RtnlLinkResponse>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MacAddr {
    pub inner: [u8; 6],
}

impl MacAddr {
    pub const fn new(inner: [u8; 6]) -> Self {
        Self {
            inner,
        }
    }
}

impl Default for MacAddr {
    fn default() -> Self {
        Self {
            inner: [0; 6],
        }
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
    InterfaceGet {
        if_id: u32,
    },
    InterfaceGetByName {
        if_name: String,
    },
    MacAddrGet {
        if_id: u32,
    },
    MacAddrSet {
        if_id: u32,
        mac_addr: MacAddr,
    },
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RtnlLinkClient {
    client: Client,
}

impl RtnlLinkClient {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
        }
    }

    pub fn interface_get_by_name(&self, name: &str) -> std::io::Result<Interface> {
        let name = name.to_owned();
        let res = self.client.send_request(RtnlLinkRequest::InterfaceGetByName { if_name: name })?;
        match res {
            RtnlLinkResponse::Interface(interface) => {
                return Ok(interface);
            },
            _ => {},
        }
        Err(std::io::Error::other("Not found"))
    }

    pub fn mac_addr_get(&self, if_id: u32) -> std::io::Result<Option<MacAddr>> {
        let res = self.client.send_request(RtnlLinkRequest::MacAddrGet { if_id })?;
        match res {
            RtnlLinkResponse::MacAddr(addr) => {
                return Ok(Some(addr));
            },
            _ => {},
        }
        Ok(None)
    }

    pub fn interface_list(&self) -> std::io::Result<Vec<Interface>> {
        let res = self.client.send_request(RtnlLinkRequest::InterfaceList)?;
        match res {
            RtnlLinkResponse::InterfaceList(list) => {
                return Ok(list);
            },
            _ => {},
        }
        Err(std::io::Error::other("Unknown error"))
    }
}

pub(crate) async fn run_server(mut server: Server, mut handle: rtnetlink::LinkHandle) {
    'reqloop: while let Some((req, respond)) = server.accept().await {
        match req {
            RtnlLinkRequest::InterfaceGetByName { if_name } => {
                let response = handle.get().match_name(if_name.to_owned()).execute();
                futures::pin_mut!(response);
                while let Ok(Some(response)) = response.try_next().await {
                    let if_index = response.header.index;
                    if if_index == 0 {
                        continue;
                    }

                    respond(RtnlLinkResponse::Interface(Interface { if_id: if_index, if_name: if_name.to_owned() }));
                    continue 'reqloop;
                }
                respond(RtnlLinkResponse::NotFound);
            },
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
                                respond(RtnlLinkResponse::MacAddr(MacAddr::new(addr[0..6].try_into().unwrap_or([0; 6]))));
                                continue 'reqloop;
                            }
                            _ => {}
                        }
                    }
                }
                respond(RtnlLinkResponse::NotFound);
            },
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

                    interfaces.push(Interface { if_id: if_index, if_name: if_name.unwrap() });
                }
                respond(RtnlLinkResponse::InterfaceList(interfaces));
            }
            _ => respond(RtnlLinkResponse::NotImplemented),
        }
    }
}
