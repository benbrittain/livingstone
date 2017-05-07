// DISCLAIMER
// THIS IS NOT A FULL FTP IMPLEMENTATION. IN FACT, IT BARELY FUNCTIONS AT ALL
// AND IGNORES MANY COMMANDS SPECS. IT IS MERELY FOR SIMPLE INTEROP WITH A SINGLE ANDROID CLIENT'S
// UPLOAD SETUP. NOR, SHOULD YOU EVER BASE ANY IDIOMATIC RUST CODE ON THIS DISASTER. HAVE A GOOD DAY.
use std::net::{TcpListener, TcpStream, Shutdown};
use std::thread;
use std::fs;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::mpsc;

use std::io::{Read, Write, BufWriter};
use std::fs::OpenOptions;
use std::time::Duration;
use std::sync::RwLock;

const WORKING_DIR: &'static str = "gpx/";

fn handle_client(mut stream: TcpStream, cord_sender: mpsc::Sender<String>) {
	match stream.write(b"220 Welcome!\r\n") {
		Err(e) => panic!("Got an error: {}", e),
		Ok(_) => {
			println!("new FTP client from {}", stream.peer_addr().unwrap());
		},
	}

    let passive_send_queue = VecDeque::new();
    let queue = Arc::new(Mutex::new(passive_send_queue));

    let file_lock = Arc::new(RwLock::new(None));

    let mut passive_listener: Option<TcpListener>;
    let (tx, rx) = channel();
    let (f_tx, f_rx) = channel();

    let mut buf;
    loop {
        buf = [0; 512];
        match stream.read(&mut buf) {
            Err(e) => panic!("Got an error: {}", e),
            Ok(m) => {
                if m == 0 { break } // EOF
            },
        };

		let cmd = String::from_utf8_lossy(&buf[0..4]);
		let args = String::from_utf8_lossy(&buf[5..]);

		match cmd.as_ref() {
            "TYPE" => {
                println!("{}", args);
                let _ = stream.write(b"331 OK.\r\n");
            }
            "USER" => {
                let _ = stream.write(b"331 OK.\r\n");
            }
            "PASS" => {
                let _ = stream.write(b"230 OK.\r\n");
            }
            "QUIT" => {
                let _ = stream.write(b"221 Goodbye.\r\n");
                break;
            }
            "SYST" => {
                let _ = stream.write(b"215 UNIX Type: L8\r\n");
            }
            "LIST" => {
                let _ = stream.write(b"150 Here comes the directory listing.\r\n");
                let paths = fs::read_dir("./").unwrap();
                {
                    let mut p_queue = queue.lock().unwrap();
                    for path in paths {
                        let send_str = format!("{}\r\n", path.unwrap().path().display());
                        p_queue.push_back(send_str);
                    }
                }
                loop {
                    let done = rx.recv().unwrap();
                    if done {
                        let _ = stream.write(b"226 Directory send OK.\r\n");
                        break;
                    }
                }
            }
            "STOR" => {
                let _ = stream.write(b"150 Opening data connection.\r\n");

                let arg_str = format!("{}", args).replace("\r", "\n");
                let arg_line = arg_str.lines().next();
                let out = WORKING_DIR.to_owned() + arg_line.unwrap().trim();
                let fout = match OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(out.as_str()) {
                        Ok(f)  => Some(Box::new(BufWriter::new(f))),
                        Err(_) => None
                    };
                {
                    let mut w = file_lock.write().unwrap();
                    *w = fout;
                }
                loop {
                    let done = f_rx.recv().unwrap();
                    if done {
                        println!("Uploaded a GPX file: {}", out);
                        let _ = stream.write(b"226 Transfer complete.\r\n");
                        let file_out = String::from(out);
                        cord_sender.send(file_out);
                        break;
                    }
                }
            }
            "CWD " => {
                // I don't want to allow this
                let _ = stream.write(b"250 OK.\r\n");
            }
            "PASV" => {
                passive_listener = Some(TcpListener::bind("192.168.10.102:0").unwrap());
                let passive_list = passive_listener.unwrap();
                let addr = passive_list.local_addr().unwrap();
                let ip_str = format!("{}", addr.ip());
                let port = addr.port();
                let out_str = format!("227 Entering Passive Mode ({},{},{}).\r\n",
                                            ip_str.replace(".", ","),
                                            port>>8&0xFF, port&0xFF);
                let _ = stream.write(out_str.as_bytes());

                let child_queue = queue.clone();
                let c = Arc::downgrade(&file_lock.clone());
                let tx = tx.clone();
                let f_tx = f_tx.clone();
                thread::spawn(move|| {
                    for pasv_stream in passive_list.incoming() {
                        let mut ps = pasv_stream.unwrap();
                        let mut buf;
                        let mut started = false;
                        loop {
                            buf = [0; 2048];
                            {
                                match c.upgrade() {
                                    None => {},
                                    Some(file_handle) => {
                                        match *file_handle.write().unwrap() {
                                            None => {},
                                            Some(ref mut fout) => {
                                                match ps.read(&mut buf) {
                                                    Err(e) => panic!("Got an error: {}", e),
                                                    Ok(m) => {
                                                        if m == 0 && started {
                                                            // Some time to flush. shitty, but functional
                                                            let _ = fout.flush();
                                                            thread::sleep(Duration::new(0, 10000));
                                                            let _ = f_tx.send((true));
                                                        } else {
                                                            started = true;
                                                            let _ = fout.write_all(&buf[..m]);
                                                            let _ = f_tx.send((false));
                                                        }
                                                    },
                                                };
                                            }
                                        }
                                    }
                                }
                            }

                            let mut p_queue = child_queue.lock().unwrap();
                            let to_send = p_queue.pop_front();
                            match to_send {
                                None => { },
                                Some(b) => {
                                    let _ = ps.write(b.as_bytes());
                                    if p_queue.len() == 0 {
                                        let _ = tx.send((true)).unwrap();
                                        let _ = ps.shutdown(Shutdown::Both);
                                    } else {
                                        let _ = tx.send((false)).unwrap();
                                    }
                                }
                            }
                        }
                    }
                });
            }
            _ => {
                println!("NOT YET IMPLEMENTED {}", cmd);
            }
        }
    }
}

pub fn start_ftpserver(tx: mpsc::Sender<String>) {
	let listener = TcpListener::bind("0.0.0.0:2121").unwrap();
    println!("listening for GPS coords on 2121");
	for stream in listener.incoming() {
        let txprime = tx.clone();
        match stream {
	        Ok(stream) => {
	            thread::spawn(move|| {
	                handle_client(stream, txprime)
	            });
	        }
	        Err(_) => { /* connection failed */ }
	    }
	}
	drop(listener);
}
