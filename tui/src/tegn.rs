//! Lista over paner til venstre og den valgte panen til høyre.

use nerdr_kjerne::merke::{Gruppe, Merke};
use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::sgr::{Farge, Linje, Stil};
use std::ops::Range;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier, Style};

use crate::bokser::{Boks, Rammer};
use crate::grupper::{Rad, glyf, tittel};
use crate::ny::Velger;
use crate::{App, Fokus, Lukke, hjelp};

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
const HVIT: Color = Color::Indexed(255);
const LYS: Color = Color::Indexed(252);
const GRÅ: Color = Color::Indexed(245);
const DIM: Color = Color::Indexed(240);
const MARKERT: Color = Color::Indexed(236);

/// Til høyre for lista, under faneraden. Med bokser står en tom rad mellom, så
/// streken fanene står på, ikke ligger rett over toppen av boksene.
pub fn visning(størrelse: Size, rammer: Rammer) -> Rect {
    let topp = if rammer == Rammer::Bokser { 2 } else { 1 };
    Rect::new(LISTE, topp, størrelse.width.saturating_sub(LISTE), størrelse.height.saturating_sub(topp))
}

pub fn alt(f: &mut Frame, app: &App) {
    let område = f.area();
    let buf = f.buffer_mut();
    tegn_liste(buf, område, app);
    tegn_faner(buf, app);
    tegn_pane(buf, app);

    if app.hjelp {
        tegn_hjelp(buf, område, &app.prefiks.navn());
    } else if let Some(velger) = &app.velger {
        let markør = tegn_velger(buf, område, velger);
        f.set_cursor_position(markør);
    } else if let (Fokus::Pane, Some((x, y)), 0, Some(boks)) = (app.fokus, app.markør, app.rull, aktiv_boks(app)) {
        let i = boks.innhold;
        if x < i.width && y < i.height {
            f.set_cursor_position((i.x + x, i.y + y));
        }
    }
}

fn tegn_liste(buf: &mut Buffer, område: Rect, app: &App) {
    let bredde = LISTE - 1;
    let hint_y = område.height.saturating_sub(1);

    let mut overskrifter = Vec::new();
    let (rader, skjult) = plasser(app, hint_y);
    for (y, høyde, rad) in rader {
        if y >= hint_y {
            break;
        }
        match rad {
            Rad::Tom => {}
            Rad::Strek => {
                let strek = "┈".repeat(bredde.saturating_sub(TEKST_X + 1) as usize);
                buf.set_stringn(TEKST_X, y, strek, TEKST, Style::new().fg(DIM));
            }
            Rad::Overskrift(gruppe) => {
                let mut x = buf.set_stringn(1, y, tittel(gruppe), bredde as usize, Style::new().fg(GRÅ)).0 + 1;
                if gruppe == Gruppe::Ferdig && skjult > 0 {
                    x = buf.set_stringn(x, y, format!("+{skjult}"), bredde as usize, Style::new().fg(DIM)).0 + 1;
                }
                let strek = "─".repeat(bredde.saturating_sub(x) as usize);
                buf.set_stringn(x, y, strek, bredde as usize, Style::new().fg(DIM));
                overskrifter.push(y);
            }
            Rad::Pane(pane, merke) => {
                let kort = kort(app, pane, merke);
                let område = Rect::new(0, y, bredde, høyde.min(hint_y - y));
                let valgt = app.valgt.as_ref().is_some_and(|v| v.id == pane.id);
                tegn_kort(buf, område, &kort, merke, valgt, app.fokus);
            }
        }
    }

    // Hintet sier noe bare når du har trykket prefikset eller nerdr spør. Ellers
    // står bare veien til hjelpen der, dempet.
    let prefiks = app.prefiks.navn();
    let (hint, farge) = match app.fokus {
        _ if matches!(app.lukke, Some(Lukke::Pane(_))) => (" lukke panen? y/n".to_string(), LYS),
        _ if matches!(app.lukke, Some(Lukke::Vindu(_))) => (" lukke vinduet? y/n".to_string(), LYS),
        _ if app.etter_prefiks => (format!(" {prefiks} … w lista · c vindu · ? mer"), LYS),
        Fokus::Liste => (" ? hjelp".to_string(), DIM),
        Fokus::Pane => (format!(" {prefiks} ? hjelp"), DIM),
    };
    buf.set_stringn(0, hint_y, hint, bredde as usize, Style::new().fg(farge));

    // Overskriftene treffer skillestreken, og øverst går streken videre til
    // fanene. Står bare FERDIG i lista, er den nederst, og da går streken over
    // lista alene.
    if !overskrifter.contains(&0) {
        buf.set_string(0, 0, "─".repeat(bredde as usize), Style::new().fg(DIM));
    }
    for y in 0..område.height {
        let tegn = match y {
            0 => "┬",
            _ if overskrifter.contains(&y) => "┤",
            _ => "│",
        };
        buf.set_stringn(bredde, y, tegn, 1, Style::new().fg(DIM));
    }
}

