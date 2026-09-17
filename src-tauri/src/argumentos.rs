//! Qué pidió la línea de comandos.
//!
//! `vasak-terminal` sólo se abría a mano. Ningún programa podía pedirle que
//! corriera algo, que es lo que hace cualquier otra terminal con `-e`, y eso la
//! dejaba afuera de todo lo que necesita abrir una consola: los instaladores,
//! los `.desktop` con `Terminal=true`, los editores. El caso que lo destapó fue
//! `limine-snapper-restore`, que prueba quince terminales por nombre —konsole,
//! kgx, gnome-terminal, foot, kitty, alacritty…— y termina en un
//! «No suitable terminal found», porque ninguna de las quince es ésta y la
//! nuestra no entendía el `-e` con el que las llama a todas.
//!
//! # Lo que se acepta
//!
//! `-e`, `--command` y `--` marcan dónde empieza el programa; las tres porque
//! las tres se usan —xterm y konsole llaman con `-e`, gnome-terminal con `--`—
//! y quien escribe el llamador no siempre puede elegir.
//!
//! Después de la marca va el programa con sus argumentos, uno por token. La
//! forma de un solo token con espacios —`-e "ls -la"`— también se entiende,
//! porque es como la escribe la gente a mano; ahí sí hay que partir, y se parte
//! con las reglas de la shell.
//!
//! # Lo que no
//!
//! Nada después de la marca se interpreta como opción nuestra. `-e algo
//! --overlay` corre `algo --overlay`, y no abre la ventana desplegable: el
//! `--overlay` es del programa, no de la terminal. Antes esto se miraba con un
//! `args().any(|a| a == "--overlay")` sobre la línea entera, que habría hecho
//! exactamente lo contrario.

use shell_words::split;
use std::path::{Path, PathBuf};

/// Lo que la línea de comandos pidió.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Invocacion {
    /// La ventana desplegable.
    pub overlay: bool,
    /// El programa a correr, ya separado en argumentos. `None` es la shell de
    /// siempre.
    pub comando: Option<Vec<String>>,
}

/// Dónde empieza el programa.
const MARCAS: [&str; 3] = ["-e", "--command", "--"];

/// Lee la línea de comandos, sin el nombre del programa.
pub fn leer(args: &[String]) -> Invocacion {
    let mut overlay = false;

    for (posicion, arg) in args.iter().enumerate() {
        if MARCAS.contains(&arg.as_str()) {
            return Invocacion {
                // Un `--overlay` de más atrás no se pierde acá: la desplegable
                // es una ventana única y compartida, y meterle un programa
                // ajeno sería colgárselo a la sesión de otro. Con un comando,
                // la ventana es común y propia.
                overlay: false,
                comando: comando_de(&args[posicion + 1..]),
            };
        }

        if arg == "--overlay" {
            overlay = true;
        }
    }

    Invocacion {
        overlay,
        comando: None,
    }
}

/// El programa que sigue a la marca.
fn comando_de(resto: &[String]) -> Option<Vec<String>> {
    let argv = match resto {
        // `-e` y nada más. No hay programa, así que se abre la shell de
        // siempre: mejor una terminal común que una ventana que se cierra sola.
        [] => return None,
        // Un solo token con espacios es la forma escrita a mano.
        [solo] if solo.contains(char::is_whitespace) => split(solo).ok()?,
        _ => resto.to_vec(),
    };

    // Un programa vacío no se puede ejecutar, y `CommandBuilder` con la cadena
    // vacía falla más tarde y más lejos.
    if argv.first().is_none_or(|programa| programa.is_empty()) {
        return None;
    }

    Some(argv)
}

/// Si el programa existe y se puede ejecutar.
///
/// Se comprueba **antes** de abrir la ventana, y no se deja para el momento de
/// arrancar el PTY. Un `-e` con un programa que no está deja, si no, una ventana
/// abierta y vacía para siempre: el error aparece en la consola del webview, que
/// nadie mira, y quien la abrió —un instalador, un `.desktop`, un guion— se
/// queda esperando algo que no va a pasar. Medido así antes de esta comprobación.
///
/// `rutas` es el PATH ya partido, por parámetro para poder probarlo: con el del
/// equipo, la prueba diría cosas distintas según qué tenga instalado quien la
/// corra.
pub fn esta_disponible(programa: &str, rutas: &[PathBuf]) -> bool {
    // Con una barra es un camino, y el PATH no interviene — igual que en la
    // shell.
    if programa.contains('/') {
        return es_ejecutable(Path::new(programa));
    }

    rutas.iter().any(|ruta| es_ejecutable(&ruta.join(programa)))
}

