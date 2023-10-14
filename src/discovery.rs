use std::error::Error;
use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::sync::Arc;
use tokio::net::UdpSocket;

type ControlEventSender = tokio::sync::broadcast::Sender<ControlEvent>;
type ControlEventReceiver = tokio::sync::broadcast::Receiver<ControlEvent>;
type PeerEventSender = tokio::sync::mpsc::Sender<PeerEvent>;
type PeerEventReceiver = tokio::sync::mpsc::Receiver<PeerEvent>;

#[derive(Clone)]
enum ControlEvent {
    Shutdown,
}

#[derive(Clone)]
enum PeerEvent {
    Dummy,
}

struct DiscoveryInner {
    sockets: Vec<Arc<UdpSocket>>,
    ctrl_tx: ControlEventSender,
    ctrl_rx: ControlEventReceiver,
    peer_tx: PeerEventSender,
    peer_rx: PeerEventReceiver,
}

impl DiscoveryInner {
    async fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            tokio::select! {
                Ok(event) = self.ctrl_rx.recv() => {
                    match event {
                        ControlEvent::Shutdown => {
                            break;
                        }
                    }
                }
                Some(event) = self.peer_rx.recv() => {
                    match event {
                        PeerEvent::Dummy => {
                            break;
                        }
                    }
                }
            }
        }
        // let mut buf = [0; 1024];
        // loop {
        //     for socket in &mut sockets {
        //         let (len, addr) = socket.recv_from(&mut buf).await?;
        //         println!("len: {}, addr: {}", len, addr);
        //     }
        // }
        Ok(())
    }
}

pub struct Discovery {
    ctrl_tx: ControlEventSender,
    inner: Option<DiscoveryInner>,
}

impl Discovery {
    pub async fn new<A>(bind_addr: A) -> Result<Self, Box<dyn Error + Send + Sync>>
    where
        A: ToSocketAddrs,
    {
        let mut sockets = vec![];
        let (ctrl_tx, ctrl_rx) = tokio::sync::broadcast::channel(10);
        let (peer_tx, peer_rx) = tokio::sync::mpsc::channel(10);
        let addrs = bind_addr.to_socket_addrs()?;
        for addr in addrs {
            println!("addr: {}", addr);
            match addr {
                SocketAddr::V4(addr_v4) => {
                    let mcast_addr = "239.255.255.250:3702".parse()?;
                    let std_socket = bind_multicast_v4(&addr_v4, &mcast_addr)?;
                    let socket = UdpSocket::from_std(std_socket)?;
                    sockets.push(socket.into());
                }
                SocketAddr::V6(addr_v6) => {
                    // TODO
                }
            }
        }
        let inner = DiscoveryInner {
            sockets,
            ctrl_tx: ctrl_tx.clone(),
            ctrl_rx,
            peer_tx,
            peer_rx,
        };
        Ok(Self {
            ctrl_tx,
            inner: Some(inner),
        })
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(mut inner) = self.inner.take() {
            tokio::task::spawn(async move {
                inner.run();
            });
        }
        Ok(())
    }
}

/// Bind socket to multicast address with IP_MULTICAST_LOOP and SO_REUSEADDR Enabled
fn bind_multicast_v4(
    bind_addr: &SocketAddrV4,
    multi_addr: &SocketAddrV4,
) -> Result<std::net::UdpSocket, Box<dyn Error + Send + Sync>> {
    use socket2::{Domain, Protocol, Socket, Type};

    assert!(multi_addr.ip().is_multicast(), "Must be multcast address");

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

    socket.set_reuse_address(true)?;
    socket.bind(&socket2::SockAddr::from(*bind_addr))?;
    socket.set_multicast_loop_v4(true)?;
    socket.join_multicast_v4(multi_addr.ip(), bind_addr.ip())?;

    Ok(socket.into())
}
