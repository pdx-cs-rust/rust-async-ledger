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
    fn check_key(good: &str, test: &str) -> anyhow::Result<()> {
        if good == test {
            Ok(())
        } else {
            bail!("illegal admin key")
        }
    }

    let (cr, mut cw) = client.into_split();
    let cr = BufReader::new(cr);
    let mut lines = cr.lines();
    while let Some(request) = lines.next_line().await? {
        let request = request.trim();
        let fields: Vec<&str> = request.split_whitespace().collect();
        let fields: &[&str] = fields.as_ref();
        match fields {
            ["init", key, account] => {
                let mut ledger = ledger.lock().await;
                check_key(&ledger.admin_key, key)?;
                if ledger.book.contains_key(*account) {
                    bail!("init of existing account");
                }
                ledger.book.insert(account.to_string(), 0);
            }
            ["delete", key, account] => {
                let mut ledger = ledger.lock().await;
                check_key(&ledger.admin_key, key)?;
                if let Some(balance) = ledger.book.get(*account) {
                    let reply = format!("{}\r\n", balance);
                    ledger.book.remove(*account);
                    cw.write_all(reply.as_bytes()).await?;
                } else {
                    bail!("delete of non-existing account");
                }
            }
            ["alter", key, account, alter] => {
                let alter: i64 = alter.parse()?;
                let mut ledger = ledger.lock().await;
                check_key(&ledger.admin_key, key)?;
                if let Some(value) = ledger.book.get_mut(*account) {
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
