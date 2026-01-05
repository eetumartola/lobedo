use lobedo_core::BuiltinNodeKind;

#[derive(Clone, Copy)]
pub(super) struct MenuItem {
    pub(super) kind: BuiltinNodeKind,
    pub(super) name: &'static str,
    pub(super) category: &'static str,
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
    vec![
        MenuItem {
            kind: BuiltinNodeKind::Box,
            name: "Box",
            category: "Sources",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Grid,
            name: "Grid",
            category: "Sources",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Sphere,
            name: "Sphere",
            category: "Sources",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Tube,
            name: "Tube",
            category: "Sources",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::File,
            name: "File",
            category: "Sources",
            submenu: Some("IO"),
        },
        MenuItem {
            kind: BuiltinNodeKind::ReadSplats,
            name: "Splat Read",
            category: "Sources",
            submenu: Some("IO"),
        },
        MenuItem {
            kind: BuiltinNodeKind::WriteSplats,
            name: "Splat Write",
            category: "Outputs",
            submenu: Some("IO"),
        },
        MenuItem {
            kind: BuiltinNodeKind::Scatter,
            name: "Scatter",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Delete,
            name: "Delete",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Prune,
            name: "Splat Prune",
            category: "Operators",
            submenu: Some("Splat"),
        },
        MenuItem {
            kind: BuiltinNodeKind::Regularize,
            name: "Splat Regularize",
            category: "Operators",
            submenu: Some("Splat"),
        },
        MenuItem {
            kind: BuiltinNodeKind::Group,
            name: "Group",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Transform,
            name: "Transform",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::CopyTransform,
            name: "Copy/Transform",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Merge,
            name: "Merge",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::CopyToPoints,
            name: "Copy to Points",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Normal,
            name: "Normal",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Color,
            name: "Color",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Noise,
            name: "Noise/Mountain",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Smooth,
            name: "Smooth",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::Ray,
            name: "Ray",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::AttributeNoise,
            name: "Attribute Noise",
            category: "Operators",
            submenu: Some("Attribute"),
        },
        MenuItem {
            kind: BuiltinNodeKind::AttributeFromFeature,
            name: "Attribute from Feature",
            category: "Operators",
            submenu: Some("Attribute"),
        },
        MenuItem {
            kind: BuiltinNodeKind::AttributeTransfer,
            name: "Attribute Transfer",
            category: "Operators",
            submenu: Some("Attribute"),
        },
        MenuItem {
            kind: BuiltinNodeKind::AttributeMath,
            name: "Attribute Math",
            category: "Operators",
            submenu: Some("Attribute"),
        },
        MenuItem {
            kind: BuiltinNodeKind::Wrangle,
            name: "Wrangle",
            category: "Operators",
            submenu: None,
        },
        MenuItem {
            kind: BuiltinNodeKind::ObjOutput,
            name: "OBJ Output",
            category: "Outputs",
            submenu: Some("IO"),
        },
        MenuItem {
            kind: BuiltinNodeKind::Output,
            name: "Output",
            category: "Outputs",
            submenu: None,
        },
    ]
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
            sub_group.items.push(*item);
        } else {
            layout.items.push(*item);
        }
    }
    layout
}
