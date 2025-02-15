/// Get the machine's public IP using [`getifaddrs`].
pub fn public_ip() -> Vec<String> {
    let addrs = nix::ifaddrs::getifaddrs().unwrap();
    let mut out: Vec<String> = Vec::new();

    for ifaddr in addrs {
        if let Some(address) = ifaddr.address {
            out.push(address.to_string().replace(":0", ""));
        }
    }

    out
}
