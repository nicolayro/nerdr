//! Lista over paner til venstre og den valgte panen til høyre.

use nerdr_kjerne::merke::{Gruppe, Merke};
use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::sgr::{Farge, Linje, Stil};
use std::ops::Range;
use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect, Size};
use ratatui::style::{Color, Modifier, Style};

use crate::bokser::{Boks, Rammer};
use crate::markering;
use crate::palett::{Rgb, bland};
use crate::grupper::{Rad, glyf, tittel};
use crate::ny::Velger;
use crate::tekst::{sted, varighet};
use crate::{App, Fokus, Lukke, Mål, hjelp};

/// Bredden på lista, med skillelinja.
pub const LISTE: u16 = 36;
/// Teksten i et kort starter etter peker, glyf og to mellomrom. Glyfene har
/// tvetydig bredde i Unicode, og mange fonter tegner dem over to kolonner.
const TEKST_X: u16 = 4;
const TEKST: usize = (LISTE - 1 - TEKST_X - 1) as usize;
const OPPGAVE_LINJER: usize = 2;
const VELGER_LINJER: u16 = 12;

const ORANSJE: Color = Color::Rgb(217, 119, 87);
const GRØNN: Color = Color::Indexed(108);
// Gråtonene i 256-paletten er faste og følger ikke fargetemaet, så teksten har
// standardfargen til terminalen og skiller seg bare med fet og svak.
const STERK: Style = Style::new().add_modifier(Modifier::BOLD);
const VANLIG: Style = Style::new();
const SVAK: Style = Style::new().add_modifier(Modifier::DIM);

/// Så mye av forgrunnen som blandes inn i bakgrunnen der noe er valgt, og der
/// musen står.
const MARKERT_ANDEL: f32 = 0.12;
const UNDER_MUSEN_ANDEL: f32 = 0.06;

static MARKERT: OnceLock<Option<Color>> = OnceLock::new();
static UNDER_MUSEN: OnceLock<Option<Color>> = OnceLock::new();

/// Valgt kort og fane får terminalens bakgrunn blandet med litt av forgrunnen,
/// så markeringen er like diskré i alle fargetemaer. Svarer ikke terminalen,
/// står de uten bakgrunn, med peker og fet tekst.
pub fn bruk_farger(farger: Option<(Rgb, Rgb)>) {
    let blandet = |andel| {
        farger.map(|(fg, bg)| {
            let (r, g, b) = bland(bg, fg, andel);
            Color::Rgb(r, g, b)
        })
    };
    let _ = MARKERT.set(blandet(MARKERT_ANDEL));
    let _ = UNDER_MUSEN.set(blandet(UNDER_MUSEN_ANDEL));
}

fn markert() -> Style {
    bakgrunn(&MARKERT)
}

fn under_musen() -> Style {
    bakgrunn(&UNDER_MUSEN)
}

fn bakgrunn(farge: &OnceLock<Option<Color>>) -> Style {
    farge.get().copied().flatten().map_or(Style::new(), |c| Style::new().bg(c))
}

/// Til høyre for lista, under faneraden.
pub fn visning(størrelse: Size) -> Rect {
    Rect::new(LISTE, 1, størrelse.width.saturating_sub(LISTE), størrelse.height.saturating_sub(1))
}

pub fn alt(f: &mut Frame, app: &App) {
    let område = f.area();
    let buf = f.buffer_mut();
    let bokser = app.bokser();
    let mus = app.under_musen();
    tegn_liste(buf, område, app, mus.as_ref());
    tegn_faner(buf, app, mus.as_ref());
    tegn_pane(buf, app, &bokser, mus.as_ref());

    if app.hjelp {
        tegn_hjelp(buf, område, &app.prefiks.navn());
    } else if let Some(velger) = &app.velger {
        let markør = tegn_velger(buf, område, velger);
        f.set_cursor_position(markør);
    } else if let (Fokus::Pane, Some((x, y)), 0, Some(boks)) = (app.fokus, app.markør, app.rull, aktiv_boks(app, &bokser)) {
        let i = boks.innhold;
        if x < i.width && y < i.height {
            f.set_cursor_position((i.x + x, i.y + y));
        }
    }
}

