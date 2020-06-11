use std::io::{self, BufRead, Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

// use log::error;

// use serde_json::json;
// use serde_json::Value;

use xi_core_lib::XiCore;
// use xi_rpc::{Peer, RpcLoop};
use xi_rpc::{Handler, RpcLoop};

// pub fn start_xi_core() -> (Writer, Reader, ClientToClientWriter) {
// NOTE:
// thread.spawn(writer_from_xi_to_client.mainloop(reader_from_client_to_xi))
// return (writer_from_client_to_xi, reader_from_xi_to_client)
pub fn start_xi_core() -> (Writer, Reader) {
    let mut xi = XiCore::new();

    let (from_client_to_xi_tx, from_client_to_xi_rx) = make_channel();
    let (writer_from_xi_to_client, reader_from_xi_to_client) = make_channel();

    // let client_to_client_writer = ClientToClientWriter(Writer(from_core_tx));

    //pub fn mainloop<R, RF, H>(&mut self, rf: RF, handler: &mut H) -> Result<(), ReadError>
    //where
    //    R: BufRead,
    //    RF: Send + FnOnce() -> R,
    //    H: Handler,
    let mut xi_event_loop = RpcLoop::new(writer_from_xi_to_client);
    thread::spawn(move || xi_event_loop.mainloop(|| from_client_to_xi_rx, &mut xi));

    (
        from_client_to_xi_tx,
        reader_from_xi_to_client,
        //    client_to_client_writer,
    )
}

fn make_channel() -> (Writer, Reader) {
    let (tx, rx) = channel();
    (Writer(tx), Reader(rx))
}

pub struct Writer(Sender<String>);

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8(buf.to_vec()).unwrap();
        self.0
            .send(s)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))
            .map(|_| buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Wraps an instance of `mpsc::Receiver`, providing convenience methods
/// for parsing received messages.
pub struct Reader(Receiver<String>);

impl Read for Reader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        unreachable!("didn't expect xi-rpc to call read");
    }
}

// Note: we don't properly implement BufRead, only the stylized call patterns
// used by xi-rpc.
impl BufRead for Reader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        unreachable!("didn't expect xi-rpc to call fill_buf");
    }

    fn consume(&mut self, _amt: usize) {
        unreachable!("didn't expect xi-rpc to call consume");
    }

    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        let event = match self.0.recv() {
            Ok(s) => s,
            Err(_) => return Ok(0),
        };

        if &event == r#"{"method":"command","params":{"method":"exit"}}"# {
            // It receive a close commmand from the writer indicating the chan
            // should be closes. The event is sent by the InputController when
            // the user ask to quit the program.
            //
            // This method is required because the chan producers a shared between
            // The InputController and the EventController threads and it's
            // impossible for the InputController to close the EventController
            // channel.
            //
            // When the Reader receives the command, it close the channel between
            // the InputController which lead to the following steps in order:
            // - The channel between the the InputController and the Core close itself.
            // - The Core event loop stop itself safely.
            // - The channel between the Core and the EventController close itself.
            // - The the EventController event loop stop itself safely.
            // - The main exit.
            return Ok(0);
        }

        buf.push_str(&event);
        Ok(event.len())
    }
}

// pub struct ClientToClientWriter(Writer);
//
// impl ClientToClientWriter {
//     pub fn client_started(&mut self, xi_config_dir: &str) {
//         self.send_rpc_notification(
//             "client_started",
//             &json!({"config_dir": xi_config_dir, }),
//         );
//     }
// }
//
// impl Peer for ClientToClientWriter {
//     fn send_rpc_notification(&self, method: &str, params: &Value) {
//         let raw_content =
//             match serde_json::to_vec(&json!({"method": method, "params": params})) {
//                 Ok(raw) => raw,
//                 Err(err) => {
//                     error!("failed to create the notification {}: {}", method, err);
//                     return;
//                 }
//             };
//
//         match self.0.write(&raw_content) {
//             Ok(_) => (),
//             Err(err) => error!("failed to send the notification {}: {}", method, err),
//         };
//     }
// }