/// En sesjon i lista: mappa, branchen helt til høyre, og oppgaven under, med
/// hvor lenge den har ventet eller jobbet helt til høyre på siste linje. Uten
/// oppgave står tiden til høyre for branchen.
struct Kort {
    mappe: String,
    branch: Option<String>,
    oppgave: Vec<String>,
    tid: Option<String>,
}

impl Kort {
    fn høyde(&self) -> u16 {
        1 + self.oppgave.len() as u16
    }
}

fn kort(app: &App, pane: &Pane, merke: Merke) -> Kort {
    let ny = app.nye.get(&pane.id);
    let mappe = app
        .claude
        .get(&pane.id)
        .and_then(|s| s.mappe.as_deref())
        .or(ny.map(|m| m.as_path()))
        .and_then(|m| m.file_name()?.to_str())
        .map_or_else(|| format!("{}:{}", pane.sesjon, pane.vindu), str::to_string);
    let tid = match merke {
        Merke::Spør | Merke::Ulest | Merke::Jobber => {
            app.claude.get(&pane.id).map(|s| varighet(crate::nå_ms().saturating_sub(s.endret)))
        }
        Merke::Sett | Merke::Starter => None,
    };
    let oppgave = match ny {
        Some(_) => vec!["starter …".to_string()],
        None => pane.oppgave().map(|o| bryt_med(o, tid.as_deref(), TEKST, OPPGAVE_LINJER)).unwrap_or_default(),
    };
    Kort { mappe, branch: app.brancher.get(&pane.id).cloned(), oppgave, tid }
}

/// Kort nok til å stå ytterst på en linje i kortet.
fn varighet(ms: u64) -> String {
    match ms / 60_000 {
        0 => "<1m".into(),
        m @ 1..60 => format!("{m}m"),
        m @ 60..1440 => format!("{}t", m / 60),
        m => format!("{}d", m / 1440),
    }
}

fn tegn_kort(buf: &mut Buffer, område: Rect, kort: &Kort, merke: Merke, valgt: bool, fokus: Fokus) {
    if valgt && fokus == Fokus::Liste {
        buf.set_style(område, Style::new().bg(MARKERT));
    }
    let dempet = !valgt && merke.gruppe() == Gruppe::Ferdig;
    let (peker, mappestil, oppgavefarge, gråfarge) = match (valgt, dempet) {
        (true, _) => ("▎", Style::new().fg(HVIT).add_modifier(Modifier::BOLD), LYS, GRÅ),
        (false, false) => (" ", Style::new().fg(LYS).add_modifier(Modifier::BOLD), GRÅ, GRÅ),
        (false, true) => (" ", Style::new().fg(GRÅ), DIM, DIM),
    };
    for y in område.top()..område.bottom() {
        buf.set_stringn(0, y, peker, 1, Style::new().fg(HVIT));
    }
    let y = område.y;
    buf.set_stringn(1, y, glyf(merke), 1, Style::new().fg(glyffarge(merke)));

    let mut slutt = TEKST_X + TEKST as u16;
    if let Some(tid) = kort.tid.as_ref().filter(|_| kort.oppgave.is_empty()) {
        slutt = høyrestilt(buf, y, slutt, tid, gråfarge).saturating_sub(2);
    }
    let bredde = slutt.saturating_sub(TEKST_X) as usize;
    let mappe = kutt(&kort.mappe, bredde);
    buf.set_stringn(TEKST_X, y, &mappe, bredde, mappestil);
    // Med mindre enn to tegn igjen blir branchen bare en …
    let plass = bredde.saturating_sub(mappe.chars().count() + 2);
    if let Some(branch) = kort.branch.as_ref().filter(|_| plass >= 2) {
        høyrestilt(buf, y, slutt, &kutt(branch, plass), gråfarge);
    }

    for (linje, y) in kort.oppgave.iter().zip(y + 1..område.bottom()) {
        buf.set_stringn(TEKST_X, y, linje, TEKST, Style::new().fg(oppgavefarge));
    }
    let siste = y + kort.høyde() - 1;
    if let Some(tid) = kort.tid.as_ref().filter(|_| siste > y && siste < område.bottom()) {
        høyrestilt(buf, siste, TEKST_X + TEKST as u16, tid, gråfarge);
    }
}

