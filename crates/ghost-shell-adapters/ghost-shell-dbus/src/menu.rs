use tokio::sync::oneshot;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MenuId {
    service: String,
    object_path: String,
}

impl MenuId {
    #[must_use]
    pub fn new(service: impl Into<String>, object_path: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            object_path: object_path.into(),
        }
    }

    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    #[must_use]
    pub fn object_path(&self) -> &str {
        &self.object_path
    }
}

impl std::fmt::Display for MenuId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}{}", self.service, self.object_path,)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MenuItemType {
    Standard,
    Separator,
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuItem {
    pub id: i32,
    pub item_type: MenuItemType,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,

    pub icon_name: Option<String>,
    pub icon_data: Vec<u8>,

    pub shortcut: Vec<Vec<String>>,

    pub toggle_type: Option<String>,
    pub toggle_state: i32,

    pub children_display: Option<String>,
    pub children: Vec<MenuItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuLayout {
    pub id: MenuId,
    pub revision: u32,
    pub root: MenuItem,
}

impl MenuLayout {
    /// Returns the top-level visible menu entries.
    ///
    /// DBusMenu uses item `0` as the root container, so consumers
    /// generally want its children rather than rendering the root itself.
    #[must_use]
    pub fn items(&self) -> &[MenuItem] {
        &self.root.children
    }
}

/// A fresh layout and a subscription scoped to its visible lifetime.
/// Drop this session when the popup closes to release the D-Bus subscription.
pub struct MenuSession {
    pub layout: MenuLayout,
    invalidated: oneshot::Receiver<()>,
}

impl MenuSession {
    pub fn new(layout: MenuLayout, invalidated: oneshot::Receiver<()>) -> Self {
        Self {
            layout,
            invalidated,
        }
    }

    /// Waits until the layout/properties change or the signal connection closes.
    pub async fn invalidated(&mut self) {
        if (&mut self.invalidated).await.is_err() {
            log::debug!("menu signal connection closed");
        }
    }
}
