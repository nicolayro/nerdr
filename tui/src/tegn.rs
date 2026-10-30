//! Lista over paner til venstre og den valgte panen til høyre.

use nerdr_kjerne::merke::{Gruppe, Merke};
use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::sgr::{Farge, Linje, Stil};
use std::ops::Range;
use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect, Size};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};

use crate::bindinger::Bindinger;
use crate::bokser::{Boks, Rammer};
use crate::markering;
use crate::palett::{Rgb, bland};
use crate::grupper::{Del, Overskrift, Rad, glyf};
use crate::handlinger::Handlinger;
use crate::ny::Velger;
use crate::tekst::varighet;
use crate::utseende::{self, Ledig, Orden, Plass, Sti, Topp, Utseende};
use crate::{App, Fokus, Knapp, Mål, hjelp};

mod kort;

/// Bredden på lista, med skillelinja, til du drar i den.
pub const LISTE: u16 = 36;
/// Smalere enn dette blir lista til en stripe med bare tegnene.
pub const LISTE_MIN: u16 = 24;
/// Stripa er peker, glyf, mellomrom og skillelinja.
pub const STRIPE: u16 = 4;
/// Treffene i handlingene og velgeren får to tredjedeler av skjermen, men
/// aldri færre rader enn dette.
const MENY_RADER: u16 = 13;
const HANDLINGER_BREDDE: u16 = 56;
const VELGER_BREDDE: u16 = 100;
const UTSEENDE_BREDDE: u16 = 58;
/// Forklaringen nederst i innstillingene har alltid så mange linjer, så
/// menyen ikke endrer høyde når du går opp og ned.
const FORKLARING_LINJER: usize = 2;
/// Nøklene i innstillingene, med streken mot verdiene rett etter.
const NØKKEL_BREDDE: u16 = 12;

const ORANSJE_RGB: Rgb = (217, 119, 87);
const ORANSJE: Color = Color::Rgb(ORANSJE_RGB.0, ORANSJE_RGB.1, ORANSJE_RGB.2);
const GRØNN: Color = Color::Indexed(108);
/// Fargen 108 i 256-paletten, til å blande toner av.
const GRØNN_RGB: Rgb = (135, 175, 135);
const BLÅ: Color = Color::Indexed(110);
/// Fargen 110 i 256-paletten.
const BLÅ_RGB: Rgb = (135, 175, 215);
// Gråtonene i 256-paletten er faste og følger ikke fargetemaet, så teksten har
// standardfargen til terminalen og skiller seg bare med fet og svak.
const STERK: Style = Style::new().add_modifier(Modifier::BOLD);
const VANLIG: Style = Style::new();
const SVAK: Style = Style::new().add_modifier(Modifier::DIM);

/// Så mye av forgrunnen som blandes inn i bakgrunnen der noe er valgt, og der
/// musen står.
const MARKERT_ANDEL: f32 = 0.12;
const PEKT_ANDEL: f32 = 0.18;
/// Rammene rundt panene, og streken mellom fanene og panene, som skal være
/// svakere enn rammen.
const RAMME_ANDEL: f32 = 0.35;
const AKTIV_RAMME_ANDEL: f32 = 0.6;
const SKILLE_ANDEL: f32 = 0.2;
const INNE_ANDEL: f32 = 0.05;
const UNDER_MUSEN_ANDEL: f32 = 0.06;
const OVERSKRIFT_ANDEL: f32 = 0.08;

/// Forgrunns- og bakgrunnsfargen til terminalen.
static FARGER: OnceLock<Option<(Rgb, Rgb)>> = OnceLock::new();

pub fn bruk_farger(farger: Option<(Rgb, Rgb)>) {
    let _ = FARGER.set(farger);
}

pub fn farger() -> Option<(Rgb, Rgb)> {
    FARGER.get().copied().flatten()
}

/// `andel` av `farge`, eller av forgrunnen, blandet inn i bakgrunnen til
/// terminalen, så det passer til alle fargetemaer. `None` når terminalen ikke
/// har sagt hvilke farger den har.
fn tonet(farge: Option<Rgb>, andel: f32) -> Option<Color> {
    let (fg, bg) = farger()?;
    let (r, g, b) = bland(bg, farge.unwrap_or(fg), andel);
    Some(Color::Rgb(r, g, b))
}

/// Valgt kort og fane får litt av forgrunnen i bakgrunnen. Uten farger fra
/// terminalen står de uten bakgrunn, med peker og fet tekst.
fn markert() -> Style {
    tonet(None, MARKERT_ANDEL).map_or(Style::new(), |c| Style::new().bg(c))
}

/// Pekeren i lista skiller seg mye mer ut enn sesjonen du er inne i, så du
/// ser hvor du står når du blar, og hvor du kom fra.
fn pekt() -> Style {
    tonet(None, PEKT_ANDEL).map_or(Style::new(), |c| Style::new().bg(c)).add_modifier(Modifier::BOLD)
}

fn inne() -> Style {
    tonet(None, INNE_ANDEL).map_or(Style::new(), |c| Style::new().bg(c))
}

/// Uten farger fra terminalen er den aktive rammen vanlig og de andre svake.
fn rammestil(aktiv: bool) -> Style {
    let (andel, uten) = if aktiv { (AKTIV_RAMME_ANDEL, VANLIG) } else { (RAMME_ANDEL, SVAK) };
    tonet(None, andel).map_or(uten, |c| Style::new().fg(c))
}

