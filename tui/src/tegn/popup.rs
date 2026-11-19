//! De fem popupene og søkemenyene.

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use std::ops::Range;
use crate::{App, hjelp};
use crate::bindinger::Bindinger;
use crate::grupper::glyf;
use crate::handlinger::Handlinger;
use crate::ny::Velger;
use crate::popup::Popup;
use crate::utseende::{self, Akse, Utseende};
use super::*;

/// Treffene i handlingene og velgeren får to tredjedeler av skjermen, men
/// aldri færre rader enn dette.
pub(super) const MENY_RADER: u16 = 13;

pub(super) const HANDLINGER_BREDDE: u16 = 56;

pub(super) const VELGER_BREDDE: u16 = 100;

pub(super) const UTSEENDE_BREDDE: u16 = 58;

/// Forklaringen nederst i innstillingene har alltid så mange linjer, så
/// menyen ikke endrer høyde når du går opp og ned.
pub(super) const FORKLARING_LINJER: usize = 2;

/// Nøklene i innstillingene, med streken mot verdiene rett etter.
pub(super) const NØKKEL_BREDDE: u16 = 12;

pub(super) const LUKKE_BREDDE: u16 = 46;

/// Linjene i popupen som spør, over knappene.
pub(super) fn lukke_tekst(lukke: &crate::Lukke) -> Vec<String> {
    let indre = (LUKKE_BREDDE - 4) as usize;
    bryt(&lukke.hva, indre, indre, 3)
}

pub(super) fn lukke_boks(app: &App, område: Rect) -> Option<Rect> {
    let lukke = app.lukke()?;
    midt_på(område, LUKKE_BREDDE, lukke_tekst(lukke).len() as u16 + 4)
}

/// Knappene på nederste rad inni popupen, med `true` for «lukk», og tasten
/// som gjør det samme.
pub(super) const SVAR: [(bool, &str, &str); 2] = [(true, " lukk", "y "), (false, " avbryt", "n ")];

pub(super) fn svar(app: &App) -> Vec<(bool, Range<u16>, u16)> {
    let Some(boks) = lukke_boks(app, app.skjerm) else { return Vec::new() };
    let (mut x, y) = (boks.x + 2, boks.bottom() - 2);
    let mut ut = Vec::new();
    for (lukk, navn, tast) in SVAR {
        let bredde = (navn.chars().count() + 2 + tast.len()) as u16;
        ut.push((lukk, x..x + bredde, y));
        x += bredde + 2;
    }
    ut
}

pub(super) fn svar_på(app: &App, x: u16, y: u16) -> Option<bool> {
    svar(app).into_iter().find(|(_, r, rad)| *rad == y && r.contains(&x)).map(|(lukk, ..)| lukk)
}

pub(super) fn i_popupen(app: &App, x: u16, y: u16) -> bool {
    lukke_boks(app, app.skjerm).is_some_and(|r| r.contains((x, y).into()))
}

pub(super) fn tegn_lukke(buf: &mut Buffer, område: Rect, app: &App, lukke: &crate::Lukke, mus: Option<&Mål>) {
    let Some(plass) = lukke_boks(app, område) else { return };
    let Some(ramme) = boks(buf, område, plass.width, plass.height, lukke.spørsmål, "") else { return };
    let indre = (ramme.width - 4) as usize;
    for (i, linje) in lukke_tekst(lukke).into_iter().enumerate() {
        buf.set_stringn(ramme.x + 2, ramme.y + 1 + i as u16, linje, indre, VANLIG);
    }
    for ((lukk, x, y), (_, navn, tast)) in svar(app).into_iter().zip(SVAR) {
        let bakgrunn = if mus == Some(&Mål::Svar(lukk)) { under_musen() } else { markert() };
        let stil = if lukk { Style::new().fg(ORANSJE).add_modifier(Modifier::BOLD) } else { VANLIG };
        let etter = buf.set_stringn(x.start, y, format!("{navn}  "), x.len(), stil.patch(bakgrunn)).0;
        buf.set_string(etter, y, tast, SVAK.patch(bakgrunn));
    }
}

/// Der en boks på `bredde` og `høyde` står midt på skjermen, eller `None` når
/// skjermen er for liten.
pub(super) fn midt_på(område: Rect, bredde: u16, høyde: u16) -> Option<Rect> {
    let bredde = område.width.saturating_sub(4).min(bredde);
    let høyde = område.height.saturating_sub(2).min(høyde);
    if bredde < 12 || høyde < 5 {
        return None;
    }
    Some(Rect::new((område.width - bredde) / 2, (område.height - høyde) / 3, bredde, høyde))
}

