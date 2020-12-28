use tokio::net::TcpListener;

#[cfg(test)]
use std::net::SocketAddr;
#[cfg(test)]
use tokio::task::JoinHandle;

mod client;
mod server;
mod big2rules;
mod muon;


fn main() {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let listener = rt.block_on(TcpListener::bind("0.0.0.0:27191")).unwrap();
    let addr = listener.local_addr().unwrap();
    println!("Starting server at {:?}", addr);
    rt.block_on(async move { server::start_server(listener).await });
}


#[tokio::test]
async fn test_connect() {
    let (addr, _) = start_server().await;

    let c1 = tokio::spawn( client::connect(addr, "Client1"));
    let c2 = tokio::spawn( client::connect(addr, "Client2"));
    let c3 = tokio::spawn( client::connect(addr, "Client3"));
    let c4 = tokio::spawn( client::connect(addr, "Client4"));


    let (_c1, _c2, _c3, _c4) = tokio::join! {
        c1,
        c2,
        c3,
        c4,
    };
}

#[cfg(test)]
async fn start_server() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = tokio::spawn(async move { server::start_server(listener).await });

    (addr, server_handle)
}