fn skillestil() -> Style {
    tonet(None, SKILLE_ANDEL).map_or(SVAK, |c| Style::new().fg(c))
}

fn overskrift_bakgrunn() -> Style {
    tonet(None, OVERSKRIFT_ANDEL).map_or(Style::new(), |c| Style::new().bg(c))
}

fn under_musen() -> Style {
    tonet(None, UNDER_MUSEN_ANDEL).map_or(Style::new(), |c| Style::new().bg(c))
}

/// Fanene og streken under dem. Med bokser er streken toppkanten på boksene,
/// og fanene har en ramme over seg.
const FANERADER: u16 = 2;

/// Raden fanene står på.
pub fn fanerad(app: &App) -> u16 {
    (app.utseende.paner == Rammer::Bokser) as u16
}

/// Radene øverst i agentlista: versjonen og hjelp, og ny sesjon og
/// sorteringen, med streken under.
const TOPPEN: u16 = 2;
const NY_RAD: u16 = 1;

/// Kolonnene fanene har plass i, innenfor rammen med bokser.
fn fanerom(app: &App) -> Range<u16> {
    let v = app.visning;
    match app.utseende.paner {
        Rammer::Bokser => v.x + 2..v.right().saturating_sub(1),
        Rammer::Linjer => v.x + 1..v.right(),
    }
}

/// Til høyre for lista, under fanene. Lista er `liste` bred, men aldri mer
/// enn halve skjermen.
pub fn visning(størrelse: Size, liste: u16) -> Rect {
    let liste = liste.min(størrelse.width / 2).max(STRIPE.min(størrelse.width));
    Rect::new(liste, FANERADER, størrelse.width.saturating_sub(liste), størrelse.height.saturating_sub(FANERADER))
}

/// Bredden på lista er der visningen starter.
fn liste_bredde(app: &App) -> u16 {
    app.visning.x
}

fn stripe(app: &App) -> bool {
    liste_bredde(app) <= STRIPE
}

pub fn alt(f: &mut Frame, app: &App) {
    let område = f.area();
    let buf = f.buffer_mut();
    let bokser = app.bokser();
    let mus = app.under_musen();
    tegn_liste(buf, område, app, mus.as_ref());
    tegn_pane(buf, app, &bokser, mus.as_ref());
    tegn_faner(buf, app, mus.as_ref());
    tegn_hint(buf, område, app);

    if let Some(rull) = app.hjelp {
        tegn_hjelp(buf, område, &app.prefiks.navn(), &app.bindinger, rull);
    } else if let Some(lukke) = &app.lukke {
        tegn_lukke(buf, område, app, lukke, mus.as_ref());
    } else if let Some(akse) = app.utseende_meny {
        tegn_utseende(buf, område, &app.utseende, akse, mus.as_ref());
    } else if let Some(handlinger) = &app.handlinger {
        let markør = tegn_handlinger(buf, område, handlinger, mus.as_ref());
        f.set_cursor_position(markør);
    } else if let Some(velger) = &app.velger {
        let markør = tegn_velger(buf, område, velger, mus.as_ref());
        f.set_cursor_position(markør);
    } else if let (Fokus::Pane, Some((x, y)), 0, Some(boks)) = (app.fokus, app.markør, app.rull, aktiv_boks(app, &bokser)) {
        let i = boks.innhold;
        if x < i.width && y < i.height {
            f.set_cursor_position((i.x + x, i.y + y));
        }
    }
}