fn tegn_liste(buf: &mut Buffer, område: Rect, app: &App, mus: Option<&Mål>) {
    let bredde = LISTE - 1;
    let hint_y = område.height.saturating_sub(1);

    let mut overskrifter = Vec::new();
    let (rader, skjult) = plasser(app, hint_y);
    for Plassert { y, høyde, rad, kort } in rader {
        if y >= hint_y {
            break;
        }
        match rad {
            Rad::Tom => {}
            Rad::Strek => {
                let strek = "┈".repeat(bredde.saturating_sub(TEKST_X + 1) as usize);
                buf.set_stringn(TEKST_X, y, strek, TEKST, SVAK);
            }
            Rad::Overskrift(gruppe) => {
                let mut x = buf.set_stringn(1, y, tittel(gruppe), bredde as usize, SVAK).0 + 1;
                if gruppe == Gruppe::Ferdig && skjult > 0 {
                    x = buf.set_stringn(x, y, format!("+{skjult}"), bredde as usize, SVAK).0 + 1;
                }
                let strek = "─".repeat(bredde.saturating_sub(x) as usize);
                buf.set_stringn(x, y, strek, bredde as usize, SVAK);
                overskrifter.push(y);
            }
            Rad::Pane(pane, merke) => {
                let Some(kort) = kort else { continue };
                let område = Rect::new(0, y, bredde, høyde.min(hint_y - y));
                let valgt = app.valgt.as_ref().is_some_and(|v| v.id == pane.id);
                let pekt = matches!(mus, Some(Mål::Kort(id)) if *id == pane.id);
                tegn_kort(buf, område, &kort, merke, valgt, app.fokus, pekt);
            }
        }
    }

    // Hintet sier noe bare når du har trykket prefikset eller nerdr spør. Ellers
    // står bare veien til hjelpen der, dempet.
    let prefiks = app.prefiks.navn();
    let (hint, stil) = match app.fokus {
        _ if matches!(app.lukke, Some(Lukke::Pane(_))) => (" lukke panen? y/n".to_string(), VANLIG),
        _ if matches!(app.lukke, Some(Lukke::Vindu(_))) => (" lukke vinduet? y/n".to_string(), VANLIG),
        _ if app.etter_prefiks => (format!(" {prefiks} … w lista · c vindu · ? mer"), VANLIG),
        Fokus::Liste => (" ? hjelp".to_string(), SVAK),
        Fokus::Pane => (format!(" {prefiks} ? hjelp"), SVAK),
    };
    buf.set_stringn(0, hint_y, hint, bredde as usize, stil);

    // Overskriftene treffer skillestreken, og øverst går streken videre til
    // fanene. Står bare FERDIG i lista, er den nederst, og da går streken over
    // lista alene.
    if !overskrifter.contains(&0) {
        buf.set_string(0, 0, "─".repeat(bredde as usize), SVAK);
    }
    for y in 0..område.height {
        let tegn = match y {
            0 => "┬",
            _ if overskrifter.contains(&y) => "┤",
            _ => "│",
        };
        buf.set_stringn(bredde, y, tegn, 1, SVAK);
    }
}

/// En Claude i lista: oppgaven som tittel, med hvor lenge den har ventet eller
/// jobbet til høyre, og under den repoet og branchen. Uten oppgave er repoet
/// tittelen.
struct Kort {
    tittel: Vec<String>,
    tid: Option<String>,
    under: Vec<String>,
}

impl Kort {
    fn høyde(&self) -> u16 {
        (self.tittel.len() + self.under.len()) as u16
    }
}

