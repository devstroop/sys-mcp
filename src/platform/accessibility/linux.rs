use serde::de::DeserializeOwned;
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, Type};

use crate::error::GuiError;
use crate::gui::types::*;

fn role_to_string(role: u32) -> String {
    match role {
        0 => "invalid".into(),
        1 => "application".into(),
        2 => "desktop_frame".into(),
        3 => "desktop_icon".into(),
        4 => "window".into(),
        5 => "dialog".into(),
        6 => "alert".into(),
        7 => "menu_bar".into(),
        8 => "menu".into(),
        9 => "menu_item".into(),
        10 => "tool_bar".into(),
        11 => "popup_menu".into(),
        12 => "combo_box".into(),
        13 => "button".into(),
        14 => "text".into(),
        15 => "entry".into(),
        16 => "check_box".into(),
        17 => "radio_button".into(),
        18 => "label".into(),
        19 => "list".into(),
        20 => "list_item".into(),
        21 => "image".into(),
        22 => "panel".into(),
        23 => "table".into(),
        24 => "table_cell".into(),
        25 => "table_row".into(),
        26 => "heading".into(),
        27 => "separator".into(),
        28 => "progress_bar".into(),
        29 => "status_bar".into(),
        30 => "scroll_bar".into(),
        31 => "link".into(),
        32 => "slider".into(),
        33 => "spin_button".into(),
        34 => "tree".into(),
        35 => "tree_item".into(),
        36 => "page_tab".into(),
        37 => "page_tab_list".into(),
        38 => "tool_tip".into(),
        39 => "canvas".into(),
        40 => "scroll_pane".into(),
        41 => "split_pane".into(),
        42 => "password_text".into(),
        43 => "toggle_button".into(),
        44 => "switch".into(),
        _ => format!("role_{}", role),
    }
}

fn parse_states(mask: u64) -> Vec<String> {
    let mut states = Vec::new();
    if mask & (1u64 << 0) != 0 {
        states.push("invalid".into());
    }
    if mask & (1u64 << 1) != 0 {
        states.push("active".into());
    }
    if mask & (1u64 << 2) != 0 {
        states.push("armed".into());
    }
    if mask & (1u64 << 3) != 0 {
        states.push("busy".into());
    }
    if mask & (1u64 << 4) != 0 {
        states.push("checked".into());
    }
    if mask & (1u64 << 5) != 0 {
        states.push("collapsed".into());
    }
    if mask & (1u64 << 6) != 0 {
        states.push("defunct".into());
    }
    if mask & (1u64 << 7) != 0 {
        states.push("editable".into());
    }
    if mask & (1u64 << 8) != 0 {
        states.push("enabled".into());
    }
    if mask & (1u64 << 9) != 0 {
        states.push("expandable".into());
    }
    if mask & (1u64 << 10) != 0 {
        states.push("expanded".into());
    }
    if mask & (1u64 << 11) != 0 {
        states.push("focusable".into());
    }
    if mask & (1u64 << 12) != 0 {
        states.push("focused".into());
    }
    if mask & (1u64 << 13) != 0 {
        states.push("has_tooltip".into());
    }
    if mask & (1u64 << 14) != 0 {
        states.push("horizontal".into());
    }
    if mask & (1u64 << 15) != 0 {
        states.push("iconified".into());
    }
    if mask & (1u64 << 16) != 0 {
        states.push("modal".into());
    }
    if mask & (1u64 << 17) != 0 {
        states.push("multi_line".into());
    }
    if mask & (1u64 << 18) != 0 {
        states.push("multiselectable".into());
    }
    if mask & (1u64 << 19) != 0 {
        states.push("opaque".into());
    }
    if mask & (1u64 << 20) != 0 {
        states.push("pressed".into());
    }
    if mask & (1u64 << 21) != 0 {
        states.push("read_only".into());
    }
    if mask & (1u64 << 22) != 0 {
        states.push("required".into());
    }
    if mask & (1u64 << 23) != 0 {
        states.push("selectable".into());
    }
    if mask & (1u64 << 24) != 0 {
        states.push("selected".into());
    }
    if mask & (1u64 << 25) != 0 {
        states.push("sensitive".into());
    }
    if mask & (1u64 << 26) != 0 {
        states.push("showing".into());
    }
    if mask & (1u64 << 27) != 0 {
        states.push("single_line".into());
    }
    if mask & (1u64 << 28) != 0 {
        states.push("stale".into());
    }
    if mask & (1u64 << 29) != 0 {
        states.push("transient".into());
    }
    if mask & (1u64 << 30) != 0 {
        states.push("truncated".into());
    }
    if mask & (1u64 << 31) != 0 {
        states.push("vertical".into());
    }
    if mask & (1u64 << 32) != 0 {
        states.push("visible".into());
    }
    if mask & (1u64 << 33) != 0 {
        states.push("visited".into());
    }
    if mask & (1u64 << 34) != 0 {
        states.push("has_popup".into());
    }
    if mask & (1u64 << 35) != 0 {
        states.push("indeterminate".into());
    }
    states
}

