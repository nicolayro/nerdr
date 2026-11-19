//! Panene i vinduet og rammene mellom dem.

use nerdr_kjerne::sgr::Linje;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use crate::{App, Fokus, markering};
use crate::bokser::{Boks, Rammer};
use super::*;

/// Rammen rundt lista, der skillelinja er høyrekanten, og rammen rundt panen,
/// der streken under fanene treffer kantene. Kanten panen starter ved, er den
/// du drar i.
pub(super) fn tegn_vinduet(buf: &mut Buffer, app: &App, mus: Option<&Mål>) {
    let (liste, pane) = rammene(app);
    let h = app.skjerm.height;
    if liste.width < 2 || pane.width < 2 || h < FANERADER + 2 {
        return;
    }
    let høyre = liste.right() - 1;
    let strek = |b: u16| "─".repeat(b as usize - 2);
    buf.set_string(0, 0, format!("╭{}╮", strek(liste.width)), SVAK);
    buf.set_string(0, h - 1, format!("╰{}╯", strek(liste.width)), SVAK);
    for y in 1..h - 1 {
        // Strekene går gjennom luften og ut i begge kantene.
        match buf[(høyre, y)].symbol() {
            "┤" => buf.set_string(0, y, "├─", SVAK),
            // Er lista for lav til å tegnes, har den ingen skillelinje.
            " " => {
                buf.set_string(0, y, "│", SVAK);
                buf.set_string(høyre, y, "│", SVAK);
            }
            _ => buf.set_string(0, y, "│", SVAK),
        }
    }
    let skille = if matches!(mus, Some(Mål::Skille)) || app.drar_skille { VANLIG } else { SVAK };
    buf.set_string(pane.x, 0, format!("╭{}╮", strek(pane.width)), SVAK);
    buf.set_string(pane.x, h - 1, format!("╰{}╯", strek(pane.width)), SVAK);
    for y in 1..h - 1 {
        buf.set_string(pane.x, y, "│", skille);
        buf.set_string(pane.right() - 1, y, if y == FANERADER { "┤" } else { "│" }, SVAK);
    }
}

/// Rammen rundt lista, med skillelinja som høyrekant, og rammen rundt panen,
/// én rute utenfor visningen.
pub(super) fn rammene(app: &App) -> (Rect, Rect) {
    let (f, v, h) = (listeflate(app), app.visning, app.skjerm.height);
    (Rect::new(0, 0, f.right(), h), Rect::new(v.x.saturating_sub(1), 0, v.width + 2, h))
}

/// Strekene tmux legger mellom panene, møter rammen og streken under fanene.
/// Bare rutene ingen pane dekker, er strek, så innholdet i panene ikke gir
/// skjøter.
pub(super) fn skjøt_strekene(buf: &mut Buffer, app: &App, bokser: &[Boks]) {
    let (_, r) = rammene(app);
    let v = app.visning;
    if v.width == 0 || v.height == 0 || r.height < 2 || v.y == 0 {
        return;
    }
    let strek = |buf: &Buffer, x: u16, y: u16, tegn: &[&str]| {
        !bokser.iter().any(|b| b.innhold.contains((x, y).into())) && tegn.contains(&buf[(x, y)].symbol())
    };
    let loddrett = ["│", "├", "┤", "┼", "┬", "┴"];
    let vannrett = ["─", "├", "┤", "┼", "┬", "┴"];
    for x in v.left()..v.right() {
        if strek(buf, x, v.y, &loddrett) {
            buf[(x, v.y - 1)].set_symbol("┬");
        }
        if strek(buf, x, v.bottom() - 1, &loddrett) {
            buf[(x, r.bottom() - 1)].set_symbol("┴");
        }
    }
    for y in v.top()..v.bottom() {
        if strek(buf, v.x, y, &vannrett) {
            buf[(r.x, y)].set_symbol("├");
        }
        if strek(buf, v.right() - 1, y, &vannrett) {
            buf[(r.right() - 1, y)].set_symbol("┤");
        }
    }
}

