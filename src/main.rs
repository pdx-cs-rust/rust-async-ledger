use std::collections::HashMap;
use std::sync::Arc;

use anyhow::bail;
use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::Mutex,
};

struct Ledger {
    book: HashMap<String, i64>,
    admin_key: String,
}

async fn process_client<R>(
    mut client: R,
    ledger: Arc<Mutex<Ledger>>,
) -> anyhow::Result<()>
where R: AsyncBufReadExt + AsyncWriteExt + Unpin
{
    fn check_key(good: &str, test: &str) -> anyhow::Result<()> {
        if good == test {
            Ok(())
        } else {
            bail!("illegal admin key")
        }
    }

    let mut request = String::new();
    client.read_line(&mut request).await?;
    let request = request.trim();
    let fields: Vec<&str> = request.split_whitespace().collect();
    let fields: &[&str] = fields.as_ref();
    eprintln!("request: {:?}", fields);
    match fields {
        ["init", key, account] => {
            let mut ledger = ledger.lock().await;
            check_key(&ledger.admin_key, key)?;
            if ledger.book.contains_key(*account) {
                bail!("init of existing account");
            }
            ledger.book.insert(account.to_string(), 0);
        }
        ["echo", ..] => {
            let reply: String = fields[1..].join(" ") + "\r\n";
            client.write_all(reply.as_bytes()).await?;
        }
        _ => bail!("unknown request"),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let admin_key = tokio::fs::read_to_string("admin-key.txt").await?;
    let admin_key = admin_key.trim().into();
    let book = HashMap::new();
    let ledger = Ledger { admin_key, book };
    let ledger = Arc::new(Mutex::new(ledger));
    let listener = TcpListener::bind("localhost:12354").await?;

    loop {
        let (client, addr) = listener.accept().await?;
        eprintln!("new client: {}", addr);
        let client = BufReader::new(client);
        let ledger = Arc::clone(&ledger);
        tokio::spawn(async move {
            process_client(client, ledger).await.unwrap();
        });
    }
}