pub struct PlatformAccessibility {
    conn: Connection,
}

impl PlatformAccessibility {
    pub fn new() -> Result<Self, GuiError> {
        let conn = Connection::session()
            .map_err(|e| GuiError::PlatformError(format!("D-Bus session: {e}")))?;
        Ok(Self { conn })
    }

    fn call_accessible<T: DeserializeOwned + Type>(
        &self,
        path: &OwnedObjectPath,
        method: &str,
    ) -> Result<T, GuiError> {
        use zbus::blocking::Proxy;
        let proxy = Proxy::new(
            &self.conn,
            "org.a11y.atspi",
            path.as_str(),
            "org.a11y.atspi.Accessible",
        )
        .map_err(|e| GuiError::PlatformError(format!("accessible proxy: {e}")))?;
        proxy
            .call_method(method, &())
            .map_err(|e| GuiError::PlatformError(format!("{method} failed: {e}")))?
            .body()
            .deserialize::<T>()
            .map_err(|e| GuiError::PlatformError(format!("{method} body: {e}")))
    }

    fn call_component<T: DeserializeOwned + Type>(
        &self,
        path: &OwnedObjectPath,
        method: &str,
        args: &(u32,),
    ) -> Result<T, GuiError> {
        use zbus::blocking::Proxy;
        let proxy = Proxy::new(
            &self.conn,
            "org.a11y.atspi",
            path.as_str(),
            "org.a11y.atspi.Component",
        )
        .map_err(|e| GuiError::PlatformError(format!("component proxy: {e}")))?;
        proxy
            .call_method(method, args)
            .map_err(|e| GuiError::PlatformError(format!("{method} failed: {e}")))?
            .body()
            .deserialize::<T>()
            .map_err(|e| GuiError::PlatformError(format!("{method} body: {e}")))
    }

    fn get_extents(&self, path: &OwnedObjectPath) -> (i32, i32, i32, i32) {
        self.call_component::<(i32, i32, i32, i32)>(path, "GetExtents", &(0u32,))
            .unwrap_or((0, 0, 0, 0))
    }

    fn get_name(&self, path: &OwnedObjectPath) -> String {
        self.call_accessible::<String>(path, "GetName")
            .unwrap_or_default()
    }

    fn get_role(&self, path: &OwnedObjectPath) -> u32 {
        self.call_accessible::<u32>(path, "GetRole").unwrap_or(0)
    }

    fn get_state(&self, path: &OwnedObjectPath) -> u64 {
        self.call_accessible::<u64>(path, "GetState").unwrap_or(0)
    }

    fn get_description(&self, path: &OwnedObjectPath) -> Option<String> {
        self.call_accessible::<String>(path, "GetDescription")
            .ok()
            .filter(|s| !s.is_empty())
    }

    fn get_children(&self, path: &OwnedObjectPath) -> Vec<OwnedObjectPath> {
        self.call_accessible::<Vec<(String, OwnedObjectPath)>>(path, "GetChildren")
            .unwrap_or_default()
            .into_iter()
            .map(|(_, p)| p)
            .collect()
    }

    // ── AT-SPI Action interface (element-level actions) ──────────────────
    // The `Accessible` helpers above hardcode the interface name and take no
    // arguments; the Action interface needs an index argument, so it gets its
    // own small helpers rather than a generic one (avoids extra trait bounds).