fn kort(app: &App, pane: &Pane, merke: Merke) -> Kort {
    let ny = app.nye.get(&pane.id).map(|(mappe, _)| mappe.as_path());
    let mappe = app.claude.get(&pane.id).and_then(|c| c.mappe.as_deref()).or(ny);
    let repo = app.repoer.get(&pane.id);
    let sted = sted(pane, mappe, repo);
    let branch = repo.and_then(|r| r.branch.as_deref());
    let tid = match merke {
        Merke::Spør | Merke::Ulest | Merke::Jobber => {
            app.claude.get(&pane.id).map(|c| varighet(crate::nå_ms().saturating_sub(c.endret)))
        }
        Merke::Sett | Merke::Starter => None,
    };
    let første = TEKST.saturating_sub(tid.as_ref().map_or(0, |t| t.chars().count() + 2));
    let oppgave = pane.oppgave().map(|o| bryt(o, første, TEKST, OPPGAVE_LINJER)).filter(|l| !l.is_empty());
    let (tittel, under) = match (ny, oppgave) {
        (Some(_), _) => (vec![kutt(&sted, første)], vec!["starter …".to_string()]),
        (None, Some(oppgave)) => (oppgave, repo_og_branch(&sted, branch)),
        (None, None) => (vec![kutt(&sted, første)], branch.map(|b| vec![kutt(&format!("› {b}"), TEKST)]).unwrap_or_default()),
    };
    Kort { tittel, tid, under }
}

/// På én linje når begge får plass, ellers branchen på linja under, så ingen
/// av dem kuttes før de må.
fn repo_og_branch(sted: &str, branch: Option<&str>) -> Vec<String> {
    let Some(branch) = branch else { return vec![kutt(sted, TEKST)] };
    let sammen = format!("{sted} › {branch}");
    if sammen.chars().count() <= TEKST {
        return vec![sammen];
    }
    vec![kutt(sted, TEKST), kutt(&format!("› {branch}"), TEKST)]
}

fn tegn_kort(buf: &mut Buffer, område: Rect, kort: &Kort, merke: Merke, valgt: bool, fokus: Fokus, pekt: bool) {
    let dempet = !valgt && merke.gruppe() == Gruppe::Ferdig;
    if valgt && fokus == Fokus::Liste {
        buf.set_style(område, markert());
    } else if pekt {
        buf.set_style(område, under_musen());
    }
    let (peker, tittelstil) = match (valgt, dempet) {
        (true, _) => ("▎", STERK),
        (false, false) => (" ", VANLIG),
        (false, true) => (" ", SVAK),
    };
    for y in område.top()..område.bottom() {
        buf.set_stringn(0, y, peker, 1, VANLIG);
    }
    buf.set_stringn(1, område.y, glyf(merke), 1, Style::new().fg(glyffarge(merke)));
    if let Some(tid) = &kort.tid {
        høyrestilt(buf, område.y, TEKST_X + TEKST as u16, tid, SVAK);
    }
    let linjer = kort.tittel.iter().map(|l| (l, tittelstil)).chain(kort.under.iter().map(|l| (l, SVAK)));
    for ((linje, stil), y) in linjer.zip(område.top()..område.bottom()) {
        buf.set_stringn(TEKST_X, y, linje, TEKST, stil);
    }
}

/// Tekst som slutter foran kolonne `slutt`. Gir kolonnen den starter i.
fn høyrestilt(buf: &mut Buffer, y: u16, slutt: u16, tekst: &str, stil: Style) -> u16 {
    let x = slutt.saturating_sub(tekst.chars().count() as u16);
    buf.set_string(x, y, tekst, stil);
    x
}

fn glyffarge(merke: Merke) -> Color {
    match merke {
        Merke::Spør | Merke::Ulest => ORANSJE,
        Merke::Jobber | Merke::Starter => GRØNN,
        Merke::Sett => Color::Reset,
    }
}

/// En rad i lista, med kortet når raden er en pane.
struct Plassert<'a> {
    y: u16,
    høyde: u16,
    rad: Rad<'a>,
    kort: Option<Kort>,
}

