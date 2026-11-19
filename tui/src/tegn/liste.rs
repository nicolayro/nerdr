//! Agentlista: kortene, sorteringen og rullingen.

use nerdr_kjerne::merke::{Gruppe, Merke};
use nerdr_kjerne::paner::Pane;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use std::ops::Range;
use crate::{App, Fokus, hjelp};
use crate::agent::Agent;
use crate::grupper::{Overskrift, Rad};
use crate::tekst::varighet;
use crate::utseende::{Akse, Orden, Kortdesign};
use super::*;

pub(super) fn tegn_liste(buf: &mut Buffer, område: Rect, app: &App, mus: Option<&Mål>) {
    let bredde = liste_bredde(app) - 1;

    let mut overskrifter = Vec::new();
    let (rader, utsnitt) = liste(app);
    let vis = utsnitt.rader();
    let på_skjermen = |y: u16| u16::try_from(utsnitt.skjerm(y)).ok().filter(|y| vis.contains(y));
    let med_strek = på_egen_rad(app);
    for Plassert { y, rad, kort, .. } in &rader {
        match rad {
            Rad::Tom => {}
            Rad::Overskrift(_) if kort.is_some() => {
                let kort = kort.as_deref().unwrap_or_default();
                tegn_kort(buf, utsnitt.skjerm(*y), &vis, bredde, kort, None);
                if med_strek && let Some(y) = på_skjermen(*y) {
                    overskrifter.push(y);
                }
            }
            Rad::Overskrift(o) => {
                if let Some(y) = kort::overskrift(buf, utsnitt.skjerm(*y), &vis, bredde, o, *y == 0, stripe(app)) {
                    overskrifter.push(y);
                }
            }
            Rad::Brettet(_, paner) => {
                let Some(kort) = kort else { continue };
                let bakgrunn = match () {
                    _ if app.fokus == Fokus::Liste && paner.iter().any(|(p, _)| app.peker.as_ref() == Some(&p.id)) => {
                        Some(pekt())
                    }
                    _ if paner.iter().any(|(p, _)| er_valgt(app, p)) => Some(inne()),
                    _ => None,
                };
                tegn_kort(buf, utsnitt.skjerm(*y), &vis, bredde, kort, bakgrunn);
            }
            Rad::Pane(pane, _) => {
                let Some(kort) = kort else { continue };
                // Inne i en pane viser kortet til den valgte hvor du er.
                let pekt = app.fokus == Fokus::Liste && app.peker.as_ref() == Some(&pane.id);
                let bakgrunn = match () {
                    _ if pekt => Some(self::pekt()),
                    _ if er_valgt(app, pane) => Some(inne()),
                    _ if matches!(mus, Some(Mål::Kort(id) | Mål::Lukk(id)) if *id == pane.id) => Some(musestil()),
                    _ => None,
                };
                tegn_kort(buf, utsnitt.skjerm(*y), &vis, bredde, kort, bakgrunn);
                if let (Some(x), Some(y)) = (lukk_x(app), på_skjermen(*y))
                    && matches!(mus, Some(Mål::Kort(id) | Mål::Lukk(id)) if *id == pane.id)
                {
                    tegn_lukk(buf, app, x, y, matches!(mus, Some(Mål::Lukk(_))));
                }
            }
        }
    }

    // Øverst er streken lik i alle sorteringene.
    buf.set_string(0, TOPPEN, "─".repeat(bredde as usize), SVAK);
    overskrifter.push(TOPPEN);
    let (pluss, tekststil) = match app.fokus {
        Fokus::Liste => (Style::new().fg(ORANSJE), VANLIG),
        Fokus::Pane => (SVAK, SVAK),
    };
    if knapper_nederst(app) {
        let y = område.height - rader_nederst(app);
        buf.set_string(0, y, "─".repeat(bredde as usize), SVAK);
        overskrifter.push(y);
    }
    for (knapp, x, y) in knapper(app) {
        let stil = if mus == Some(&Mål::Knapp(knapp)) { musestil() } else { Style::new() };
        if stripe(app) {
            buf.set_string(0, y, " + ", pluss.patch(stil));
            continue;
        }
        let Some(&(_, navn, tast)) = KNAPPER.iter().find(|k| k.0 == knapp) else { continue };
        buf.set_style(Rect::new(x.start, y, x.end - x.start, 1), stil);
        if y > TOPPEN {
            let tast = app.bindinger.vis(&KeyEvent::new(KeyCode::Char(tast), KeyModifiers::NONE));
            let tast = format!("{} {tast}", app.prefiks.navn());
            let tast_x = x.end.saturating_sub(tast.chars().count() as u16 + 1);
            if knapp == Knapp::Ny {
                buf.set_string(1, y, "+", pluss.patch(stil));
            }
            buf.set_stringn(3, y, navn, tast_x.saturating_sub(4) as usize, tekststil.patch(stil));
            buf.set_string(tast_x, y, tast, SVAK.patch(stil));
            continue;
        }
        if knapp == Knapp::Ny {
            buf.set_string(1, y, "+", pluss.patch(stil));
            buf.set_stringn(3, y, navn, x.end.saturating_sub(4) as usize, tekststil.patch(stil));
            continue;
        }
        let fra = x.end.saturating_sub(navn.chars().count() as u16 + 1);
        buf.set_string(fra, y, navn, tekststil.patch(stil));
        if knapp == Knapp::Hjelp {
            buf.set_string(fra, y, "?", SVAK.patch(stil));
        }
    }
    if !stripe(app) {
        let versjon = format!(" nerdr {}", hjelp::versjon());
        buf.set_stringn(0, 0, versjon, høyre_kolonne(app).saturating_sub(1) as usize, SVAK);
    }

    if let Some((x, tekst)) = sortering(app) {
        let stil = if mus == Some(&Mål::Sortering) { musestil().patch(VANLIG) } else { SVAK };
        buf.set_string(x.start, NY_RAD, tekst, stil);
    }

    // Overskriftene treffer skillestreken. Når lista ruller, står pilen i den
    // øverste raden, på høyde med fanene.
    if utsnitt.ruller {
        let kort = |p: &&Plassert| matches!(p.rad, Rad::Pane(..) | Rad::Brettet(..));
        let mut over = rader.iter().filter(kort).filter(|p| p.y + p.høyde <= utsnitt.rull).peekable();
        let under = rader.iter().filter(kort).any(|p| p.y >= utsnitt.rull + utsnitt.rom);
        if over.peek().is_some() {
            let venter = over.any(|p| match &p.rad {
                Rad::Pane(_, m) => m.gruppe() == Gruppe::Venter,
                Rad::Brettet(_, paner) => paner.iter().any(|(_, m)| m.gruppe() == Gruppe::Venter),
                _ => false,
            });
            let stil = if venter { Style::new().fg(ORANSJE) } else { SVAK };
            buf.set_stringn(1, vis.start - 1, " ↑ ", bredde as usize, stil);
        }
        if under {
            buf.set_stringn(1, vis.end, " ↓ ", bredde as usize, SVAK);
        }
    }
    // Skillelinja lyser når musen er over den, så du ser at den kan dras.
    let skille = if matches!(mus, Some(Mål::Skille)) || app.drar_skille { VANLIG } else { SVAK };
    for y in 0..område.height {
        let tegn = match y {
            _ if overskrifter.contains(&y) => "┤",
            _ => "│",
        };
        buf.set_stringn(bredde, y, tegn, 1, skille);
    }
}

