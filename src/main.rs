use std::env;
use std::io;
use std::io::prelude::*;
use std::net;
use std::process;
use std::sync::mpsc;
use std::thread;
use std::time;

fn exit(exit_code: i32, message: &(impl std::fmt::Display + ?Sized)) -> ! {
    println!("{}", message);
    process::exit(exit_code)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match Config::new(&args) {
        Ok(config) => config,
        Err(e) => exit(1, e),
    };
    let socket_addr = net::SocketAddrV4::new(config.address, config.port);
    if config.listen {
        let listener = match net::TcpListener::bind(socket_addr) {
            Ok(listener) => listener,
            Err(e) => exit(3, &e),
        };
        for stream_result in listener.incoming() {
            let stream = match stream_result {
                Ok(stream) => stream,
                Err(e) => exit(4, &e),
            };
            handle_connection(stream);
        }
    } else {
        let connection = match net::TcpStream::connect(socket_addr) {
            Ok(conn) => conn,
            Err(e) => exit(5, &e),
        };
        handle_connection(connection);
    }
}

struct Config {
    address: net::Ipv4Addr,
    port: u16,
    listen: bool,
}

impl Config {
    fn new(args: &[String]) -> Result<Config, &str> {
        let mut listen = false;
        match args.len() {
            1..=2 => return Err("Too few args"),
            4 => listen = true,
            5.. => return Err("Too many args"),
            _ => (),
        }
        let address = match args[1].parse::<net::Ipv4Addr>() {
            Ok(address) => address,
            Err(_e) => return Err("Invalid ipv4 address"),
        };
        let port = match args[2].parse::<u16>() {
            Ok(port) => port,
            Err(_e) => return Err("Invalid port number"),
        };
        Ok(Config {
            address,
            port,
            listen,
        })
    }
}

fn spawn_stdin_channel() -> mpsc::Receiver<[u8; 1500]> {
    let (tx, rx) = mpsc::channel::<[u8; 1500]>();
    thread::spawn(move || loop {
        let mut in_buffer = [0; 1500];
        io::stdin().read(&mut in_buffer).unwrap();
        tx.send(in_buffer).unwrap();
    });
    rx
}

fn handle_connection(mut stream: net::TcpStream) {
    stream.set_nodelay(true).expect("set_nodelay failure");
    stream.set_read_timeout(Some(time::Duration::from_millis(1000))).expect("set_read_timeout failure");
    let stdin_channel = spawn_stdin_channel();
    loop {
        let mut in_buffer = [0; 1500];
        let mut in_buf_flag = false;
        let mut out_buffer = [0; 1500];
        let mut out_buf_flag = false;
        match stream.read(&mut in_buffer) {
            Ok(_content) => in_buf_flag = true,
            _ => {},
        }
        if in_buf_flag {
            println!("{}", String::from_utf8_lossy(&in_buffer[..]));
        }
        match stdin_channel.try_recv() {
            Ok(input) => {
                out_buffer = input;
                out_buf_flag = true;
            },
            Err(mpsc::TryRecvError::Empty) => (),
            Err(mpsc::TryRecvError::Disconnected) => {
                println!("mpsc channel disconnected");
                break;
            },
        }
        if out_buf_flag {
            stream.write(&out_buffer).unwrap();
        }
    }
}