/// Radene i lista, og hvor mange i FERDIG som ikke fikk plass. Får alt plass,
/// legges FERDIG nederst, med en tom rad over hintet, så det du har lagt bort
/// står for seg selv. Ellers vises så mange i FERDIG som får plass, og alltid
/// den som er valgt.
fn plasser<'a>(app: &'a App, plass: u16) -> (Vec<Plassert<'a>>, usize) {
    let høyde = |k: &Option<Kort>| k.as_ref().map_or(1, Kort::høyde);
    let mut rader: Vec<(Rad, Option<Kort>)> = app
        .rader()
        .into_iter()
        .map(|r| {
            let kort = if let Rad::Pane(pane, merke) = r { Some(kort(app, pane, merke)) } else { None };
            (r, kort)
        })
        .collect();
    let ferdig = rader.iter().position(|(r, _)| matches!(r, Rad::Overskrift(Gruppe::Ferdig))).unwrap_or(rader.len());
    let over: u16 = rader[..ferdig].iter().map(|(_, k)| høyde(k)).sum();
    let under: u16 = rader[ferdig..].iter().map(|(_, k)| høyde(k)).sum();

    let mut skjult = 0;
    let ferdig_y = if over + under < plass {
        plass - under - 1
    } else {
        let kort: Vec<(Rad, Option<Kort>)> =
            rader.split_off(ferdig).into_iter().filter(|(r, _)| matches!(r, Rad::Pane(..))).collect();
        if !kort.is_empty() {
            let valgt = app.valgt.as_ref().and_then(|v| kort.iter().position(|(r, _)| matches!(r, Rad::Pane(p, _) if p.id == v.id)));
            let høyder: Vec<u16> = kort.iter().map(|(_, k)| høyde(k)).collect();
            let vis = synlige(&høyder, valgt, plass.saturating_sub(over + 1));
            skjult = kort.len() - vis.len();
            rader.push((Rad::Overskrift(Gruppe::Ferdig), None));
            for (i, k) in kort.into_iter().enumerate().skip(vis.start).take(vis.len()) {
                if i > vis.start {
                    rader.push((Rad::Strek, None));
                }
                rader.push(k);
            }
        }
        over
    };

    let mut y = 0;
    let mut plassert = Vec::with_capacity(rader.len());
    for (i, (rad, kort)) in rader.into_iter().enumerate() {
        if i == ferdig {
            y = ferdig_y;
        }
        let høyde = høyde(&kort);
        plassert.push(Plassert { y, høyde, rad, kort });
        y += høyde;
    }
    (plassert, skjult)
}

/// Kortene med `høyder` som får plass på `rom` rader, med en strek mellom hvert.
/// Er det valgte langt nede, er det det nederste som vises. Det første får
/// alltid plass, selv om bare toppen av det vises.
fn synlige(høyder: &[u16], valgt: Option<usize>, rom: u16) -> Range<usize> {
    if rom == 0 || høyder.is_empty() {
        return 0..0;
    }
    let får_plass = |fra: usize, til: usize| høyder[fra..til].iter().sum::<u16>() + (til - fra) as u16 - 1 <= rom;
    let mut til = 1;
    while til < høyder.len() && får_plass(0, til + 1) {
        til += 1;
    }
    match valgt {
        Some(v) if v >= til => {
            let mut fra = v;
            while fra > 0 && får_plass(fra - 1, v + 1) {
                fra -= 1;
            }
            fra..v + 1
        }
        _ => 0..til,
    }
}

/// Panen som står på rad `y` i lista.
pub fn pane_på(app: &App, y: u16) -> Option<Pane> {
    let hint_y = app.visning.bottom().saturating_sub(1);
    if y >= hint_y {
        return None;
    }
    plasser(app, hint_y).0.into_iter().find_map(|p| match p.rad {
        Rad::Pane(pane, _) if (p.y..p.y + p.høyde).contains(&y) => Some(pane.clone()),
        _ => None,
    })
}

/// Et vindu i sesjonen, som fane i statuslinja.
struct Fane {
    fra: u16,
    til: u16,
    navn: String,
    vindu: String,
    vist: bool,
}

fn faner(app: &App) -> Vec<Fane> {
    let mut x = app.visning.x + 1;
    let mut ut = Vec::new();
    for p in app.faner() {
        let navn = kutt(&p.vindunavn, 20);
        let til = x + navn.chars().count() as u16 + 2;
        let vist = app.vist.as_ref() == Some(&p.vindu_id);
        ut.push(Fane { fra: x, til, navn, vindu: p.vindu_id.clone(), vist });
        x = til + 1;
    }
    ut
}

