//! User interface text in the languages the macOS version ships with,
//! adapted to Windows terminology. The language follows the Windows display
//! language, falling back to English.

use std::sync::OnceLock;

use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
use windows::core::PWSTR;

use crate::action::WindowAction;

pub struct Strings {
    pub center: &'static str,
    pub fullscreen: &'static str,
    pub left_half: &'static str,
    pub right_half: &'static str,
    pub top_half: &'static str,
    pub bottom_half: &'static str,
    pub upper_left: &'static str,
    pub lower_left: &'static str,
    pub upper_right: &'static str,
    pub lower_right: &'static str,
    pub next_display: &'static str,
    pub previous_display: &'static str,
    pub next_third: &'static str,
    pub previous_third: &'static str,
    pub make_larger: &'static str,
    pub make_smaller: &'static str,
    pub undo: &'static str,
    pub redo: &'static str,

    pub section_basics: &'static str,
    pub section_halves: &'static str,
    pub section_corners: &'static str,
    pub section_displays_and_thirds: &'static str,
    pub section_resize: &'static str,
    pub section_history: &'static str,

    pub menu_about: &'static str,
    pub menu_disable: &'static str,
    pub menu_disable_for_an_hour: &'static str,
    /// `{}` is replaced with the application's name.
    pub menu_disable_for_application: &'static str,
    pub menu_settings: &'static str,
    pub menu_quit: &'static str,

    pub launch_at_login: &'static str,
    pub run_mode_label: &'static str,
    pub run_in_system_tray: &'static str,
    pub run_in_background: &'static str,
    pub restore_defaults: &'static str,

    pub recorder_click_to_record: &'static str,
    pub recorder_enter_shortcut: &'static str,
    pub recorder_stop_recording: &'static str,
    pub recorder_use_existing: &'static str,
    pub recorder_clear_shortcut: &'static str,
    pub recorder_not_registered: &'static str,

    pub alert_restore_defaults_message: &'static str,
    pub alert_restore_defaults_informative: &'static str,
    pub alert_running_in_background_message: &'static str,
    pub alert_running_in_background_informative: &'static str,
    /// `{}` is replaced with the shortcut.
    pub alert_shortcut_in_use_message: &'static str,
    /// `{}` is replaced with the shortcut.
    pub alert_shortcut_in_use_informative: &'static str,
    /// `{}` is replaced with the shortcut.
    pub alert_system_shortcut_informative: &'static str,
    pub button_ok: &'static str,
    pub button_cancel: &'static str,
    pub do_not_show_again: &'static str,

    pub about_description: &'static str,
    pub about_license: &'static str,

    pub shift_key: &'static str,
}

impl Strings {
    pub fn action(&self, action: WindowAction) -> &'static str {
        match action {
            WindowAction::Center => self.center,
            WindowAction::Fullscreen => self.fullscreen,
            WindowAction::LeftHalf => self.left_half,
            WindowAction::RightHalf => self.right_half,
            WindowAction::TopHalf => self.top_half,
            WindowAction::BottomHalf => self.bottom_half,
            WindowAction::UpperLeft => self.upper_left,
            WindowAction::LowerLeft => self.lower_left,
            WindowAction::UpperRight => self.upper_right,
            WindowAction::LowerRight => self.lower_right,
            WindowAction::NextDisplay => self.next_display,
            WindowAction::PreviousDisplay => self.previous_display,
            WindowAction::NextThird => self.next_third,
            WindowAction::PreviousThird => self.previous_third,
            WindowAction::Larger => self.make_larger,
            WindowAction::Smaller => self.make_smaller,
            WindowAction::Undo => self.undo,
            WindowAction::Redo => self.redo,
        }
    }
}

/// Fills the `{}` placeholder of a string.
pub fn format(template: &str, argument: &str) -> String {
    template.replacen("{}", argument, 1)
}

/// The strings for the Windows display language.
pub fn strings() -> &'static Strings {
    static STRINGS: OnceLock<&'static Strings> = OnceLock::new();
    STRINGS.get_or_init(|| {
        preferred_languages()
            .iter()
            .find_map(|language| strings_for_language(language))
            .unwrap_or(&ENGLISH)
    })
}

