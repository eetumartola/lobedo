use egui::Ui;

use lobedo_core::{node_definition, node_specs, BuiltinNodeKind};

use super::utils::submenu_menu_button;

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

pub(super) fn render_menu_layout(ui: &mut Ui, layout: MenuLayout) -> Option<BuiltinNodeKind> {
    for submenu in layout.submenus {
        let mut picked = None;
        submenu_menu_button(ui, submenu.name, |ui| {
            for item in &submenu.items {
                if ui.button(item.name.as_str()).clicked() {
                    picked = Some(item.kind);
                    ui.close();
                }
            }
        });
        if picked.is_some() {
            return picked;
        }
    }

    for item in layout.items {
        if ui.button(item.name.as_str()).clicked() {
            return Some(item.kind);
        }
    }

    None
}

fn submenu_for_kind(kind: BuiltinNodeKind) -> Option<&'static str> {
    match kind {
        BuiltinNodeKind::File
        | BuiltinNodeKind::ReadSplats
        | BuiltinNodeKind::WriteSplats
        | BuiltinNodeKind::ObjOutput => Some("IO"),
        BuiltinNodeKind::Prune
        | BuiltinNodeKind::Regularize
        | BuiltinNodeKind::SplatLod
        | BuiltinNodeKind::SplatToMesh
        | BuiltinNodeKind::SplatDeform
        | BuiltinNodeKind::SplatDelight
        | BuiltinNodeKind::SplatIntegrate => Some("Splat"),
        BuiltinNodeKind::AttributeNoise
        | BuiltinNodeKind::ErosionNoise
        | BuiltinNodeKind::AttributeFromFeature
        | BuiltinNodeKind::AttributeFromVolume
        | BuiltinNodeKind::AttributeTransfer
        | BuiltinNodeKind::AttributeMath
        | BuiltinNodeKind::UvTexture
        | BuiltinNodeKind::UvUnwrap
        | BuiltinNodeKind::UvView => Some("Attribute"),
        BuiltinNodeKind::Material => Some("Materials"),
        BuiltinNodeKind::VolumeFromGeometry
        | BuiltinNodeKind::VolumeCombine
        | BuiltinNodeKind::VolumeToMesh => Some("Volume"),
        _ => None,
    }
}