/// En boks med ramme midt på skjermen, med tittel øverst og hint nederst.
pub(super) fn boks(buf: &mut Buffer, område: Rect, bredde: u16, høyde: u16, tittel: &str, hint: &str) -> Option<Rect> {
    let boks = midt_på(område, bredde, høyde)?;
    let bredde = boks.width;
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
    if !hint.is_empty() {
        buf.set_stringn(boks.x + 2, boks.bottom() - 1, format!(" {hint} "), tekst, SVAK.fg(Color::Reset));
    }
    Some(boks)
}

pub(super) const HJELP_BREDDE: u16 = 52;

pub(super) fn hjelp_linjer(prefiks: &str, bindinger: &Bindinger) -> Vec<(String, Style)> {
    let tast = |koder: &[KeyCode]| {
        let taster: Vec<KeyEvent> = koder.iter().map(|&k| KeyEvent::new(k, KeyModifiers::NONE)).collect();
        // Pilene står sammen, men en tast du har bundet, trenger luft rundt.
        let piler = taster.iter().all(|k| !matches!(k.code, KeyCode::Char(_)) && !bindinger.er_din(k));
        let navn: Vec<String> = taster.iter().map(|k| bindinger.vis(k)).collect();
        format!("{prefiks} {}", navn.join(if piler { "" } else { " " }))
    };
    let alle = hjelp::OVERSIKT.iter().flat_map(|(_, taster)| taster.iter());
    let bredde = alle.map(|(koder, _)| tast(koder).chars().count()).max().unwrap_or(0) + 3;
    let mut linjer = Vec::new();
    for (overskrift, taster) in hjelp::OVERSIKT {
        linjer.push((overskrift.to_string(), STERK));
        for (koder, hva) in taster {
            linjer.push((format!("{:<bredde$}{hva}", tast(koder)), VANLIG));
        }
        linjer.push((String::new(), VANLIG));
    }
    let merker: Vec<String> =
        hjelp::MERKER.iter().map(|(m, hva)| format!("{} {}", glyf(*m), hva.split([' ', ',']).next().unwrap_or(hva))).collect();
    linjer.push((merker.join("  "), VANLIG));
    linjer.push((String::new(), VANLIG));
    linjer.push((format!("{:<bredde$}alt annet, søkbart", tast(&[KeyCode::Char(':')])), VANLIG));
    linjer
}

/// Hjelpen på skjermen, og hvor mange linjer den kan rulles. 0 når alt får plass.
pub(super) fn hjelp_boks(område: Rect, linjer: usize) -> Option<(Rect, u16)> {
    let boks = midt_på(område, HJELP_BREDDE, linjer as u16 + 2)?;
    Some((boks, (linjer as u16).saturating_sub(boks.height - 2)))
}

pub fn hjelp_rull_maks(app: &App) -> u16 {
    let linjer = hjelp_linjer(&app.prefiks.navn(), &app.bindinger).len();
    hjelp_boks(app.skjerm, linjer).map_or(0, |(_, maks)| maks)
}

pub(super) fn tegn_hjelp(buf: &mut Buffer, område: Rect, prefiks: &str, bindinger: &Bindinger, rull: u16) {
    let linjer = hjelp_linjer(prefiks, bindinger);
    let Some((plass, maks)) = hjelp_boks(område, linjer.len()) else { return };
    let hint = if maks > 0 { "↑↓ rull · trykk en tast for å lukke" } else { "trykk en tast for å lukke" };
    let Some(boks) = boks(buf, område, plass.width, plass.height, "nerdr", hint) else { return };
    let rull = rull.min(maks);
    let x = boks.x + 2;
    let indre = (boks.width - 4) as usize;
    for ((tekst, stil), y) in linjer.iter().skip(rull as usize).zip(boks.y + 1..boks.bottom() - 1) {
        buf.set_stringn(x, y, tekst, indre, *stil);
    }
    // Som pilene i lista, men i rammen, så de ikke tar en linje av teksten.
    let pil = SVAK.fg(Color::Reset);
    if rull > 0 {
        buf.set_string(boks.right() - 5, boks.y, " ↑ ", pil);
    }
    if rull < maks {
        buf.set_string(boks.right() - 5, boks.bottom() - 1, " ↓ ", pil);
    }
}