/// Vinduene i sesjonen på en strek i øverste rad, som fortsetter overskriften
/// i lista. Vinduet som vises, har bakgrunn.
fn tegn_faner(buf: &mut Buffer, app: &App, mus: Option<&Mål>) {
    let v = app.visning;
    buf.set_string(v.x, 0, "─".repeat(v.width as usize), SVAK);
    for f in faner(app).into_iter().take_while(|f| f.til <= v.right()) {
        let pekt = matches!(mus, Some(Mål::Fane(vindu)) if *vindu == f.vindu);
        let stil = match (f.vist, pekt) {
            (true, _) => markert().add_modifier(Modifier::BOLD),
            (false, true) => under_musen(),
            (false, false) => SVAK,
        };
        buf.set_string(f.fra, 0, format!(" {} ", f.navn), stil);
    }
}

/// Vinduet til fanen i kolonne `x`.
pub fn fane_på(app: &App, x: u16) -> Option<String> {
    faner(app).into_iter().find(|f| (f.fra..f.til).contains(&x)).map(|f| f.vindu)
}

/// En boks med ramme midt på skjermen, med tittel øverst og hint nederst,
/// eller `None` når skjermen er for liten.
fn boks(buf: &mut Buffer, område: Rect, bredde: u16, høyde: u16, tittel: &str, hint: &str) -> Option<Rect> {
    let bredde = område.width.saturating_sub(4).min(bredde);
    let høyde = område.height.saturating_sub(2).min(høyde);
    if bredde < 12 || høyde < 5 {
        return None;
    }
    let boks = Rect::new((område.width - bredde) / 2, (område.height - høyde) / 3, bredde, høyde);
    let ramme = Style::new().fg(ORANSJE);
    buf.set_style(boks, Style::reset());
    for y in boks.top()..boks.bottom() {
        let (v, fyll, h) = match y {
            y if y == boks.top() => ("╭", "─", "╮"),
            y if y == boks.bottom() - 1 => ("╰", "─", "╯"),
            _ => ("│", " ", "│"),
        };
        buf.set_string(boks.x, y, format!("{v}{}{h}", fyll.repeat(bredde as usize - 2)), ramme);
    }
    // Teksten inni har ingen forgrunn selv, og ville ellers arvet den fra rammen.
    buf.set_style(boks.inner(Margin::new(1, 1)), Style::reset());
    let tekst = (bredde - 4) as usize;
    buf.set_stringn(boks.x + 2, boks.y, format!(" {tittel} "), tekst, STERK.fg(Color::Reset));
    buf.set_stringn(boks.x + 2, boks.bottom() - 1, format!(" {hint} "), tekst, SVAK.fg(Color::Reset));
    Some(boks)
}

fn tegn_hjelp(buf: &mut Buffer, område: Rect, prefiks: &str) {
    const BREDDE: u16 = 58;
    let indre = (BREDDE - 4) as usize;
    let om = bryt(hjelp::OM, indre, indre, 3);
    let høyde = om.len() + 1 + hjelp::TASTER.len() + 1 + hjelp::PREFIKSTASTER.len() + 1 + hjelp::MERKER.len() + 1 + 2 + 2;
    let Some(boks) = boks(buf, område, BREDDE, høyde as u16, "nerdr", "trykk en tast for å lukke") else { return };
    let x = boks.x + 2;
    let indre = (boks.width - 4) as usize;
    let mut y = boks.y + 1..boks.bottom() - 1;
    let mut linje = |tekst: &str, stil: Style| {
        if let Some(y) = y.next() {
            buf.set_stringn(x, y, tekst, indre, stil);
        }
    };
    for l in om {
        linje(&l, VANLIG);
    }
    linje("", Style::new());
    for (tast, hva) in hjelp::TASTER {
        linje(&format!("{tast:<9}{hva}"), VANLIG);
    }
    linje("", Style::new());
    for (tast, hva) in hjelp::PREFIKSTASTER {
        let tast = if tast == "prefiks" { format!("{prefiks} {prefiks}") } else { format!("{prefiks} {tast}") };
        linje(&format!("{tast:<13}{hva}"), VANLIG);
    }
    linje("", Style::new());
    for (merke, hva) in hjelp::MERKER {
        linje(&format!("{}  {hva}", glyf(merke)), VANLIG);
    }
    linje("", Style::new());
    linje(hjelp::BARE_TMUX, SVAK);
    linje(&hjelp::mapper(), SVAK);
}

