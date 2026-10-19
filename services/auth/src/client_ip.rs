use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientIpError {
    MissingForwardedIp,
    InvalidForwardedIp,
}

fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ipv6) => match ipv6.to_ipv4_mapped() {
            Some(ipv4) => IpAddr::V4(ipv4),
            None => IpAddr::V6(ipv6),
        },
        IpAddr::V4(ipv4) => IpAddr::V4(ipv4),
    }
}

pub fn resolve_client_ip(
    peer: SocketAddr,
    headers: &HeaderMap,
    trusted_proxies: &[IpAddr],
) -> Result<IpAddr, ClientIpError> {
    let peer_ip = normalize_ip(peer.ip());

    let is_trusted_proxy = trusted_proxies
        .iter()
        .any(|trusted| normalize_ip(*trusted) == peer_ip);

    if !is_trusted_proxy {
        return Ok(peer_ip);
    }

    let mut forwarded_values = headers.get_all("x-real-ip").iter();

    let value = forwarded_values
        .next()
        .ok_or(ClientIpError::MissingForwardedIp)?;

    if forwarded_values.next().is_some() {
        return Err(ClientIpError::InvalidForwardedIp);
    }

    let forwarded_ip = value
        .to_str()
        .map_err(|_| ClientIpError::InvalidForwardedIp)?
        .parse::<IpAddr>()
        .map_err(|_| ClientIpError::InvalidForwardedIp)?;

    Ok(normalize_ip(forwarded_ip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn direct_client_cannot_override_its_ip() {
        let peer = "192.0.2.10:45000".parse().unwrap();
        let trusted_proxies = ["192.0.2.20".parse().unwrap()];
        let mut headers = HeaderMap::new();

        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.99"));
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("198.51.100.100"),
        );

        let result = resolve_client_ip(peer, &headers, &trusted_proxies);

        assert_eq!(result, Ok("192.0.2.10".parse().unwrap()));
    }

    #[test]
    fn trusted_proxy_can_supply_client_ip() {
        let peer = "192.0.2.20:45000".parse().unwrap();
        let trusted_proxies = ["192.0.2.20".parse().unwrap()];
        let mut headers = HeaderMap::new();

        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.10"));

        let result = resolve_client_ip(peer, &headers, &trusted_proxies);

        assert_eq!(result, Ok("198.51.100.10".parse().unwrap()));
    }

    #[test]
    fn trusted_proxy_must_supply_client_ip() {
        let peer = "192.0.2.20:45000".parse().unwrap();
        let trusted_proxies = ["192.0.2.20".parse().unwrap()];

        let result = resolve_client_ip(peer, &HeaderMap::new(), &trusted_proxies);

        assert_eq!(result, Err(ClientIpError::MissingForwardedIp));
    }

    #[test]
    fn trusted_proxy_cannot_supply_invalid_or_multiple_ips() {
        let peer = "192.0.2.20:45000".parse().unwrap();
        let trusted_proxies = ["192.0.2.20".parse().unwrap()];

        for value in ["not-an-ip", "198.51.100.10, 198.51.100.11"] {
            let mut headers = HeaderMap::new();
            headers.insert("x-real-ip", HeaderValue::from_str(value).unwrap());

            assert_eq!(
                resolve_client_ip(peer, &headers, &trusted_proxies),
                Err(ClientIpError::InvalidForwardedIp)
            );
        }

        let mut headers = HeaderMap::new();
        headers.append("x-real-ip", HeaderValue::from_static("198.51.100.10"));
        headers.append("x-real-ip", HeaderValue::from_static("198.51.100.11"));

        assert_eq!(
            resolve_client_ip(peer, &headers, &trusted_proxies),
            Err(ClientIpError::InvalidForwardedIp)
        );
    }

    #[test]
    fn ipv4_mapped_ipv6_has_the_same_identity_as_ipv4() {
        let peer = "[::ffff:192.0.2.10]:45000".parse().unwrap();

        let result = resolve_client_ip(peer, &HeaderMap::new(), &[]);

        assert_eq!(result, Ok("192.0.2.10".parse().unwrap()));
    }
}