    /// Number of actions the element exposes (0 if it has none or does not
    /// implement the Action interface).
    fn get_action_count(&self, path: &OwnedObjectPath) -> i32 {
        use zbus::blocking::Proxy;
        let proxy = match Proxy::new(
            &self.conn,
            "org.a11y.atspi",
            path.as_str(),
            "org.a11y.atspi.Action",
        ) {
            Ok(p) => p,
            Err(_) => return 0,
        };
        proxy
            .call_method("GetNActions", &())
            .ok()
            .and_then(|m| m.body().deserialize::<i32>().ok())
            .unwrap_or(0)
    }

    fn get_action_name(&self, path: &OwnedObjectPath, index: i32) -> Option<String> {
        use zbus::blocking::Proxy;
        let proxy = Proxy::new(
            &self.conn,
            "org.a11y.atspi",
            path.as_str(),
            "org.a11y.atspi.Action",
        )
        .ok()?;
        proxy
            .call_method("GetName", &(index,))
            .ok()
            .and_then(|m| m.body().deserialize::<String>().ok())
            .filter(|s| !s.is_empty())
    }

    fn do_action(&self, path: &OwnedObjectPath, index: i32) -> Result<bool, GuiError> {
        use zbus::blocking::Proxy;
        let proxy = Proxy::new(
            &self.conn,
            "org.a11y.atspi",
            path.as_str(),
            "org.a11y.atspi.Action",
        )
        .map_err(|e| GuiError::PlatformError(format!("action proxy: {e}")))?;
        proxy
            .call_method("DoAction", &(index,))
            .map_err(|e| GuiError::PlatformError(format!("DoAction failed: {e}")))?
            .body()
            .deserialize::<bool>()
            .map_err(|e| GuiError::PlatformError(format!("DoAction body: {e}")))
    }

    fn action_names(&self, path: &OwnedObjectPath) -> Vec<String> {
        (0..self.get_action_count(path))
            .filter_map(|i| self.get_action_name(path, i))
            .collect()
    }

    /// Whether the element answers to an extra AT-SPI interface. Reported by the
    /// Accessible interface itself, so this is one call rather than a probe per
    /// candidate interface.
    fn has_interface(&self, path: &OwnedObjectPath, interface: &str) -> bool {
        self.call_accessible::<Vec<String>>(path, "GetInterfaces")
            .map(|names| names.iter().any(|name| name == interface))
            .unwrap_or(false)
    }

    /// Write the element's whole text through the EditableText interface. The
    /// Action interface only carries verbs — there is no AT-SPI action that can
    /// hold an operand — so a value has to go to a different D-Bus method
    /// (`docs/ISSUES.md` #46). `SetTextContents` replaces the string, which is
    /// what "set this field to X" means; an element that is not editable has no
    /// EditableText interface and the D-Bus error says so.
    fn set_text_contents(&self, path: &OwnedObjectPath, text: &str) -> Result<(), GuiError> {
        use zbus::blocking::Proxy;
        let proxy = Proxy::new(
            &self.conn,
            "org.a11y.atspi",
            path.as_str(),
            "org.a11y.atspi.EditableText",
        )
        .map_err(|e| GuiError::PlatformError(format!("editable-text proxy: {e}")))?;
        proxy
            .call_method("SetTextContents", &(text,))
            .map_err(|e| GuiError::PlatformError(format!(
                "SetTextContents failed on '{}': {e} — this element does not implement the AT-SPI EditableText interface, so its value is read-only",
                path.as_str()
            )))?;
        Ok(())
    }

    /// Element ids on Linux are AT-SPI D-Bus object paths (see `build_tree`),
    /// so they resolve directly — no cache or re-walk needed.
    fn parse_path(element_id: &str) -> Result<OwnedObjectPath, GuiError> {
        element_id.try_into().map_err(|_| {
            GuiError::PlatformError(format!(
                "invalid element id '{element_id}': expected an AT-SPI object path exactly as returned by accessibility_tree / accessibility_find"
            ))
        })
    }