/// Knappen som bytter sortering, høyrestilt ved siden av ny sesjon. Den er
/// like bred for alle sorteringene, så den ikke flytter seg når du klikker.
pub(super) fn sortering(app: &App) -> Option<(Range<u16>, String)> {
    if stripe(app) {
        return None;
    }
    let sorteringer = Akse::Orden.verdier();
    let bredde = sorteringer.iter().map(|s| s.meny.chars().count()).max().unwrap_or(0);
    let tekst = format!(" {:>bredde$} ", sorteringer[app.utseende.indeks(Akse::Orden)].meny);
    let slutt = liste_bredde(app) - 1;
    let start = slutt.checked_sub(tekst.chars().count() as u16).filter(|&x| x >= NY_BREDDE)?;
    Some((start..slutt, tekst))
}

pub(super) fn sortering_på(app: &App, x: u16, y: u16) -> bool {
    y == NY_RAD && sortering(app).is_some_and(|(r, _)| r.contains(&x))
}

/// Prosjektet står på en rad for seg over kortene.
pub(super) fn på_egen_rad(app: &App) -> bool {
    app.utseende.orden == Orden::Mappe && !stripe(app)
}

pub(super) fn er_valgt(app: &App, pane: &Pane) -> bool {
    app.valgt.as_ref().is_some_and(|v| v.id == pane.id)
}