#[cfg(unix)]
fn es_ejecutable(ruta: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(ruta)
        .map(|datos| datos.is_file() && datos.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn es_ejecutable(ruta: &Path) -> bool {
    ruta.is_file()
}

/// El PATH del entorno, partido. Vacío si no hay.
pub fn rutas_del_entorno() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn leer_de(args: &[&str]) -> Invocacion {
        leer(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    fn comando(args: &[&str]) -> Option<Vec<String>> {
        Some(args.iter().map(|a| a.to_string()).collect())
    }

    #[test]
    fn sin_argumentos_es_la_shell_de_siempre() {
        assert_eq!(leer_de(&[]), Invocacion::default());
    }

    #[test]
    fn overlay_sigue_siendo_overlay() {
        assert_eq!(
            leer_de(&["--overlay"]),
            Invocacion {
                overlay: true,
                comando: None
            }
        );
    }

    #[test]
    fn el_programa_va_despues_de_la_marca() {
        // Es la forma con la que llama `limine-snapper-restore`: cada argumento
        // por separado.
        assert_eq!(
            leer_de(&["-e", "sudo", "limine-snapper-sync", "--restore"]).comando,
            comando(&["sudo", "limine-snapper-sync", "--restore"])
        );
    }

    #[test]
    fn las_tres_marcas_valen_lo_mismo() {
        for marca in MARCAS {
            assert_eq!(
                leer_de(&[marca, "htop"]).comando,
                comando(&["htop"]),
                "la marca {marca} no empezó el comando"
            );
        }
    }

    #[test]
    fn un_solo_token_con_espacios_se_parte_como_en_la_shell() {
        assert_eq!(
            leer_de(&["-e", "ls -la '/un directorio'"]).comando,
            comando(&["ls", "-la", "/un directorio"])
        );
    }

    #[test]
    fn un_solo_token_sin_espacios_es_el_programa_entero() {
        assert_eq!(leer_de(&["-e", "htop"]).comando, comando(&["htop"]));
    }

    #[test]
    fn los_argumentos_del_programa_no_son_opciones_nuestras() {
        // Lo importante es el `--overlay`: con la comprobación vieja —un
        // `any()` sobre la línea entera— esto abría la desplegable en vez de
        // correr el programa.
        let invocacion = leer_de(&["-e", "mi-programa", "--overlay"]);

        assert!(!invocacion.overlay);
        assert_eq!(invocacion.comando, comando(&["mi-programa", "--overlay"]));
    }

    #[test]
    fn un_overlay_anterior_no_sobrevive_al_comando() {
        let invocacion = leer_de(&["--overlay", "-e", "htop"]);

        assert!(!invocacion.overlay);
        assert_eq!(invocacion.comando, comando(&["htop"]));
    }

    #[test]
    fn la_marca_sola_no_es_un_comando() {
        assert_eq!(leer_de(&["-e"]).comando, None);
    }

    #[test]
    fn un_programa_vacio_no_es_un_comando() {
        assert_eq!(leer_de(&["-e", ""]).comando, None);
        assert_eq!(leer_de(&["-e", "   "]).comando, None);
    }

    #[test]
    fn una_comilla_sin_cerrar_no_es_un_comando() {
        // `split` falla, y correr media línea sería peor que no correr nada.
        assert_eq!(leer_de(&["-e", "echo 'sin cerrar"]).comando, None);
    }

    /// Un directorio con un ejecutable y un archivo común adentro.
    fn un_directorio_con(ejecutable: &str, comun: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let base = std::env::temp_dir().join(format!(
            "vasak-terminal-pruebas-{}-{ejecutable}",
            std::process::id()
        ));
        std::fs::create_dir_all(&base).expect("no se pudo crear el directorio");

        std::fs::write(base.join(ejecutable), "#!/bin/sh\n").expect("no se pudo escribir");
        std::fs::set_permissions(
            base.join(ejecutable),
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("no se pudo dar permiso");

        std::fs::write(base.join(comun), "no soy ejecutable").expect("no se pudo escribir");
        std::fs::set_permissions(base.join(comun), std::fs::Permissions::from_mode(0o644))
            .expect("no se pudo quitar permiso");

        base
    }

    #[test]
    fn un_programa_del_path_esta_disponible() {
        let dir = un_directorio_con("si-esta", "no-ejecutable");

        assert!(esta_disponible("si-esta", &[dir.clone()]));
        assert!(!esta_disponible("no-esta", &[dir]));
    }

    #[test]
    fn un_archivo_sin_permiso_de_ejecucion_no_cuenta() {
        // Si contara, la ventana se abriría igual y el fallo aparecería
        // después, que es justo lo que esta comprobación evita.
        let dir = un_directorio_con("igual", "no-ejecutable");

        assert!(!esta_disponible("no-ejecutable", &[dir]));
    }

    #[test]
    fn un_camino_con_barra_no_mira_el_path() {
        let dir = un_directorio_con("con-barra", "otro");
        let completo = dir.join("con-barra");

        assert!(esta_disponible(completo.to_str().unwrap(), &[]));
        assert!(!esta_disponible("/no/existe/este-programa", &[dir]));
    }

    #[test]
    fn un_directorio_no_es_un_programa() {
        let dir = un_directorio_con("cualquiera", "otro");

        assert!(!esta_disponible(dir.to_str().unwrap(), &[]));
    }

    #[test]
    fn una_ruta_suelta_no_es_un_comando() {
        // Ésa es la otra entrada, la del `%f` del .desktop: un archivo o un
        // directorio que se resuelve aparte. Acá tiene que salir `None` para
        // que ese camino siga existiendo.
        assert_eq!(leer_de(&["/home/alguien/guion.sh"]).comando, None);
    }
}
