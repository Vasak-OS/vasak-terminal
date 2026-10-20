//! Prueba de guardia del afinado del WebKit.
//!
//! No corre GTK ni WebKit: lee el fuente y verifica que el cableado siga en su
//! sitio. El glue real vive en `commands/webkit.rs`, que no se puede ejercitar
//! sin un display, así que esto comprueba que las palancas seguras, el registro
//! del comando y la llamada del frontend no se borren por descuido.

use std::fs;
use std::path::PathBuf;

fn read_source(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("no se pudo leer {relative}: {err}"))
}

#[test]
fn el_glue_tiene_las_palancas_seguras() {
    let source = read_source("src/commands/webkit.rs");

    assert!(
        source.contains("pub fn tune_webview"),
        "falta el comando `tune_webview` en commands/webkit.rs"
    );
    assert!(
        source.contains("set_enable_developer_extras(false)"),
        "falta apagar los extras de desarrollo"
    );
    assert!(
        source.contains("set_enable_back_forward_navigation_gestures(false)"),
        "falta apagar los gestos de navegación atrás/adelante"
    );
    assert!(
        source.contains("CacheModel::DocumentViewer"),
        "falta el caché de visor de documento"
    );
}

#[test]
fn el_glue_no_toca_la_aceleracion_ni_webgl() {
    let source = read_source("src/commands/webkit.rs");

    assert!(
        !source.contains("set_hardware_acceleration_policy"),
        "no se debe tocar la aceleración por hardware: el emulador tironearía"
    );
    assert!(
        !source.contains("set_enable_webgl"),
        "no se debe tocar WebGL: el emulador tironearía"
    );
}

#[test]
fn el_comando_esta_registrado_en_lib() {
    let source = read_source("src/lib.rs");

    assert!(
        source.contains("commands::webkit::tune_webview"),
        "`tune_webview` no está en el generate_handler de lib.rs"
    );
}

#[test]
fn el_frontend_invoca_el_afinado() {
    let source = read_source("../src/App.vue");

    assert!(
        source.contains("invoke('tune_webview')"),
        "App.vue no invoca `tune_webview` al montar"
    );
}