fn strings_for_language(language: &str) -> Option<&'static Strings> {
    let primary = language.split(['-', '_']).next()?.to_ascii_lowercase();
    match primary.as_str() {
        "en" => Some(&ENGLISH),
        "es" => Some(&SPANISH),
        "fi" => Some(&FINNISH),
        "fr" => Some(&FRENCH),
        "it" => Some(&ITALIAN),
        "pt" => Some(&PORTUGUESE),
        _ => None,
    }
}

fn preferred_languages() -> Vec<String> {
    let mut count = 0u32;
    let mut length = 0u32;
    // SAFETY: a null buffer asks for the required buffer length.
    if unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut length) }
        .is_err()
        || length == 0
    {
        return Vec::new();
    }
    let mut buffer = vec![0u16; length as usize];
    // SAFETY: the buffer holds `length` UTF-16 code units as requested.
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            Some(PWSTR(buffer.as_mut_ptr())),
            &mut length,
        )
    }
    .is_err()
    {
        return Vec::new();
    }
    // The buffer is a double null terminated list of language names.
    buffer
        .split(|unit| *unit == 0)
        .filter(|name| !name.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

static ENGLISH: Strings = Strings {
    center: "Center",
    fullscreen: "Fullscreen",
    left_half: "Left Half",
    right_half: "Right Half",
    top_half: "Top Half",
    bottom_half: "Bottom Half",
    upper_left: "Upper Left",
    lower_left: "Lower Left",
    upper_right: "Upper Right",
    lower_right: "Lower Right",
    next_display: "Next Display",
    previous_display: "Previous Display",
    next_third: "Next Third",
    previous_third: "Previous Third",
    make_larger: "Make Larger",
    make_smaller: "Make Smaller",
    undo: "Undo",
    redo: "Redo",
    section_basics: "Basics",
    section_halves: "Halves",
    section_corners: "Corners",
    section_displays_and_thirds: "Displays & Thirds",
    section_resize: "Resize",
    section_history: "History",
    menu_about: "About Spectacle",
    menu_disable: "Disable",
    menu_disable_for_an_hour: "for an hour",
    menu_disable_for_application: "for {}",
    menu_settings: "Settings",
    menu_quit: "Quit Spectacle",
    launch_at_login: "Launch Spectacle at sign-in",
    run_mode_label: "Run",
    run_in_system_tray: "in the system tray",
    run_in_background: "as a background application",
    restore_defaults: "Restore Defaults\u{2026}",
    recorder_click_to_record: "Click to record",
    recorder_enter_shortcut: "Enter shortcut",
    recorder_stop_recording: "Stop recording",
    recorder_use_existing: "Use existing",
    recorder_clear_shortcut: "Clear shortcut",
    recorder_not_registered: "In use by another application",
    alert_restore_defaults_message: "This will restore Spectacle's default shortcuts",
    alert_restore_defaults_informative: "Would you like to restore the default shortcuts? Any custom shortcuts will be lost.",
    alert_running_in_background_message: "This will cause Spectacle to run in the background",
    alert_running_in_background_informative: "Run Spectacle in the background without an icon in the system tray.\n\nTo access Spectacle's settings, open Spectacle again from the Start menu.",
    alert_shortcut_in_use_message: "Shortcut {} already in use.",
    alert_shortcut_in_use_informative: "The shortcut \u{201C}{}\u{201D} is already in use. Please select a new shortcut.",
    alert_system_shortcut_informative: "The shortcut \u{201C}{}\u{201D} is already used by Windows or another application.\n\nTo use this shortcut, change it in the application that uses it, or choose another shortcut.",
    button_ok: "OK",
    button_cancel: "Cancel",
    do_not_show_again: "Do not show this message again",
    about_description: "Easily organize windows without using a mouse.",
    about_license: "Spectacle is distributed under the MIT License.",
    shift_key: "Shift",
};

static SPANISH: Strings = Strings {
    center: "Centro",
    fullscreen: "Pantalla completa",
    left_half: "Mitad izquierda",
    right_half: "Mitad derecha",
    top_half: "Mitad superior",
    bottom_half: "Mitad inferior",
    upper_left: "Superior izquierda",
    lower_left: "Inferior izquierda",
    upper_right: "Superior derecha",
    lower_right: "Inferior derecha",
    next_display: "Monitor siguiente",
    previous_display: "Monitor anterior",
    next_third: "Tercio siguiente",
    previous_third: "Tercio anterior",
    make_larger: "Hacer más grande",
    make_smaller: "Hacer más pequeño",
    undo: "Deshacer",
    redo: "Rehacer",
    section_basics: "Básicos",
    section_halves: "Mitades",
    section_corners: "Esquinas",
    section_displays_and_thirds: "Pantallas y tercios",
    section_resize: "Tamaño",
    section_history: "Historial",
    menu_about: "Acerca de Spectacle",
    menu_disable: "Desactivar",
    menu_disable_for_an_hour: "durante una hora",
    menu_disable_for_application: "para {}",
    menu_settings: "Configuración",
    menu_quit: "Salir de Spectacle",
    launch_at_login: "Abrir Spectacle al iniciar sesión",
    run_mode_label: "Ejecutar",
    run_in_system_tray: "en la bandeja del sistema",
    run_in_background: "como aplicación en segundo plano",
    restore_defaults: "Restaurar predeterminados\u{2026}",
    recorder_click_to_record: "Clic para grabar",
    recorder_enter_shortcut: "Introduzca el atajo",
    recorder_stop_recording: "Detener grabación",
    recorder_use_existing: "Usar existente",
    recorder_clear_shortcut: "Borrar atajo",
    recorder_not_registered: "En uso por otra aplicación",
    alert_restore_defaults_message: "Esto restaurará los atajos predeterminados de Spectacle",
    alert_restore_defaults_informative: "¿Desea restaurar los atajos predeterminados? Se perderán los atajos personalizados.",
    alert_running_in_background_message: "Esto hará que Spectacle se ejecute en segundo plano",
    alert_running_in_background_informative: "Spectacle se ejecutará en segundo plano sin un icono en la bandeja del sistema.\n\nPara acceder a la configuración de Spectacle, vuelva a abrir Spectacle desde el menú Inicio.",
    alert_shortcut_in_use_message: "El atajo {} ya está en uso.",
    alert_shortcut_in_use_informative: "El atajo \u{201C}{}\u{201D} ya está en uso. Seleccione otro atajo.",
    alert_system_shortcut_informative: "El atajo \u{201C}{}\u{201D} ya lo utiliza Windows u otra aplicación.\n\nPara usar este atajo, cámbielo en la aplicación que lo utiliza o elija otro atajo.",
    button_ok: "Aceptar",
    button_cancel: "Cancelar",
    do_not_show_again: "No volver a mostrar este mensaje",
    about_description: "Organice fácilmente sus ventanas sin usar el ratón.",
    about_license: "Spectacle se distribuye bajo la licencia MIT.",
    shift_key: "Mayús",
};

static FINNISH: Strings = Strings {
    center: "Keskitä",
    fullscreen: "Koko näyttö",
    left_half: "Vasen puoli",
    right_half: "Oikea puoli",
    top_half: "Yläpuoli",
    bottom_half: "Alapuoli",
    upper_left: "Ylävasen",
    lower_left: "Alavasen",
    upper_right: "Yläoikea",
    lower_right: "Alaoikea",
    next_display: "Seuraava näyttö",
    previous_display: "Edellinen näyttö",
    next_third: "Seuraava kolmannes",
    previous_third: "Edellinen kolmannes",
    make_larger: "Suurenna",
    make_smaller: "Pienennä",
    undo: "Kumoa",
    redo: "Tee uudelleen",
    section_basics: "Perustoiminnot",
    section_halves: "Puolikkaat",
    section_corners: "Kulmat",
    section_displays_and_thirds: "Näytöt ja kolmannekset",
    section_resize: "Koko",
    section_history: "Historia",
    menu_about: "Tietoja Spectaclesta",
    menu_disable: "Poista käytöstä",
    menu_disable_for_an_hour: "tunniksi",
    menu_disable_for_application: "ohjelmassa {}",
    menu_settings: "Asetukset",
    menu_quit: "Lopeta Spectacle",
    launch_at_login: "Käynnistä Spectacle kirjauduttaessa",
    run_mode_label: "Suorita",
    run_in_system_tray: "ilmaisinalueella",
    run_in_background: "taustaohjelmana",
    restore_defaults: "Palauta oletukset\u{2026}",
    recorder_click_to_record: "Nauhoita napsauttamalla",
    recorder_enter_shortcut: "Syötä oikotie",
    recorder_stop_recording: "Pysäytä nauhoitus",
    recorder_use_existing: "Käytä aiempaa",
    recorder_clear_shortcut: "Tyhjennä oikotie",
    recorder_not_registered: "Toisen sovelluksen käytössä",
    alert_restore_defaults_message: "Tämä palauttaa Spectaclen oletusarvoiset näppäinoikotiet",
    alert_restore_defaults_informative: "Haluatko palauttaa oletusarvoiset näppäinoikotiet? Menetät itse määrittämäsi oikotiet.",
    alert_running_in_background_message: "Spectacle suoritetaan taustalla",
    alert_running_in_background_informative: "Spectacle suoritetaan taustalla ilman kuvaketta ilmaisinalueella.\n\nVoit avata Spectaclen asetukset käynnistämällä Spectaclen uudelleen Käynnistä-valikosta.",
    alert_shortcut_in_use_message: "Oikotie {} on jo käytössä.",
    alert_shortcut_in_use_informative: "Oikotie \u{201D}{}\u{201D} on jo käytössä. Valitse jokin muu oikotie.",
    alert_system_shortcut_informative: "Oikotie \u{201D}{}\u{201D} on jo Windowsin tai toisen sovelluksen käytössä.\n\nVoit käyttää tätä oikotietä, jos vaihdat sen sitä käyttävässä sovelluksessa, tai voit valita jonkin muun oikotien.",
    button_ok: "OK",
    button_cancel: "Peruuta",
    do_not_show_again: "Älä näytä tätä viestiä uudelleen",
    about_description: "Järjestä ikkunat helposti ilman hiirtä.",
    about_license: "Spectacle jaetaan MIT-lisenssillä.",
    shift_key: "Vaihto",
};

static FRENCH: Strings = Strings {
    center: "Centrer",
    fullscreen: "Plein écran",
    left_half: "Moitié gauche",
    right_half: "Moitié droite",
    top_half: "Moitié haute",
    bottom_half: "Moitié basse",
    upper_left: "En haut à gauche",
    lower_left: "En bas à gauche",
    upper_right: "En haut à droite",
    lower_right: "En bas à droite",
    next_display: "Écran suivant",
    previous_display: "Écran précédent",
    next_third: "Tiers suivant",
    previous_third: "Tiers précédent",
    make_larger: "Agrandir",
    make_smaller: "Réduire",
    undo: "Annuler",
    redo: "Rétablir",
    section_basics: "Essentiels",
    section_halves: "Moitiés",
    section_corners: "Coins",
    section_displays_and_thirds: "Écrans et tiers",
    section_resize: "Taille",
    section_history: "Historique",
    menu_about: "À propos de Spectacle",
    menu_disable: "Désactiver",
    menu_disable_for_an_hour: "pendant une heure",
    menu_disable_for_application: "pour {}",
    menu_settings: "Paramètres",
    menu_quit: "Quitter Spectacle",
    launch_at_login: "Ouvrir Spectacle à l'ouverture de session",
    run_mode_label: "Exécuter",
    run_in_system_tray: "dans la zone de notification",
    run_in_background: "comme une application d'arrière-plan",
    restore_defaults: "Rétablir par défaut\u{2026}",
    recorder_click_to_record: "Cliquez pour enregistrer",
    recorder_enter_shortcut: "Saisissez le raccourci",
    recorder_stop_recording: "Arrêter l'enregistrement",
    recorder_use_existing: "Utiliser l'existant",
    recorder_clear_shortcut: "Effacer le raccourci",
    recorder_not_registered: "Utilisé par une autre application",
    alert_restore_defaults_message: "Cela rétablira les raccourcis par défaut de Spectacle",
    alert_restore_defaults_informative: "Voulez-vous rétablir les raccourcis par défaut\u{00A0}? Les raccourcis personnalisés seront perdus.",
    alert_running_in_background_message: "Spectacle fonctionnera en arrière-plan",
    alert_running_in_background_informative: "Spectacle fonctionnera en arrière-plan sans icône dans la zone de notification.\n\nPour accéder aux paramètres de Spectacle, ouvrez de nouveau Spectacle depuis le menu Démarrer.",
    alert_shortcut_in_use_message: "Raccourci {} déjà utilisé.",
    alert_shortcut_in_use_informative: "Le raccourci «\u{00A0}{}\u{00A0}» est déjà utilisé. Veuillez en choisir un autre.",
    alert_system_shortcut_informative: "Le raccourci «\u{00A0}{}\u{00A0}» est déjà utilisé par Windows ou par une autre application.\n\nPour utiliser ce raccourci, modifiez-le dans l'application qui l'utilise ou choisissez-en un autre.",
    button_ok: "OK",
    button_cancel: "Annuler",
    do_not_show_again: "Ne plus afficher ce message",
    about_description: "Organisez facilement vos fenêtres sans utiliser la souris.",
    about_license: "Spectacle est distribué sous licence MIT.",
    shift_key: "Maj",
};

static ITALIAN: Strings = Strings {
    center: "Centra",
    fullscreen: "A tutto schermo",
    left_half: "Metà sinistra",
    right_half: "Metà destra",
    top_half: "Metà superiore",
    bottom_half: "Metà inferiore",
    upper_left: "In alto a sinistra",
    lower_left: "In basso a sinistra",
    upper_right: "In alto a destra",
    lower_right: "In basso a destra",
    next_display: "Monitor successivo",
    previous_display: "Monitor precedente",
    next_third: "Terzo successivo",
    previous_third: "Terzo precedente",
    make_larger: "Ingrandisci",
    make_smaller: "Riduci",
    undo: "Annulla",
    redo: "Ripristina",
    section_basics: "Essenziali",
    section_halves: "Metà",
    section_corners: "Angoli",
    section_displays_and_thirds: "Schermi e terzi",
    section_resize: "Dimensioni",
    section_history: "Cronologia",
    menu_about: "Informazioni su Spectacle",
    menu_disable: "Disattiva",
    menu_disable_for_an_hour: "per un'ora",
    menu_disable_for_application: "per {}",
    menu_settings: "Impostazioni",
    menu_quit: "Esci da Spectacle",
    launch_at_login: "Apri Spectacle all'accesso",
    run_mode_label: "Esegui",
    run_in_system_tray: "nell'area di notifica",
    run_in_background: "come applicazione in background",
    restore_defaults: "Ripristina predefiniti\u{2026}",
    recorder_click_to_record: "Fai clic per registrare",
    recorder_enter_shortcut: "Premi i tasti",
    recorder_stop_recording: "Interrompi registrazione",
    recorder_use_existing: "Usa esistente",
    recorder_clear_shortcut: "Cancella combinazione",
    recorder_not_registered: "In uso da un'altra applicazione",
    alert_restore_defaults_message: "Verranno ripristinate le combinazioni predefinite di Spectacle",
    alert_restore_defaults_informative: "Vuoi ripristinare le combinazioni predefinite? Le combinazioni personalizzate andranno perse.",
    alert_running_in_background_message: "Spectacle sarà eseguito in background",
    alert_running_in_background_informative: "Spectacle sarà eseguito in background senza un'icona nell'area di notifica.\n\nPer accedere alle impostazioni di Spectacle, apri di nuovo Spectacle dal menu Start.",
    alert_shortcut_in_use_message: "Combinazione {} già assegnata.",
    alert_shortcut_in_use_informative: "La combinazione \u{201C}{}\u{201D} è già assegnata. Scegline un'altra.",
    alert_system_shortcut_informative: "La combinazione \u{201C}{}\u{201D} è già usata da Windows o da un'altra applicazione.\n\nPer usare questa combinazione, modificala nell'applicazione che la usa oppure scegline un'altra.",
    button_ok: "OK",
    button_cancel: "Annulla",
    do_not_show_again: "Non mostrare più questo messaggio",
    about_description: "Organizza facilmente le finestre senza usare il mouse.",
    about_license: "Spectacle è distribuito con licenza MIT.",
    shift_key: "Maiusc",
};

static PORTUGUESE: Strings = Strings {
    center: "Centralizar",
    fullscreen: "Preencher Tela",
    left_half: "Metade Esquerda",
    right_half: "Metade Direita",
    top_half: "Metade Superior",
    bottom_half: "Metade Inferior",
    upper_left: "Esquerda Superior",
    lower_left: "Esquerda Inferior",
    upper_right: "Direita Superior",
    lower_right: "Direita Inferior",
    next_display: "Monitor Seguinte",
    previous_display: "Monitor Anterior",
    next_third: "Terço Seguinte",
    previous_third: "Terço Anterior",
    make_larger: "Aumentar",
    make_smaller: "Diminuir",
    undo: "Desfazer",
    redo: "Refazer",
    section_basics: "Básico",
    section_halves: "Metades",
    section_corners: "Cantos",
    section_displays_and_thirds: "Telas e terços",
    section_resize: "Tamanho",
    section_history: "Histórico",
    menu_about: "Sobre o Spectacle",
    menu_disable: "Desativar",
    menu_disable_for_an_hour: "por uma hora",
    menu_disable_for_application: "no {}",
    menu_settings: "Configurações",
    menu_quit: "Encerrar Spectacle",
    launch_at_login: "Abrir ao iniciar a sessão",
    run_mode_label: "Manter",
    run_in_system_tray: "na bandeja do sistema",
    run_in_background: "em 2º plano",
    restore_defaults: "Restaurar Padrões\u{2026}",
    recorder_click_to_record: "Clique para gravar",
    recorder_enter_shortcut: "Digite o atalho",
    recorder_stop_recording: "Parar de gravar",
    recorder_use_existing: "Usar existente",
    recorder_clear_shortcut: "Limpar atalho",
    recorder_not_registered: "Em uso por outro aplicativo",
    alert_restore_defaults_message: "Os atalhos padrão do Spectacle serão restaurados.",
    alert_restore_defaults_informative: "Deseja restaurar os atalhos padrão? Todos os atalhos personalizados serão perdidos.",
    alert_running_in_background_message: "O Spectacle funcionará em 2º plano.",
    alert_running_in_background_informative: "O Spectacle funcionará em 2º plano sem um ícone na bandeja do sistema.\n\nPara acessar as configurações do Spectacle, abra-o novamente pelo menu Iniciar.",
    alert_shortcut_in_use_message: "O atalho {} já está em uso.",
    alert_shortcut_in_use_informative: "O atalho \u{201C}{}\u{201D} já está em uso. Selecione um novo atalho.",
    alert_system_shortcut_informative: "O atalho \u{201C}{}\u{201D} já é usado pelo Windows ou por outro aplicativo.\n\nPara usá-lo, altere-o no aplicativo que o utiliza ou escolha outro atalho.",
    button_ok: "OK",
    button_cancel: "Cancelar",
    do_not_show_again: "Não mostrar esta mensagem novamente",
    about_description: "Organize facilmente as janelas sem usar o mouse.",
    about_license: "O Spectacle é distribuído sob a licença MIT.",
    shift_key: "Shift",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_languages_by_primary_subtag() {
        assert!(std::ptr::eq(
            strings_for_language("fr-CA").unwrap(),
            &FRENCH
        ));
        assert!(std::ptr::eq(
            strings_for_language("pt-BR").unwrap(),
            &PORTUGUESE
        ));
        assert!(std::ptr::eq(strings_for_language("ES").unwrap(), &SPANISH));
        assert!(strings_for_language("de-DE").is_none());
    }

    #[test]
    fn templates_have_exactly_one_placeholder() {
        for strings in [&ENGLISH, &SPANISH, &FINNISH, &FRENCH, &ITALIAN, &PORTUGUESE] {
            for template in [
                strings.menu_disable_for_application,
                strings.alert_shortcut_in_use_message,
                strings.alert_shortcut_in_use_informative,
                strings.alert_system_shortcut_informative,
            ] {
                assert_eq!(template.matches("{}").count(), 1, "{template}");
            }
        }
    }
}
