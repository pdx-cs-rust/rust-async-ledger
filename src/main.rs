use std::collections::HashMap;

use anyhow::bail;
use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::Mutex,
};

type Ledger = HashMap<String, i64>;

async fn process_client<R>(
    mut client: R,
    _admin_key: &str,
    _ledger: &Mutex<Ledger>,
) -> anyhow::Result<()>
where R: AsyncBufReadExt + AsyncWriteExt + Unpin
{
    let mut request = String::new();
    client.read_line(&mut request).await?;
    let request = request.trim();
    eprintln!("request: {}", request);
    let fields: Vec<&str> = request.split_whitespace().collect();
    let fields: &[&str] = fields.as_ref();
    match fields {
        &["echo"] => {
            let reply: String = fields.join(" ");
            client.write_all(reply.as_bytes()).await?;
        }
        _ => bail!("unknown request"),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let admin_key = tokio::fs::read_to_string("admin-key.txt").await?;
    let admin_key = admin_key.trim();
    let ledger: Mutex<Ledger> = Mutex::new(HashMap::new());
    let listener = TcpListener::bind("localhost:12354").await?;
    loop {
        let (client, addr) = listener.accept().await?;
        eprintln!("new client: {}", addr);
        let client = BufReader::new(client);
        tokio::spawn(async move {
            process_client(client, &admin_key, &ledger).await.unwrap();
        });
    }
}
