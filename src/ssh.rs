use std::{
    fs::File,
    io::BufReader,
    net::{SocketAddr, ToSocketAddrs},
    path::Path,
    time::Duration,
};

use dirs::home_dir;
use log::debug;
use mio::{net::TcpStream, Events, Interest, Poll, Token};
use ssh2::{MethodType, Session};
use ssh2_config::{HostParams, ParseRule, SshConfig};

pub const SSH_TOKEN: Token = Token(0);
const TCP_TIMEOUT: Duration = Duration::from_secs(10);

#[inline]
fn check_connected(tcp: &mut TcpStream) -> Result<(), ()> {
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);
    poll.registry()
        .register(tcp, SSH_TOKEN, Interest::WRITABLE | Interest::READABLE)
        .unwrap();

    //  3. Wait for a (writable) event.
    loop {
        if let Err(_) = poll.poll(&mut events, Some(TCP_TIMEOUT)) {
            return Err(());
        }

        for event in events.iter() {
            match event.token() {
                SSH_TOKEN => {
                    //  4. Check `TcpStream::take_error`. If it returns an error, then
                    //     something went wrong. If it returns `Ok(None)`, then proceed to
                    //     step 5.
                    if let Err(err) = tcp.take_error() {
                        println!("Something went wrong {}", err);
                        poll.registry().deregister(tcp).unwrap();
                        return Err(());
                    }
                    //  5. Check `TcpStream::peer_addr`. If it returns `libc::EINPROGRESS` or
                    //     `ErrorKind::NotConnected` it means the stream is not yet connected,
                    //     go back to step 3. If it returns an address it means the stream is
                    //     connected, go to step 6. If another error is returned something
                    //     went wrong.
                    if let Err(err) = tcp.peer_addr() {
                        if err.kind() == std::io::ErrorKind::NotConnected {
                            continue;
                        }
                        if err.raw_os_error() == Some(115) {
                            println!("libc::EINPROGRESS");
                            continue;
                        }
                        poll.registry().deregister(tcp).unwrap();
                        return Err(());
                    }

                    poll.registry().deregister(tcp).unwrap();
                    return Ok(());
                }
                _ => unreachable!(),
            }
        }
    }
}

///  1. Call `TcpStream::connect`
///  2. Register the returned stream with at least [write interest].
///  3. Wait for a (writable) event.
///  4. Check `TcpStream::take_error`. If it returns an error, then
///     something went wrong. If it returns `Ok(None)`, then proceed to
///     step 5.
///  5. Check `TcpStream::peer_addr`. If it returns `libc::EINPROGRESS` or
///     `ErrorKind::NotConnected` it means the stream is not yet connected,
///     go back to step 3. If it returns an address it means the stream is
///     connected, go to step 6. If another error is returned something
///     went wrong.
///  6. Now the stream can be used.
fn connect_tcp(host: &str) -> Option<(TcpStream, Poll, Events)> {
    debug!("Connecting to host {}...", host);
    let socket_addresses: Vec<SocketAddr> = match host.to_socket_addrs() {
        Ok(s) => s.collect(),
        Err(err) => {
            eprintln!("Could not parse host: {}", err);
            return None;
        }
    };

    for socket_addr in socket_addresses.into_iter() {
        let mut tcp = match TcpStream::connect(socket_addr) {
            Ok(stream) => stream,
            Err(_) => {
                println!("Continuing with next TCP stream");
                continue;
            }
        };

        if let Ok(_) = check_connected(&mut tcp) {
            let poll = Poll::new().unwrap();
            let events = Events::with_capacity(1024);
            poll.registry()
                .register(&mut tcp, SSH_TOKEN, Interest::WRITABLE | Interest::READABLE)
                .unwrap();

            return Some((tcp, poll, events));
        }

        println!("Polling next socket address");
    }

    return None;
}

pub fn new_session(host: &str, password: &str) -> Result<(Session, Poll, Events), ()> {
    let config = read_config();
    let params = config.query(host);

    // Parse SSH host
    let (username, host) = if host.contains("@") {
        let split: Vec<&str> = host.split("@").collect();
        if split.len() != 2 {
            eprintln!("Bad SSH 'username@host': {}", host);
            return Err(());
        }
        (Some(split[0]), split[1])
    } else {
        (None, host)
    };
    let host = params.host_name.as_deref().unwrap_or(host);
    let port = params.port.unwrap_or(22);
    let host = match host.contains(':') {
        true => host.to_string(),
        false => format!("{}:{}", host, port),
    };

    // Parse username
    let username = match params.user.as_ref() {
        Some(u) => {
            u.clone()
        }
        None => if let Some(username) = username {
            username.to_string()
        } else {
            eprintln!("No username provided for SSH");
            return Err(());
        },
    };
    debug!("SSH username: {}, host: {}", username, host);

    // Connect to host
    let (tcp, poll, events) = match connect_tcp(&host) {
        Some(ret) => ret,
        None => {
            eprintln!("No suitable socket address found; connection timeout");
            return Err(());
        }
    };

    // Create SSH session
    let mut session = Session::new().unwrap();
    configure_session(&mut session, &params);
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();

    // Authenticate
    let code = match session.userauth_agent(&username) {
        Ok(_) => {
            return Ok((session, poll, events));
        }
        Err(err) => err.code(),
    };

    match code {
        ssh2::ErrorCode::Session(-18) => {
            debug!("Error authenticating with user agent, trying password")
        }
        _ => return Err(()),
    }

    if let Err(err) = session.userauth_password("tomaz", password) {
        println!("Password authentication failed: {}!", err);
        return Err(());
    }

    if !session.authenticated() {
        println!("Authentication failed without reason!");
        return Err(());
    }

    println!("Established connection with {}", host);
    return Ok((session, poll, events));
}

