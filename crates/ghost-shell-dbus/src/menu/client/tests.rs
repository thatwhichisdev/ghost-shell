use std::{
    io::{BufRead as _, BufReader},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    },
    time::Duration,
};

use zbus::{connection::Builder, object_server::SignalEmitter, zvariant::Value};

use super::*;

const PATH: &str = "/Menu";
const SERVICE: &str = "org.ghost_shell.TestMenu";

// Each test owns a private bus, never the user's desktop bus.
struct TestBus(Child);

impl TestBus {
    fn start() -> (Self, String) {
        let child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("tests require dbus-daemon");
        let mut bus = Self(child);
        let mut address = String::new();
        BufReader::new(bus.0.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        (bus, address.trim().to_owned())
    }
}

impl Drop for TestBus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct TestMenu {
    id: Arc<AtomicI32>,
    activated: Arc<AtomicI32>,
    about_to_show_supported: bool,
    invalidate_during_fetch: bool,
}

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl TestMenu {
    fn about_to_show(&self, id: i32) -> zbus::fdo::Result<bool> {
        assert_eq!(id, ROOT_ITEM_ID);
        if !self.about_to_show_supported {
            return Err(zbus::fdo::Error::UnknownMethod("AboutToShow".into()));
        }
        Ok(false)
    }

    async fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> (u32, RawMenuItem) {
        assert_eq!(parent_id, ROOT_ITEM_ID);
        assert_eq!(recursion_depth, UNLIMITED_RECURSION);
        assert!(property_names.is_empty());
        let id = self.id.load(Ordering::SeqCst);
        let child: RawMenuItem = (id, HashMap::new(), Vec::new());
        if self.invalidate_during_fetch {
            Self::layout_updated(&emitter, 2, 0)
                .await
                .unwrap();
        }
        (
            1,
            (
                0,
                HashMap::new(),
                vec![OwnedValue::try_from(Value::from(child)).unwrap()],
            ),
        )
    }

    fn event(
        &self,
        id: i32,
        event_id: &str,
        _data: OwnedValue,
        _timestamp: u32,
    ) -> zbus::fdo::Result<()> {
        if id != self.id.load(Ordering::SeqCst) {
            return Err(zbus::fdo::Error::Failed("stale item ID".into()));
        }
        assert_eq!(event_id, "clicked");
        self.activated.store(id, Ordering::SeqCst);
        Ok(())
    }

    #[zbus(signal)]
    async fn layout_updated(
        emitter: &SignalEmitter<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn items_properties_updated(
        emitter: &SignalEmitter<'_>,
        updated: Vec<(i32, HashMap<String, OwnedValue>)>,
        removed: Vec<(i32, Vec<String>)>,
    ) -> zbus::Result<()>;
}

async fn exercise_menu(about_to_show_supported: bool, invalidate_during_fetch: bool) {
    let (_bus, address) = TestBus::start();
    let id = Arc::new(AtomicI32::new(3));
    let activated = Arc::new(AtomicI32::new(0));
    let server = Builder::address(address.as_str())
        .unwrap()
        .name(SERVICE)
        .unwrap()
        .serve_at(
            PATH,
            TestMenu {
                id: id.clone(),
                activated: activated.clone(),
                about_to_show_supported,
                invalidate_during_fetch,
            },
        )
        .unwrap()
        .build()
        .await
        .unwrap();
    let connection = Builder::address(address.as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    let menu = MenuId::new(SERVICE, PATH);
    let mut first = DbusMenuClient::open(&connection, menu.clone())
        .await
        .unwrap();
    assert_eq!(first.layout.items()[0].id, 3);

    if invalidate_during_fetch {
        tokio::time::timeout(Duration::from_secs(2), first.invalidated())
            .await
            .unwrap();
        return;
    }

    let interface = server
        .object_server()
        .interface::<_, TestMenu>(PATH)
        .await
        .unwrap();
    // Simulate Electron rebuilding its menu with entirely new IDs.
    id.store(2796, Ordering::SeqCst);
    TestMenu::layout_updated(interface.signal_emitter(), 2, 0)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), first.invalidated())
        .await
        .unwrap();

    let mut reopened = DbusMenuClient::open(&connection, menu.clone())
        .await
        .unwrap();
    let fresh_id = reopened.layout.items()[0].id;
    assert_eq!(fresh_id, 2796);
    DbusMenuClient::activate(&connection, &menu, fresh_id)
        .await
        .unwrap();
    assert_eq!(activated.load(Ordering::SeqCst), 2796);

    TestMenu::items_properties_updated(
        interface.signal_emitter(),
        vec![(
            fresh_id,
            HashMap::from([("enabled".into(), OwnedValue::from(false))]),
        )],
        vec![],
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(2), reopened.invalidated())
        .await
        .unwrap();
}

#[tokio::test]
async fn reopening_uses_fresh_ids_and_open_menus_are_invalidated() {
    tokio::time::timeout(Duration::from_secs(10), exercise_menu(true, false))
        .await
        .unwrap();
}

#[tokio::test]
async fn exporters_without_about_to_show_still_work() {
    tokio::time::timeout(Duration::from_secs(10), exercise_menu(false, false))
        .await
        .unwrap();
}

#[tokio::test]
async fn changes_during_layout_fetch_are_not_lost() {
    tokio::time::timeout(Duration::from_secs(10), exercise_menu(true, true))
        .await
        .unwrap();
}
