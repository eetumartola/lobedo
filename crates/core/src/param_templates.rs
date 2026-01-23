use crate::param_spec::ParamSpec;

pub fn transform_params(include_pivot: bool) -> Vec<ParamSpec> {
    let mut specs = vec![
        ParamSpec::vec3("translate", "Translate")
            .with_help("Translation in X/Y/Z."),
        ParamSpec::vec3("rotate_deg", "Rotate")
            .with_help("Rotation in degrees (XYZ)."),
        ParamSpec::vec3("scale", "Scale")
            .with_help("Scale factors (XYZ)."),
    ];
    if include_pivot {
        specs.push(ParamSpec::vec3("pivot", "Pivot").with_help("Pivot point."));
    }
    specs
}

pub fn selection_shape_specs(
    include_selection: bool,
    include_attribute: bool,
) -> Vec<ParamSpec> {
    let mut shape_options = vec![("box", "Box"), ("sphere", "Sphere"), ("plane", "Plane")];
    if include_selection {
        shape_options.push(("selection", "Selection"));
    }
    if include_attribute {
        shape_options.push(("attribute", "Attribute"));
    }
    vec![
        ParamSpec::string_enum("shape", "Shape", shape_options)
            .with_help("Selection shape."),
        ParamSpec::bool("invert", "Invert")
            .with_help("Invert selection."),
        ParamSpec::vec3("center", "Center")
            .with_help("Shape center.")
            .visible_when_string_in("shape", &["box", "sphere"]),
        ParamSpec::vec3("size", "Size")
            .with_help("Box size in X/Y/Z.")
            .visible_when_string_in("shape", &["box", "sphere"]),
        ParamSpec::float_slider("radius", "Radius", 0.0, 1000.0)
            .with_help("Sphere radius.")
            .hidden(),
        ParamSpec::vec3("plane_origin", "Plane Origin")
            .with_help("Plane origin.")
            .visible_when_string("shape", "plane"),
        ParamSpec::vec3("plane_normal", "Plane Normal")
            .with_help("Plane normal.")
            .visible_when_string("shape", "plane"),
    ]
}
