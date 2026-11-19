//! Farger, tonene av dem og stilene tegningen bruker.

use nerdr_kjerne::sgr::{Farge, Stil};
use ratatui::style::{Color, Modifier, Style};
use std::sync::OnceLock;
use crate::palett::{Rgb, bland};

pub(super) const ORANSJE_RGB: Rgb = (217, 119, 87);

pub(super) const ORANSJE: Color = Color::Rgb(ORANSJE_RGB.0, ORANSJE_RGB.1, ORANSJE_RGB.2);

pub(super) const GRØNN: Color = Color::Indexed(108);

/// Fargen 108 i 256-paletten, til å blande toner av.
pub(super) const GRØNN_RGB: Rgb = (135, 175, 135);

// Gråtonene i 256-paletten er faste og følger ikke fargetemaet, så teksten har
// standardfargen til terminalen og skiller seg bare med fet og svak.
pub(super) const STERK: Style = Style::new().add_modifier(Modifier::BOLD);

pub(super) const VANLIG: Style = Style::new();

pub(super) const SVAK: Style = Style::new().add_modifier(Modifier::DIM);

/// Så mye av forgrunnen som blandes inn i bakgrunnen der noe er valgt, og der
/// musen står.
pub(super) const MARKERT_ANDEL: f32 = 0.12;

pub(super) const PEKT_ANDEL: f32 = 0.18;

/// Rammene rundt panene, og streken mellom fanene og panene, som skal være
/// svakere enn rammen.
pub(super) const RAMME_ANDEL: f32 = 0.35;

pub(super) const AKTIV_RAMME_ANDEL: f32 = 0.6;

pub(super) const SKILLE_ANDEL: f32 = 0.2;

pub(super) const INNE_ANDEL: f32 = 0.05;

pub(super) const UNDER_MUSEN_ANDEL: f32 = 0.06;

/// Forgrunns- og bakgrunnsfargen til terminalen.
pub(super) static FARGER: OnceLock<Option<(Rgb, Rgb)>> = OnceLock::new();

pub fn bruk_farger(farger: Option<(Rgb, Rgb)>) {
    let _ = FARGER.set(farger);
}

pub fn farger() -> Option<(Rgb, Rgb)> {
    FARGER.get().copied().flatten()
}

/// `andel` av `farge`, eller av forgrunnen, blandet inn i bakgrunnen til
/// terminalen, så det passer til alle fargetemaer. `None` når terminalen ikke
/// har sagt hvilke farger den har.
pub(super) fn tonet(farge: Option<Rgb>, andel: f32) -> Option<Color> {
    let (fg, bg) = farger()?;
    let (r, g, b) = bland(bg, farge.unwrap_or(fg), andel);
    Some(Color::Rgb(r, g, b))
}

/// Valgt kort og fane får litt av forgrunnen i bakgrunnen. Uten farger fra
/// terminalen står de uten bakgrunn, med peker og fet tekst.
pub(super) fn markert() -> Style {
    tonet(None, MARKERT_ANDEL).map_or(Style::new(), |c| Style::new().bg(c))
}

/// Pekeren i lista skiller seg mye mer ut enn sesjonen du er inne i, så du
/// ser hvor du står når du blar, og hvor du kom fra.
pub(super) fn pekt() -> Style {
    tonet(None, PEKT_ANDEL).map_or(Style::new(), |c| Style::new().bg(c)).add_modifier(Modifier::BOLD)
}

pub(super) fn inne() -> Style {
    tonet(None, INNE_ANDEL).map_or(Style::new(), |c| Style::new().bg(c))
}

/// Uten farger fra terminalen er den aktive rammen vanlig og de andre svake.
pub(super) fn rammestil(aktiv: bool) -> Style {
    let (andel, uten) = if aktiv { (AKTIV_RAMME_ANDEL, VANLIG) } else { (RAMME_ANDEL, SVAK) };
    tonet(None, andel).map_or(uten, |c| Style::new().fg(c))
}

pub(super) fn skillestil() -> Style {
    tonet(None, SKILLE_ANDEL).map_or(SVAK, |c| Style::new().fg(c))
}

pub(super) fn musestil() -> Style {
    tonet(None, UNDER_MUSEN_ANDEL).map_or(Style::new(), |c| Style::new().bg(c))
}

pub(super) fn stil(s: Stil) -> Style {
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

pub(super) fn farge(f: Farge) -> Color {
    match f {
        Farge::Standard => Color::Reset,
        Farge::Indeks(i) => Color::Indexed(i),
        Farge::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