/// Mappevelgeren midt på skjermen. Gir der markøren skal stå i søkefeltet.
fn tegn_velger(buf: &mut Buffer, område: Rect, velger: &Velger) -> (u16, u16) {
    let (tittel, hint) = match velger.fortsett {
        false => ("ny claude", "⏎ start · C-r tidligere samtaler · esc avbryt"),
        true => ("tidligere samtaler", "⏎ fortsett · C-r ny · esc avbryt"),
    };
    let Some(boks) = boks(buf, område, 64, VELGER_LINJER + 4, tittel, hint) else {
        return (0, 0);
    };
    let bredde = boks.width;
    let høyde = boks.height;
    let indre = (bredde - 4) as usize;

    let treff = velger.treff();
    let x = boks.x + 2;
    let y = boks.y + 1;
    let prompt = if velger.fortsett { "samtale › " } else { "mappe › " };
    let søk = kutt(&velger.søk, indre.saturating_sub(prompt.chars().count() + 8));
    let etter = buf.set_stringn(x, y, prompt, indre, Style::new().fg(ORANSJE)).0;
    let markør = buf.set_stringn(etter, y, &søk, indre, VANLIG).0;
    let antall = format!("{}/{}", treff.len(), velger.antall());
    buf.set_string(boks.right() - 2 - antall.chars().count() as u16, y, antall, SVAK);

    let linjer = (høyde - 3) as usize;
    let fra = velger.valgt.saturating_sub(linjer - 1);
    for (i, t) in treff.iter().enumerate().skip(fra).take(linjer) {
        let y = boks.y + 2 + (i - fra) as u16;
        let valgt = i == velger.valgt;
        if valgt {
            buf.set_style(Rect::new(boks.x + 1, y, bredde - 2, 1), markert());
            buf.set_string(boks.x + 1, y, "▎", VANLIG);
        }
        let stil = if valgt { STERK } else { VANLIG };
        let k = t.kandidat;
        let mut plass = indre;
        if let Some(alder) = &k.alder {
            let bredde = alder.chars().count();
            buf.set_string(boks.right() - 2 - bredde as u16, y, alder, SVAK);
            plass = plass.saturating_sub(bredde + 2);
        }
        // Mappenavnet foran tittelen til en samtale er grått.
        let mappe = k.samtale.as_ref().and_then(|_| k.vis.find("  ")).map_or(0, |i| k.vis[..i].chars().count());
        for (n, c) in kutt(&k.vis, plass).chars().enumerate() {
            let stil = match () {
                _ if t.tegn.contains(&n) => Style::new().fg(ORANSJE),
                _ if n < mappe => SVAK,
                _ => stil,
            };
            buf.set_string(x + n as u16, y, c.encode_utf8(&mut [0; 4]), stil);
        }
    }
    (markør, y)
}

