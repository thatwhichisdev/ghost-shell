use std::{
    cmp::Reverse,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::{
    AnyElement, Context, ObjectFit, RenderImage, Subscription, Window, div, img,
    prelude::*, px,
};

const ICON_SIZE: f32 = 20.0;
const ITEM_SIZE: f32 = 24.0;

pub struct TrayWidget;
