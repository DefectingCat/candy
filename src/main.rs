use std::net::TcpListener;

fn main() {
    let listener  = TcpListener::bind("127.0.0.1:4000").expect("cannon listen on port 4000");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("Connection established!");
    }
}