/// Hvor overskriftene og valgene står i innstillingene.
pub(super) struct Valgmeny {
    pub(super) boks: Rect,
    pub(super) overskrifter: Vec<(u16, &'static str)>,
    /// Aksen, raden og kolonnene til verdiene.
    pub(super) rader: Vec<(Akse, u16, Vec<Range<u16>>)>,
    pub(super) forklaring: u16,
}

pub(super) fn valgmeny(område: Rect, u: &Utseende) -> Option<Valgmeny> {
    let (mut overskrifter, mut rader) = (Vec::new(), Vec::new());
    let mut y = 0;
    for (i, (tittel, akser)) in utseende::MENY.iter().enumerate() {
        y += (i > 0) as u16;
        overskrifter.push((y, *tittel));
        y += 1;
        for &akse in akser.iter().filter(|&&a| u.gjelder(a)) {
            rader.push((akse, y));
            y += 1;
        }
    }
    let forklaring = y + 1;
    let boks = midt_på(område, UTSEENDE_BREDDE, forklaring + FORKLARING_LINJER as u16 + 2)?;
    let (x, topp) = (boks.x + 2, boks.y + 1);
    let rader = rader
        .into_iter()
        .map(|(akse, y)| {
            let mut vx = x + NØKKEL_BREDDE + 2;
            let kolonner = akse
                .verdier()
                .iter()
                .map(|verdi| {
                    let r = vx..vx + verdi.meny.chars().count() as u16;
                    vx = r.end + 2;
                    r
                })
                .collect();
            (akse, topp + y, kolonner)
        })
        .collect();
    let overskrifter = overskrifter.into_iter().map(|(y, t)| (topp + y, t)).collect();
    Some(Valgmeny { boks, overskrifter, rader, forklaring: topp + forklaring })
}

/// Verdien under musen i innstillingene, som aksen og indeksen.
pub(super) fn verdi_på(app: &App, x: u16, y: u16) -> Option<(Akse, usize)> {
    let meny = valgmeny(app.skjerm, &app.utseende)?;
    let (akse, _, kolonner) = meny.rader.into_iter().find(|r| r.1 == y)?;
    kolonner.iter().position(|r| r.contains(&x)).map(|i| (akse, i))
}

pub(super) fn i_valgmenyen(app: &App, x: u16, y: u16) -> bool {
    valgmeny(app.skjerm, &app.utseende).is_some_and(|m| m.boks.contains((x, y).into()))
}

/// Konfigurasjonen midt på skjermen, med aksen `valgt` markert. Valgene som
/// ikke gjelder, er borte.
pub(super) fn tegn_utseende(buf: &mut Buffer, område: Rect, u: &Utseende, valgt: Akse, mus: Option<&Mål>) {
    let Some(meny) = valgmeny(område, u) else { return };
    let (b, h) = (meny.boks.width, meny.boks.height);
    let Some(ramme) = boks(buf, område, b, h, "innstillinger", "↑↓ velg · ←→ endre · klikk · esc lukk") else { return };
    let (x, bunn) = (ramme.x + 2, ramme.bottom() - 1);
    let indre = (ramme.width - 4) as usize;
    for (y, tittel) in meny.overskrifter.into_iter().filter(|(y, _)| *y < bunn) {
        buf.set_string(x, y, tittel, STERK);
    }
    for (akse, y, kolonner) in meny.rader.into_iter().filter(|r| r.1 < bunn) {
        if akse == valgt {
            buf.set_style(Rect::new(ramme.x + 1, y, ramme.width - 2, 1), markert());
            buf.set_string(ramme.x + 1, y, "▎", VANLIG);
        }
        buf.set_string(x, y, akse.i_menyen(), if akse == valgt { STERK } else { SVAK });
        buf.set_string(x + NØKKEL_BREDDE, y, "│", SVAK);
        for (i, (r, verdi)) in kolonner.into_iter().zip(akse.verdier()).enumerate() {
            let stil = match () {
                _ if i == u.indeks(akse) => Style::new().fg(ORANSJE).add_modifier(Modifier::BOLD),
                _ if mus == Some(&Mål::Verdi(akse, i)) => under_musen().patch(VANLIG),
                _ => VANLIG,
            };
            buf.set_stringn(r.start, y, verdi.meny, ramme.right().saturating_sub(r.start + 1) as usize, stil);
        }
    }
    let forklaring = bryt(valgt.forklaring(), indre, indre, FORKLARING_LINJER);
    for (linje, y) in forklaring.into_iter().zip(meny.forklaring..bunn) {
        buf.set_string(x, y, linje, SVAK);
    }
}

/// Rammen, søkefeltet og streken under det.
pub(super) const OVER_TREFFENE: u16 = 3;

/// Boksen til handlingene eller velgeren, og hvor mange treff den har plass til.
pub(super) fn menyboks(område: Rect, bredde: u16) -> Option<(Rect, usize)> {
    let rader = (område.height * 2 / 3).saturating_sub(OVER_TREFFENE + 1).max(MENY_RADER);
    let boks = midt_på(område, bredde, rader + OVER_TREFFENE + 1)?;
    Some((boks, boks.height.saturating_sub(OVER_TREFFENE + 1) as usize))
}

pub(super) fn søkestrek(buf: &mut Buffer, boks: Rect) {
    let strek = format!("├{}┤", "─".repeat(boks.width as usize - 2));
    buf.set_string(boks.x, boks.y + 2, strek, Style::new().fg(ORANSJE));
}

pub fn menyrader(app: &App) -> usize {
    menyboks(app.skjerm, HANDLINGER_BREDDE).map_or(1, |(_, rader)| rader.max(1))
}

/// Rullet så `valgt` vises, og uten tomme rader nederst.
pub fn følg(valgt: usize, rull: usize, rader: usize, antall: usize) -> usize {
    let rull = if valgt < rull { valgt } else if valgt >= rull + rader { valgt + 1 - rader } else { rull };
    rull.min(antall.saturating_sub(rader))
}

/// Boksen og treffene til handlingene eller velgeren, den som er åpen, med
/// hvor langt de er rullet og hvor mange det er.
pub(super) fn åpen_meny(app: &App) -> Option<(Rect, usize, usize, usize)> {
    let (bredde, rull, antall) = match &app.popup {
        Some(Popup::Handlinger(h)) => (HANDLINGER_BREDDE, h.felt.rull, h.treff().len()),
        Some(Popup::Velger(v)) => (VELGER_BREDDE, v.felt.rull, v.antall_treff()),
        _ => return None,
    };
    let (boks, rader) = menyboks(app.skjerm, bredde)?;
    Some((boks, rader, rull, antall))
}

/// Treffet under musen i handlingene eller velgeren.
pub(super) fn meny_rad_på(app: &App, x: u16, y: u16) -> Option<usize> {
    let (boks, rader, rull, antall) = åpen_meny(app)?;
    if !(boks.x + 1..boks.right() - 1).contains(&x) {
        return None;
    }
    let rad = y.checked_sub(boks.y + OVER_TREFFENE)? as usize;
    (rad < rader && rull + rad < antall).then_some(rull + rad)
}

pub(super) fn i_menyboks(app: &App, x: u16, y: u16) -> bool {
    åpen_meny(app).is_some_and(|(boks, ..)| boks.contains((x, y).into()))
}

/// Pilene i rammen når det er flere treff over eller under.
pub(super) fn menypiler(buf: &mut Buffer, boks: Rect, fra: usize, rader: usize, antall: usize) {
    let pil = SVAK.fg(Color::Reset);
    if fra > 0 {
        buf.set_string(boks.right() - 5, boks.y, " ↑ ", pil);
    }
    if fra + rader < antall {
        buf.set_string(boks.right() - 5, boks.bottom() - 1, " ↓ ", pil);
    }
}

/// Bakgrunnen til treff nummer `i`: markert når den er valgt, og en tone
/// når musen er over den.
pub(super) fn menyrad(buf: &mut Buffer, boks: Rect, y: u16, valgt: bool, pekt: bool) {
    let rad = Rect::new(boks.x + 1, y, boks.width - 2, 1);
    if valgt {
        buf.set_style(rad, markert());
        buf.set_string(boks.x + 1, y, "▎", VANLIG);
    } else if pekt {
        buf.set_style(rad, under_musen());
    }
}

/// Handlingene midt på skjermen, med tasten til høyre. Gir der markøren
/// skal stå i søkefeltet.
pub(super) fn tegn_handlinger(buf: &mut Buffer, område: Rect, handlinger: &Handlinger, mus: Option<&Mål>) -> (u16, u16) {
    let Some((plass, linjer)) = menyboks(område, HANDLINGER_BREDDE) else { return (0, 0) };
    let Some(boks) = boks(buf, område, plass.width, plass.height, "handlinger", "⏎ kjør · klikk · esc lukk") else {
        return (0, 0);
    };
    søkestrek(buf, boks);
    let indre = (boks.width - 4) as usize;
    let treff = handlinger.treff();
    let (x, y) = (boks.x + 2, boks.y + 1);
    let etter = buf.set_stringn(x, y, "› ", indre, Style::new().fg(ORANSJE)).0;
    let markør = buf.set_stringn(etter, y, kutt(&handlinger.felt.søk, indre.saturating_sub(10)), indre, VANLIG).0;
    let antall = format!("{}/{}", treff.len(), handlinger.antall());
    buf.set_string(boks.right() - 2 - antall.chars().count() as u16, y, antall, SVAK);

    let fra = handlinger.felt.rull;
    for (i, (k, tegn)) in treff.iter().enumerate().skip(fra).take(linjer) {
        let y = boks.y + OVER_TREFFENE + (i - fra) as u16;
        let valgt = i == handlinger.felt.valgt;
        menyrad(buf, boks, y, valgt, mus == Some(&Mål::Rad(i)));
        let bredde = k.tast.chars().count();
        buf.set_string(boks.right() - 2 - bredde as u16, y, &k.tast, SVAK);
        let stil = if valgt { STERK } else { VANLIG };
        for (n, c) in kutt(&k.navn, indre.saturating_sub(bredde + 2)).chars().enumerate() {
            let stil = if tegn.contains(&n) { Style::new().fg(ORANSJE) } else { stil };
            buf.set_string(x + n as u16, y, c.encode_utf8(&mut [0; 4]), stil);
        }
    }
    menypiler(buf, boks, fra, linjer, treff.len());
    (markør, y)
}

/// Mappevelgeren midt på skjermen. Gir der markøren skal stå i søkefeltet.
pub(super) fn tegn_velger(buf: &mut Buffer, område: Rect, velger: &Velger, mus: Option<&Mål>) -> (u16, u16) {
    let (tittel, hint) = match velger.fortsett {
        false => ("ny sesjon", "⏎ start · C-r hent samtale · esc avbryt"),
        true => ("hent samtale", "⏎ hent · C-r ny sesjon · esc avbryt"),
    };
    let Some((plass, linjer)) = menyboks(område, VELGER_BREDDE) else { return (0, 0) };
    let Some(boks) = boks(buf, område, plass.width, plass.height, tittel, hint) else {
        return (0, 0);
    };
    søkestrek(buf, boks);
    let bredde = boks.width;
    let indre = (bredde - 4) as usize;

    let treff = velger.treff();
    let x = boks.x + 2;
    let y = boks.y + 1;
    let prompt = if velger.fortsett { "samtale › " } else { "mappe › " };
    let søk = kutt(velger.søk(), indre.saturating_sub(prompt.chars().count() + 8));
    let etter = buf.set_stringn(x, y, prompt, indre, Style::new().fg(ORANSJE)).0;
    let markør = buf.set_stringn(etter, y, &søk, indre, VANLIG).0;
    let søker = if velger.søker && !velger.fortsett { " …" } else { "" };
    let antall = match søk.is_empty() && !velger.fortsett {
        true => format!("{}{søker}", velger.antall()),
        false => format!("{}/{}{søker}", treff.len(), velger.antall()),
    };
    buf.set_string(boks.right() - 2 - antall.chars().count() as u16, y, antall, SVAK);

    let fra = velger.felt.rull;
    for (i, t) in treff.iter().enumerate().skip(fra).take(linjer) {
        let y = boks.y + OVER_TREFFENE + (i - fra) as u16;
        let valgt = i == velger.felt.valgt;
        menyrad(buf, boks, y, valgt, mus == Some(&Mål::Rad(i)));
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
        // En sti som ikke får plass, kuttes foran, så mappa du velger, vises.
        let lengde = k.vis.chars().count();
        let foran = k.samtale.is_none() && lengde > plass;
        let vis = if foran { kutt_foran(&k.vis, plass) } else { kutt(&k.vis, plass) };
        let hopp = if foran { lengde + 1 - plass } else { 0 };
        for (n, c) in vis.chars().enumerate() {
            // Tegnet i `k.vis` som står her. … foran står for det som er kuttet.
            let i = match (foran, n) {
                (true, 0) => None,
                (true, n) => Some(hopp + n - 1),
                (false, n) => Some(n),
            };
            let stil = match () {
                _ if i.is_some_and(|i| t.tegn.contains(&i)) => Style::new().fg(ORANSJE),
                _ if n < mappe => SVAK,
                _ => stil,
            };
            buf.set_string(x + n as u16, y, c.encode_utf8(&mut [0; 4]), stil);
        }
    }
    menypiler(buf, boks, fra, linjer, treff.len());
    (markør, y)
}
