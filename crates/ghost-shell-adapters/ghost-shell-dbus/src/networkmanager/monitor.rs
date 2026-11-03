use std::collections::HashSet;

use futures_util::StreamExt;
use snafu::ResultExt;
use tokio::sync::mpsc;
use zbus::{Connection, MatchRule, MessageStream, proxy::CacheProperties};

use super::interface::{
    NetworkManagerProxy,
    access_point::AccessPointProxy,
    device::{DeviceProxy, wireless::WirelessDeviceProxy},
    enums::{DeviceState, DeviceType},
};

const SERVICE: &str = "org.freedesktop.NetworkManager";
const ROOT: &str = "/org/freedesktop/NetworkManager";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub wired: Vec<String>,
    pub wireless: Vec<WirelessConnection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WirelessConnection {
    pub interface: String,
    pub ssid: String,
    pub strength: Option<u8>,
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Could not monitor NetworkManager"))]
    Bus { source: zbus::Error },
    #[snafu(display("Could not read NetworkManager connection state"))]
    Snapshot { source: zbus::Error },
    #[snafu(display("NetworkManager signal stream ended"))]
    StreamEnded,
}

/// Subscribes before discovery. Dropping this future releases the bus and listeners.
pub async fn watch(output: mpsc::Sender<Result<Snapshot, Error>>) -> Result<(), Error> {
    let connection = Connection::system()
        .await
        .context(BusSnafu)?;
    watch_connection(&connection, output).await
}

async fn watch_connection(
    connection: &Connection,
    output: mpsc::Sender<Result<Snapshot, Error>>,
) -> Result<(), Error> {
    let changes = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(SERVICE)
        .context(BusSnafu)?
        .path_namespace(ROOT)
        .context(BusSnafu)?
        .build();
    let owners = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.DBus")
        .context(BusSnafu)?
        .interface("org.freedesktop.DBus")
        .context(BusSnafu)?
        .member("NameOwnerChanged")
        .context(BusSnafu)?
        .add_arg(SERVICE)
        .context(BusSnafu)?
        .build();
    let mut changes = MessageStream::for_match_rule(changes, connection, Some(128))
        .await
        .context(BusSnafu)?;
    let mut owners = MessageStream::for_match_rule(owners, connection, Some(8))
        .await
        .context(BusSnafu)?;
    let mut previous = None;
    let mut watched = HashSet::new();
    loop {
        let failed = match snapshot(connection).await {
            Ok((snapshot, paths)) => {
                watched = paths;
                if previous.as_ref() != Some(&snapshot) {
                    previous = Some(snapshot.clone());
                    if output.send(Ok(snapshot)).await.is_err() {
                        return Ok(());
                    }
                }
                false
            }
            Err(source) => {
                previous = None;
                if output
                    .send(Err(Error::Snapshot { source }))
                    .await
                    .is_err()
                {
                    return Ok(());
                }
                true
            }
        };
        loop {
            tokio::select! {
                _ = output.closed() => return Ok(()),
                event = owners.next() => { check_signal(event)?; break; }
                event = changes.next() => {
                    let message = check_signal(event)?;
                    if relevant(&message, &watched) { break; }
                }
                // Retry only failed reads; a healthy connection has no timer.
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)), if failed => break,
            }
        }
    }
}

fn check_signal(
    message: Option<zbus::Result<zbus::Message>>,
) -> Result<zbus::Message, Error> {
    message
        .ok_or(Error::StreamEnded)?
        .context(BusSnafu)
}

fn relevant(message: &zbus::Message, watched: &HashSet<String>) -> bool {
    let header = message.header();
    let Some(path) = header.path() else {
        return false;
    };
    if path.as_str() == ROOT {
        return true;
    }
    if !watched.contains(path.as_str()) {
        return false;
    }
    if header
        .member()
        .is_some_and(|member| member.as_str() == "PropertiesChanged")
    {
        type Properties = (
            String,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
            Vec<String>,
        );
        if let Ok((_, changed, invalidated)) = message.body().deserialize::<Properties>()
        {
            return changed
                .keys()
                .chain(invalidated.iter())
                .any(|key| {
                    matches!(
                        key.as_str(),
                        "State"
                            | "DeviceType"
                            | "Interface"
                            | "ActiveAccessPoint"
                            | "Strength"
                            | "Ssid"
                    )
                });
        }
    }
    false
}

