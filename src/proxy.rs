use hyper::{
    Body, Client, Method, Request, Response, Server,
    client::HttpConnector,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
};
use rand::Rng;
use std::net::{IpAddr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpSocket,
    time::{Duration, timeout},
};

pub async fn start_proxy(
    listen_addr: SocketAddr,
    (ipv6, prefix_len): (Ipv6Addr, u8),
) -> Result<(), Box<dyn std::error::Error>> {
    let make_service = make_service_fn(move |_: &AddrStream| async move {
        Ok::<_, hyper::Error>(service_fn(move |req| {
            Proxy {
                ipv6: ipv6.into(),
                prefix_len,
            }
            .proxy(req)
        }))
    });

    Server::bind(&listen_addr)
        .http1_preserve_header_case(true)
        .http1_title_case_headers(true)
        .serve(make_service)
        .await
        .map_err(|err| err.into())
}

#[derive(Clone)]
pub(crate) struct Proxy {
    pub ipv6: u128,
    pub prefix_len: u8,
}

impl Proxy {
    pub(crate) async fn proxy(self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        match if req.method() == Method::CONNECT {
            self.process_connect(req).await
        } else {
            self.process_request(req).await
        } {
            Ok(resp) => Ok(resp),
            Err(e) => Err(e),
        }
    }

    async fn process_connect(self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        tokio::task::spawn(async move {
            let remote_addr = req.uri().authority().map(|auth| auth.to_string()).unwrap();
            let mut upgraded = hyper::upgrade::on(req).await.unwrap();
            self.tunnel(&mut upgraded, remote_addr).await
        });
        Ok(Response::new(Body::empty()))
    }

    async fn process_request(self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let mut http = HttpConnector::new();

        if let Some(host) = req.uri().host() {
            if let Ok(addrs) = (host, 0).to_socket_addrs() {
                let has_ipv6 = addrs.filter(|addr| addr.is_ipv6()).next().is_some();
                if has_ipv6 {
                    let bind_addr = get_rand_ipv6(self.ipv6, self.prefix_len);
                    http.set_local_address(Some(bind_addr));
                }
            }
        }

        let client = Client::builder()
            .http1_title_case_headers(true)
            .http1_preserve_header_case(true)
            .build(http);
        let res = client.request(req).await?;
        Ok(res)
    }

    async fn tunnel<A>(self, upgraded: &mut A, addr_str: String) -> std::io::Result<()>
    where
        A: AsyncRead + AsyncWrite + Unpin + ?Sized,
    {
        if let Ok(addrs) = addr_str.to_socket_addrs() {
            let addrs: Vec<_> = addrs.collect();

            if let Some(ipv6_addr) = addrs.iter().find(|addr| addr.is_ipv6()) {
                let socket = TcpSocket::new_v6()?;
                let bind_addr = get_rand_ipv6_socket_addr(self.ipv6, self.prefix_len);

                if socket.bind(bind_addr).is_ok() {
                    match timeout(Duration::from_millis(300), socket.connect(*ipv6_addr)).await {
                        Ok(Ok(mut server)) => {
                            let _ = tokio::io::copy_bidirectional(upgraded, &mut server).await?;
                            return Ok(());
                        }
                        _ => {
                            println!(
                                "IPv6 connection failed or timed out for {}, falling back to IPv4",
                                addr_str
                            );
                        }
                    }
                }
            }

            if let Some(ipv4_addr) = addrs.iter().find(|addr| addr.is_ipv4()) {
                let socket = TcpSocket::new_v4()?;
                if let Ok(mut server) = socket.connect(*ipv4_addr).await {
                    let _ = tokio::io::copy_bidirectional(upgraded, &mut server).await?;
                    return Ok(());
                }
            }

            println!("All connection attempts failed for {}", addr_str);
        } else {
            println!("Failed to resolve address: {}", addr_str);
        }

        Ok(())
    }
}

fn get_rand_ipv6_socket_addr(ipv6: u128, prefix_len: u8) -> SocketAddr {
    let mut rng = rand::thread_rng();
    SocketAddr::new(get_rand_ipv6(ipv6, prefix_len), rng.gen::<u16>())
}

fn get_rand_ipv6(mut ipv6: u128, prefix_len: u8) -> IpAddr {
    let rand: u128 = rand::thread_rng().gen();
    let net_part = (ipv6 >> (128 - prefix_len)) << (128 - prefix_len);
    let host_part = (rand << prefix_len) >> prefix_len;
    ipv6 = net_part | host_part;
    IpAddr::V6(ipv6.into())
}