pub(super) fn tegn_pane(buf: &mut Buffer, app: &App, bokser: &[Boks], mus: Option<&Mål>) {
    let v = app.visning;
    if v.width == 0 {
        return;
    }
    if app.valgt.is_none() {
        let bredde = v.width.saturating_sub(4).min(64) as usize;
        let mut y = v.y + 1..v.bottom();
        for (i, avsnitt) in tom_liste(app.utenfor_tmux, &app.prefiks.navn()).iter().enumerate() {
            let stil = if i == 0 { VANLIG } else { SVAK };
            for linje in bryt(avsnitt, bredde, bredde, usize::MAX) {
                let Some(y) = y.next() else { return };
                buf.set_stringn(v.x + 2, y, linje, bredde, stil);
            }
            y.next();
        }
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
        let i = boks.innhold;
        let topp = app.aktiv_pane.historie as i64 - app.aktiv_pane.rull as i64;
        if let (Some(markering), Some(bilde)) =
            (app.vist_markering().filter(|m| m.pane == boks.pane), app.bilder.get(&boks.pane))
        {
            for (y, rad) in (0..i.height).zip(bilde) {
                let innhold = markering::innhold(rad);
                for x in markering.kolonner(topp + y as i64, i.width).into_iter().flatten().filter(|&x| x < innhold) {
                    // Snudd, så det som allerede er invertert, også synes markert.
                    buf[(i.x + x, i.y + y)].modifier.toggle(Modifier::REVERSED);
                }
            }
        }
        if let Some(kopi) = app.aktiv_pane.kopimodus.as_ref().filter(|k| k.pane == boks.pane) {
            let (linje, x) = kopi.markør;
            if let Ok(y) = u16::try_from(linje - topp)
                && y < i.height
                && x < i.width
            {
                // Oransje, så markøren synes også inni markeringen.
                let rute = &mut buf[(i.x + x, i.y + y)];
                rute.modifier.remove(Modifier::REVERSED);
                rute.set_style(Style::new().fg(Color::Black).bg(ORANSJE));
            }
        }
    }
    tegn_rammer(buf, bokser, &lyse);
    if app.utseende.paner != Rammer::Bokser && bokser.len() > 1 {
        tegn_linjer(buf, app, bokser, &lyse);
    }
    let merkelapp = match () {
        _ if app.beskjed.is_some() => app.beskjed.as_ref().map(|(tekst, _)| format!(" {tekst} ")),
        _ if app.kopiert.is_some() => Some(" kopiert ".to_string()),
        _ if app.aktiv_pane.kopimodus.is_some() && app.aktiv_pane.rull > 0 => Some(format!(" kopi {}/{} ", app.aktiv_pane.rull, app.aktiv_pane.historie)),
        _ if app.aktiv_pane.kopimodus.is_some() => Some(" kopi ".to_string()),
        _ if app.aktiv_pane.rull > 0 => Some(format!(" {}/{} ", app.aktiv_pane.rull, app.aktiv_pane.historie)),
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

pub(super) fn aktiv_boks<'a>(app: &App, bokser: &'a [Boks]) -> Option<&'a Boks> {
    let aktiv = app.aktiv()?;
    bokser.iter().find(|b| b.pane == aktiv.id)
}

pub(super) fn tegn_bilde(buf: &mut Buffer, område: Rect, bilde: &[Linje]) {
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
pub(super) fn tegn_linjer(buf: &mut Buffer, app: &App, bokser: &[Boks], lyse: &[&Boks]) {
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

pub(super) fn rammetegn(opp: bool, ned: bool, venstre: bool, høyre: bool) -> &'static str {
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

/// Rammene rundt boksene. Naboer deler kanten, så tegnet i hver rute kommer
/// av hvilke kanter som møtes der. En kant som hører til en lys boks, er lys.
pub(super) fn tegn_rammer(buf: &mut Buffer, bokser: &[Boks], lyse: &[&Boks]) {
    let rammer: Vec<(Rect, bool)> = bokser
        .iter()
        .filter_map(|b| Some((b.ramme?, lyse.iter().any(|l| l.pane == b.pane))))
        .filter(|(r, _)| r.width >= 2 && r.height >= 2)
        .collect();
    let vannrett = |r: &Rect, y: u16, fra: u16, til: u16| (y == r.y || y + 1 == r.bottom()) && r.x <= fra && til < r.right();
    let loddrett = |r: &Rect, x: u16, fra: u16, til: u16| (x == r.x || x + 1 == r.right()) && r.y <= fra && til < r.bottom();
    for (r, _) in &rammer {
        let kant = (r.x..r.right()).flat_map(|x| [(x, r.y), (x, r.bottom() - 1)]);
        let sider = (r.y..r.bottom()).flat_map(|y| [(r.x, y), (r.right() - 1, y)]);
        for (x, y) in kant.chain(sider) {
            let finnes = |f: &dyn Fn(&Rect) -> bool| rammer.iter().any(|(r, _)| f(r));
            let opp = y > 0 && finnes(&|r| loddrett(r, x, y - 1, y));
            let ned = finnes(&|r| loddrett(r, x, y, y + 1));
            let venstre = x > 0 && finnes(&|r| vannrett(r, y, x - 1, x));
            let høyre = finnes(&|r| vannrett(r, y, x, x + 1));
            let tegn = match (opp, ned, venstre, høyre) {
                (false, true, false, true) => "╭",
                (false, true, true, false) => "╮",
                (true, false, false, true) => "╰",
                (true, false, true, false) => "╯",
                (o, n, v, h) => rammetegn(o, n, v, h),
            };
            let på_kanten = |r: &Rect| (r.x..r.right()).contains(&x) && (r.y..r.bottom()).contains(&y);
            let lys = rammer.iter().any(|(r, lys)| *lys && på_kanten(r) && (vannrett(r, y, x, x) || loddrett(r, x, y, y)));
            buf[(x, y)].set_symbol(tegn).set_style(rammestil(lys));
        }
    }
}