async fn snapshot(connection: &Connection) -> zbus::Result<(Snapshot, HashSet<String>)> {
    // Signals are consumed separately; uncached reads cannot race a proxy cache update.
    let manager = NetworkManagerProxy::builder(connection)
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    let mut snapshot = Snapshot::default();
    let mut watched = HashSet::new();
    for path in manager.get_devices().await? {
        watched.insert(path.to_string());
        let device = DeviceProxy::builder(connection)
            .path(path.clone())?
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        let kind = DeviceType::from(device.device_type().await?);
        if !matches!(kind, DeviceType::Ethernet | DeviceType::Wifi)
            || DeviceState::from(device.state().await?) != DeviceState::Activated
        {
            continue;
        }
        let interface = device.interface().await?;
        if kind == DeviceType::Ethernet {
            snapshot.wired.push(interface);
        } else {
            let wireless = WirelessDeviceProxy::builder(connection)
                .path(path)?
                .cache_properties(CacheProperties::No)
                .build()
                .await?;
            let access_point = wireless.active_access_point().await?;
            let (ssid, strength) = if access_point.as_str() == "/" {
                (String::new(), None)
            } else {
                watched.insert(access_point.to_string());
                let access_point = AccessPointProxy::builder(connection)
                    .path(access_point)?
                    .cache_properties(CacheProperties::No)
                    .build()
                    .await?;
                (
                    String::from_utf8_lossy(&access_point.ssid().await?).into_owned(),
                    Some(access_point.strength().await?.min(100)),
                )
            };
            snapshot.wireless.push(WirelessConnection {
                interface,
                ssid,
                strength,
            });
        }
    }
    snapshot.wired.sort();
    snapshot
        .wireless
        .sort_by(|left, right| left.interface.cmp(&right.interface));
    Ok((snapshot, watched))
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader},
        process::{Child, Command, Stdio},
        sync::{
            Arc,
            atomic::{AtomicU8, AtomicU32, Ordering},
        },
        time::Duration,
    };

    use zbus::{connection::Builder, zvariant::OwnedObjectPath};

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    const WIRED: &str = "/org/freedesktop/NetworkManager/Devices/1";
    const WIFI: &str = "/org/freedesktop/NetworkManager/Devices/2";
    const ACCESS_POINT: &str = "/org/freedesktop/NetworkManager/AccessPoint/1";

    struct TestBus(Child);
    impl Drop for TestBus {
        fn drop(&mut self) {
            if let Err(error) = self.0.kill() {
                eprintln!("Test bus cleanup: {error}");
            }
            if let Err(error) = self.0.wait() {
                eprintln!("Test bus wait: {error}");
            }
        }
    }

    struct Manager;
    #[zbus::interface(name = "org.freedesktop.NetworkManager")]
    impl Manager {
        fn get_devices(&self) -> zbus::fdo::Result<Vec<OwnedObjectPath>> {
            [WIRED, WIFI]
                .into_iter()
                .map(|path| {
                    OwnedObjectPath::try_from(path)
                        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
                })
                .collect()
        }
    }

    struct Device {
        kind: u32,
        state: Arc<AtomicU32>,
    }
    #[zbus::interface(name = "org.freedesktop.NetworkManager.Device")]
    impl Device {
        #[zbus(property)]
        fn device_type(&self) -> u32 {
            self.kind
        }
        #[zbus(property)]
        fn state(&self) -> u32 {
            self.state.load(Ordering::SeqCst)
        }
        #[zbus(property)]
        fn interface(&self) -> &str {
            if self.kind == 1 { "eth0" } else { "wlan0" }
        }
    }

    struct Wireless;
    #[zbus::interface(name = "org.freedesktop.NetworkManager.Device.Wireless")]
    impl Wireless {
        #[zbus(property)]
        fn active_access_point(&self) -> zbus::fdo::Result<OwnedObjectPath> {
            OwnedObjectPath::try_from(ACCESS_POINT)
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
        }
    }

    struct AccessPoint(Arc<AtomicU8>);
    #[zbus::interface(name = "org.freedesktop.NetworkManager.AccessPoint")]
    impl AccessPoint {
        #[zbus(property)]
        fn ssid(&self) -> Vec<u8> {
            b"Test Wi-Fi".to_vec()
        }
        #[zbus(property)]
        fn strength(&self) -> u8 {
            self.0.load(Ordering::SeqCst)
        }
    }

    async fn next(
        receiver: &mut mpsc::Receiver<Result<Snapshot, Error>>,
    ) -> Result<Result<Snapshot, Error>, Box<dyn std::error::Error>> {
        Ok(
            tokio::time::timeout(Duration::from_secs(3), receiver.recv())
                .await?
                .ok_or("Monitor closed")?,
        )
    }

    #[tokio::test]
    async fn watches_connection_signal_disconnect_and_service_restart() -> TestResult {
        let mut bus = TestBus(
            Command::new("dbus-daemon")
                .args(["--session", "--nofork", "--print-address=1"])
                .stdout(Stdio::piped())
                .spawn()?,
        );
        let mut address = String::new();
        BufReader::new(
            bus.0
                .stdout
                .take()
                .ok_or("Missing bus stdout")?,
        )
        .read_line(&mut address)?;
        let wired_state = Arc::new(AtomicU32::new(100));
        let wifi_state = Arc::new(AtomicU32::new(100));
        let strength = Arc::new(AtomicU8::new(80));
        let server = Builder::address(address.trim())?
            .name(SERVICE)?
            .serve_at(ROOT, Manager)?
            .serve_at(
                WIRED,
                Device {
                    kind: 1,
                    state: wired_state.clone(),
                },
            )?
            .serve_at(
                WIFI,
                Device {
                    kind: 2,
                    state: wifi_state.clone(),
                },
            )?
            .serve_at(WIFI, Wireless)?
            .serve_at(ACCESS_POINT, AccessPoint(strength.clone()))?
            .build()
            .await?;
        let client = Builder::address(address.trim())?
            .build()
            .await?;
        let (sender, mut receiver) = mpsc::channel(16);
        let watcher = watch_connection(&client, sender);
        tokio::pin!(watcher);
        let exercise = async {
            let initial = next(&mut receiver).await??;
            assert_eq!(initial.wired, ["eth0"]);
            assert_eq!(
                initial
                    .wireless
                    .first()
                    .and_then(|connection| connection.strength),
                Some(80)
            );
            let access_point = server
                .object_server()
                .interface::<_, AccessPoint>(ACCESS_POINT)
                .await?;
            strength.store(20, Ordering::SeqCst);
            access_point
                .get()
                .await
                .strength_changed(access_point.signal_emitter())
                .await?;
            let changed = next(&mut receiver).await??;
            assert_eq!(
                changed
                    .wireless
                    .first()
                    .and_then(|connection| connection.strength),
                Some(20)
            );
            let device = server
                .object_server()
                .interface::<_, Device>(WIFI)
                .await?;
            wifi_state.store(30, Ordering::SeqCst);
            device
                .get()
                .await
                .state_changed(device.signal_emitter())
                .await?;
            let disconnected = next(&mut receiver).await??;
            assert!(disconnected.wireless.is_empty());
            assert_eq!(disconnected.wired, ["eth0"]);
            server.release_name(SERVICE).await?;
            assert!(next(&mut receiver).await?.is_err());
            server.request_name(SERVICE).await?;
            assert_eq!(next(&mut receiver).await??.wired, ["eth0"]);
            drop(receiver);
            TestResult::Ok(())
        };
        tokio::select! {
            result = &mut watcher => { result?; return Err("Monitor stopped before test completed".into()); }
            result = exercise => result?,
        }
        tokio::time::timeout(Duration::from_secs(3), watcher).await??;
        Ok(())
    }
}
