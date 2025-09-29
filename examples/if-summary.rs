use ftth_rtnl::RtnlClient;


fn main() -> std::io::Result<()> {
    let client = RtnlClient::new();
    let link_client = client.link();
    let addr_client = client.address();
    let ifs = link_client.interface_list()?;
    for interface in ifs {
        println!("Interface: {} ({})", &interface.if_name, interface.if_id);
        let if_id = interface.if_id;
        let mac = link_client.mac_addr_get(if_id)?;
        if let Some(mac) = mac {
            println!("  MAC address: {}", mac);
        } else {
            println!("  No MAC address");
        }
        let v4addrs = addr_client.ipv4_addrs_get(Some(if_id))?;
        println!("  IPv4 addresses:");
        for addr in v4addrs {
            println!("    {}", addr);
        }
        let v6addrs = addr_client.ipv6_addrs_get(Some(if_id))?;
        println!("  IPv6 addresses:");
        for addr in v6addrs {
            println!("    {}", addr);
        }
        println!("");
    }
    Ok(())
}