/// En Claude i lista: oppgaven som tittel, med hvor lenge den har ventet,
/// jobbet eller vært ferdig, og repoet og branchen. Uten oppgave er repoet
/// tittelen.
pub(super) fn kort_for_pane(app: &App, pane: &Pane, merke: Merke) -> Vec<kort::Kortlinje> {
    let repo = app.repoer.get(&pane.id);
    // En som starter, er ikke i registeret ennå, og har ingen tid.
    let agent = app.agenter.get(&pane.id);
    let tid = agent.and_then(Agent::endret).map(|endret| varighet(crate::nå_ms().saturating_sub(endret)));
    let info = kort::Info {
        merke,
        oppgave: match agent {
            Some(Agent::Rest { programmer, .. }) => Some(format!("uten Claude: {programmer}")),
            _ => None,
        }
        .or_else(|| pane.oppgave().map(str::to_string)),
        sted: app.sted(pane),
        branch: repo.and_then(|r| r.branch.clone()),
        tid,
        sted_over: app.utseende.orden == Orden::Mappe,
        undermappe: app.undermappe(pane),
    };
    kort::kort(app.utseende, &info, liste_bredde(app) - 1, er_valgt(app, pane), stripe(app))
}

/// Kortet med første linje på rad `topp`, som kan være over skjermen når
/// lista ruller. Bare linjene på radene i `vis` tegnes.
pub(super) fn tegn_kort(buf: &mut Buffer, topp: i32, vis: &Range<u16>, bredde: u16, kort: &[kort::Kortlinje], bakgrunn: Option<Style>) {
    for (i, linje) in kort.iter().enumerate() {
        let Some(y) = u16::try_from(topp + i as i32).ok().filter(|y| vis.contains(y)) else { continue };
        if let Some(bakgrunn) = bakgrunn {
            buf.set_style(Rect::new(0, y, bredde, 1), bakgrunn);
        }
        for bit in linje {
            buf.set_stringn(bit.x, y, &bit.tekst, bredde.saturating_sub(bit.x) as usize, bit.stil);
        }
    }
}

/// En rad i lista, med kortet når raden er en pane.
pub(super) struct Plassert<'a> {
    pub(super) y: u16,
    pub(super) høyde: u16,
    pub(super) rad: Rad<'a>,
    pub(super) kort: Option<Vec<kort::Kortlinje>>,
}

/// Radene i lista, etter hverandre fra toppen.
pub(super) fn plasser(app: &App) -> Vec<Plassert<'_>> {
    let mut y = 0;
    let bredde = liste_bredde(app) - 1;
    let på_rad = på_egen_rad(app);
    kort::ordne(app.rader())
        .into_iter()
        .map(|rad| {
            let overskrift = matches!(rad, Rad::Overskrift(Overskrift::Del(_)));
            let kort = match &rad {
                Rad::Pane(pane, merke) => Some(kort_for_pane(app, pane, *merke)),
                Rad::Overskrift(Overskrift::Mappe(navn)) if på_rad => Some(vec![kort::prosjektrad(navn, &[], bredde, false, false)]),
                Rad::Brettet(navn, paner) => {
                    let merker: Vec<Merke> = paner.iter().map(|(_, m)| *m).collect();
                    let valgt = paner.iter().any(|(p, _)| er_valgt(app, p));
                    Some(vec![match på_rad {
                        true => kort::prosjektrad(navn, &merker, bredde, true, valgt),
                        false => kort::brettet_stripe(&merker, valgt),
                    }])
                }
                _ => None,
            };
            let høyde = match kort.as_ref() {
                Some(k) => k.len() as u16,
                None if overskrift => kort::overskrift_høyde(y == 0, stripe(app)),
                None => 1,
            };
            let plassert = Plassert { y, høyde, rad, kort };
            y += høyde;
            plassert
        })
        .collect()
}

/// Rader til kortene, under streken øverst.
pub(super) fn plass(app: &App) -> u16 {
    listeflate(app).height.saturating_sub(TOPPEN + 1 + rader_nederst(app))
}

pub(super) fn liste(app: &App) -> (Vec<Plassert<'_>>, Utsnitt) {
    let rader = plasser(app);
    let total = rader.last().map_or(0, |p| p.y + p.høyde);
    let utsnitt = Utsnitt { topp: TOPPEN + 1, ..Utsnitt::ny(total, plass(app), app.liste_rull) };
    (rader, utsnitt)
}

/// Den delen av lista som vises. Får ikke alt plass, ruller lista, og øverste
/// og nederste rad sier hvor mange kort som er over og under.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Utsnitt {
    /// Første rad i lista som vises.
    pub rull: u16,
    /// Rader til kortene.
    pub rom: u16,
    pub ruller: bool,
    /// Rader på skjermen over lista.
    pub topp: u16,
}

