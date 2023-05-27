use std::collections::HashMap;
use std::sync::LazyLock;

use shared::util::NoneOr;
use uuid::Uuid;

use super::font::{Font, FontData};

use super::{Asset, AssetManager, Renderer, Texture};

impl AssetManager {
    pub fn new() -> Self {
        AssetManager {
            file_assets: HashMap::new(),
            id_assets: HashMap::new(),
            fonts: HashMap::new(),
        }
    }

    pub const fn reserve_id() -> LazyLock<Uuid> {
        LazyLock::new(|| Uuid::new_v4())
    }

    // File
    fn add_image_for_file<'a>(&'a mut self, file: &str, tex: Texture) -> &'a Texture {
        self.file_assets.insert(file.to_string(), tex);
        self.file_assets
            .get(&file.to_string())
            .expect("Failed to add texture")
    }

    pub fn get_asset_by_file<'a>(&'a self, file: &String) -> Option<&'a Texture> {
        self.file_assets.get(file)
    }

    pub fn get_or_load_asset_by_file<'a>(&'a mut self, file: &String, r: &Renderer) -> &'a Texture {
        if self.get_asset_by_file(file).is_none() {
            self.file_assets
                .insert(file.to_string(), r.create_texture_from_file(file));
        }
        self.get_asset_by_file(file).expect("File to load asset")
    }

    // Id
    pub fn get_asset_by_id<'a>(&'a self, id: Uuid) -> Option<&'a Texture> {
        self.id_assets.get(&id)
    }

    pub fn new_texture(&mut self, tex: Texture) -> Uuid {
        let id = Uuid::new_v4();
        self.id_assets.insert(id, tex);
        id
    }

    pub fn add_texture_for_id(&mut self, id: Uuid, tex: Texture) {
        self.id_assets.insert(id, tex);
    }

    // Asset
    pub fn load_asset<'a>(&'a mut self, r: &Renderer, asset: &'a Asset) -> Option<&'a Texture> {
        match asset {
            Asset::File(file) => Some(self.get_or_load_asset_by_file(file, r)),
            Asset::Id(id) => self.get_asset_by_id(*id),
        }
    }

    // Font
    pub fn get_font<'a>(&'a mut self, data: FontData) -> &'a Font {
        if self.fonts.get(&data).is_none() {
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
            self.fonts
                .insert(data.clone(), Font::from_file(&file, min_size));
        }
        self.fonts.get(&data).expect("Failed to load font")
    }
}
