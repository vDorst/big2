use {
    asynchronous_codec::{Framed, LinesCodec},
    futures::StreamExt,
    futures::{
        future::{ok, ready},
        select, FutureExt,
    },
    log,
    mio::TcpListener,
    ws_stream_tungstenite::*,
};

fn main() {
    block_on(async {
        join!(client());
    });
}

// We make requests to the server and receive responses.
//
async fn client() {
    let url = Url::parse("ws://127.0.0.1:3000/ws").unwrap();
    let socket = ok(url).and_then(connect_async).await.expect("ws handshake");
    let ws = WsStream::new(socket.0);

    // This is a bit unfortunate, but the websocket specs say that the server should
    // close the underlying tcp connection. Hovever, since are the client, we want to
    // use a timeout just in case the server does not close the connection after a reasonable
    // amount of time. Thus when we want to initiate the close, we need to keep polling
    // the stream to drive the close handshake to completion, but we also want to be able
    // to cancel waiting on the stream if it takes to long. Thus we need to break from our
    // normal processing loop and thus need to mark here whether we broke because we want
    // to close or because the server has already closed and the stream already returned
    // None.
    //
    let mut our_shutdown = false;

    let (mut sink, mut stream) = Framed::new(ws, LinesCodec {}).split();

    // Do some actual business logic
    // This could run in a separate task.
    //
    sink.send("Hi from client\n".to_string())
        .await
        .expect("send request");

    while let Some(msg) = stream.next().await {
        let msg = match msg {
            Err(e) => {
                error!("Error on client stream: {:?}", e);

                // if possible ws_stream will try to close the connection with a clean handshake,
                // but if the error persists, it will just give up and return None next iteration,
                // after which we should drop the connection.
                //
                continue;
            }

            Ok(m) => m,
        };

        info!("client received: {}", msg.trim());

        // At some point client decides to disconnect. We will still have to poll the stream
        // to be sure the close handshake get's driven to completion, but we need to timeout
        // if the server never closes the connection. So we need to break from this loop and
        // notify the following code that the break was because we close and not because it
        // returned None.
        //
        debug!("close client side");
        sink.close().await.expect("close out");
        our_shutdown = true;
        break;
    }

    // This takes care of the timeout
    //
    if our_shutdown {
        // We want a future that ends when the stream ends, and that polls it in the mean time.
        // We don't want to consider any more messages from the server though.
        //
        let stream_end = stream.for_each(|_| ready(())).fuse();
        let timeout = Delay::new(Duration::from_secs(1)).fuse();

        pin_mut!(timeout);
        pin_mut!(stream_end);

        // select over timer and end of stream
        //
        select! {
            _ = timeout    => {}
            _ = stream_end => {}
        }
    }

    info!("client end");
}
