use std::collections::HashMap;
use std::sync::Arc;

use anyhow::bail;
use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

struct Ledger {
    book: HashMap<String, i64>,
    admin_key: String,
}

async fn process_client(
    client: TcpStream,
    ledger: Arc<Mutex<Ledger>>,
) -> anyhow::Result<()> {
    let mut auth: Option<String> = None;
    let (cr, mut cw) = client.into_split();
    let cr = BufReader::new(cr);
    let mut lines = cr.lines();

    fn check_authorized(a: &Option<String>) -> Result<&str, anyhow::Error> {
        if let Some(account) = a {
            Ok(account)
        } else {
            bail!("unauthorized request")
        }
    }

    while let Some(request) = lines.next_line().await? {
        let request = request.trim();
        let fields: Vec<&str> = request.split_whitespace().collect();
        let fields: &[&str] = fields.as_ref();
        match fields {
            ["auth", key, account] => {
                let ledger = ledger.lock().await;
                if *key == ledger.admin_key {
                    auth = Some(account.to_string());
                } else {
                    bail!("failed auth");
                }
            }
            ["init"] => {
                let account = check_authorized(&auth)?;
                let mut ledger = ledger.lock().await;
                if ledger.book.contains_key(account) {
                    bail!("init of existing account");
                }
                ledger.book.insert(account.to_string(), 0);
            }
            ["delete"] => {
                let account = check_authorized(&auth)?;
                let mut ledger = ledger.lock().await;
                if let Some(balance) = ledger.book.get(account) {
                    let reply = format!("{}\r\n", balance);
                    ledger.book.remove(account);
                    cw.write_all(reply.as_bytes()).await?;
                    auth = None;
                } else {
                    bail!("delete of non-existing account");
                }
            }
            ["alter", alter] => {
                let account = check_authorized(&auth)?;
                let alter: i64 = alter.parse()?;
                let mut ledger = ledger.lock().await;
                if let Some(value) = ledger.book.get_mut(account) {
                    *value += alter;
                } else {
                    bail!("alter of non-existing account");
                }
            }
            ["echo", ..] => {
                let reply: String = fields[1..].join(" ") + "\r\n";
                cw.write_all(reply.as_bytes()).await?;
            }
            ["exit"] => {
                break;
            }
            _ => bail!("unknown request"),
        }
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
        let ledger = Arc::clone(&ledger);
        tokio::spawn(async move {
            process_client(client, ledger).await.unwrap_or_else(|e| {
                eprintln!("{}: failed transaction: {}", addr, e)
            });
        });
    }
}
