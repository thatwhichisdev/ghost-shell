type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
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
    assert!(matches!(
        Server::bind(path.clone(), requests).await,
        Err(ghost_shell_ipc::Error::AlreadyRunning { .. })
    ));
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
                .ok_or("request channel closed")?;
            assert!(matches!(
                request.request,
                Request::Bar {
                    action: BarAction::Toggle
                }
            ));
            request
                .reply
                .send(reply.clone())
                .map_err(|_| "reply receiver dropped")?;
            assert_eq!(
                serde_json::to_value(client.read().await?)?,
                serde_json::to_value(reply)?
            );
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::select! {
        result = server.run() => panic!("server ended before exchange: {result:?}"),
        result = exchange => result?,
    }
    assert!(!path.exists());
    Ok(())
}

#[tokio::test]
async fn connection_error_retains_path_and_io_source() -> Result<()> {
    use std::error::Error as _;
    let path =
        std::env::temp_dir().join(format!("ghost-missing-{}.sock", std::process::id()));
    let error = match Client::connect(path.clone()).await {
        Err(error) => error,
        Ok(_) => panic!("Unexpected connection to nonexistent test socket"),
    };
    assert!(
        matches!(&error, ghost_shell_ipc::Error::Socket { operation: "connect to", path: actual, .. } if actual == &path)
    );
    let source = error
        .source()
        .and_then(|source| source.downcast_ref::<std::io::Error>())
        .ok_or("missing IO source")?;
    assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
    let report = snafu::Report::from_error(error).to_string();
    assert!(report.contains(&path.to_string_lossy().to_string()));
    assert!(report.contains("No such file"));
    Ok(())
}