    fn build_tree(
        &self,
        path: &OwnedObjectPath,
        depth: u32,
    ) -> Result<AccessibilityNode, GuiError> {
        let name = self.get_name(path);
        let role_num = self.get_role(path);
        let state_mask = self.get_state(path);
        let desc = self.get_description(path);

        let states = parse_states(state_mask);
        let role = role_to_string(role_num);

        let (x, y, w, h) = self.get_extents(path);
        let cx = if w > 0 { Some(x + w / 2) } else { None };
        let cy = if h > 0 { Some(y + h / 2) } else { None };
        let id = path.as_str().to_string();

        let mut children = Vec::new();
        if depth > 0 {
            for child_path in self.get_children(path) {
                if let Ok(node) = self.build_tree(&child_path, depth - 1) {
                    children.push(node);
                }
            }
        }

        Ok(AccessibilityNode {
            id,
            role,
            name: if name.is_empty() { None } else { Some(name) },
            value: None,
            description: desc,
            x: Some(x),
            y: Some(y),
            width: Some(w as u32),
            height: Some(h as u32),
            cx,
            cy,
            children,
            actions: vec![],
            states,
        })
    }

    pub fn get_tree(
        &self,
        _window_id: Option<u64>,
        max_depth: Option<u32>,
    ) -> Result<AccessibilityNode, GuiError> {
        let depth = max_depth.unwrap_or(10);
        let root_path: OwnedObjectPath = "/org/a11y/atspi/accessible/root"
            .try_into()
            .map_err(|e| GuiError::PlatformError(format!("invalid path: {e}")))?;
        self.build_tree(&root_path, depth)
    }

    pub fn find_elements(
        &self,
        query: AccessibilityQuery,
    ) -> Result<Vec<AccessibilityNode>, GuiError> {
        let tree = self.get_tree(query.window_id, query.max_depth)?;
        let mut results = Vec::new();
        Self::search_tree(&tree, &query, &mut results);
        Ok(results)
    }

    fn search_tree(
        node: &AccessibilityNode,
        query: &AccessibilityQuery,
        results: &mut Vec<AccessibilityNode>,
    ) {
        let name_matches = query
            .name
            .as_ref()
            .map(|q| {
                node.name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&q.to_lowercase()))
                    .unwrap_or(false)
            })
            .unwrap_or(true);
        let role_matches = query
            .role
            .as_ref()
            .map(|q| node.role.to_lowercase().contains(&q.to_lowercase()))
            .unwrap_or(true);
        if name_matches && role_matches {
            results.push(node.clone());
        }
        for child in &node.children {
            Self::search_tree(child, query, results);
        }
    }

    pub fn get_element_properties(&self, element_id: &str) -> Result<AccessibilityNode, GuiError> {
        let path = Self::parse_path(element_id)?;
        let mut node = self.build_tree(&path, 0)?;
        // `build_tree` deliberately leaves `actions` empty: querying the Action
        // interface for every node would add a D-Bus round-trip per element to
        // every tree walk. Fill it in for the single element asked about here.
        node.actions = self.action_names(&path);
        // EditableText is not an action, so it never appears in the list above;
        // it is the element's own declaration that a value can be written here.
        if self.has_interface(&path, "org.a11y.atspi.EditableText") {
            node.actions.push("set_value".to_string());
        }
        Ok(node)
    }

    pub fn invoke_element_action(
        &self,
        element_id: &str,
        action: &str,
        value: Option<&str>,
    ) -> Result<(), GuiError> {
        let path = Self::parse_path(element_id)?;
        let normalized = action.trim().to_ascii_lowercase().replace('-', "_");
        if matches!(normalized.as_str(), "set_value" | "setvalue" | "set_text") {
            let Some(wanted) = value else {
                return Err(GuiError::PlatformError(format!(
                    "the '{action}' action on element '{element_id}' writes a value and needs a value argument; none was given"
                )));
            };
            // Empty string is a legitimate write: it clears the field.
            return self.set_text_contents(&path, wanted);
        }
        let names = self.action_names(&path);
        let index = names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(action))
            .ok_or_else(|| {
                GuiError::PlatformError(format!(
                    "action '{action}' is not available on element '{element_id}'; available actions: [{}]",
                    names.join(", ")
                ))
            })? as i32;

        if self.do_action(&path, index)? {
            Ok(())
        } else {
            Err(GuiError::PlatformError(format!(
                "AT-SPI refused DoAction({index} = '{action}') on element '{element_id}'"
            )))
        }
    }
}
