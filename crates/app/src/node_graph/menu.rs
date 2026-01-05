use lobedo_core::{node_definition, node_specs, BuiltinNodeKind};

#[derive(Clone)]
pub(super) struct MenuItem {
    pub(super) kind: BuiltinNodeKind,
    pub(super) name: String,
    pub(super) category: String,
    pub(super) submenu: Option<&'static str>,
}

pub(super) struct MenuLayout {
    pub(super) items: Vec<MenuItem>,
    pub(super) submenus: Vec<MenuSubgroup>,
}

pub(super) struct MenuSubgroup {
    pub(super) name: &'static str,
    pub(super) items: Vec<MenuItem>,
}

pub(super) fn builtin_menu_items() -> Vec<MenuItem> {
    node_specs()
        .iter()
        .map(|spec| {
            let definition = node_definition(spec.kind);
            MenuItem {
                kind: spec.kind,
                name: definition.name,
                category: definition.category,
                submenu: submenu_for_kind(spec.kind),
            }
        })
        .collect()
}

pub(super) fn menu_layout(items: &[MenuItem]) -> MenuLayout {
    let mut layout = MenuLayout {
        items: Vec::new(),
        submenus: Vec::new(),
    };
    for item in items {
        if let Some(submenu) = item.submenu {
            let sub_group = match layout.submenus.iter_mut().find(|sub| sub.name == submenu) {
                Some(sub) => sub,
                None => {
                    layout.submenus.push(MenuSubgroup {
                        name: submenu,
                        items: Vec::new(),
                    });
                    layout.submenus.last_mut().unwrap()
                }
            };
            sub_group.items.push(item.clone());
        } else {
            layout.items.push(item.clone());
        }
    }
    layout
}

fn submenu_for_kind(kind: BuiltinNodeKind) -> Option<&'static str> {
    match kind {
        BuiltinNodeKind::File
        | BuiltinNodeKind::ReadSplats
        | BuiltinNodeKind::WriteSplats
        | BuiltinNodeKind::ObjOutput => Some("IO"),
        BuiltinNodeKind::Prune | BuiltinNodeKind::Regularize | BuiltinNodeKind::SplatLod => {
            Some("Splat")
        }
        BuiltinNodeKind::AttributeNoise
        | BuiltinNodeKind::AttributeFromFeature
        | BuiltinNodeKind::AttributeTransfer
        | BuiltinNodeKind::AttributeMath => Some("Attribute"),
        _ => None,
    }
}
