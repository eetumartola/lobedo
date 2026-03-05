#[derive(Debug, Clone)]
pub enum ImageData {
    RgbF32 {
        width: u32,
        height: u32,
        data: Vec<f32>,
    },
    R32F {
        width: u32,
        height: u32,
        data: Vec<f32>,
    },
    R32U {
        width: u32,
        height: u32,
        data: Vec<u32>,
    },
}

impl ImageData {
    pub fn width(&self) -> u32 {
        match self {
            ImageData::RgbF32 { width, .. } => *width,
            ImageData::R32F { width, .. } => *width,
            ImageData::R32U { width, .. } => *width,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            ImageData::RgbF32 { height, .. } => *height,
            ImageData::R32F { height, .. } => *height,
            ImageData::R32U { height, .. } => *height,
        }
    }

    pub fn len(&self) -> usize {
        (self.width() as usize).saturating_mul(self.height() as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn rgb_data(&self) -> Option<(&[f32], u32, u32)> {
        match self {
            ImageData::RgbF32 {
                width,
                height,
                data,
            } => Some((data.as_slice(), *width, *height)),
            _ => None,
        }
    }

    pub fn depth_data(&self) -> Option<(&[f32], u32, u32)> {
        match self {
            ImageData::R32F {
                width,
                height,
                data,
            } => Some((data.as_slice(), *width, *height)),
            _ => None,
        }
    }

    pub fn seg_data(&self) -> Option<(&[u32], u32, u32)> {
        match self {
            ImageData::R32U {
                width,
                height,
                data,
            } => Some((data.as_slice(), *width, *height)),
            _ => None,
        }
    }

    pub fn same_size(&self, other: &ImageData) -> bool {
        self.width() == other.width() && self.height() == other.height()
    }

    pub fn from_rgb(width: u32, height: u32, data: Vec<f32>) -> Result<Self, String> {
        let expected = width as usize * height as usize * 3;
        if data.len() != expected {
            return Err(format!(
                "RGB image data length {} does not match {}x{}",
                data.len(),
                width,
                height
            ));
        }
        Ok(ImageData::RgbF32 {
            width,
            height,
            data,
        })
    }

    pub fn from_depth(width: u32, height: u32, data: Vec<f32>) -> Result<Self, String> {
        let expected = width as usize * height as usize;
        if data.len() != expected {
            return Err(format!(
                "Depth image data length {} does not match {}x{}",
                data.len(),
                width,
                height
            ));
        }
        Ok(ImageData::R32F {
            width,
            height,
            data,
        })
    }

    pub fn from_seg(width: u32, height: u32, data: Vec<u32>) -> Result<Self, String> {
        let expected = width as usize * height as usize;
        if data.len() != expected {
            return Err(format!(
                "Segmentation image data length {} does not match {}x{}",
                data.len(),
                width,
                height
            ));
        }
        Ok(ImageData::R32U {
            width,
            height,
            data,
        })
    }
}
