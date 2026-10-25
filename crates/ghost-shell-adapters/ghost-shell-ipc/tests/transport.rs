use anyhow::{Context as _, Result};
use ghost_shell_ipc::{
    BarAction, Client, Request, RequestClient, RequestServer, Response, Server,
};
use tokio::sync::mpsc;

#[tokio::test]
async fn root_contract_round_trips_replies_and_releases_socket() -> Result<()> {
    let path = std::env::temp_dir()
        .join(format!("ghost-ipc-contract-{}.sock", std::process::id()));
    let (requests, mut receiver) = mpsc::channel(2);
    let server = Server::bind(path.clone(), requests.clone()).await?;
    assert!(
        Server::bind(path.clone(), requests)
            .await
            .is_err()
    );
    let exchange = async {
        let mut client = Client::connect(path.clone()).await?;
        for reply in [Ok(Response::Handled), Err("operation rejected".to_owned())] {
            client
                .write(Request::Bar {
                    action: BarAction::Toggle,
                })
                .await?;
            let request = receiver
                .recv()
                .await
                .context("request channel closed")?;
            assert!(matches!(
                request.request,
                Request::Bar {
                    action: BarAction::Toggle
                }
            ));
            request
                .reply
                .send(reply.clone())
                .map_err(|_| anyhow::anyhow!("reply receiver dropped"))?;
            assert_eq!(
                serde_json::to_value(client.read().await?)?,
                serde_json::to_value(reply)?
            );
        }
        Ok::<_, anyhow::Error>(())
    };
    tokio::select! {
        result = server.run() => anyhow::bail!("server ended before exchange: {result:?}"),
        result = exchange => result?,
    }
    assert!(!path.exists());
    Ok(())
}
