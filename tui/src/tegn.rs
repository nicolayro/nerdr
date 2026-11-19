//! Lista over paner til venstre og den valgte panen til høyre.


use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};

use crate::bokser::Rammer;
use crate::popup::Popup;
use crate::{App, Fokus};

mod kort;
pub mod treff;
mod stil;
pub use stil::*;
mod liste;
pub use liste::*;
mod faner;
use faner::*;
mod knapper;
pub use knapper::*;
mod pane;
use pane::*;
mod popup;
pub use popup::*;
pub use treff::Mål;

/// Bredden på lista, med skillelinja, til du drar i den.
pub const LISTE: u16 = 36;
/// Smalere enn dette blir lista til en stripe med bare tegnene.
pub const LISTE_MIN: u16 = 24;
/// Stripa er peker, glyf, mellomrom og skillelinja.
pub const STRIPE: u16 = 4;

/// I et vindu er stripa i en ramme, med luft til venstre.
pub fn stripebredde(paner: Rammer) -> u16 {
    if paner == Rammer::Vindu { STRIPE + 3 } else { STRIPE }
}

/// Til høyre for lista, under fanene. Lista er `liste` bred, men aldri mer
/// enn halve skjermen. I et vindu er `liste` begge rammene med, og visningen
/// er inne i den rundt panen, under fanene og streken.
pub fn visning(størrelse: Size, liste: u16, paner: Rammer) -> Rect {
    let (b, h) = (størrelse.width, størrelse.height);
    let liste = liste.min(b / 2).max(stripebredde(paner).min(b));
    match paner {
        Rammer::Vindu => Rect::new(liste, FANERADER + 1, b.saturating_sub(liste + 1), h.saturating_sub(FANERADER + 2)),
        _ => Rect::new(liste, FANERADER, b.saturating_sub(liste), h.saturating_sub(FANERADER)),
    }
}

/// Der agentlista tegnes, med skillelinja lengst til høyre. I et vindu er
/// skillelinja høyrekanten på rammen rundt lista, og det er luft til venstre.
fn listeflate(app: &App) -> Rect {
    let (x, h) = (app.visning.x, app.skjerm.height);
    match app.utseende.paner {
        Rammer::Vindu => Rect::new(2, 1, x.saturating_sub(3), h.saturating_sub(2)),
        _ => Rect::new(0, 0, x, h),
    }
}

/// Kolonnen og raden i lista, når `x` og `y` er inni den.
fn i_lista(app: &App, x: u16, y: u16) -> Option<(u16, u16)> {
    let f = listeflate(app);
    (f.contains((x, y).into())).then(|| (x - f.x, y - f.y))
}

fn liste_bredde(app: &App) -> u16 {
    listeflate(app).width
}

fn stripe(app: &App) -> bool {
    liste_bredde(app) <= STRIPE
}

pub fn alt(f: &mut Frame, app: &App) {
    let område = f.area();
    let buf = f.buffer_mut();
    let bokser = app.bokser();
    let mus = app.under_musen();
    let flate = listeflate(app).intersection(område);
    // Uten plass til toppen og en rad kort tegnes ingenting.
    if flate.height > TOPPEN && flate.width > 1 {
        if flate.x == 0 && flate.y == 0 {
            tegn_liste(buf, flate, app, mus.as_ref());
        } else {
            // Treffene i lista regner fra øverst til venstre i den, så tegningen
            // gjør det samme og flyttes inn i rammen.
            let lokal = Rect::new(0, 0, flate.width, flate.height);
            let mut lerret = Buffer::empty(lokal);
            tegn_liste(&mut lerret, lokal, app, mus.as_ref());
            lerret.area = flate;
            buf.merge(&lerret);
        }
    }
    let vindu = app.utseende.paner == Rammer::Vindu;
    if vindu {
        tegn_vinduet(buf, app, mus.as_ref());
    }
    tegn_pane(buf, app, &bokser, mus.as_ref());
    tegn_faner(buf, app, mus.as_ref());
    if vindu {
        skjøt_strekene(buf, app, &bokser);
    }
    tegn_hint(buf, område, app);

    match &app.popup {
        Some(Popup::Hjelp(rull)) => tegn_hjelp(buf, område, &app.prefiks.navn(), &app.bindinger, *rull),
        Some(Popup::Lukke(lukke)) => tegn_lukke(buf, område, app, lukke, mus.as_ref()),
        Some(Popup::Utseende(akse)) => tegn_utseende(buf, område, &app.utseende, *akse, mus.as_ref()),
        Some(Popup::Handlinger(handlinger)) => {
            let markør = tegn_handlinger(buf, område, handlinger, mus.as_ref());
            f.set_cursor_position(markør);
        }
        Some(Popup::Velger(velger)) => {
            let markør = tegn_velger(buf, område, velger, mus.as_ref());
            f.set_cursor_position(markør);
        }
        None => {
            if let (Fokus::Pane, Some((x, y)), 0, Some(boks)) = (app.fokus, app.aktiv_pane.markør, app.aktiv_pane.rull, aktiv_boks(app, &bokser)) {
                let i = boks.innhold;
                if x < i.width && y < i.height {
                    f.set_cursor_position((i.x + x, i.y + y));
                }
            }
        }
    }
}