fn tegn_liste(buf: &mut Buffer, område: Rect, app: &App, mus: Option<&Mål>) {
    let bredde = liste_bredde(app) - 1;

    let mut overskrifter = Vec::new();
    let (rader, utsnitt) = liste(app);
    let vis = utsnitt.rader();
    let på_skjermen = |y: u16| u16::try_from(utsnitt.skjerm(y)).ok().filter(|y| vis.contains(y));
    let rad_bakgrunn = (på_egen_rad(app) && app.utseende.topp == Topp::Rad).then(overskrift_bakgrunn);
    let med_strek = på_egen_rad(app) && app.utseende.topp != Topp::Rad;
    for Plassert { y, rad, kort, .. } in &rader {
        match rad {
            Rad::Tom => {}
            Rad::Overskrift(_) if kort.is_some() => {
                let kort = kort.as_deref().unwrap_or_default();
                tegn_kort(buf, utsnitt.skjerm(*y), &vis, bredde, kort, rad_bakgrunn);
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
                    _ => rad_bakgrunn,
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
                    _ if matches!(mus, Some(Mål::Kort(id) | Mål::Lukk(id)) if *id == pane.id) => Some(under_musen()),
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
    for (knapp, x, y) in knapper(app) {
        let stil = if mus == Some(&Mål::Knapp(knapp)) { under_musen() } else { Style::new() };
        if stripe(app) {
            buf.set_string(0, y, " + ", pluss.patch(stil));
            continue;
        }
        let Some(&(_, navn, _)) = KNAPPER.iter().find(|k| k.0 == knapp) else { continue };
        buf.set_style(Rect::new(x.start, y, x.end - x.start, 1), stil);
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
        let stil = if mus == Some(&Mål::Sortering) { under_musen().patch(VANLIG) } else { SVAK };
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
            let stil = match (venter, app.utseende.farge) {
                (false, _) => SVAK,
                (true, utseende::Farge::På) => Style::new().fg(ORANSJE),
                (true, utseende::Farge::Av) => STERK,
            };
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
fn sortering(app: &App) -> Option<(Range<u16>, String)> {
    if stripe(app) {
        return None;
    }
    let bredde = utseende::SORTERINGER.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let tekst = format!(" {:>bredde$} ", utseende::SORTERINGER[app.utseende.indeks(utseende::ORDEN)]);
    let slutt = liste_bredde(app) - 1;
    let start = slutt.checked_sub(tekst.chars().count() as u16).filter(|&x| x >= NY_BREDDE)?;
    Some((start..slutt, tekst))
}

pub fn sortering_på(app: &App, x: u16, y: u16) -> bool {
    y == NY_RAD && sortering(app).is_some_and(|(r, _)| r.contains(&x))
}

/// Når du har trykket prefikset, eller er i kopimodus. Det står nederst, så
/// toppen ikke endrer seg.
fn hint(app: &App) -> Option<String> {
    match () {
        _ if app.etter_prefiks => Some(format!(" {} … f ny · w agentlista · ? mer", app.prefiks.navn())),
        _ if app.kopimodus.is_some() => Some(" v marker · y kopier · q ut".to_string()),
        _ => None,
    }
}

/// Fra stripa går hintet inn over panen, og derfor tegnes det etter den.
fn tegn_hint(buf: &mut Buffer, område: Rect, app: &App) {
    let Some(hint) = hint(app) else { return };
    let bredde = if stripe(app) { område.width } else { liste_bredde(app) - 1 };
    let b = bredde as usize;
    buf.set_stringn(0, område.height.saturating_sub(1), format!("{hint:<b$}"), b, VANLIG);
}

/// Prosjektet står på en rad for seg over kortene.
fn på_egen_rad(app: &App) -> bool {
    let u = app.utseende;
    u.orden == Orden::Mappe && !stripe(app)
}

fn er_valgt(app: &App, pane: &Pane) -> bool {
    app.valgt.as_ref().is_some_and(|v| v.id == pane.id)
}

/// En Claude i lista: oppgaven som tittel, med hvor lenge den har ventet,
/// jobbet eller vært ferdig, og repoet og branchen. Uten oppgave er repoet
/// tittelen.
fn kort(app: &App, pane: &Pane, merke: Merke) -> Vec<kort::Linje> {
    let repo = app.repoer.get(&pane.id);
    // En som starter, er ikke i registeret ennå, og har ingen tid.
    let tid = app.claude.get(&pane.id).map(|c| varighet(crate::nå_ms().saturating_sub(c.endret)));
    let info = kort::Info {
        merke,
        oppgave: app.rester.get(&pane.id).map(|p| format!("uten Claude: {p}")).or_else(|| pane.oppgave().map(str::to_string)),
        sted: app.sted(pane),
        branch: repo.and_then(|r| r.branch.clone()),
        tid,
        starter: app.nye.contains_key(&pane.id),
        sted_over: app.utseende.orden == Orden::Mappe,
        undermappe: if app.utseende.sti == Sti::På { app.undermappe(pane) } else { None },
    };
    kort::kort(app.utseende, &info, liste_bredde(app) - 1, er_valgt(app, pane), stripe(app))
}

/// Kortet med første linje på rad `topp`, som kan være over skjermen når
/// lista ruller. Bare linjene på radene i `vis` tegnes.
fn tegn_kort(buf: &mut Buffer, topp: i32, vis: &Range<u16>, bredde: u16, kort: &[kort::Linje], bakgrunn: Option<Style>) {
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
struct Plassert<'a> {
    y: u16,
    høyde: u16,
    rad: Rad<'a>,
    kort: Option<Vec<kort::Linje>>,
}

/// Radene i lista, etter hverandre fra toppen.
fn plasser(app: &App) -> Vec<Plassert<'_>> {
    let mut y = 0;
    let midten = app.utseende.ledig == Ledig::Midten && app.utseende.orden == Orden::Nerdr;
    let u = app.utseende;
    let bredde = liste_bredde(app) - 1;
    let på_rad = på_egen_rad(app);
    kort::ordne(app.rader())
        .into_iter()
        .map(|rad| {
            if midten && rad == Rad::Overskrift(Overskrift::Del(Del::Ledige)) {
                y = y.max(plass(app) / 2);
            }
            let overskrift = matches!(rad, Rad::Overskrift(Overskrift::Del(_)));
            let kort = match &rad {
                Rad::Pane(pane, merke) => Some(kort(app, pane, *merke)),
                Rad::Overskrift(Overskrift::Mappe(navn)) if på_rad => Some(vec![kort::prosjektrad(u, navn, &[], bredde, false, false)]),
                Rad::Brettet(navn, paner) => {
                    let merker: Vec<Merke> = paner.iter().map(|(_, m)| *m).collect();
                    let valgt = paner.iter().any(|(p, _)| er_valgt(app, p));
                    Some(vec![match på_rad {
                        true => kort::prosjektrad(u, navn, &merker, bredde, true, valgt),
                        false => kort::brettet_stripe(u, &merker, valgt),
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
fn plass(app: &App) -> u16 {
    app.visning.bottom().saturating_sub(TOPPEN + 1)
}

fn liste(app: &App) -> (Vec<Plassert<'_>>, Utsnitt) {
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
    fn skjerm(&self, y: u16) -> i32 {
        y as i32 - self.rull as i32 + self.ruller as i32 + self.topp as i32
    }

    /// Radene på skjermen kortene står på.
    fn rader(&self) -> Range<u16> {
        let topp = self.topp + self.ruller as u16;
        topp..topp + self.rom
    }

    fn i_lista(&self, y: u16) -> Option<u16> {
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

/// Panen som står på rad `y` i lista.
pub fn pane_på(app: &App, y: u16) -> Option<Pane> {
    let (rader, utsnitt) = liste(app);
    let y = utsnitt.i_lista(y)?;
    rader.into_iter().find_map(|p| match p.rad {
        Rad::Pane(pane, _) if (p.y..p.y + p.høyde).contains(&y) => Some(pane.clone()),
        _ => None,
    })
}

/// Prosjektet med overskriften, eller som er brettet sammen, på rad `y`.
pub fn prosjekt_på(app: &App, y: u16) -> Option<String> {
    let (rader, utsnitt) = liste(app);
    let y = utsnitt.i_lista(y)?;
    rader.into_iter().find_map(|p| match p.rad {
        Rad::Overskrift(Overskrift::Mappe(navn)) | Rad::Brettet(navn, _) if p.y == y => Some(navn),
        _ => None,
    })
}

/// Kolonnen der `×` står på kortet under musen: midt der tiden står, som
/// oftest er tre tegn, som `<1m`.
fn lukk_x(app: &App) -> Option<u16> {
    let bredde = liste_bredde(app) - 1;
    match app.utseende.plass {
        _ if stripe(app) => None,
        Plass::Minimal => bredde.checked_sub(3),
        _ => bredde.checked_sub(6),
    }
}

/// `×` i stedet for tiden, på den samme plassen, så ingenting rundt flytter
/// seg når musen kommer. I boksen tas luften rundt tiden med.
fn tegn_lukk(buf: &mut Buffer, app: &App, x: u16, y: u16, pekt: bool) {
    let stil = if pekt { Style::new().fg(ORANSJE).add_modifier(Modifier::BOLD) } else { STERK };
    let luft = if app.utseende.plass == Plass::Minimal { "   " } else { "     " };
    let bredde = luft.len() as u16;
    buf.set_string(x.saturating_sub(bredde / 2), y, luft, Style::new());
    buf.set_string(x, y, "×", stil);
}

/// Panen til kortet der `×` står på `x` og `y`. Ruta på hver side treffer
/// også, så den er lettere å treffe.
pub fn lukk_på(app: &App, x: u16, y: u16) -> Option<Pane> {
    if lukk_x(app).is_none_or(|l| l.abs_diff(x) > 1) {
        return None;
    }
    let (rader, utsnitt) = liste(app);
    let y = utsnitt.i_lista(y)?;
    rader.into_iter().find_map(|p| match p.rad {
        Rad::Pane(pane, _) if p.y == y => Some(pane.clone()),
        _ => None,
    })
}

/// Opp eller ned når rad `y` er en av pilene som viser at lista ruller.
pub fn pil_på(app: &App, y: u16) -> Option<bool> {
    let (_, utsnitt) = liste(app);
    let vis = utsnitt.rader();
    match () {
        _ if !utsnitt.ruller => None,
        _ if y + 1 == vis.start => Some(true),
        _ if y == vis.end => Some(false),
        _ => None,
    }
}

/// Et vindu i sesjonen, som fane over panene.
struct Fane {
    fra: u16,
    til: u16,
    navn: String,
    vindu: String,
    vist: bool,
    ikon: Ikon,
}

/// Tegnet foran navnet på fanen, og kolonnene til navnet.
struct Ikon {
    tegn: String,
    stil: Style,
    bredde: u16,
}

/// Luft til høyre for navnet, med `×` i.
const FANELUFT: u16 = 3;
/// Tegnet foran navnet: merket til Claude-en, eller den som haster mest når
/// det er flere i vinduet, og ellers nummeret på vinduet. Glyfene har
/// tvetydig bredde i Unicode, og mange fonter tegner dem bredere enn
/// sifrene, så de får et mellomrom ekstra.
fn ikon(app: &App, p: &Pane) -> Ikon {
    let haster = app
        .alle
        .iter()
        .filter(|q| q.vindu_id == p.vindu_id)
        .filter_map(|q| app.merke(q))
        .min_by_key(|m| (m.gruppe(), *m != Merke::Spør));
    let (tegn, stil) = match haster {
        Some(m) => (glyf(m).to_string(), kort::glyfstil(app.utseende, m)),
        None => (p.vindu.clone(), SVAK),
    };
    let bredde = tegn.chars().count() as u16 + 1 + haster.is_some() as u16;
    Ikon { tegn, stil, bredde }
}

impl Fane {
    /// Kolonnen til `×`. Plassen står i fanen også når `×` ikke vises, så
    /// fanene ikke endrer bredde når musen kommer.
    fn lukk(&self) -> u16 {
        self.til - 2
    }
}

fn faner(app: &App) -> Vec<Fane> {
    let mut x = fanerom(app).start;
    let mut ut = Vec::new();
    for p in app.faner() {
        let navn = kutt(&p.vindunavn, 20);
        let ikon = ikon(app, p);
        let til = x + 1 + ikon.bredde + navn.chars().count() as u16 + FANELUFT;
        let vist = app.vist.as_ref() == Some(&p.vindu_id);
        ut.push(Fane { fra: x, til, navn, vindu: p.vindu_id.clone(), vist, ikon });
        x = til;
    }
    ut
}

/// Vinduene i sesjonen som faner med en strek under, og med bokser i en ramme
/// over panene, der streken er toppkanten på boksene. Fanen som vises, er en
/// lapp med bakgrunn, og den og fanen under musen har `×`. Tegnes etter
/// panene, fordi streken går inn i toppkanten på boksene.
fn tegn_faner(buf: &mut Buffer, app: &App, mus: Option<&Mål>) {
    let v = app.visning;
    let (rom, rad) = (fanerom_til_fanene(app), fanerad(app));
    let boks = app.utseende.paner == Rammer::Bokser;
    let faner: Vec<Fane> = faner(app).into_iter().take_while(|f| f.til < rom.end).collect();
    let y = v.y - 1 + boks as u16;
    // Rammen over fanene har samme farge som boksen under.
    let ramme = if boks && buf[(v.x, y)].symbol() == "╭" { buf[(v.x, y)].style() } else { SVAK };
    // Ved siden av rammen over fanene står skillelinja for seg.
    let fra = if boks { v.x } else { v.x.saturating_sub(1) };
    for x in fra..v.right() {
        let rute = &buf[(x, y)];
        let (symbol, stil) = (rute.symbol().to_string(), rute.style());
        let skillelinja = x + 1 == v.x;
        // Merkelappen på rammen, som « kopi », står.
        if !skillelinja && (!matches!(symbol.as_str(), " " | "─" | "╭" | "╮" | "┬") || stil.add_modifier.contains(Modifier::REVERSED)) {
            continue;
        }
        let fra_boks = matches!(symbol.as_str(), "╭" | "╮" | "┬");
        let kant = boks && (x == v.x || x + 1 == v.right());
        let (opp, ned) = if skillelinja { (true, true) } else { (kant, fra_boks) };
        let venstre = match () {
            _ if skillelinja => matches!(symbol.as_str(), "┤" | "┼"),
            _ => !(boks && x == v.x),
        };
        let høyre = x + 1 < v.right();
        let tegn = match (opp, ned, venstre, høyre) {
            (false, true, false, true) => "╭",
            (false, true, true, false) => "╮",
            (o, n, v, h) => rammetegn(o, n, v, h),
        };
        // Der en loddrett kant går gjennom streken, har den fargen til rammen,
        // ellers ser kanten brutt ut.
        let stil = match () {
            _ if skillelinja => stil,
            _ if fra_boks => stil,
            _ if kant => ramme,
            _ if boks => skillestil(),
            _ => SVAK,
        };
        buf[(x, y)].set_symbol(tegn).set_style(stil);
    }
    if boks && v.width >= 2 {
        buf.set_string(v.x, 0, format!("╭{}╮", "─".repeat(v.width as usize - 2)), ramme);
        buf.set_string(v.x, rad, "│", ramme);
        buf.set_string(v.right() - 1, rad, "│", ramme);
    }
    for f in &faner {
        let på_kryss = matches!(mus, Some(Mål::LukkFane(vindu)) if *vindu == f.vindu);
        let pekt = på_kryss || matches!(mus, Some(Mål::Fane(vindu)) if *vindu == f.vindu);
        let (tekst, lapp) = match () {
            _ if f.vist => (STERK, markert()),
            _ if pekt => (VANLIG, under_musen()),
            _ => (SVAK, Style::new()),
        };
        buf.set_style(Rect::new(f.fra, rad, f.til - f.fra, 1), lapp);
        buf.set_string(f.fra + 1, rad, &f.ikon.tegn, lapp.patch(f.ikon.stil));
        buf.set_string(f.fra + 1 + f.ikon.bredde, rad, &f.navn, lapp.patch(tekst));
        if pekt || f.vist {
            let kryss = match på_kryss {
                true => Style::new().fg(ORANSJE).add_modifier(Modifier::BOLD),
                false => SVAK,
            };
            buf.set_string(f.lukk(), rad, "×", lapp.patch(kryss));
        }
    }
    if let Some(pluss) = ny_fane(app) {
        let stil = if mus == Some(&Mål::NyFane) { VANLIG } else { SVAK };
        buf.set_string(pluss.start, rad, " + ", stil);
    }
    let lukk = lukk_nerdr(app);
    let stil = if mus == Some(&Mål::Knapp(Knapp::Avslutt)) { under_musen().patch(VANLIG) } else { SVAK };
    buf.set_string(lukk.start, rad, LUKK_NERDR, stil);
}

/// Vinduet til fanen i kolonne `x`, med luften etter den.
pub fn fane_på(app: &App, x: u16) -> Option<String> {
    faner(app).into_iter().find(|f| (f.fra..=f.til).contains(&x)).map(|f| f.vindu)
}

/// Vinduet til fanen med `×` i kolonne `x`. Rutene på hver side treffer
/// også, så den er lettere å treffe.
pub fn lukk_fane_på(app: &App, x: u16) -> Option<String> {
    faner(app).into_iter().find(|f| f.lukk().abs_diff(x) <= 1).map(|f| f.vindu)
}

const LUKK_NERDR: &str = "lukk nerdr";

/// Høyrestilt på faneraden.
fn lukk_nerdr(app: &App) -> Range<u16> {
    let slutt = fanerom(app).end.saturating_sub(1);
    slutt.saturating_sub(LUKK_NERDR.len() as u16)..slutt
}

pub fn lukk_nerdr_på(app: &App, x: u16) -> bool {
    lukk_nerdr(app).contains(&x)
}

/// Fanene, og `+` etter dem, står fram til lukk nerdr.
fn fanerom_til_fanene(app: &App) -> Range<u16> {
    fanerom(app).start..lukk_nerdr(app).start.saturating_sub(1)
}

/// `+` etter fanene, når et vindu vises og det er plass til den.
fn ny_fane(app: &App) -> Option<Range<u16>> {
    app.valgt.as_ref()?;
    let rom = fanerom_til_fanene(app);
    let fra = faner(app).last().map_or(rom.start, |f| f.til);
    (fra + 3 <= rom.end).then_some(fra..fra + 3)
}

pub fn ny_fane_på(app: &App, x: u16) -> bool {
    ny_fane(app).is_some_and(|r| r.contains(&x))
}

/// Knappene øverst i agentlista, med tasten etter prefikset som gjør det
/// samme.
const KNAPPER: [(Knapp, &str, char); 3] = [
    (Knapp::Ny, "ny sesjon", 'f'),
    (Knapp::Hjelp, "? hjelp", '?'),
    (Knapp::Avslutt, LUKK_NERDR, 'd'),
];

/// Et klikk på knappen er det samme som prefikset og denne tasten.
pub fn tasten_til(knapp: Knapp) -> char {
    KNAPPER.iter().find(|k| k.0 == knapp).map_or('?', |k| k.2)
}
/// Plussen og «ny sesjon», med luft til sorteringen.
const NY_BREDDE: u16 = 14;

/// Der sorteringen og hjelp står, høyrestilt.
fn høyre_kolonne(app: &App) -> u16 {
    sortering(app).map_or(liste_bredde(app) - 1, |(x, _)| x.start)
}

/// Knappene i agentlista, med kolonnene og raden de står på.
fn knapper(app: &App) -> Vec<(Knapp, Range<u16>, u16)> {
    if stripe(app) {
        return vec![(Knapp::Ny, 0..3, NY_RAD)];
    }
    let (midt, slutt) = (høyre_kolonne(app), liste_bredde(app) - 1);
    let mut ut = vec![(Knapp::Ny, 0..midt, NY_RAD)];
    if midt < slutt {
        ut.push((Knapp::Hjelp, midt..slutt, 0));
    }
    ut
}

pub fn knapp_på(app: &App, x: u16, y: u16) -> Option<Knapp> {
    knapper(app).into_iter().find(|(_, r, rad)| *rad == y && r.contains(&x)).map(|(k, ..)| k)
}

const LUKKE_BREDDE: u16 = 46;

fn skjerm(app: &App) -> Rect {
    let v = app.visning;
    Rect::new(0, 0, v.right(), v.bottom())
}

/// Linjene i popupen som spør, over knappene.
fn lukke_tekst(lukke: &crate::Lukke) -> Vec<String> {
    let indre = (LUKKE_BREDDE - 4) as usize;
    bryt(&lukke.hva, indre, indre, 3)
}

fn lukke_boks(app: &App, område: Rect) -> Option<Rect> {
    let lukke = app.lukke.as_ref()?;
    midt_på(område, LUKKE_BREDDE, lukke_tekst(lukke).len() as u16 + 4)
}

/// Knappene på nederste rad inni popupen, med `true` for «lukk», og tasten
/// som gjør det samme.
const SVAR: [(bool, &str, &str); 2] = [(true, " lukk", "y "), (false, " avbryt", "n ")];

fn svar(app: &App) -> Vec<(bool, Range<u16>, u16)> {
    let Some(boks) = lukke_boks(app, skjerm(app)) else { return Vec::new() };
    let (mut x, y) = (boks.x + 2, boks.bottom() - 2);
    let mut ut = Vec::new();
    for (lukk, navn, tast) in SVAR {
        let bredde = (navn.chars().count() + 2 + tast.len()) as u16;
        ut.push((lukk, x..x + bredde, y));
        x += bredde + 2;
    }
    ut
}

pub fn svar_på(app: &App, x: u16, y: u16) -> Option<bool> {
    svar(app).into_iter().find(|(_, r, rad)| *rad == y && r.contains(&x)).map(|(lukk, ..)| lukk)
}

pub fn i_popupen(app: &App, x: u16, y: u16) -> bool {
    lukke_boks(app, skjerm(app)).is_some_and(|r| r.contains((x, y).into()))
}

fn tegn_lukke(buf: &mut Buffer, område: Rect, app: &App, lukke: &crate::Lukke, mus: Option<&Mål>) {
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
fn midt_på(område: Rect, bredde: u16, høyde: u16) -> Option<Rect> {
    let bredde = område.width.saturating_sub(4).min(bredde);
    let høyde = område.height.saturating_sub(2).min(høyde);
    if bredde < 12 || høyde < 5 {
        return None;
    }
    Some(Rect::new((område.width - bredde) / 2, (område.height - høyde) / 3, bredde, høyde))
}

/// En boks med ramme midt på skjermen, med tittel øverst og hint nederst.
fn boks(buf: &mut Buffer, område: Rect, bredde: u16, høyde: u16, tittel: &str, hint: &str) -> Option<Rect> {
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

const HJELP_BREDDE: u16 = 52;

fn hjelp_linjer(prefiks: &str, bindinger: &Bindinger) -> Vec<(String, Style)> {
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
fn hjelp_boks(område: Rect, linjer: usize) -> Option<(Rect, u16)> {
    let boks = midt_på(område, HJELP_BREDDE, linjer as u16 + 2)?;
    Some((boks, (linjer as u16).saturating_sub(boks.height - 2)))
}

pub fn hjelp_rull_maks(app: &App) -> u16 {
    let linjer = hjelp_linjer(&app.prefiks.navn(), &app.bindinger).len();
    hjelp_boks(skjerm(app), linjer).map_or(0, |(_, maks)| maks)
}

fn tegn_hjelp(buf: &mut Buffer, område: Rect, prefiks: &str, bindinger: &Bindinger, rull: u16) {
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
struct Valgmeny {
    boks: Rect,
    overskrifter: Vec<(u16, &'static str)>,
    /// Aksen, raden og kolonnene til verdiene.
    rader: Vec<(usize, u16, Vec<Range<u16>>)>,
    forklaring: u16,
}

fn valgmeny(område: Rect, u: &Utseende) -> Option<Valgmeny> {
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
            let kolonner = Utseende::i_menyen(akse)
                .1
                .iter()
                .map(|verdi| {
                    let r = vx..vx + verdi.chars().count() as u16;
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
pub fn verdi_på(app: &App, x: u16, y: u16) -> Option<(usize, usize)> {
    let meny = valgmeny(skjerm(app), &app.utseende)?;
    let (akse, _, kolonner) = meny.rader.into_iter().find(|r| r.1 == y)?;
    kolonner.iter().position(|r| r.contains(&x)).map(|i| (akse, i))
}

pub fn i_valgmenyen(app: &App, x: u16, y: u16) -> bool {
    valgmeny(skjerm(app), &app.utseende).is_some_and(|m| m.boks.contains((x, y).into()))
}

/// Konfigurasjonen midt på skjermen, med aksen `valgt` markert. Valgene som
/// ikke gjelder, er borte.
fn tegn_utseende(buf: &mut Buffer, område: Rect, u: &Utseende, valgt: usize, mus: Option<&Mål>) {
    let Some(meny) = valgmeny(område, u) else { return };
    let (b, h) = (meny.boks.width, meny.boks.height);
    let Some(ramme) = boks(buf, område, b, h, "innstillinger", "↑↓ velg · ←→ endre · klikk · esc lukk") else { return };
    let (x, bunn) = (ramme.x + 2, ramme.bottom() - 1);
    let indre = (ramme.width - 4) as usize;
    for (y, tittel) in meny.overskrifter.into_iter().filter(|(y, _)| *y < bunn) {
        buf.set_string(x, y, tittel, STERK);
    }
    for (akse, y, kolonner) in meny.rader.into_iter().filter(|r| r.1 < bunn) {
        let (navn, verdier) = Utseende::i_menyen(akse);
        if akse == valgt {
            buf.set_style(Rect::new(ramme.x + 1, y, ramme.width - 2, 1), markert());
            buf.set_string(ramme.x + 1, y, "▎", VANLIG);
        }
        buf.set_string(x, y, navn, if akse == valgt { STERK } else { SVAK });
        buf.set_string(x + NØKKEL_BREDDE, y, "│", SVAK);
        for (i, (r, verdi)) in kolonner.into_iter().zip(verdier).enumerate() {
            let stil = match () {
                _ if i == u.indeks(akse) => Style::new().fg(ORANSJE).add_modifier(Modifier::BOLD),
                _ if mus == Some(&Mål::Verdi(akse, i)) => under_musen().patch(VANLIG),
                _ => VANLIG,
            };
            buf.set_stringn(r.start, y, *verdi, ramme.right().saturating_sub(r.start + 1) as usize, stil);
        }
    }
    let forklaring = bryt(utseende::FORKLARING[valgt], indre, indre, FORKLARING_LINJER);
    for (linje, y) in forklaring.into_iter().zip(meny.forklaring..bunn) {
        buf.set_string(x, y, linje, SVAK);
    }
}

/// Rammen, søkefeltet og streken under det.
const OVER_TREFFENE: u16 = 3;

/// Boksen til handlingene eller velgeren, og hvor mange treff den har plass til.
fn menyboks(område: Rect, bredde: u16) -> Option<(Rect, usize)> {
    let rader = (område.height * 2 / 3).saturating_sub(OVER_TREFFENE + 1).max(MENY_RADER);
    let boks = midt_på(område, bredde, rader + OVER_TREFFENE + 1)?;
    Some((boks, boks.height.saturating_sub(OVER_TREFFENE + 1) as usize))
}

fn søkestrek(buf: &mut Buffer, boks: Rect) {
    let strek = format!("├{}┤", "─".repeat(boks.width as usize - 2));
    buf.set_string(boks.x, boks.y + 2, strek, Style::new().fg(ORANSJE));
}

pub fn menyrader(app: &App) -> usize {
    menyboks(skjerm(app), HANDLINGER_BREDDE).map_or(1, |(_, rader)| rader.max(1))
}

/// Rullet så `valgt` vises, og uten tomme rader nederst.
pub fn følg(valgt: usize, rull: usize, rader: usize, antall: usize) -> usize {
    let rull = if valgt < rull { valgt } else if valgt >= rull + rader { valgt + 1 - rader } else { rull };
    rull.min(antall.saturating_sub(rader))
}

/// Boksen og treffene til handlingene eller velgeren, den som er åpen, med
/// hvor langt de er rullet og hvor mange det er.
fn åpen_meny(app: &App) -> Option<(Rect, usize, usize, usize)> {
    let (bredde, rull, antall) = match (&app.handlinger, &app.velger) {
        (Some(h), _) => (HANDLINGER_BREDDE, h.rull, h.treff().len()),
        (None, Some(v)) => (VELGER_BREDDE, v.rull, v.antall_treff()),
        _ => return None,
    };
    let (boks, rader) = menyboks(skjerm(app), bredde)?;
    Some((boks, rader, rull, antall))
}

/// Treffet under musen i handlingene eller velgeren.
pub fn meny_rad_på(app: &App, x: u16, y: u16) -> Option<usize> {
    let (boks, rader, rull, antall) = åpen_meny(app)?;
    if !(boks.x + 1..boks.right() - 1).contains(&x) {
        return None;
    }
    let rad = y.checked_sub(boks.y + OVER_TREFFENE)? as usize;
    (rad < rader && rull + rad < antall).then_some(rull + rad)
}

pub fn i_menyboks(app: &App, x: u16, y: u16) -> bool {
    åpen_meny(app).is_some_and(|(boks, ..)| boks.contains((x, y).into()))
}

/// Pilene i rammen når det er flere treff over eller under.
fn menypiler(buf: &mut Buffer, boks: Rect, fra: usize, rader: usize, antall: usize) {
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
fn menyrad(buf: &mut Buffer, boks: Rect, y: u16, valgt: bool, pekt: bool) {
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
fn tegn_handlinger(buf: &mut Buffer, område: Rect, handlinger: &Handlinger, mus: Option<&Mål>) -> (u16, u16) {
    let Some((plass, linjer)) = menyboks(område, HANDLINGER_BREDDE) else { return (0, 0) };
    let Some(boks) = boks(buf, område, plass.width, plass.height, "handlinger", "⏎ kjør · klikk · esc lukk") else {
        return (0, 0);
    };
    søkestrek(buf, boks);
    let indre = (boks.width - 4) as usize;
    let treff = handlinger.treff();
    let (x, y) = (boks.x + 2, boks.y + 1);
    let etter = buf.set_stringn(x, y, "› ", indre, Style::new().fg(ORANSJE)).0;
    let markør = buf.set_stringn(etter, y, kutt(&handlinger.søk, indre.saturating_sub(10)), indre, VANLIG).0;
    let antall = format!("{}/{}", treff.len(), handlinger.antall());
    buf.set_string(boks.right() - 2 - antall.chars().count() as u16, y, antall, SVAK);

    let fra = handlinger.rull;
    for (i, (k, tegn)) in treff.iter().enumerate().skip(fra).take(linjer) {
        let y = boks.y + OVER_TREFFENE + (i - fra) as u16;
        let valgt = i == handlinger.valgt;
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
fn tegn_velger(buf: &mut Buffer, område: Rect, velger: &Velger, mus: Option<&Mål>) -> (u16, u16) {
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

    let fra = velger.rull;
    for (i, t) in treff.iter().enumerate().skip(fra).take(linjer) {
        let y = boks.y + OVER_TREFFENE + (i - fra) as u16;
        let valgt = i == velger.valgt;
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

/// Det som står i stedet for panen når lista er tom, skrevet for den som
/// ikke kjenner tmux.
fn tom_liste(utenfor_tmux: usize, prefiks: &str) -> Vec<String> {
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

fn tegn_pane(buf: &mut Buffer, app: &App, bokser: &[Boks], mus: Option<&Mål>) {
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
        let topp = app.historie as i64 - app.rull as i64;
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
        if let Some(kopi) = app.kopimodus.as_ref().filter(|k| k.pane == boks.pane) {
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
    if app.utseende.paner == Rammer::Linjer && bokser.len() > 1 {
        tegn_linjer(buf, app, bokser, &lyse);
    }
    let merkelapp = match () {
        _ if app.kopiert.is_some() => Some(" kopiert ".to_string()),
        _ if app.kopimodus.is_some() && app.rull > 0 => Some(format!(" kopi {}/{} ", app.rull, app.historie)),
        _ if app.kopimodus.is_some() => Some(" kopi ".to_string()),
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

/// Rammene rundt boksene. Naboer deler kanten, så tegnet i hver rute kommer
/// av hvilke kanter som møtes der. En kant som hører til en lys boks, er lys.
fn tegn_rammer(buf: &mut Buffer, bokser: &[Boks], lyse: &[&Boks]) {
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
        assert_eq!(visning(Size::new(120, 40), 50).x, 50);
        assert_eq!(visning(Size::new(80, 40), 50).x, 40);
        assert_eq!(visning(Size::new(80, 40), STRIPE).x, STRIPE);
        assert_eq!(visning(Size::new(6, 40), LISTE).x, 4);
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