/// Tekst som slutter foran kolonne `slutt`. Gir kolonnen den starter i.
fn høyrestilt(buf: &mut Buffer, y: u16, slutt: u16, tekst: &str, farge: Color) -> u16 {
    let x = slutt.saturating_sub(tekst.chars().count() as u16);
    buf.set_string(x, y, tekst, Style::new().fg(farge));
    x
}

fn glyffarge(merke: Merke) -> Color {
    match merke {
        Merke::Spør | Merke::Ulest => ORANSJE,
        Merke::Jobber | Merke::Starter => GRØNN,
        Merke::Sett => LYS,
    }
}

/// Radene i lista med y og høyde, og hvor mange i FERDIG som ikke fikk plass.
/// Får alt plass, legges FERDIG nederst, med en tom rad over hintet, så det du
/// har lagt bort står for seg selv. Ellers vises så mange i FERDIG som får
/// plass, og alltid den som er valgt.
fn plasser<'a>(app: &'a App, plass: u16) -> (Vec<(u16, u16, Rad<'a>)>, usize) {
    let mut rader: Vec<(Rad, u16)> = app
        .rader()
        .into_iter()
        .map(|r| {
            let høyde = if let Rad::Pane(pane, merke) = r { kort(app, pane, merke).høyde() } else { 1 };
            (r, høyde)
        })
        .collect();
    let ferdig = rader.iter().position(|(r, _)| matches!(r, Rad::Overskrift(Gruppe::Ferdig))).unwrap_or(rader.len());
    let over: u16 = rader[..ferdig].iter().map(|(_, h)| h).sum();
    let under: u16 = rader[ferdig..].iter().map(|(_, h)| h).sum();

    let mut skjult = 0;
    let ferdig_y = if over + under < plass {
        plass - under - 1
    } else {
        let kort: Vec<(Rad, u16)> = rader.split_off(ferdig).into_iter().filter(|(r, _)| matches!(r, Rad::Pane(..))).collect();
        if !kort.is_empty() {
            let valgt = app.valgt.as_ref().and_then(|v| kort.iter().position(|(r, _)| matches!(r, Rad::Pane(p, _) if p.id == v.id)));
            let høyder: Vec<u16> = kort.iter().map(|(_, h)| *h).collect();
            let vis = synlige(&høyder, valgt, plass.saturating_sub(over + 1));
            skjult = kort.len() - vis.len();
            rader.push((Rad::Overskrift(Gruppe::Ferdig), 1));
            for (i, k) in kort.into_iter().enumerate().skip(vis.start).take(vis.len()) {
                if i > vis.start {
                    rader.push((Rad::Strek, 1));
                }
                rader.push(k);
            }
        }
        over
    };

    let mut y = 0;
    let mut plassert = Vec::with_capacity(rader.len());
    for (i, (rad, høyde)) in rader.into_iter().enumerate() {
        if i == ferdig {
            y = ferdig_y;
        }
        plassert.push((y, høyde, rad));
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
    plasser(app, hint_y).0.into_iter().find_map(|(topp, høyde, rad)| match rad {
        Rad::Pane(pane, _) if (topp..topp + høyde).contains(&y) => Some(pane.clone()),
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
fn tegn_faner(buf: &mut Buffer, app: &App) {
    let v = app.visning;
    buf.set_string(v.x, 0, "─".repeat(v.width as usize), Style::new().fg(DIM));
    for f in faner(app).into_iter().take_while(|f| f.til <= v.right()) {
        let stil = match f.vist {
            true => Style::new().fg(HVIT).bg(MARKERT).add_modifier(Modifier::BOLD),
            false => Style::new().fg(GRÅ),
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
    let tekst = (bredde - 4) as usize;
    buf.set_stringn(boks.x + 2, boks.y, format!(" {tittel} "), tekst, Style::new().fg(HVIT).add_modifier(Modifier::BOLD));
    buf.set_stringn(boks.x + 2, boks.bottom() - 1, format!(" {hint} "), tekst, Style::new().fg(GRÅ));
    Some(boks)
}

fn tegn_hjelp(buf: &mut Buffer, område: Rect, prefiks: &str) {
    const BREDDE: u16 = 58;
    let indre = (BREDDE - 4) as usize;
    let om = bryt(hjelp::OM, indre, 3);
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
        linje(&l, Style::new().fg(LYS));
    }
    linje("", Style::new());
    for (tast, hva) in hjelp::TASTER {
        linje(&format!("{tast:<9}{hva}"), Style::new().fg(LYS));
    }
    linje("", Style::new());
    for (tast, hva) in hjelp::PREFIKSTASTER {
        let tast = if tast == "prefiks" { format!("{prefiks} {prefiks}") } else { format!("{prefiks} {tast}") };
        linje(&format!("{tast:<13}{hva}"), Style::new().fg(LYS));
    }
    linje("", Style::new());
    for (merke, hva) in hjelp::MERKER {
        linje(&format!("{}  {hva}", glyf(merke)), Style::new().fg(LYS));
    }
    linje("", Style::new());
    linje(hjelp::BARE_TMUX, Style::new().fg(GRÅ));
    linje(&hjelp::mapper(), Style::new().fg(GRÅ));
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
    let markør = buf.set_stringn(etter, y, &søk, indre, Style::new().fg(HVIT)).0;
    let antall = format!("{}/{}", treff.len(), velger.antall());
    buf.set_string(boks.right() - 2 - antall.chars().count() as u16, y, antall, Style::new().fg(DIM));

    let linjer = (høyde - 3) as usize;
    let fra = velger.valgt.saturating_sub(linjer - 1);
    for (i, t) in treff.iter().enumerate().skip(fra).take(linjer) {
        let y = boks.y + 2 + (i - fra) as u16;
        let valgt = i == velger.valgt;
        if valgt {
            buf.set_style(Rect::new(boks.x + 1, y, bredde - 2, 1), Style::new().bg(MARKERT));
            buf.set_string(boks.x + 1, y, "▎", Style::new().fg(HVIT));
        }
        let farge = if valgt { HVIT } else { LYS };
        let k = t.kandidat;
        let mut plass = indre;
        if let Some(alder) = &k.alder {
            let bredde = alder.chars().count();
            buf.set_string(boks.right() - 2 - bredde as u16, y, alder, Style::new().fg(DIM));
            plass = plass.saturating_sub(bredde + 2);
        }
        // Mappenavnet foran tittelen til en samtale er grått.
        let mappe = k.samtale.as_ref().and_then(|_| k.vis.find("  ")).map_or(0, |i| k.vis[..i].chars().count());
        for (n, c) in kutt(&k.vis, plass).chars().enumerate() {
            let farge = match () {
                _ if t.tegn.contains(&n) => ORANSJE,
                _ if n < mappe => GRÅ,
                _ => farge,
            };
            buf.set_string(x + n as u16, y, c.encode_utf8(&mut [0; 4]), Style::new().fg(farge));
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

/// Som `bryt`, men med plass til `tid` ytterst på siste linje. Fyller teksten
/// alle linjene, kuttes den siste.
fn bryt_med(tekst: &str, tid: Option<&str>, bredde: usize, maks: usize) -> Vec<String> {
    let Some(tid) = tid else { return bryt(tekst, bredde, maks) };
    let mut linjer = bryt(&format!("{tekst} {tid}"), bredde, maks);
    if let Some(siste) = linjer.last_mut() {
        if siste == tid {
            siste.clear();
            return linjer;
        }
        if let Some(uten) = siste.strip_suffix(&format!(" {tid}")) {
            *siste = uten.to_string();
            return linjer;
        }
    }
    let mut linjer = bryt(tekst, bredde, maks);
    if let Some(siste) = linjer.last_mut() {
        *siste = kutt(siste, bredde.saturating_sub(tid.chars().count() + 1));
    }
    linjer
}

/// Bryter på mellomrom over høyst `maks` linjer, og ender med … når teksten
/// ikke får plass.
fn bryt(tekst: &str, bredde: usize, maks: usize) -> Vec<String> {
    let mut linjer: Vec<String> = Vec::new();
    for ord in tekst.split_whitespace() {
        match linjer.last_mut() {
            Some(l) if l.chars().count() + 1 + ord.chars().count() <= bredde => {
                l.push(' ');
                l.push_str(ord);
            }
            _ => linjer.push(ord.to_string()),
        }
    }
    let for_lang = linjer.len() > maks;
    linjer.truncate(maks);
    let mut linjer: Vec<String> = linjer.iter().map(|l| kutt(l, bredde)).collect();
    if for_lang && let Some(siste) = linjer.last_mut() {
        *siste = siste.chars().take(bredde.saturating_sub(1)).chain(['…']).collect();
    }
    linjer
}

fn tegn_pane(buf: &mut Buffer, app: &App) {
    let v = app.visning;
    if v.width == 0 {
        return;
    }
    if app.valgt.is_none() {
        buf.set_stringn(v.x + 2, v.y + 1, "Ingen Claude-sesjoner i tmux. n starter en ny.", v.width as usize, Style::new().fg(GRÅ));
        return;
    }
    let aktiv = app.aktiv().map(|p| p.id.as_str());
    let bokser = app.bokser();
    for boks in &bokser {
        if let Some(bilde) = app.bilder.get(&boks.pane) {
            tegn_bilde(buf, boks.innhold, bilde);
        }
        if let Some(ramme) = boks.ramme {
            // Den aktive har vanlig farge når du er inne i vinduet, så du ser
            // hvor tastene går.
            let farge = if app.fokus == Fokus::Pane && aktiv == Some(&boks.pane) { Color::Reset } else { DIM };
            tegn_ramme(buf, ramme, Style::new().fg(farge));
        }
    }
    if app.rammer == Rammer::Linjer && bokser.len() > 1 {
        tegn_linjer(buf, app, &bokser);
    }
    if let (true, Some(boks)) = (app.rull > 0, aktiv_boks(app)) {
        let hvor = format!(" {}/{} ", app.rull, app.historie);
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

fn aktiv_boks(app: &App) -> Option<Boks> {
    let aktiv = app.aktiv()?;
    app.bokser().into_iter().find(|b| b.pane == aktiv.id)
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
fn tegn_linjer(buf: &mut Buffer, app: &App, bokser: &[Boks]) {
    let Some(oppsett) = app.oppsett() else { return };
    let v = app.visning;
    let (bredde, høyde) = (oppsett.bredde.min(v.width), oppsett.høyde.min(v.height));
    let inni = |r: Rect, x: u16, y: u16| (r.left()..r.right()).contains(&x) && (r.top()..r.bottom()).contains(&y);
    let dekket = |x: u16, y: u16| bokser.iter().any(|b| inni(b.innhold, v.x + x, v.y + y));
    let strek = |x: i32, y: i32| x >= 0 && y >= 0 && x < bredde as i32 && y < høyde as i32 && !dekket(x as u16, y as u16);
    let aktiv = aktiv_boks(app).filter(|_| app.fokus == Fokus::Pane).map(|b| b.innhold);
    for y in 0..høyde {
        for x in 0..bredde {
            if dekket(x, y) {
                continue;
            }
            let (xi, yi) = (x as i32, y as i32);
            let tegn = rammetegn(strek(xi, yi - 1), strek(xi, yi + 1), strek(xi - 1, yi), strek(xi + 1, yi));
            // Strekene rundt den aktive har vanlig farge, så du ser hvor tastene går.
            let ved_aktiv = aktiv.is_some_and(|a| {
                let (x, y) = (v.x + x, v.y + y);
                x + 1 >= a.x && x <= a.right() && y + 1 >= a.y && y <= a.bottom()
            });
            let farge = if ved_aktiv { Color::Reset } else { DIM };
            buf.set_string(v.x + x, v.y + y, tegn, Style::new().fg(farge));
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
        assert_eq!(bryt("Fiks testene", 31, 2), ["Fiks testene"]);
        assert_eq!(
            bryt("Forbindelser i Newcustomere2elocal for case", 31, 2),
            ["Forbindelser i", "Newcustomere2elocal for case"]
        );
        assert_eq!(bryt("en to tre fire", 7, 2), ["en to", "tre…"]);
        assert_eq!(bryt("Langtordsomikkefårplass", 10, 2), ["Langtords…"]);
    }

    #[test]
    fn tiden_får_plass_ytterst_på_siste_linje() {
        assert_eq!(bryt_med("Fiks testene", Some("3m"), 16, 2), ["Fiks testene"]);
        assert_eq!(bryt_med("Fiks alle testene", Some("14m"), 12, 1), ["Fiks al…"]);
        assert_eq!(bryt_med("Fiks testene", Some("3m"), 12, 2), ["Fiks testene", ""]);
        assert_eq!(bryt_med("Fiks", None, 16, 2), ["Fiks"]);
    }

    #[test]
    fn varighet_i_minutter_timer_og_døgn() {
        assert_eq!(varighet(59_000), "<1m");
        assert_eq!(varighet(14 * 60_000), "14m");
        assert_eq!(varighet(3 * 3_600_000 + 60_000), "3t");
        assert_eq!(varighet(2 * 86_400_000), "2d");
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