impl Utsnitt {
    pub fn ny(total: u16, plass: u16, rull: u16) -> Utsnitt {
        if total <= plass {
            return Utsnitt { rull: 0, rom: plass, ruller: false, topp: 0 };
        }
        let rom = plass.saturating_sub(2);
        Utsnitt { rull: rull.min(total - rom), rom, ruller: true, topp: 0 }
    }

    /// Raden på skjermen der rad `y` i lista står, også når den er utenfor.
    pub(super) fn skjerm(&self, y: u16) -> i32 {
        y as i32 - self.rull as i32 + self.ruller as i32 + self.topp as i32
    }

    /// Radene på skjermen kortene står på.
    pub(super) fn rader(&self) -> Range<u16> {
        let topp = self.topp + self.ruller as u16;
        topp..topp + self.rom
    }

    pub(super) fn i_lista(&self, y: u16) -> Option<u16> {
        self.rader().contains(&y).then(|| y - self.rader().start + self.rull)
    }

    /// Rullet så radene i `valgt` vises. Er de flere enn utsnittet, vises toppen.
    pub fn følg(mut self, valgt: Range<u16>) -> Utsnitt {
        if !self.ruller {
            return self;
        }
        if valgt.end > self.rull + self.rom {
            self.rull = valgt.end - self.rom;
        }
        self.rull = self.rull.min(valgt.start);
        self
    }
}

/// Utsnittet nå, og radene til den markerte i lista, med overskriften over
/// når den er den første i gruppen.
pub fn utsnitt(app: &App) -> (Utsnitt, Option<Range<u16>>) {
    let (rader, utsnitt) = liste(app);
    let i = app.markert().and_then(|id| {
        rader.iter().position(|p| match &p.rad {
            Rad::Pane(pane, _) => pane.id == id,
            Rad::Brettet(_, paner) => paner.iter().any(|(pane, _)| pane.id == id),
            _ => false,
        })
    });
    let valgt = i.map(|i| {
        let p = &rader[i];
        let topp = match i.checked_sub(1).map(|f| &rader[f]) {
            Some(Plassert { rad: Rad::Overskrift(_), y, .. }) => *y,
            _ => p.y,
        };
        topp..p.y + p.høyde
    });
    (utsnitt, valgt)
}

/// Kolonnen der `×` står på kortet under musen: midt der tiden står, som
/// oftest er tre tegn, som `<1m`.
pub(super) fn lukk_x(app: &App) -> Option<u16> {
    let bredde = liste_bredde(app) - 1;
    match app.utseende.kortdesign {
        _ if stripe(app) => None,
        Kortdesign::Minimal => bredde.checked_sub(3),
        _ => bredde.checked_sub(6),
    }
}

/// `×` i stedet for tiden, på den samme plassen, så ingenting rundt flytter
/// seg når musen kommer. I boksen tas luften rundt tiden med.
pub(super) fn tegn_lukk(buf: &mut Buffer, app: &App, x: u16, y: u16, pekt: bool) {
    let stil = if pekt { Style::new().fg(ORANSJE).add_modifier(Modifier::BOLD) } else { STERK };
    let luft = if app.utseende.kortdesign == Kortdesign::Minimal { "   " } else { "     " };
    let bredde = luft.len() as u16;
    buf.set_string(x.saturating_sub(bredde / 2), y, luft, Style::new());
    buf.set_string(x, y, "×", stil);
}

/// Det som står i stedet for panen når lista er tom, skrevet for den som
/// ikke kjenner tmux.
pub(super) fn tom_liste(utenfor_tmux: usize, prefiks: &str) -> Vec<String> {
    let mut avsnitt = vec![
        "Her er det tomt foreløpig.".to_string(),
        format!(
            "nerdr viser Claude-ene dine som kjører i tmux, og lar deg bytte mellom dem. Klikk på «+ ny sesjon» \
             øverst, eller trykk {prefiks} f, for å starte en ny Claude i en av mappene dine, og {prefiks} r \
             for å fortsette en samtale du har hatt før."
        ),
    ];
    avsnitt.push(match utenfor_tmux {
        0 => return avsnitt,
        1 => format!(
            "Du har én Claude åpen i en vanlig terminal, utenfor tmux, og den ser ikke nerdr. Vil du ha \
             den her, avslutter du den der den kjører, og fortsetter samtalen med {prefiks} r."
        ),
        n => format!(
            "Du har {n} Claude-er åpne i vanlige terminaler, utenfor tmux, og dem ser ikke nerdr. Vil du ha \
             en av dem her, avslutter du den der den kjører, og fortsetter samtalen med {prefiks} r."
        ),
    });
    avsnitt
}
