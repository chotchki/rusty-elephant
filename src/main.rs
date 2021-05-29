//Vendor Imports
#[macro_use] 
extern crate log;
extern crate simplelog;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use simplelog::{CombinedLogger, TermLogger, LevelFilter, Config, TerminalMode, ColorChoice};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;


//Application Imports
mod codec;
use codec::{NetworkFrame,PgCodec};
mod constants;
mod engine;
use engine::io::PageManager;
use engine::TransactionGenerator;
mod processor;
use processor::ClientProcessor;

#[tokio::main]
async fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed, ColorChoice::Auto)
        ]
    ).unwrap();

    info!("Welcome to the Rusty Elephant!");

    //Start the I/O system first
    let page_manager = Arc::new(PageManager::new());

    let transaction_generator = Arc::new(TransactionGenerator::new(0));

    //Bind to a fixed port
    let port:u32 = 50000;
    let listener = TcpListener::bind(format!("{}{}", "127.0.0.1:", port)).await.unwrap();

    info!("Up and listening on port {}", port);

    loop {
        let (stream, client_addr) = listener.accept().await.unwrap();

        info!("Got a connection from {}", client_addr);
        
        let pm = page_manager.clone();
        let tg = transaction_generator.clone();
        tokio::spawn(async move {
            let codec = PgCodec{};
            let (mut sink, mut input) = Framed::new(stream, codec).split();

            let process = ClientProcessor::new(pm, tg);
            while let Some(Ok(event)) = input.next().await {
                let responses:Vec<NetworkFrame> = match process.process(event) {
                    Ok(responses) => responses,
                    Err(e) => {
                        warn!("Had a processing error {}", e);
                        break;
                    }
                };

                for response in responses {
                    match sink.send(response).await {
                        Ok(_) => {},
                        Err(e) => {
                            warn!("Unable to send response {}", e);
                            break;
                        }
                    }
                }
            }
        });
    }
}