fn kutt(tekst: &str, bredde: usize) -> String {
    if tekst.chars().count() <= bredde {
        return tekst.to_string();
    }
    let mut kuttet: String = tekst.chars().take(bredde.saturating_sub(1)).collect();
    kuttet.push('…');
    kuttet
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

fn tegn_pane(buf: &mut Buffer, app: &App, bokser: &[Boks], mus: Option<&Mål>) {
    let v = app.visning;
    if v.width == 0 {
        return;
    }
    if app.valgt.is_none() {
        buf.set_stringn(v.x + 2, v.y + 1, "Ingen Claude-sesjoner i tmux. n starter en ny.", v.width as usize, SVAK);
        return;
    }
    // Den aktive har vanlig farge når du er inne i vinduet, så du ser hvor
    // tastene går. Den under musen får det samme, fordi et klikk gjør den aktiv.
    let aktiv = app.aktiv().map(|p| p.id.as_str()).filter(|_| app.fokus == Fokus::Pane);
    let lyse: Vec<&Boks> = bokser
        .iter()
        .filter(|b| aktiv == Some(&b.pane) || matches!(mus, Some(Mål::Pane(id)) if *id == b.pane))
        .collect();
    for boks in bokser {
        if let Some(bilde) = app.bilder.get(&boks.pane) {
            tegn_bilde(buf, boks.innhold, bilde);
        }
        if let (Some(markering), Some(bilde)) =
            (app.markering.as_ref().filter(|m| m.pane == boks.pane), app.bilder.get(&boks.pane))
        {
            let i = boks.innhold;
            let topp = app.historie as i64 - app.rull as i64;
            for (y, rad) in (0..i.height).zip(bilde) {
                let innhold = markering::innhold(rad);
                for x in markering.kolonner(topp + y as i64, i.width).into_iter().flatten().filter(|&x| x < innhold) {
                    // Snudd, så det som allerede er invertert, også synes markert.
                    buf[(i.x + x, i.y + y)].modifier.toggle(Modifier::REVERSED);
                }
            }
        }
        if let Some(ramme) = boks.ramme {
            let stil = if lyse.iter().any(|b| b.pane == boks.pane) { VANLIG } else { SVAK };
            tegn_ramme(buf, ramme, stil);
        }
    }
    if app.rammer == Rammer::Linjer && bokser.len() > 1 {
        tegn_linjer(buf, app, bokser, &lyse);
    }
    let merkelapp = match () {
        _ if app.kopiert.is_some() => Some(" kopiert ".to_string()),
        _ if app.rull > 0 => Some(format!(" {}/{} ", app.rull, app.historie)),
        _ => None,
    };
    if let (Some(hvor), Some(boks)) = (merkelapp, aktiv_boks(app, bokser)) {
        let bredde = hvor.chars().count() as u16;
        // På toppen av rammen, eller øverst i panen når den ikke har ramme.
        let (høyre, y) = match boks.ramme {
            Some(r) => (r.right().saturating_sub(2), r.y),
            None => (boks.innhold.right(), boks.innhold.y),
        };
        if bredde + 2 < boks.innhold.width {
            buf.set_string(høyre - bredde, y, hvor, Style::new().fg(ORANSJE).add_modifier(Modifier::REVERSED));
        }
    }
}

fn aktiv_boks<'a>(app: &App, bokser: &'a [Boks]) -> Option<&'a Boks> {
    let aktiv = app.aktiv()?;
    bokser.iter().find(|b| b.pane == aktiv.id)
}

fn tegn_bilde(buf: &mut Buffer, område: Rect, bilde: &[Linje]) {
    for (y, linje) in (område.top()..område.bottom()).zip(bilde) {
        let mut x = område.x;
        for bit in linje {
            if x >= område.right() {
                break;
            }
            x = buf.set_stringn(x, y, &bit.tekst, (område.right() - x) as usize, stil(bit.stil)).0;
        }
    }
}

/// Rutene i vinduet som ingen pane dekker, er strekene tmux legger mellom
/// panene. Tegnet i hver rute avhenger av hvilke naboer som også er strek, så
/// kryss og hjørner blir riktige.
fn tegn_linjer(buf: &mut Buffer, app: &App, bokser: &[Boks], lyse: &[&Boks]) {
    let Some(oppsett) = app.oppsett() else { return };
    let v = app.visning;
    let (bredde, høyde) = (oppsett.bredde.min(v.width), oppsett.høyde.min(v.height));
    let inni = |r: Rect, x: u16, y: u16| (r.left()..r.right()).contains(&x) && (r.top()..r.bottom()).contains(&y);
    let dekket = |x: u16, y: u16| bokser.iter().any(|b| inni(b.innhold, v.x + x, v.y + y));
    let strek = |x: i32, y: i32| x >= 0 && y >= 0 && x < bredde as i32 && y < høyde as i32 && !dekket(x as u16, y as u16);
    for y in 0..høyde {
        for x in 0..bredde {
            if dekket(x, y) {
                continue;
            }
            let (xi, yi) = (x as i32, y as i32);
            let tegn = rammetegn(strek(xi, yi - 1), strek(xi, yi + 1), strek(xi - 1, yi), strek(xi + 1, yi));
            let ved_lys = lyse.iter().map(|b| b.innhold).any(|a| {
                let (x, y) = (v.x + x, v.y + y);
                x + 1 >= a.x && x <= a.right() && y + 1 >= a.y && y <= a.bottom()
            });
            let stil = if ved_lys { VANLIG } else { SVAK };
            buf.set_string(v.x + x, v.y + y, tegn, stil);
        }
    }
}

