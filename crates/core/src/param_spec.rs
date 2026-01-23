#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Float,
    Int,
    Bool,
    Vec2,
    Vec3,
    String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamWidget {
    Default,
    Slider,
    Combo,
    Gradient,
    Code,
    Path,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamRange {
    Float { min: f32, max: f32 },
    Int { min: i32, max: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamOption {
    Int { value: i32, label: &'static str },
    String { value: &'static str, label: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamPathKind {
    ReadMesh,
    WriteObj,
    WriteGltf,
    ReadSplat,
    WriteSplat,
    ReadTexture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamCondition {
    Bool { key: &'static str, value: bool },
    Int { key: &'static str, value: i32 },
    IntIn { key: &'static str, values: Vec<i32> },
    String { key: &'static str, value: &'static str },
    StringIn { key: &'static str, values: Vec<&'static str> },
}

#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: ParamKind,
    pub widget: ParamWidget,
    pub range: Option<ParamRange>,
    pub options: Vec<ParamOption>,
    pub path_kind: Option<ParamPathKind>,
    pub help: Option<&'static str>,
    pub visible: bool,
    pub visible_when: Vec<ParamCondition>,
}

impl ParamSpec {
    pub fn new(key: &'static str, label: &'static str, kind: ParamKind) -> Self {
        Self {
            key,
            label,
            kind,
            widget: ParamWidget::Default,
            range: None,
            options: Vec::new(),
            path_kind: None,
            help: None,
            visible: true,
            visible_when: Vec::new(),
        }
    }

    pub fn float(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Float)
    }

    pub fn float_slider(key: &'static str, label: &'static str, min: f32, max: f32) -> Self {
        Self::new(key, label, ParamKind::Float).with_range(ParamRange::Float { min, max }, true)
    }

    pub fn int(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Int)
    }

    pub fn int_slider(key: &'static str, label: &'static str, min: i32, max: i32) -> Self {
        Self::new(key, label, ParamKind::Int).with_range(ParamRange::Int { min, max }, true)
    }

    pub fn int_enum(
        key: &'static str,
        label: &'static str,
        options: Vec<(i32, &'static str)>,
    ) -> Self {
        let options = options
            .into_iter()
            .map(|(value, label)| ParamOption::Int { value, label })
            .collect();
        Self::new(key, label, ParamKind::Int)
            .with_options(options, true)
    }

    pub fn bool(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Bool)
    }

    pub fn vec2(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Vec2)
    }

    pub fn vec3(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Vec3)
    }

    pub fn string(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::String)
    }

    pub fn path(key: &'static str, label: &'static str, kind: ParamPathKind) -> Self {
        Self::string(key, label).with_path_kind(kind)
    }

    pub fn gradient(key: &'static str, label: &'static str) -> Self {
        Self::string(key, label).with_widget(ParamWidget::Gradient)
    }

    pub fn code(key: &'static str, label: &'static str) -> Self {
        Self::string(key, label).with_widget(ParamWidget::Code)
    }

    pub fn string_enum(
        key: &'static str,
        label: &'static str,
        options: Vec<(&'static str, &'static str)>,
    ) -> Self {
        let options = options
            .into_iter()
            .map(|(value, label)| ParamOption::String { value, label })
            .collect();
        Self::new(key, label, ParamKind::String)
            .with_options(options, true)
    }

    pub fn with_help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }

    pub fn with_widget(mut self, widget: ParamWidget) -> Self {
        self.widget = widget;
        self
    }

    pub fn with_path_kind(mut self, kind: ParamPathKind) -> Self {
        self.path_kind = Some(kind);
        self.widget = ParamWidget::Path;
        self
    }

    pub fn with_range(mut self, range: ParamRange, slider: bool) -> Self {
        self.range = Some(range);
        if slider {
            self.widget = ParamWidget::Slider;
        }
        self
    }

    pub fn with_options(mut self, options: Vec<ParamOption>, combo: bool) -> Self {
        self.options = options;
        if combo {
            self.widget = ParamWidget::Combo;
        }
        self
    }

    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    pub fn visible_when_bool(mut self, key: &'static str, value: bool) -> Self {
        self.visible_when.push(ParamCondition::Bool { key, value });
        self
    }

    pub fn visible_when_int(mut self, key: &'static str, value: i32) -> Self {
        self.visible_when.push(ParamCondition::Int { key, value });
        self
    }

    pub fn visible_when_int_in(mut self, key: &'static str, values: &[i32]) -> Self {
        self.visible_when.push(ParamCondition::IntIn {
            key,
            values: values.to_vec(),
        });
        self
    }

    pub fn visible_when_string(mut self, key: &'static str, value: &'static str) -> Self {
        self.visible_when.push(ParamCondition::String { key, value });
        self
    }

    pub fn visible_when_string_in(
        mut self,
        key: &'static str,
        values: &[&'static str],
    ) -> Self {
        self.visible_when.push(ParamCondition::StringIn {
            key,
            values: values.to_vec(),
        });
        self
    }

    pub fn is_visible(&self, params: &crate::graph::NodeParams) -> bool {
        if !self.visible {
            return false;
        }
        self.visible_when
            .iter()
            .all(|condition| condition.matches(params))
    }
}

impl ParamCondition {
    fn matches(&self, params: &crate::graph::NodeParams) -> bool {
        use crate::graph::ParamValue;

        match self {
            ParamCondition::Bool { key, value } => params
                .values
                .get(*key)
                .and_then(|param| match param {
                    ParamValue::Bool(v) => Some(*v),
                    _ => None,
                })
                .map(|v| v == *value)
                .unwrap_or(false),
            ParamCondition::Int { key, value } => params
                .values
                .get(*key)
                .and_then(|param| match param {
                    ParamValue::Int(v) => Some(*v),
                    _ => None,
                })
                .map(|v| v == *value)
                .unwrap_or(false),
            ParamCondition::IntIn { key, values } => params
                .values
                .get(*key)
                .and_then(|param| match param {
                    ParamValue::Int(v) => Some(*v),
                    _ => None,
                })
                .map(|v| values.contains(&v))
                .unwrap_or(false),
            ParamCondition::String { key, value } => params
                .values
                .get(*key)
                .and_then(|param| match param {
                    ParamValue::String(v) => Some(v.as_str()),
                    _ => None,
                })
                .map(|v| v.eq_ignore_ascii_case(value))
                .unwrap_or(false),
            ParamCondition::StringIn { key, values } => params
                .values
                .get(*key)
                .and_then(|param| match param {
                    ParamValue::String(v) => Some(v.as_str()),
                    _ => None,
                })
                .map(|v| values.iter().any(|candidate| v.eq_ignore_ascii_case(candidate)))
                .unwrap_or(false),
        }
    }
}