fn kutt(tekst: &str, bredde: usize) -> String {
    if tekst.chars().count() <= bredde {
        return tekst.to_string();
    }
    let mut kuttet: String = tekst.chars().take(bredde.saturating_sub(1)).collect();
    kuttet.push('…');
    kuttet
}

/// Som `kutt`, men fra starten, for stier der slutten sier mest.
fn kutt_foran(tekst: &str, bredde: usize) -> String {
    let antall = tekst.chars().count();
    if antall <= bredde {
        return tekst.to_string();
    }
    ['…'].into_iter().chain(tekst.chars().skip(antall + 1 - bredde.max(1))).collect()
}

/// Bryter på mellomrom over høyst `maks` linjer, der den første er `første`
/// bred, og ender med … når teksten ikke får plass.
fn bryt(tekst: &str, første: usize, bredde: usize, maks: usize) -> Vec<String> {
    let plass = |i: usize| if i == 0 { første } else { bredde };
    let mut linjer: Vec<String> = Vec::new();
    for ord in tekst.split_whitespace() {
        let i = linjer.len().saturating_sub(1);
        match linjer.last_mut() {
            Some(l) if l.chars().count() + 1 + ord.chars().count() <= plass(i) => {
                l.push(' ');
                l.push_str(ord);
            }
            _ => linjer.push(ord.to_string()),
        }
    }
    let for_lang = linjer.len() > maks;
    linjer.truncate(maks);
    let mut linjer: Vec<String> = linjer.iter().enumerate().map(|(i, l)| kutt(l, plass(i))).collect();
    let siste = linjer.len().saturating_sub(1);
    if for_lang && let Some(l) = linjer.last_mut() {
        *l = l.chars().take(plass(siste).saturating_sub(1)).chain(['…']).collect();
    }
    linjer
}

#[cfg(test)]
mod tester {
    use super::*;
    use crate::bindinger::Bindinger;

    #[test]
    fn rammetegn_etter_naboene() {
        assert_eq!(rammetegn(true, true, false, false), "│");
        assert_eq!(rammetegn(false, false, true, true), "─");
        assert_eq!(rammetegn(true, true, false, true), "├");
    }

    #[test]
    fn bryter_på_ord_og_kutter_det_som_ikke_får_plass() {
        assert_eq!(bryt("Fiks testene", 31, 31, 2), ["Fiks testene"]);
        assert_eq!(
            bryt("Lenkesjekken i dokumentasjonssiden gir feil", 31, 31, 2),
            ["Lenkesjekken i", "dokumentasjonssiden gir feil"]
        );
        assert_eq!(bryt("en to tre fire", 7, 7, 2), ["en to", "tre…"]);
        assert_eq!(bryt("Langtordsomikkefårplass", 10, 10, 2), ["Langtords…"]);
    }

    #[test]
    fn kutter_stier_foran() {
        assert_eq!(kutt_foran("~/git/nerdr", 20), "~/git/nerdr");
        assert_eq!(kutt_foran("~/git/nerdr", 8), "…t/nerdr");
    }

    #[test]
    fn første_linje_kan_være_kortere() {
        assert_eq!(bryt("en to tre", 5, 9, 2), ["en to", "tre"]);
        assert_eq!(bryt("en to tre", 2, 9, 2), ["en", "to tre"]);
        assert_eq!(bryt("en to tre fire fem", 2, 9, 2), ["en", "to tre…"]);
    }

    #[test]
    fn lista_får_aldri_mer_enn_halve_skjermen() {
        assert_eq!(visning(Size::new(120, 40), 50, Rammer::Linjer).x, 50);
        assert_eq!(visning(Size::new(80, 40), 50, Rammer::Linjer).x, 40);
        assert_eq!(visning(Size::new(80, 40), STRIPE, Rammer::Linjer).x, STRIPE);
        assert_eq!(visning(Size::new(6, 40), LISTE, Rammer::Linjer).x, 4);
    }

    fn tegnet_hjelp(bredde: u16, høyde: u16, rull: u16) -> Vec<String> {
        let område = Rect::new(0, 0, bredde, høyde);
        let mut buf = Buffer::empty(område);
        tegn_hjelp(&mut buf, område, "C-b", &Bindinger::default(), rull);
        (0..høyde).map(|y| (0..bredde).map(|x| buf[(x, y)].symbol()).collect()).collect()
    }