fn rammetegn(opp: bool, ned: bool, venstre: bool, høyre: bool) -> &'static str {
    match (opp, ned, venstre, høyre) {
        (true, true, true, true) => "┼",
        (true, true, false, true) => "├",
        (true, true, true, false) => "┤",
        (false, true, true, true) => "┬",
        (true, false, true, true) => "┴",
        (false, true, false, true) => "┌",
        (false, true, true, false) => "┐",
        (true, false, false, true) => "└",
        (true, false, true, false) => "┘",
        (_, _, false, false) => "│",
        _ => "─",
    }
}

fn tegn_ramme(buf: &mut Buffer, r: Rect, stil: Style) {
    if r.width < 2 || r.height < 2 {
        return;
    }
    let strek = "─".repeat(r.width as usize - 2);
    buf.set_string(r.x, r.y, format!("╭{strek}╮"), stil);
    buf.set_string(r.x, r.bottom() - 1, format!("╰{strek}╯"), stil);
    for y in r.y + 1..r.bottom() - 1 {
        buf.set_string(r.x, y, "│", stil);
        buf.set_string(r.right() - 1, y, "│", stil);
    }
}

fn stil(s: Stil) -> Style {
    let mut stil = Style::new().fg(farge(s.fg)).bg(farge(s.bg));
    for (på, m) in [
        (s.fet, Modifier::BOLD),
        (s.svak, Modifier::DIM),
        (s.kursiv, Modifier::ITALIC),
        (s.understrek, Modifier::UNDERLINED),
        (s.blink, Modifier::SLOW_BLINK),
        (s.invers, Modifier::REVERSED),
        (s.skjult, Modifier::HIDDEN),
        (s.overstrek, Modifier::CROSSED_OUT),
    ] {
        if på {
            stil = stil.add_modifier(m);
        }
    }
    stil
}

fn farge(f: Farge) -> Color {
    match f {
        Farge::Standard => Color::Reset,
        Farge::Indeks(i) => Color::Indexed(i),
        Farge::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tester {
    use super::*;

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
            bryt("Forbindelser i Newcustomere2elocal for case", 31, 31, 2),
            ["Forbindelser i", "Newcustomere2elocal for case"]
        );
        assert_eq!(bryt("en to tre fire", 7, 7, 2), ["en to", "tre…"]);
        assert_eq!(bryt("Langtordsomikkefårplass", 10, 10, 2), ["Langtords…"]);
    }

    #[test]
    fn første_linje_kan_være_kortere() {
        assert_eq!(bryt("en to tre", 5, 9, 2), ["en to", "tre"]);
        assert_eq!(bryt("en to tre", 2, 9, 2), ["en", "to tre"]);
        assert_eq!(bryt("en to tre fire fem", 2, 9, 2), ["en", "to tre…"]);
    }

    #[test]
    fn repo_og_branch_på_én_linje_når_det_er_plass() {
        assert_eq!(repo_og_branch("nerdr", Some("main")), ["nerdr › main"]);
        assert_eq!(repo_og_branch("nerdr", None), ["nerdr"]);
        assert_eq!(
            repo_og_branch("api-bm-customer", Some("legg-til-audit-kode")),
            ["api-bm-customer", "› legg-til-audit-kode"]
        );
    }

    #[test]
    fn så_mange_kort_som_får_plass_og_alltid_det_valgte() {
        // Tre kort på tre rader med strek mellom er 11 rader.
        assert_eq!(synlige(&[3, 3, 3], None, 11), 0..3);
        assert_eq!(synlige(&[3, 3, 3], None, 10), 0..2);
        assert_eq!(synlige(&[3, 3, 3], Some(2), 7), 1..3);
        assert_eq!(synlige(&[3, 3, 3], Some(1), 7), 0..2);
        assert_eq!(synlige(&[3, 3, 3], None, 2), 0..1);
        assert_eq!(synlige(&[3], None, 0), 0..0);
    }
}
