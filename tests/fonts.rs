//! The bundled legacy fonts load under the family and weight that
//! `terminal_settings::FONT_FACES` asks the text system for.

use termian_native::terminal_settings::{font_files, FONT_FACES};

#[test]
fn every_face_resolves_to_its_files() {
    for face in FONT_FACES {
        let files = font_files(&face);
        assert_eq!(files.len(), face.files.len(), "{}: files missing", face.label);
        let loaded: Vec<(String, u16)> = files
            .into_iter()
            .map(|bytes| {
                let font = font_kit::loaders::default::Font::from_bytes(std::sync::Arc::new(bytes), 0).unwrap();
                (font.family_name(), font.properties().weight.0 as u16)
            })
            .collect();
        let has = |(family, weight): (&str, u16)| loaded.iter().any(|(f, w)| f == family && *w == weight);
        assert!(has(face.regular), "{}: regular {:?} not in {loaded:?}", face.label, face.regular);
        if face.files.len() > 1 {
            assert!(has(face.bold), "{}: bold {:?} not in {loaded:?}", face.label, face.bold);
        }
    }
}