    #[test]
    fn hjelpen_får_plass_i_80x24() {
        let linjer = hjelp_linjer("C-b", &Bindinger::default());
        assert_eq!(hjelp_boks(Rect::new(0, 0, 80, 24), linjer.len()).unwrap().1, 0);
        let rader = tegnet_hjelp(80, 24, 0);
        for (tekst, _) in linjer.iter().filter(|(t, _)| !t.is_empty()) {
            assert!(rader.iter().any(|rad| rad.contains(tekst.as_str())), "{tekst}");
        }
    }

    #[test]
    fn pilene_i_rammen_viser_hva_som_er_over_og_under() {
        let maks = hjelp_boks(Rect::new(0, 0, 80, 16), hjelp_linjer("C-b", &Bindinger::default()).len()).unwrap().1;
        let pil = |rader: &[String], pil: &str| rader.iter().any(|r| r.contains(pil));
        let toppen = tegnet_hjelp(80, 16, 0);
        assert!(!pil(&toppen, " ↑ ") && pil(&toppen, " ↓ ") && pil(&toppen, "↑↓ rull · trykk en tast for å lukke"));
        let midten = tegnet_hjelp(80, 16, 1);
        assert!(pil(&midten, " ↑ ") && pil(&midten, " ↓ "));
        // Lenger enn bunnen går den ikke.
        let bunnen = tegnet_hjelp(80, 16, maks + 10);
        assert!(pil(&bunnen, " ↑ ") && !pil(&bunnen, " ↓ "));
        assert_eq!(bunnen, tegnet_hjelp(80, 16, maks));
    }

    #[test]
    fn hjelpen_viser_tastene_fra_tmux_oppsettet() {
        let linjer: Vec<String> = ["| split-window -h", "- split-window -v", "h select-pane -L", "j select-pane -D", "k select-pane -U", "l select-pane -R"]
            .iter()
            .map(|b| format!("bind-key -T prefix {b}"))
            .collect();
        let bindinger = Bindinger::ny(&nerdr_kjerne::bindinger::les(&linjer));
        let tekst: Vec<String> = hjelp_linjer("C-a", &bindinger).into_iter().map(|(t, _)| t).collect();
        assert!(tekst.iter().any(|t| t.starts_with("C-a | -  ") && t.ends_with("del side om side, over og under")));
        assert!(tekst.iter().any(|t| t.starts_with("C-a h k j l ")));
        assert!(hjelp_linjer("C-b", &Bindinger::default()).iter().any(|(t, _)| t.starts_with("C-b ←↑↓→ ")));
    }

    #[test]
    fn hjelpen_som_får_plass_ruller_ikke() {
        let (_, maks) = hjelp_boks(Rect::new(0, 0, 120, 60), hjelp_linjer("C-b", &Bindinger::default()).len()).unwrap();
        assert_eq!(maks, 0);
        let rader = tegnet_hjelp(120, 60, 0);
        assert!(rader.iter().any(|r| r.contains("trykk en tast")));
        assert!(!rader.iter().any(|r| r.contains(" ↑ ") || r.contains(" ↓ ")));
    }

    #[test]
    fn utsnittet_ruller_bare_når_lista_ikke_får_plass() {
        assert_eq!(Utsnitt::ny(10, 12, 5), Utsnitt { rull: 0, rom: 12, ruller: false, topp: 0 });
        // To rader går til pilene.
        assert_eq!(Utsnitt::ny(20, 12, 50), Utsnitt { rull: 10, rom: 10, ruller: true, topp: 0 });
        let u = Utsnitt::ny(20, 12, 4);
        assert_eq!((u.skjerm(4), u.i_lista(1), u.i_lista(0), u.i_lista(11)), (1, Some(4), None, None));
    }

    #[test]
    fn menyen_ruller_først_når_den_valgte_når_kanten() {
        assert_eq!(følg(5, 0, 10, 30), 0);
        assert_eq!(følg(10, 0, 10, 30), 1);
        assert_eq!(følg(4, 6, 10, 30), 4);
        // Færre treff etter et søk: ingen tomme rader nederst.
        assert_eq!(følg(2, 20, 10, 12), 2);
        assert_eq!(følg(11, 20, 10, 12), 2);
    }

    #[test]
    fn utsnittet_følger_den_valgte() {
        let u = Utsnitt::ny(30, 12, 0);
        assert_eq!(u.følg(2..5).rull, 0);
        assert_eq!(u.følg(14..17).rull, 7);
        assert_eq!(Utsnitt { rull: 20, ..u }.følg(14..17).rull, 14);
        // Høyere enn utsnittet: toppen vises.
        assert_eq!(u.følg(3..20).rull, 3);
    }
}
