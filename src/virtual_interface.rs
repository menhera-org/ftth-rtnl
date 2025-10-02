#![allow(unreachable_patterns)]

use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr};

use ftth_common::channel::{AsyncWorldClient, AsyncWorldServer};
use futures::TryStreamExt;
use netlink_packet_core::{DefaultNla, Nla};
use netlink_packet_route::link::{
    InfoData, InfoGreTap, InfoGreTap6, InfoGreTun, InfoGreTun6, InfoKind, InfoVlan, LinkMessage,
};
use rtnetlink::{LinkMessageBuilder, LinkUnspec};

pub(crate) type Client =
    AsyncWorldClient<RtnlVirtualInterfaceRequest, RtnlVirtualInterfaceResponse>;
pub(crate) type Server =
    AsyncWorldServer<RtnlVirtualInterfaceRequest, RtnlVirtualInterfaceResponse>;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlVirtualInterfaceRequest {
    Create(VirtualInterfaceSpec),
    Configure(VirtualInterfaceUpdate),
    Delete(VirtualInterfaceDelete),
    GetIndexByName(String),
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtnlVirtualInterfaceResponse {
    Success,
    Failed,
    NotFound,
    Index(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RtnlVirtualInterfaceClient {
    client: Client,
}

impl RtnlVirtualInterfaceClient {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn create(&self, spec: VirtualInterfaceSpec) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlVirtualInterfaceRequest::Create(spec))?;
        handle_basic_response("Create virtual interface", res)
    }

    pub fn configure(&self, update: VirtualInterfaceUpdate) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlVirtualInterfaceRequest::Configure(update))?;
        handle_basic_response("Configure virtual interface", res)
    }

    pub fn delete(&self, delete: VirtualInterfaceDelete) -> io::Result<()> {
        let res = self
            .client
            .send_request(RtnlVirtualInterfaceRequest::Delete(delete))?;
        handle_basic_response("Delete virtual interface", res)
    }

    pub fn get_index_by_name(&self, name: &str) -> io::Result<u32> {
        match self
            .client
            .send_request(RtnlVirtualInterfaceRequest::GetIndexByName(
                name.to_string(),
            ))? {
            RtnlVirtualInterfaceResponse::Index(index) => Ok(index),
            RtnlVirtualInterfaceResponse::NotFound => Err(io::Error::new(
                ErrorKind::NotFound,
                format!("Virtual interface {name} not found"),
            )),
            other => Err(io::Error::other(format!(
                "Unexpected response while fetching index: {:?}",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualInterfaceSpec {
    pub name: String,
    pub kind: VirtualInterfaceKind,
    pub admin_up: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualInterfaceUpdate {
    pub if_id: u32,
    pub new_name: Option<String>,
    pub kind: VirtualInterfaceKind,
    pub admin_up: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VirtualInterfaceDelete {
    ByIndex(u32),
    ByName(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum VirtualInterfaceKind {
    Gre(GreConfig),
    Gretap(GreConfig),
    Ip6Gre(Gre6Config),
    Ip6Gretap(Gre6Config),
    IpIp(IpIpConfig),
    Ip6Tnl(Ip6TnlConfig),
    Vlan(VlanConfig),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GreConfig {
    pub local: Ipv4Addr,
    pub remote: Ipv4Addr,
    pub ttl: Option<u8>,
    pub tos: Option<u8>,
    pub key: Option<u32>,
    pub encap_limit: Option<u8>,
    pub pmtudisc: bool,
    pub ignore_df: bool,
    pub link: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gre6Config {
    pub local: Ipv6Addr,
    pub remote: Ipv6Addr,
    pub hop_limit: Option<u8>,
    pub traffic_class: Option<u8>,
    pub key: Option<u32>,
    pub encap_limit: Option<u8>,
    pub pmtudisc: bool,
    pub ignore_df: bool,
    pub link: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IpIpConfig {
    pub local: Ipv4Addr,
    pub remote: Ipv4Addr,
    pub ttl: Option<u8>,
    pub tos: Option<u8>,
    pub encap_limit: Option<u8>,
    pub pmtudisc: bool,
    pub link: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ip6TnlConfig {
    pub local: Ipv6Addr,
    pub remote: Ipv6Addr,
    pub hop_limit: Option<u8>,
    pub traffic_class: Option<u8>,
    pub flow_label: Option<u32>,
    pub encap_limit: Option<u8>,
    pub pmtudisc: bool,
    pub link: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VlanConfig {
    pub base_ifindex: Option<u32>,
    pub vlan_id: Option<u16>,
}

const IFLA_GRE_LINK: u16 = 1;
const IFLA_GRE_IKEY: u16 = 4;
const IFLA_GRE_OKEY: u16 = 5;
const IFLA_GRE_LOCAL: u16 = 6;
const IFLA_GRE_REMOTE: u16 = 7;
const IFLA_GRE_TTL: u16 = 8;
const IFLA_GRE_TOS: u16 = 9;
const IFLA_GRE_PMTUDISC: u16 = 10;
const IFLA_GRE_ENCAP_LIMIT: u16 = 11;
const IFLA_GRE_IGNORE_DF: u16 = 19;

const IFLA_IPTUN_LINK: u16 = 1;
const IFLA_IPTUN_LOCAL: u16 = 2;
const IFLA_IPTUN_REMOTE: u16 = 3;
const IFLA_IPTUN_TTL: u16 = 4;
const IFLA_IPTUN_TOS: u16 = 5;
const IFLA_IPTUN_ENCAP_LIMIT: u16 = 6;
const IFLA_IPTUN_FLOWINFO: u16 = 7;
const IFLA_IPTUN_PMTUDISC: u16 = 10;

const NLA_HEADER_LEN: usize = 4;
const NLA_ALIGNTO: usize = 4;

fn align_nla(len: usize) -> usize {
    (len + NLA_ALIGNTO - 1) & !(NLA_ALIGNTO - 1)
}

fn handle_basic_response(op: &str, response: RtnlVirtualInterfaceResponse) -> io::Result<()> {
    match response {
        RtnlVirtualInterfaceResponse::Success => Ok(()),
        RtnlVirtualInterfaceResponse::NotFound => Err(io::Error::new(
            ErrorKind::NotFound,
            format!("{}: target not found", op),
        )),
        RtnlVirtualInterfaceResponse::Failed => Err(io::Error::other(format!("{} failed", op))),
        other => Err(io::Error::other(format!(
            "{} returned unexpected response: {:?}",
            op, other
        ))),
    }
}

pub(crate) async fn run_server(mut server: Server, mut handle: rtnetlink::LinkHandle) {
    while let Some((req, respond)) = server.accept().await {
        match req {
            RtnlVirtualInterfaceRequest::Create(spec) => {
                let message = match build_create_message(&spec) {
                    Ok(msg) => msg,
                    Err(err) => {
                        log::warn!("Failed to build virtual interface {}: {}", spec.name, err);
                        respond(RtnlVirtualInterfaceResponse::Failed);
                        continue;
                    }
                };

                match handle.add(message).execute().await {
                    Ok(()) => respond(RtnlVirtualInterfaceResponse::Success),
                    Err(rtnetlink::Error::NetlinkError(err_msg)) => {
                        log::warn!(
                            "Netlink error creating virtual interface {}: {}",
                            spec.name,
                            err_msg
                        );
                        respond(netlink_error_to_response(err_msg.to_io()));
                    }
                    Err(err) => {
                        log::warn!("Failed to create virtual interface {}: {}", spec.name, err);
                        respond(RtnlVirtualInterfaceResponse::Failed);
                    }
                }
            }
            RtnlVirtualInterfaceRequest::Configure(update) => {
                let message = match build_update_message(&update) {
                    Ok(msg) => msg,
                    Err(err) => {
                        log::warn!(
                            "Failed to build virtual interface update for {}: {}",
                            update.if_id,
                            err
                        );
                        respond(RtnlVirtualInterfaceResponse::Failed);
                        continue;
                    }
                };

                match handle.set(message).execute().await {
                    Ok(()) => respond(RtnlVirtualInterfaceResponse::Success),
                    Err(rtnetlink::Error::NetlinkError(err_msg)) => {
                        respond(netlink_error_to_response(err_msg.to_io()));
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to configure virtual interface {}: {}",
                            update.if_id,
                            err
                        );
                        respond(RtnlVirtualInterfaceResponse::Failed);
                    }
                }
            }
            RtnlVirtualInterfaceRequest::Delete(delete) => {
                let result = match resolve_delete_target(&mut handle, &delete).await {
                    Ok(index) => handle.del(index).execute().await.map_err(|err| match err {
                        rtnetlink::Error::NetlinkError(e) => e.to_io(),
                        other => io::Error::other(other.to_string()),
                    }),
                    Err(err) => Err(err),
                };

                match result {
                    Ok(()) => respond(RtnlVirtualInterfaceResponse::Success),
                    Err(err) if err.kind() == ErrorKind::NotFound => {
                        respond(RtnlVirtualInterfaceResponse::NotFound)
                    }
                    Err(err) => {
                        log::warn!("Failed to delete virtual interface: {}", err);
                        respond(RtnlVirtualInterfaceResponse::Failed);
                    }
                }
            }
            RtnlVirtualInterfaceRequest::GetIndexByName(name) => {
                match resolve_index_by_name(&mut handle, &name).await {
                    Ok(Some(index)) => respond(RtnlVirtualInterfaceResponse::Index(index)),
                    Ok(None) => respond(RtnlVirtualInterfaceResponse::NotFound),
                    Err(err) => {
                        log::warn!("Failed to resolve virtual interface {}: {}", name, err);
                        respond(RtnlVirtualInterfaceResponse::Failed);
                    }
                }
            }
        }
    }
}

fn netlink_error_to_response(err: io::Error) -> RtnlVirtualInterfaceResponse {
    match err.kind() {
        ErrorKind::NotFound => RtnlVirtualInterfaceResponse::NotFound,
        _ => RtnlVirtualInterfaceResponse::Failed,
    }
}

fn build_create_message(spec: &VirtualInterfaceSpec) -> io::Result<LinkMessage> {
    validate_create_kind(&spec.kind)?;
    let info_kind = virtual_interface_kind_to_info_kind(&spec.kind);
    let mut builder = LinkMessageBuilder::<LinkUnspec>::new_with_info_kind(info_kind)
        .name(spec.name.clone())
        .set_info_data(build_info_data(&spec.kind)?);

    if spec.admin_up {
        builder = builder.up();
    }

    if let Some(link) = virtual_interface_link(&spec.kind) {
        builder = builder.link(link);
    }

    Ok(builder.build())
}

fn validate_create_kind(kind: &VirtualInterfaceKind) -> io::Result<()> {
    match kind {
        VirtualInterfaceKind::Vlan(cfg) => {
            if cfg.base_ifindex.is_none() {
                return Err(io::Error::other(
                    "VLAN creation requires a parent interface (--dev)",
                ));
            }
            if cfg.vlan_id.is_none() {
                return Err(io::Error::other("VLAN creation requires --vlan-id"));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn build_update_message(update: &VirtualInterfaceUpdate) -> io::Result<LinkMessage> {
    let info_kind = virtual_interface_kind_to_info_kind(&update.kind);
    let mut builder = LinkMessageBuilder::<LinkUnspec>::new_with_info_kind(info_kind)
        .index(update.if_id)
        .set_info_data(build_info_data(&update.kind)?);

    if let Some(name) = &update.new_name {
        builder = builder.name(name.clone());
    }

    if let Some(link) = virtual_interface_link(&update.kind) {
        builder = builder.link(link);
    }

    if let Some(up) = update.admin_up {
        builder = if up { builder.up() } else { builder.down() };
    }

    Ok(builder.build())
}

async fn resolve_delete_target(
    handle: &mut rtnetlink::LinkHandle,
    delete: &VirtualInterfaceDelete,
) -> io::Result<u32> {
    match delete {
        VirtualInterfaceDelete::ByIndex(index) => Ok(*index),
        VirtualInterfaceDelete::ByName(name) => resolve_index_by_name(handle, name)
            .await?
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Virtual interface not found")),
    }
}

async fn resolve_index_by_name(
    handle: &mut rtnetlink::LinkHandle,
    name: &str,
) -> io::Result<Option<u32>> {
    let response = handle.get().match_name(name.to_string()).execute();
    futures::pin_mut!(response);
    while let Ok(Some(msg)) = response.try_next().await {
        if msg.header.index != 0 {
            return Ok(Some(msg.header.index));
        }
    }
    Ok(None)
}

fn virtual_interface_kind_to_info_kind(kind: &VirtualInterfaceKind) -> InfoKind {
    match kind {
        VirtualInterfaceKind::Gre(_) => InfoKind::GreTun,
        VirtualInterfaceKind::Gretap(_) => InfoKind::GreTap,
        VirtualInterfaceKind::Ip6Gre(_) => InfoKind::GreTun6,
        VirtualInterfaceKind::Ip6Gretap(_) => InfoKind::GreTap6,
        VirtualInterfaceKind::IpIp(_) => InfoKind::IpTun,
        VirtualInterfaceKind::Ip6Tnl(_) => InfoKind::Other("ip6tnl".into()),
        VirtualInterfaceKind::Vlan(_) => InfoKind::Vlan,
    }
}

fn virtual_interface_link(kind: &VirtualInterfaceKind) -> Option<u32> {
    match kind {
        VirtualInterfaceKind::Gre(cfg) | VirtualInterfaceKind::Gretap(cfg) => cfg.link,
        VirtualInterfaceKind::Ip6Gre(cfg) | VirtualInterfaceKind::Ip6Gretap(cfg) => cfg.link,
        VirtualInterfaceKind::IpIp(cfg) => cfg.link,
        VirtualInterfaceKind::Ip6Tnl(cfg) => cfg.link,
        VirtualInterfaceKind::Vlan(cfg) => cfg.base_ifindex,
    }
}

fn build_info_data(kind: &VirtualInterfaceKind) -> io::Result<InfoData> {
    match kind {
        VirtualInterfaceKind::Gre(cfg) => Ok(InfoData::GreTun(
            gre_nlas(cfg).into_iter().map(InfoGreTun::Other).collect(),
        )),
        VirtualInterfaceKind::Gretap(cfg) => Ok(InfoData::GreTap(
            gre_nlas(cfg).into_iter().map(InfoGreTap::Other).collect(),
        )),
        VirtualInterfaceKind::Ip6Gre(cfg) => Ok(InfoData::GreTun6(
            gre6_nlas(cfg).into_iter().map(InfoGreTun6::Other).collect(),
        )),
        VirtualInterfaceKind::Ip6Gretap(cfg) => Ok(InfoData::GreTap6(
            gre6_nlas(cfg).into_iter().map(InfoGreTap6::Other).collect(),
        )),
        VirtualInterfaceKind::IpIp(cfg) => {
            let nlas = iptunnel_v4_nlas(cfg);
            Ok(InfoData::Other(encode_default_nlas(&nlas)))
        }
        VirtualInterfaceKind::Ip6Tnl(cfg) => {
            let nlas = iptunnel_v6_nlas(cfg);
            Ok(InfoData::Other(encode_default_nlas(&nlas)))
        }
        VirtualInterfaceKind::Vlan(cfg) => {
            let mut infos = Vec::new();
            if let Some(id) = cfg.vlan_id {
                infos.push(InfoVlan::Id(id));
            }
            Ok(InfoData::Vlan(infos))
        }
    }
}

fn gre_nlas(cfg: &GreConfig) -> Vec<DefaultNla> {
    let mut nlas = Vec::new();
    nlas.push(DefaultNla::new(IFLA_GRE_LOCAL, cfg.local.octets().to_vec()));
    nlas.push(DefaultNla::new(
        IFLA_GRE_REMOTE,
        cfg.remote.octets().to_vec(),
    ));

    if let Some(ttl) = cfg.ttl {
        nlas.push(DefaultNla::new(IFLA_GRE_TTL, vec![ttl]));
    }

    if let Some(tos) = cfg.tos {
        nlas.push(DefaultNla::new(IFLA_GRE_TOS, vec![tos]));
    }

    if let Some(key) = cfg.key {
        let bytes = key.to_be_bytes().to_vec();
        nlas.push(DefaultNla::new(IFLA_GRE_IKEY, bytes.clone()));
        nlas.push(DefaultNla::new(IFLA_GRE_OKEY, bytes));
    }

    let limit = cfg.encap_limit.unwrap_or(0xff);
    nlas.push(DefaultNla::new(IFLA_GRE_ENCAP_LIMIT, vec![limit]));

    nlas.push(DefaultNla::new(
        IFLA_GRE_PMTUDISC,
        vec![if cfg.pmtudisc { 1 } else { 0 }],
    ));

    nlas.push(DefaultNla::new(
        IFLA_GRE_IGNORE_DF,
        vec![if cfg.ignore_df { 1 } else { 0 }],
    ));

    if let Some(link) = cfg.link {
        nlas.push(DefaultNla::new(IFLA_GRE_LINK, link.to_ne_bytes().to_vec()));
    }

    nlas
}

fn gre6_nlas(cfg: &Gre6Config) -> Vec<DefaultNla> {
    let mut nlas = Vec::new();
    nlas.push(DefaultNla::new(IFLA_GRE_LOCAL, cfg.local.octets().to_vec()));
    nlas.push(DefaultNla::new(
        IFLA_GRE_REMOTE,
        cfg.remote.octets().to_vec(),
    ));

    if let Some(hop) = cfg.hop_limit {
        nlas.push(DefaultNla::new(IFLA_GRE_TTL, vec![hop]));
    }

    if let Some(tc) = cfg.traffic_class {
        nlas.push(DefaultNla::new(IFLA_GRE_TOS, vec![tc]));
    }

    if let Some(key) = cfg.key {
        let bytes = key.to_be_bytes().to_vec();
        nlas.push(DefaultNla::new(IFLA_GRE_IKEY, bytes.clone()));
        nlas.push(DefaultNla::new(IFLA_GRE_OKEY, bytes));
    }

    let limit = cfg.encap_limit.unwrap_or(0xff);
    nlas.push(DefaultNla::new(IFLA_GRE_ENCAP_LIMIT, vec![limit]));

    nlas.push(DefaultNla::new(
        IFLA_GRE_PMTUDISC,
        vec![if cfg.pmtudisc { 1 } else { 0 }],
    ));

    nlas.push(DefaultNla::new(
        IFLA_GRE_IGNORE_DF,
        vec![if cfg.ignore_df { 1 } else { 0 }],
    ));

    if let Some(link) = cfg.link {
        nlas.push(DefaultNla::new(IFLA_GRE_LINK, link.to_ne_bytes().to_vec()));
    }

    nlas
}

fn iptunnel_v4_nlas(cfg: &IpIpConfig) -> Vec<DefaultNla> {
    let mut nlas = Vec::new();
    nlas.push(DefaultNla::new(
        IFLA_IPTUN_LOCAL,
        cfg.local.octets().to_vec(),
    ));
    nlas.push(DefaultNla::new(
        IFLA_IPTUN_REMOTE,
        cfg.remote.octets().to_vec(),
    ));

    if let Some(ttl) = cfg.ttl {
        nlas.push(DefaultNla::new(IFLA_IPTUN_TTL, vec![ttl]));
    }

    if let Some(tos) = cfg.tos {
        nlas.push(DefaultNla::new(IFLA_IPTUN_TOS, vec![tos]));
    }

    let limit = cfg.encap_limit.unwrap_or(0xff);
    nlas.push(DefaultNla::new(IFLA_IPTUN_ENCAP_LIMIT, vec![limit]));

    nlas.push(DefaultNla::new(
        IFLA_IPTUN_PMTUDISC,
        vec![if cfg.pmtudisc { 1 } else { 0 }],
    ));

    if let Some(link) = cfg.link {
        nlas.push(DefaultNla::new(
            IFLA_IPTUN_LINK,
            link.to_ne_bytes().to_vec(),
        ));
    }

    nlas
}

fn iptunnel_v6_nlas(cfg: &Ip6TnlConfig) -> Vec<DefaultNla> {
    let mut nlas = Vec::new();
    nlas.push(DefaultNla::new(
        IFLA_IPTUN_LOCAL,
        cfg.local.octets().to_vec(),
    ));
    nlas.push(DefaultNla::new(
        IFLA_IPTUN_REMOTE,
        cfg.remote.octets().to_vec(),
    ));

    if let Some(hop) = cfg.hop_limit {
        nlas.push(DefaultNla::new(IFLA_IPTUN_TTL, vec![hop]));
    }

    if let Some(tc) = cfg.traffic_class {
        nlas.push(DefaultNla::new(IFLA_IPTUN_TOS, vec![tc]));
    }

    if let Some(flow) = cfg.flow_label {
        nlas.push(DefaultNla::new(
            IFLA_IPTUN_FLOWINFO,
            flow.to_be_bytes().to_vec(),
        ));
    }

    let limit = cfg.encap_limit.unwrap_or(0xff);
    nlas.push(DefaultNla::new(IFLA_IPTUN_ENCAP_LIMIT, vec![limit]));

    nlas.push(DefaultNla::new(
        IFLA_IPTUN_PMTUDISC,
        vec![if cfg.pmtudisc { 1 } else { 0 }],
    ));

    if let Some(link) = cfg.link {
        nlas.push(DefaultNla::new(
            IFLA_IPTUN_LINK,
            link.to_ne_bytes().to_vec(),
        ));
    }

    nlas
}

fn encode_default_nlas(nlas: &[DefaultNla]) -> Vec<u8> {
    let mut buffer = Vec::new();
    for nla in nlas {
        let value_len = nla.value_len();
        let payload_len = value_len + NLA_HEADER_LEN;
        let aligned_len = align_nla(payload_len);
        let start = buffer.len();
        buffer.resize(start + aligned_len, 0);
        let target = &mut buffer[start..start + aligned_len];
        let len_bytes = (payload_len as u16).to_ne_bytes();
        target[0..2].copy_from_slice(&len_bytes);
        let kind_bytes = (nla.kind() as u16).to_ne_bytes();
        target[2..4].copy_from_slice(&kind_bytes);
        nla.emit_value(&mut target[4..payload_len]);
        // trailing padding already zeroed by resize
    }
    buffer
}
