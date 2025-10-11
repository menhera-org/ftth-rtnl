pub mod address;
pub mod link;
pub mod neighbor;
pub mod route;
pub mod virtual_interface;

use std::any::Any;
use std::sync::{Arc, Mutex, OnceLock};

pub use ipnet::{IpNet, Ipv4Net, Ipv6Net};
pub use neighbor::{NeighborDelete, NeighborEntry};
pub use netlink_packet_route::address::AddressScope;
pub use netlink_packet_route::neighbour::{NeighbourFlags, NeighbourState};
pub use netlink_packet_route::route::RouteNextHopFlags;
pub use route::{Ipv4Route, Ipv6Route, RouteNextHopInfo};
pub use virtual_interface::{
    Gre6Config, GreConfig, Ip6TnlConfig, IpIpConfig, VirtualInterfaceDelete, VirtualInterfaceKind,
    VirtualInterfaceSpec, VirtualInterfaceUpdate, VlanConfig,
};

use ftth_common::channel::create_pair;

use futures::{FutureExt, future::join_all};


static CLIENT: OnceLock<RtnlClient> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct RtnlClient {
    address: address::RtnlAddressClient,
    link: link::RtnlLinkClient,
    neighbor: neighbor::RtnlNeighborClient,
    route: route::RtnlRouteClient,
    virtual_interface: virtual_interface::RtnlVirtualInterfaceClient,
    
    #[allow(dead_code)]
    receiver: Arc<Mutex<Option<Box<dyn Any + Send>>>>,
}

impl RtnlClient {
    pub fn new() -> Self {
        CLIENT.get_or_init(|| Self::new_inner()).clone()
    }

    pub(crate) fn new_inner() -> Self {
        let (address_tx, address_rx) = create_pair();
        let (link_tx, link_rx) = create_pair();
        let (neighbor_tx, neighbor_rx) = create_pair();
        let (route_tx, route_rx) = create_pair();
        let (virtual_interface_tx, virtual_interface_rx) = create_pair();

        let receiver_container = Arc::new(Mutex::new(None));
        let receiver_container_clone = receiver_container.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Tokio runtime building error: {}", e);
                    return;
                }
            };

            let _ = rt.block_on(async {
                let (connection, handle, receiver) = rtnetlink::new_connection()?;

                {
                    *(receiver_container_clone.lock().map_err(|_e| std::io::Error::other("Poison error"))?) = Some(Box::new(receiver) as Box<dyn Any + Send>);
                }

                tokio::spawn(connection);

                let mut futures = Vec::new();
                futures.push(address::run_server(address_rx, handle.address()).boxed());
                futures.push(link::run_server(link_rx, handle.link()).boxed());
                futures.push(neighbor::run_server(neighbor_rx, handle.neighbours()).boxed());
                futures.push(route::run_server(route_rx, handle.route()).boxed());
                futures.push(
                    virtual_interface::run_server(virtual_interface_rx, handle.link()).boxed(),
                );

                join_all(futures).await;



                Ok::<(), std::io::Error>(())
            });
        });

        Self {
            address: address::RtnlAddressClient::new(address_tx),
            link: link::RtnlLinkClient::new(link_tx),
            neighbor: neighbor::RtnlNeighborClient::new(neighbor_tx),
            route: route::RtnlRouteClient::new(route_tx),
            virtual_interface: virtual_interface::RtnlVirtualInterfaceClient::new(
                virtual_interface_tx,
            ),
            receiver: receiver_container,
        }
    }

    pub fn address(&self) -> address::RtnlAddressClient {
        self.address.clone()
    }

    pub fn link(&self) -> link::RtnlLinkClient {
        self.link.clone()
    }

    pub fn neighbor(&self) -> neighbor::RtnlNeighborClient {
        self.neighbor.clone()
    }

    pub fn route(&self) -> route::RtnlRouteClient {
        self.route.clone()
    }

    pub fn virtual_interface(&self) -> virtual_interface::RtnlVirtualInterfaceClient {
        self.virtual_interface.clone()
    }
}
