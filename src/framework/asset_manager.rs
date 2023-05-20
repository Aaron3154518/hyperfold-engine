use std::collections::HashMap;

use shared::util::NoneOr;
use uuid::Uuid;

use super::{
    font::{Font, FontAccess, FontData, FontTrait},
    renderer::RendererTrait,
    texture::Texture,
};

pub enum Asset {
    File(String),
    Id(Uuid),
    Texture(Texture),
}

pub struct AssetManager {
    file_assets: HashMap<String, Texture>,
    id_assets: HashMap<Uuid, Texture>,
    fonts: HashMap<FontData, Font>,
}

impl AssetManager {
    pub fn new() -> Self {
        AssetManager {
            file_assets: HashMap::new(),
            id_assets: HashMap::new(),
            fonts: HashMap::new(),
        }
    }
}

impl AssetManagerTrait for AssetManager {
    fn asset_manager<'a>(&'a self) -> &'a AssetManager {
        self
    }

    fn asset_manager_mut<'a>(&'a mut self) -> &'a mut AssetManager {
        self
    }
}

pub trait AssetManagerTrait {
    fn asset_manager<'a>(&'a self) -> &'a AssetManager;
    fn asset_manager_mut<'a>(&'a mut self) -> &'a mut AssetManager;

    fn get_asset_by_file<'a>(&'a self, file: &String) -> Option<&'a Texture> {
        self.asset_manager().file_assets.get(file)
    }

    fn get_or_load_asset_by_file<'a>(
        &'a mut self,
        file: &String,
        r: &impl RendererTrait,
    ) -> &'a Texture {
        if self.get_asset_by_file(file).is_none() {
            let am = self.asset_manager_mut();
            am.file_assets
                .insert(file.to_string(), Texture::from_file(r, file));
            self.get_asset_by_file(file).expect("Failed to load asset")
        } else {
            self.get_asset_by_file(file).expect("File to get asset")
        }
    }

    fn get_asset_by_id<'a>(&'a self, id: Uuid) -> Option<&'a Texture> {
        self.asset_manager().id_assets.get(&id)
    }

    fn add_texture(&mut self, tex: Texture) -> Asset {
        let id = Uuid::new_v4();
        self.asset_manager_mut().id_assets.insert(id, tex);
        Asset::Id(id)
    }

    fn add_image<'a>(&'a mut self, file: &str, tex: Texture) -> Option<&'a Texture> {
        let am = self.asset_manager_mut();
        am.file_assets.insert(file.to_string(), tex);
        am.file_assets.get(&file.to_string())
    }

    fn get_font(&mut self, data: FontData) -> Option<FontAccess> {
        let am = self.asset_manager_mut();
        match am.fonts.get(&data) {
            Some(f) => Some(f.access()),
            None => {
                // Min is always too small or just right, max is too big
                let (mut min_size, mut max_size) = (1, 10);
                // If both dimensions are none, use smallest font
                if data.w.is_some() || data.h.is_some() {
                    let mut dim = Font::from_file(&data.file, min_size).size_text(&data.sample);
                    // While too small
                    while data.w.is_none_or(|w| dim.w as u32 <= *w)
                        && data.h.is_none_or(|h| dim.h as u32 <= *h)
                    {
                        min_size = max_size;
                        max_size *= 2;
                        dim = Font::from_file(&data.file, max_size).size_text(&data.sample);
                    }

                    // Terminate when max_size (too big) is right after min_size (too small)
                    while max_size - min_size > 1 {
                        let size = (max_size + min_size) / 2;
                        dim = Font::from_file(&data.file, size).size_text(&data.sample);
                        // Too big
                        if data.w.is_some_and(|w| dim.w as u32 > w)
                            || data.h.is_some_and(|h| dim.h as u32 > h)
                        {
                            max_size = size;
                        } else {
                            // Too small or just right
                            min_size = size;
                        }
                    }
                }

                let file = data.file.to_string();
                am.fonts
                    .try_insert(data, Font::from_file(&file, min_size))
                    .ok()
                    .map(|f| f.access())
            }
        }
    }
}