fn read_config() -> SshConfig {
    let mut config_path = home_dir().expect("Failed to get home_dir for guest OS");
    config_path.extend(Path::new(".ssh/config"));

    let mut reader = match File::open(config_path.as_path()) {
        Ok(f) => BufReader::new(f),
        Err(err) => panic!("Could not open file '{}': {}", config_path.display(), err),
    };
    match SshConfig::default().parse(&mut reader, ParseRule::STRICT) {
        Ok(config) => config,
        Err(err) => panic!("Failed to parse configuration: {}", err),
    }
}

fn configure_session(session: &mut Session, params: &HostParams) {
    if let Some(compress) = params.compression {
        println!("compression: {}", compress);
        session.set_compress(compress);
    }
    if params.tcp_keep_alive.unwrap_or(false) && params.server_alive_interval.is_some() {
        let interval = params.server_alive_interval.unwrap().as_secs() as u32;
        println!("keepalive interval: {} seconds", interval);
        session.set_keepalive(true, interval);
    }
    // algos
    if let Some(algos) = params.kex_algorithms.as_deref() {
        if let Err(err) = session.method_pref(MethodType::Kex, algos.join(",").as_str()) {
            panic!("Could not set KEX algorithms: {}", err);
        }
    }
    if let Some(algos) = params.host_key_algorithms.as_deref() {
        if let Err(err) = session.method_pref(MethodType::HostKey, algos.join(",").as_str()) {
            panic!("Could not set host key algorithms: {}", err);
        }
    }
    if let Some(algos) = params.ciphers.as_deref() {
        if let Err(err) = session.method_pref(MethodType::CryptCs, algos.join(",").as_str()) {
            panic!("Could not set crypt algorithms (client-server): {}", err);
        }
        if let Err(err) = session.method_pref(MethodType::CryptSc, algos.join(",").as_str()) {
            panic!("Could not set crypt algorithms (server-client): {}", err);
        }
    }
    if let Some(algos) = params.mac.as_deref() {
        if let Err(err) = session.method_pref(MethodType::MacCs, algos.join(",").as_str()) {
            panic!("Could not set MAC algorithms (client-server): {}", err);
        }
        if let Err(err) = session.method_pref(MethodType::MacSc, algos.join(",").as_str()) {
            panic!("Could not set MAC algorithms (server-client): {}", err);
        }
    }
}

// fn authenticate_ssh(parent: &IvyTmuxWindow) {
//     let app = parent.application().unwrap();

//     let dialog = Window::builder()
//         .application(&app)
//         .title("Attach new Tmux session")
//         .modal(true)
//         .transient_for(parent)
//         .build();

//     let header_bar = HeaderBar::new();
//     let content = Box::new(Orientation::Vertical, 5);

//     // Tmux session input
//     let session_label = Label::new(Some("Tmux session:"));
//     let session_input = Entry::new();
//     content.append(&session_label);
//     content.append(&session_input);

//     // SSH input
//     let ssh_label = Label::new(Some("SSH host:"));
//     let ssh_input = Entry::new();
//     content.append(&ssh_label);
//     content.append(&ssh_input);

//     // Button
//     let button = Button::builder().label("Attach").build();
//     content.append(&button);

//     let window_box = Box::new(Orientation::Vertical, 0);
//     window_box.append(&header_bar);
//     window_box.append(&content);
//     dialog.set_content(Some(&window_box));

//     button.connect_clicked(glib::clone!(
//         #[weak]
//         dialog,
//         move |_| {
//             let tmux_session = session_input.text();
//             let ssh_target = ssh_input.text();

//             // let app = dialog.application();
//             // dialog.close();

//             // if let Some(app) = app {
//             //     let app: IvyApplication = app.downcast().unwrap();
//             //     let ssh_target = if ssh_target.is_empty() {
//             //         None
//             //     } else {
//             //         Some(ssh_target.as_str())
//             //     };
//             //     app.new_window(Some(tmux_session.as_str()), ssh_target);
//             // }
//         }
//     ));

//     dialog.present();
// }
